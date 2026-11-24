//! Streaming ZIP/ZIP64 for native projects. No archive-sized allocation and no extraction.
use std::{
    collections::{BTreeMap, HashSet},
    io::{self, Read, Seek, SeekFrom, Write},
};

fn error(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn n16(b: &[u8], at: usize) -> io::Result<u16> {
    Ok(u16::from_le_bytes(
        b.get(at..at + 2)
            .ok_or_else(|| error("Truncated ZIP"))?
            .try_into()
            .unwrap(),
    ))
}
fn n32(b: &[u8], at: usize) -> io::Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(at..at + 4)
            .ok_or_else(|| error("Truncated ZIP"))?
            .try_into()
            .unwrap(),
    ))
}
fn n64(b: &[u8], at: usize) -> io::Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(at..at + 8)
            .ok_or_else(|| error("Truncated ZIP64"))?
            .try_into()
            .unwrap(),
    ))
}
fn p16(b: &mut Vec<u8>, v: u16) {
    b.extend(v.to_le_bytes());
}
fn p32(b: &mut Vec<u8>, v: u32) {
    b.extend(v.to_le_bytes());
}
fn p64(b: &mut Vec<u8>, v: u64) {
    b.extend(v.to_le_bytes());
}
fn name_valid(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains(['\\', ':', '\0'])
        && !name.split('/').any(|p| p == ".." || p == ".")
}

struct Count<W> {
    inner: W,
    bytes: u64,
}
impl<W: Write> Write for Count<W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(b)?;
        self.bytes = self
            .bytes
            .checked_add(n as u64)
            .ok_or_else(|| error("Archive size overflow"))?;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
struct Check<W> {
    inner: W,
    crc: crc32fast::Hasher,
    bytes: u64,
}
impl<W: Write> Write for Check<W> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(b)?;
        self.crc.update(&b[..n]);
        self.bytes = self
            .bytes
            .checked_add(n as u64)
            .ok_or_else(|| error("Entry size overflow"))?;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
struct Record {
    name: String,
    offset: u64,
    raw: u64,
    compressed: u64,
    crc: u32,
}
pub(crate) struct Writer<W: Write> {
    out: Count<W>,
    entries: Vec<Record>,
    names: HashSet<String>,
}
impl<W: Write> Writer<W> {
    pub(crate) fn new(out: W) -> Self {
        Self {
            out: Count {
                inner: out,
                bytes: 0,
            },
            entries: vec![],
            names: HashSet::new(),
        }
    }
    pub(crate) fn entry(
        &mut self,
        name: &str,
        write: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    ) -> io::Result<()> {
        if !name_valid(name)
            || name.len() > u16::MAX as usize
            || !self.names.insert(name.to_lowercase())
        {
            return Err(error("Invalid or duplicate ZIP name"));
        }
        let offset = self.out.bytes;
        let mut h = Vec::with_capacity(64 + name.len());
        p32(&mut h, 0x04034b50);
        p16(&mut h, 45);
        p16(&mut h, 0x808);
        p16(&mut h, 8);
        p32(&mut h, 0);
        p32(&mut h, 0);
        p32(&mut h, u32::MAX);
        p32(&mut h, u32::MAX);
        p16(&mut h, name.len() as u16);
        p16(&mut h, 20);
        h.extend(name.as_bytes());
        p16(&mut h, 1);
        p16(&mut h, 16);
        p64(&mut h, 0);
        p64(&mut h, 0);
        self.out.write_all(&h)?;
        let begin = self.out.bytes;
        let (raw, crc) = {
            let encoder =
                flate2::write::DeflateEncoder::new(&mut self.out, flate2::Compression::fast());
            let mut check = Check {
                inner: encoder,
                crc: crc32fast::Hasher::new(),
                bytes: 0,
            };
            write(&mut check)?;
            let Check {
                inner,
                crc,
                bytes: raw,
            } = check;
            let crc = crc.finalize();
            inner.finish()?;
            (raw, crc)
        };
        let compressed = self.out.bytes - begin;
        h.clear();
        p32(&mut h, 0x08074b50);
        p32(&mut h, crc);
        p64(&mut h, compressed);
        p64(&mut h, raw);
        self.out.write_all(&h)?;
        self.entries.push(Record {
            name: name.into(),
            offset,
            raw,
            compressed,
            crc,
        });
        Ok(())
    }
    pub(crate) fn finish(mut self) -> io::Result<()> {
        let start = self.out.bytes;
        let mut h = Vec::new();
        for e in &self.entries {
            h.clear();
            p32(&mut h, 0x02014b50);
            p16(&mut h, 45);
            p16(&mut h, 45);
            p16(&mut h, 0x808);
            p16(&mut h, 8);
            p32(&mut h, 0);
            p32(&mut h, e.crc);
            p32(&mut h, u32::MAX);
            p32(&mut h, u32::MAX);
            p16(&mut h, e.name.len() as u16);
            p16(&mut h, 28);
            p16(&mut h, 0);
            p16(&mut h, 0);
            p16(&mut h, 0);
            p32(&mut h, 0);
            p32(&mut h, u32::MAX);
            h.extend(e.name.as_bytes());
            p16(&mut h, 1);
            p16(&mut h, 24);
            p64(&mut h, e.raw);
            p64(&mut h, e.compressed);
            p64(&mut h, e.offset);
            self.out.write_all(&h)?;
        }
        let length = self.out.bytes - start;
        let end = self.out.bytes;
        h.clear();
        p32(&mut h, 0x06064b50);
        p64(&mut h, 44);
        p16(&mut h, 45);
        p16(&mut h, 45);
        p32(&mut h, 0);
        p32(&mut h, 0);
        p64(&mut h, self.entries.len() as u64);
        p64(&mut h, self.entries.len() as u64);
        p64(&mut h, length);
        p64(&mut h, start);
        p32(&mut h, 0x07064b50);
        p32(&mut h, 0);
        p64(&mut h, end);
        p32(&mut h, 1);
        p32(&mut h, 0x06054b50);
        p16(&mut h, 0);
        p16(&mut h, 0);
        p16(&mut h, u16::MAX);
        p16(&mut h, u16::MAX);
        p32(&mut h, u32::MAX);
        p32(&mut h, u32::MAX);
        p16(&mut h, 0);
        self.out.write_all(&h)?;
        self.out.flush()
    }
}

