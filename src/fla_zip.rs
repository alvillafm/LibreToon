//! Minimal ZIP container for modern FLA (XFL). No entries are extracted to disk.
use std::io::Read;
const LIMIT: usize = 64 * 1024 * 1024;
fn u16_at(b: &[u8], p: usize) -> Result<u16, String> {
    Ok(u16::from_le_bytes(
        b.get(p..p + 2).ok_or("ZIP truncado")?.try_into().unwrap(),
    ))
}
fn u32_at(b: &[u8], p: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4).ok_or("ZIP truncado")?.try_into().unwrap(),
    ))
}
fn push16(b: &mut Vec<u8>, v: u16) {
    b.extend(v.to_le_bytes());
}
fn push32(b: &mut Vec<u8>, v: u32) {
    b.extend(v.to_le_bytes());
}
pub fn pack(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = vec![];
    let mut central = vec![];
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32fast::hash(data);
        let size = data.len() as u32;
        push32(&mut out, 0x04034b50);
        for n in [20, 0x800, 0, 0, 33] {
            push16(&mut out, n);
        }
        push32(&mut out, crc);
        push32(&mut out, size);
        push32(&mut out, size);
        push16(&mut out, name.len() as u16);
        push16(&mut out, 0);
        out.extend(name.as_bytes());
        out.extend(data);
        push32(&mut central, 0x02014b50);
        for n in [20, 20, 0x800, 0, 0, 33] {
            push16(&mut central, n);
        }
        push32(&mut central, crc);
        push32(&mut central, size);
        push32(&mut central, size);
        push16(&mut central, name.len() as u16);
        for _ in 0..4 {
            push16(&mut central, 0);
        }
        push32(&mut central, 0);
        push32(&mut central, offset);
        central.extend(name.as_bytes());
    }
    let start = out.len() as u32;
    let size = central.len() as u32;
    out.extend(central);
    push32(&mut out, 0x06054b50);
    push16(&mut out, 0);
    push16(&mut out, 0);
    push16(&mut out, entries.len() as u16);
    push16(&mut out, entries.len() as u16);
    push32(&mut out, size);
    push32(&mut out, start);
    push16(&mut out, 0);
    out
}
pub fn document(bytes: &[u8]) -> Result<String, String> {
    xml_text(entry(bytes, b"DOMDocument.xml")?.ok_or("No se encuentra DOMDocument.xml en el FLA")?)
}
pub fn xml_text(data: Vec<u8>) -> Result<String, String> {
    if data.starts_with(&[0xff, 0xfe]) || data.starts_with(&[0xfe, 0xff]) {
        if data.len() % 2 != 0 {
            return Err("XML UTF-16 truncado".into());
        }
        let little = data[0] == 0xff;
        let words = data[2..].chunks_exact(2).map(|c| {
            if little {
                u16::from_le_bytes([c[0], c[1]])
            } else {
                u16::from_be_bytes([c[0], c[1]])
            }
        });
        let text = char::decode_utf16(words)
            .collect::<Result<String, _>>()
            .map_err(|e| e.to_string())?;
        Ok(text
            .replacen("encoding=\"UTF-16\"", "encoding=\"UTF-8\"", 1)
            .replacen("encoding=\"utf-16\"", "encoding=\"utf-8\"", 1))
    } else {
        String::from_utf8(data).map_err(|e| e.to_string())
    }
}
/// Central directory is indexed once when importing a library.
pub struct Archive<'a> {
    bytes: &'a [u8],
    entries: std::collections::HashMap<Vec<u8>, usize>,
    directory_size_recovered: bool,
}
impl<'a> Archive<'a> {
    pub fn new(bytes: &'a [u8]) -> Result<Self, String> {
        let end=(bytes.len().saturating_sub(65557)..bytes.len().saturating_sub(3)).rev().find(|&i|bytes.get(i..i+4)==Some(&[0x50,0x4b,0x05,0x06])&&u16_at(bytes,i+20).is_ok_and(|len|i+22+len as usize==bytes.len())).ok_or("Este archivo no es un FLA moderno (CS5 o posterior). Los FLA binarios antiguos no están admitidos.")?;
        if u16_at(bytes, end + 4)? != 0 || u16_at(bytes, end + 6)? != 0 {
            return Err("ZIP dividido no admitido".into());
        }
        let count = u16_at(bytes, end + 10)?;
        if count != u16_at(bytes, end + 8)? {
            return Err("Directorio ZIP incoherente".into());
        }
        let mut p = u32_at(bytes, end + 16)? as usize;
        let start = p;
        let declared_size = u32_at(bytes, end + 12)? as usize;
        if start > end {
            return Err("Directorio ZIP fuera de rango".into());
        }
        // Parse the declared number of records against the physical EOCD boundary.
        // Animate 24 files can overstate the directory size by 54 bytes.
        let stop = end;
        let mut entries = std::collections::HashMap::new();
        for _ in 0..count {
            if u32_at(bytes, p)? != 0x02014b50 {
                return Err("Directorio FLA dañado".into());
            }
            let len = u16_at(bytes, p + 28)? as usize;
            let extra = u16_at(bytes, p + 30)? as usize;
            let comment = u16_at(bytes, p + 32)? as usize;
            let name = bytes
                .get(p + 46..p + 46 + len)
                .ok_or("Nombre ZIP truncado")?;
            if entries.insert(name.to_vec(), p).is_some() {
                return Err("Entrada ZIP duplicada".into());
            }
            p = p
                .checked_add(46 + len + extra + comment)
                .filter(|p| *p <= stop)
                .ok_or("Directorio ZIP truncado")?;
        }
        if p != stop {
            return Err("Tamaño de directorio ZIP incoherente".into());
        }
        let actual_size = p - start;
        let directory_size_recovered = declared_size != actual_size;
        if directory_size_recovered
            && !(declared_size == actual_size + 54 && entries.contains_key(b"mimetype".as_slice()))
        {
            return Err("Tamaño de directorio ZIP incoherente".into());
        }
        Ok(Self {
            bytes,
            entries,
            directory_size_recovered,
        })
    }
    pub fn directory_size_recovered(&self) -> bool {
        self.directory_size_recovered
    }
    pub fn entry(&self, wanted: &[u8]) -> Result<Option<Vec<u8>>, String> {
        let Some(&p) = self.entries.get(wanted) else {
            return Ok(None);
        };
        let bytes = self.bytes;
        let name = wanted;
        let flags = u16_at(bytes, p + 8)?;
        let method = u16_at(bytes, p + 10)?;
        let crc = u32_at(bytes, p + 16)?;
        let compressed = u32_at(bytes, p + 20)? as usize;
        let uncompressed = u32_at(bytes, p + 24)? as usize;
        let local = u32_at(bytes, p + 42)? as usize;
        if flags & 1 != 0 || uncompressed > LIMIT || compressed > LIMIT {
            return Err("FLA cifrado o demasiado grande".into());
        }
        if u32_at(bytes, local)? != 0x04034b50 {
            return Err("Cabecera ZIP dañada".into());
        }
        let local_name_len = u16_at(bytes, local + 26)? as usize;
        if u16_at(bytes, local + 6)? != flags
            || u16_at(bytes, local + 8)? != method
            || bytes.get(local + 30..local + 30 + local_name_len) != Some(name)
        {
            return Err("Cabecera local ZIP no coincide con el directorio".into());
        }
        let start =
            local + 30 + u16_at(bytes, local + 26)? as usize + u16_at(bytes, local + 28)? as usize;
        let data = bytes
            .get(start..start + compressed)
            .ok_or("Contenido FLA truncado")?;
        let decoded = match method {
            0 => data.to_vec(),
            8 => {
                let mut out = vec![];
                flate2::read::DeflateDecoder::new(data)
                    .take((LIMIT + 1) as u64)
                    .read_to_end(&mut out)
                    .map_err(|e| e.to_string())?;
                out
            }
            _ => return Err("Compresión FLA no admitida".into()),
        };
        if decoded.len() != uncompressed || crc32fast::hash(&decoded) != crc {
            return Err("DOMDocument.xml dañado".into());
        }
        Ok(Some(decoded))
    }
}
pub fn entry(bytes: &[u8], wanted: &[u8]) -> Result<Option<Vec<u8>>, String> {
    Archive::new(bytes)?.entry(wanted)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archive_round_trip_and_corruption() {
        let b = pack(&[("DOMDocument.xml", b"<DOMDocument/>".to_vec())]);
        assert_eq!(document(&b).unwrap(), "<DOMDocument/>");
        let mut corrupt = b;
        corrupt[45] ^= 1;
        assert!(document(&corrupt).is_err());
    }
}
