//! Versioned LibreToon project container. No interchange preview is required.
use crate::{Document, fla_zip::Archive};
const DOCUMENT: &str = "LIBRETOON/document.json";
const MANIFEST: &str = "LIBRETOON/manifest.json";
pub const EXTENSION: &str = "ltoon";
pub const LIMIT: usize = 256 * 1024 * 1024;
const MANIFEST_V1: &[u8] = br#"{"format":"LibreToon","version":1}"#;

#[derive(serde::Deserialize)]
struct Manifest {
    format: String,
    version: u32,
}

pub fn is_native_path(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(EXTENSION))
}

pub fn encode(doc: &Document) -> Result<Vec<u8>, String> {
    let mut snapshot = doc.clone();
    crate::raster::prune(&mut snapshot);
    snapshot.validate()?;
    pack(&snapshot, vec![(MANIFEST, MANIFEST_V1)])
}

pub(crate) fn pack(doc: &Document, header: Vec<(&str, &[u8])>) -> Result<Vec<u8>, String> {
    let mut metadata = doc.clone();
    for asset in &mut metadata.extras.media.assets {
        asset.data = Default::default();
    }
    for reference in &mut metadata.extras.references {
        reference.data = Default::default();
    }
    let native = serde_json::to_vec(&metadata).map_err(|e| e.to_string())?;
    if native.len() > 64 * 1024 * 1024 {
        return Err("Proyecto demasiado grande para el contenedor de proyecto".into());
    }
    let names: Vec<_> = doc
        .extras
        .media
        .assets
        .iter()
        .map(|asset| media_entry(asset.id))
        .collect();
    let mut entries = Vec::new();
    entries.extend(header);
    entries.push((DOCUMENT, native.as_slice()));
    entries.extend(
        names
            .iter()
            .zip(doc.extras.media.assets.iter())
            .map(|(name, asset)| (name.as_str(), asset.data.as_slice())),
    );
    let reference_names: Vec<_> = doc
        .extras
        .references
        .iter()
        .filter(|r| !r.data.is_empty())
        .map(|r| format!("LIBRETOON/references/{}.image", r.id))
        .collect();
    entries.extend(
        reference_names
            .iter()
            .zip(doc.extras.references.iter().filter(|r| !r.data.is_empty()))
            .map(|(name, r)| (name.as_str(), r.data.as_slice())),
    );
    let size = entries.iter().try_fold(22usize, |sum, (name, data)| {
        sum.checked_add(76 + name.len() * 2)?
            .checked_add(data.len())
    });
    if entries.len() > u16::MAX as usize || size.is_none_or(|size| size > LIMIT) {
        return Err("El proyecto supera el límite de 256 MiB".into());
    }
    Ok(crate::fla_zip::pack_slices(&entries))
}

fn media_entry(id: u64) -> String {
    format!("LIBRETOON/media/{id}")
}

pub fn decode_if_native(bytes: &[u8]) -> Result<Option<Document>, String> {
    let archive = Archive::new(bytes)?;
    let Some(manifest) = archive.entry_limited(MANIFEST.as_bytes(), 4096)? else {
        return Ok(None);
    };
    if archive
        .names()
        .any(|name| name.eq_ignore_ascii_case(b"DOMDocument.xml"))
    {
        return Err("A native LibreToon project cannot contain an XFL preview".into());
    }
    let manifest: Manifest =
        serde_json::from_slice(&manifest).map_err(|error| error.to_string())?;
    if manifest.format != "LibreToon" || manifest.version != 1 {
        return Err(format!(
            "Unsupported LibreToon project format or version ({})",
            manifest.version
        ));
    }
    let document = archive
        .entry(DOCUMENT.as_bytes())?
        .ok_or("Missing LibreToon document")?;
    restore(&archive, &document).map(Some)
}

pub(crate) fn restore(archive: &Archive<'_>, bytes: &[u8]) -> Result<Document, String> {
    let mut doc: Document = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let mut budget = LIMIT.saturating_sub(bytes.len());
    let mut read = |path: &str, limit: usize| -> Result<crate::shared::SharedVec<u8>, String> {
        let data = archive
            .entry_limited(path.as_bytes(), budget.min(limit))?
            .ok_or_else(|| format!("Missing project resource: {path}"))?;
        budget -= data.len();
        Ok(data.into())
    };
    for asset in &mut doc.extras.media.assets {
        if asset.data.is_empty() {
            asset.data = read(&media_entry(asset.id), 64 * 1024 * 1024)?;
        }
    }
    for reference in &mut doc.extras.references {
        if reference.data.is_empty() && reference.dimensions != [0, 0] {
            reference.data = read(
                &format!("LIBRETOON/references/{}.image", reference.id),
                16 * 1024 * 1024,
            )?;
        }
    }
    doc.validate()?;
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_fla_migrates_to_native_without_xfl_preview() {
        let mut document = crate::fresh_document();
        document.layers[0].name = "Editable art".into();
        document.layers[0].frames[0].push(crate::Mark {
            nodes: vec![
                crate::document::Node::corner([10., 10.]),
                crate::document::Node {
                    position: [40., 20.],
                    in_handle: [30., 15.],
                    out_handle: [50., 15.],
                },
            ]
            .into(),
            ..Default::default()
        });
        let legacy = crate::fla::encode(&document).unwrap();
        let migrated = crate::fla::decode(&legacy).unwrap();
        let native = encode(&migrated).unwrap();
        let archive = Archive::new(&native).unwrap();
        assert!(archive.entry(b"DOMDocument.xml").unwrap().is_none());
        assert_eq!(
            archive.entry(MANIFEST.as_bytes()).unwrap().unwrap(),
            MANIFEST_V1
        );
        let restored = decode_if_native(&native).unwrap().unwrap();
        assert_eq!(
            serde_json::to_vec(&restored).unwrap(),
            serde_json::to_vec(&document).unwrap()
        );
    }

    #[test]
    fn native_open_refuses_unknown_versions_and_missing_document() {
        for manifest in [
            br#"{"format":"LibreToon","version":999}"#.as_slice(),
            MANIFEST_V1,
        ] {
            let bytes = crate::fla_zip::pack_slices(&[(MANIFEST, manifest)]);
            assert!(decode_if_native(&bytes).is_err());
        }
    }
}
