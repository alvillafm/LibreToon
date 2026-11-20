//! Import and export bounded preset collections.
use crate::design::{Icon, button};
use eframe::egui;
use serde::{Serialize, de::DeserializeOwned};
use std::{io::Read, path::Path};

const CAPACITY: usize = 64;
const FILE_LIMIT: u64 = 16 * 1024 * 1024;

fn read<T: DeserializeOwned>(path: &Path, valid: impl Fn(&T) -> bool) -> Result<Vec<T>, String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(FILE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > FILE_LIMIT {
        return Err("Preset file exceeds 16 MiB".into());
    }
    let values: Vec<T> = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if !values.iter().all(valid) {
        return Err("Invalid preset collection".into());
    }
    Ok(values)
}

pub(crate) fn exchange<T: Serialize + DeserializeOwned>(
    ui: &mut egui::Ui,
    presets: &mut Vec<T>,
    extension: &str,
    valid: impl Fn(&T) -> bool,
    status: &mut String,
) -> bool {
    let Some(values) = transfer(ui, presets, extension, CAPACITY, valid, status) else {
        return false;
    };
    let changed = !values.is_empty();
    presets.extend(values);
    changed
}

pub(crate) fn transfer<T: Serialize + DeserializeOwned>(
    ui: &mut egui::Ui,
    presets: &[T],
    extension: &str,
    capacity: usize,
    valid: impl Fn(&T) -> bool,
    status: &mut String,
) -> Option<Vec<T>> {
    let (import, export) = ui
        .horizontal_wrapped(|ui| {
            (
                button(ui, Icon::Open, "Abrir", "", false, presets.len() < capacity).clicked(),
                button(ui, Icon::Save, "Exportar", "", false, !presets.is_empty()).clicked(),
            )
        })
        .inner;
    let dialog = || rfd::FileDialog::new().add_filter("LibreToon", &[extension]);
    if import && let Some(path) = dialog().pick_file() {
        match read(&path, valid) {
            Ok(values) if values.len() <= capacity.saturating_sub(presets.len()) => {
                return Some(values);
            }
            Ok(_) => *status = "Preset collection exceeds the available slots".into(),
            Err(error) => *status = error,
        }
    }
    if export
        && let Some(mut path) = dialog()
            .set_file_name(format!("presets.{extension}"))
            .save_file()
    {
        if path.extension().is_none() {
            path.set_extension(extension);
        }
        let result = serde_json::to_vec(presets)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                if bytes.len() as u64 > FILE_LIMIT {
                    return Err("Preset file exceeds 16 MiB".into());
                }
                std::fs::write(path, bytes).map_err(|error| error.to_string())
            });
        if let Err(error) = result {
            *status = error;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_validated_before_publication() {
        let path = std::env::temp_dir().join(format!(
            "libretoon-preset-test-{}.json",
            crate::features::id()
        ));
        std::fs::write(&path, br#"[1,2,3]"#).unwrap();
        assert_eq!(read::<u32>(&path, |value| *value <= 3).unwrap(), [1, 2, 3]);
        assert!(read::<u32>(&path, |value| *value <= 2).is_err());
        std::fs::write(&path, br#"[1,2,"#).unwrap();
        assert!(read::<u32>(&path, |_| true).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