#[derive(Clone)]
struct Entry {
    offset: u64,
    raw: u64,
    compressed: u64,
    crc: u32,
    method: u16,
    flags: u16,
}
pub(crate) struct Reader<R: Read + Seek> {
    input: R,
    entries: BTreeMap<String, Entry>,
    directory: u64,
}
impl<R: Read + Seek> Reader<R> {
    pub(crate) fn new(mut input: R) -> io::Result<Self> {
        let length = input.seek(SeekFrom::End(0))?;
        let size = length.min(65557) as usize;
        input.seek(SeekFrom::End(-(size as i64)))?;
        let mut tail = vec![0; size];
        input.read_exact(&mut tail)?;
        let at = (0..size.saturating_sub(21))
            .rev()
            .find(|&i| {
                tail.get(i..i + 4) == Some(b"PK\x05\x06")
                    && n16(&tail, i + 20).is_ok_and(|n| i + 22 + n as usize == size)
            })
            .ok_or_else(|| error("Missing ZIP directory"))?;
        if n16(&tail, at + 4)? != 0 || n16(&tail, at + 6)? != 0 {
            return Err(error("Multi-disk ZIP is unsupported"));
        }
        let mut count = n16(&tail, at + 10)? as u64;
        let mut bytes = n32(&tail, at + 12)? as u64;
        let mut directory = n32(&tail, at + 16)? as u64;
        if count == u16::MAX as u64 || bytes == u32::MAX as u64 || directory == u32::MAX as u64 {
            let end = length - size as u64 + at as u64;
            if end < 20 {
                return Err(error("Missing ZIP64 locator"));
            }
            input.seek(SeekFrom::Start(end - 20))?;
            let mut locator = [0; 20];
            input.read_exact(&mut locator)?;
            if n32(&locator, 0)? != 0x07064b50 || n32(&locator, 4)? != 0 || n32(&locator, 16)? != 1
            {
                return Err(error("Invalid ZIP64 locator"));
            }
            let location = n64(&locator, 8)?;
            if location.checked_add(56).is_none_or(|n| n > end - 20) {
                return Err(error("Invalid ZIP64 directory offset"));
            }
            input.seek(SeekFrom::Start(location))?;
            let mut record = [0; 56];
            input.read_exact(&mut record)?;
            if n32(&record, 0)? != 0x06064b50
                || n64(&record, 4)? < 44
                || n32(&record, 16)? != 0
                || n32(&record, 20)? != 0
                || n64(&record, 24)? != n64(&record, 32)?
            {
                return Err(error("Invalid ZIP64 directory"));
            }
            count = n64(&record, 32)?;
            bytes = n64(&record, 40)?;
            directory = n64(&record, 48)?;
        }
        // Bounds on structural complexity, not on the project file's size.
        if count > 1_000_000
            || directory.checked_add(bytes).is_none_or(|n| n > length)
            || count > bytes / 46
        {
            return Err(error("Invalid ZIP directory bounds"));
        }
        input.seek(SeekFrom::Start(directory))?;
        let mut entries = BTreeMap::new();
        let mut lower = std::collections::HashSet::new();
        let finish = directory + bytes;
        for _ in 0..count {
            if input
                .stream_position()?
                .checked_add(46)
                .is_none_or(|n| n > finish)
            {
                return Err(error("Truncated ZIP directory"));
            }
            let mut h = [0; 46];
            input.read_exact(&mut h)?;
            if n32(&h, 0)? != 0x02014b50 {
                return Err(error("Invalid ZIP directory record"));
            }
            let flags = n16(&h, 8)?;
            let method = n16(&h, 10)?;
            if flags & !0x808 != 0 || ![0, 8].contains(&method) || n16(&h, 34)? != 0 {
                return Err(error("Unsupported encrypted or compressed ZIP entry"));
            }
            let nl = n16(&h, 28)? as usize;
            let el = n16(&h, 30)? as usize;
            let cl = n16(&h, 32)? as usize;
            if input
                .stream_position()?
                .checked_add((nl + el + cl) as u64)
                .is_none_or(|n| n > finish)
            {
                return Err(error("Truncated ZIP name"));
            }
            let mut name = vec![0; nl];
            input.read_exact(&mut name)?;
            let name =
                String::from_utf8(name).map_err(|_| error("Non-UTF8 native resource name"))?;
            let mut extra = vec![0; el];
            input.read_exact(&mut extra)?;
            input.seek(SeekFrom::Current(cl as i64))?;
            let mut raw = n32(&h, 24)? as u64;
            let mut compressed = n32(&h, 20)? as u64;
            let mut offset = n32(&h, 42)? as u64;
            if [raw, compressed, offset].contains(&(u32::MAX as u64)) {
                let mut pos = 0;
                let mut zip64 = None;
                while pos + 4 <= extra.len() {
                    let tag = n16(&extra, pos)?;
                    let len = n16(&extra, pos + 2)? as usize;
                    let data = extra
                        .get(pos + 4..pos + 4 + len)
                        .ok_or_else(|| error("Truncated ZIP extra field"))?;
                    if tag == 1 {
                        zip64 = Some(data);
                        break;
                    }
                    pos += 4 + len;
                }
                let data = zip64.ok_or_else(|| error("Missing ZIP64 sizes"))?;
                let mut pos = 0;
                for value in [&mut raw, &mut compressed, &mut offset] {
                    if *value == u32::MAX as u64 {
                        *value = n64(data, pos)?;
                        pos += 8;
                    }
                }
            }
            if !name_valid(&name) || !lower.insert(name.to_ascii_lowercase()) || offset >= directory
            {
                return Err(error("Invalid or duplicate native resource"));
            }
            entries.insert(
                name,
                Entry {
                    offset,
                    raw,
                    compressed,
                    crc: n32(&h, 16)?,
                    method,
                    flags,
                },
            );
        }
        if input.stream_position()? != finish {
            return Err(error("Invalid ZIP directory length"));
        }
        Ok(Self {
            input,
            entries,
            directory,
        })
    }
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }
    pub(crate) fn read<T>(
        &mut self,
        name: &str,
        limit: Option<u64>,
        read: impl FnOnce(&mut dyn Read) -> io::Result<T>,
    ) -> io::Result<T> {
        let entry = self
            .entries
            .get(name)
            .cloned()
            .ok_or_else(|| error("Missing native project resource"))?;
        if limit.is_some_and(|n| entry.raw > n) {
            return Err(error("Native resource exceeds its individual safety limit"));
        }
        self.input.seek(SeekFrom::Start(entry.offset))?;
        let mut h = [0; 30];
        self.input.read_exact(&mut h)?;
        if n32(&h, 0)? != 0x04034b50 || n16(&h, 6)? != entry.flags || n16(&h, 8)? != entry.method {
            return Err(error("Inconsistent ZIP local header"));
        }
        let nl = n16(&h, 26)? as usize;
        let el = n16(&h, 28)? as u64;
        let mut local = vec![0; nl];
        self.input.read_exact(&mut local)?;
        if local != name.as_bytes() {
            return Err(error("Inconsistent ZIP entry name"));
        }
        self.input.seek(SeekFrom::Current(el as i64))?;
        if self
            .input
            .stream_position()?
            .checked_add(entry.compressed)
            .is_none_or(|n| n > self.directory)
        {
            return Err(error("Invalid ZIP entry extent"));
        }
        let take = (&mut self.input).take(entry.compressed);
        let mut decoded: Box<dyn Read + '_> = if entry.method == 8 {
            Box::new(flate2::read::DeflateDecoder::new(take))
        } else {
            Box::new(take)
        };
        let mut verified = Verified {
            inner: &mut decoded,
            crc: crc32fast::Hasher::new(),
            bytes: 0,
            entry: &entry,
        };
        let value = read(&mut verified)?;
        io::copy(&mut verified, &mut io::sink())?;
        if verified.bytes != entry.raw || verified.crc.finalize() != entry.crc {
            return Err(error("Native resource checksum mismatch"));
        }
        Ok(value)
    }
    pub(crate) fn bytes(&mut self, name: &str, limit: u64) -> io::Result<Vec<u8>> {
        self.read(name, Some(limit), |r| {
            let mut b = Vec::new();
            r.read_to_end(&mut b)?;
            Ok(b)
        })
    }
}
struct Verified<'a> {
    inner: &'a mut dyn Read,
    crc: crc32fast::Hasher,
    bytes: u64,
    entry: &'a Entry,
}
impl Read for Verified<'_> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(b)?;
        self.bytes = self
            .bytes
            .checked_add(n as u64)
            .ok_or_else(|| error("Native entry size overflow"))?;
        if self.bytes > self.entry.raw {
            return Err(error("Native resource exceeds declared size"));
        }
        self.crc.update(&b[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zip64_streaming_roundtrip_and_checksum_validation() {
        let mut bytes = vec![];
        let mut writer = Writer::new(&mut bytes);
        writer.entry("data", |w| w.write_all(&[7; 8192])).unwrap();
        writer.finish().unwrap();
        let mut reader = Reader::new(io::Cursor::new(&bytes)).unwrap();
        assert_eq!(reader.bytes("data", 10000).unwrap(), vec![7; 8192]);
        assert!(reader.bytes("data", 100).is_err());
        let directory = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        bytes[directory + 16] ^= 1;
        let mut reader = Reader::new(io::Cursor::new(bytes)).unwrap();
        assert!(reader.bytes("data", 10000).is_err());
    }
    #[test]
    fn zip64_offsets_above_four_gib_do_not_truncate() {
        struct Sparse {
            base: u64,
            data: Vec<u8>,
            position: u64,
        }
        impl Read for Sparse {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                let end = self.base + self.data.len() as u64;
                if self.position >= end {
                    return Ok(0);
                }
                let n = b.len().min((end - self.position) as usize);
                for (i, v) in b[..n].iter_mut().enumerate() {
                    let at = self.position + i as u64;
                    *v = if at < self.base {
                        0
                    } else {
                        self.data[(at - self.base) as usize]
                    };
                }
                self.position += n as u64;
                Ok(n)
            }
        }
        impl Seek for Sparse {
            fn seek(&mut self, p: SeekFrom) -> io::Result<u64> {
                let n = match p {
                    SeekFrom::Start(v) => v as i128,
                    SeekFrom::End(v) => self.base as i128 + self.data.len() as i128 + v as i128,
                    SeekFrom::Current(v) => self.position as i128 + v as i128,
                };
                self.position = u64::try_from(n).map_err(|_| error("seek"))?;
                Ok(self.position)
            }
        }
        let base = u32::MAX as u64 + 4096;
        let mut bytes = vec![];
        let mut writer = Writer::new(&mut bytes);
        writer.out.bytes = base;
        writer
            .entry("large-offset", |w| w.write_all(b"preserved"))
            .unwrap();
        writer.finish().unwrap();
        let mut reader = Reader::new(Sparse {
            base,
            data: bytes,
            position: 0,
        })
        .unwrap();
        assert_eq!(reader.bytes("large-offset", 100).unwrap(), b"preserved");
    }
    #[test]
    fn failed_stream_writes_report_io_errors_without_publishing() {
        struct Full;
        impl Write for Full {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::StorageFull, "disk full"))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut writer = Writer::new(Full);
        assert!(writer.entry("data", |w| w.write_all(b"data")).is_err());
    }
    #[test]
    fn classic_zip_still_reads_and_duplicate_or_truncated_resources_fail() {
        let bytes = crate::fla_zip::pack(&[("data", vec![1, 2, 3])]);
        let mut reader = Reader::new(io::Cursor::new(bytes)).unwrap();
        assert_eq!(reader.bytes("data", 3).unwrap(), vec![1, 2, 3]);
        let mut writer = Writer::new(Vec::new());
        assert!(writer.entry("../data", |_| Ok(())).is_err());
        writer.entry("a", |_| Ok(())).unwrap();
        assert!(writer.entry("a", |_| Ok(())).is_err());
        assert!(Reader::new(io::Cursor::new(b"PK\x05\x06")).is_err());
    }
}
