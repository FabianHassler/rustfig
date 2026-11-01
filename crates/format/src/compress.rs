//! Compressed Fig files: `.fig.gz`, `.fig.Z`, `.fig.z`, `.fig.bz2`, `.fig.xz`.
//!
//! xfig opens these by piping the file through `gunzip`, `uncompress`,
//! `bunzip2` or `unxz` (`file_on_disk`/`open_stream`, `f_picobj.c`) and never
//! saves them compressed; RustFig reads all of them without an external
//! program and also writes them. A stream is recognised by its magic bytes
//! (the file name's suffix is only a hint), and what comes out is capped
//! at [`MAX_DECOMPRESSED_BYTES`] so that a decompression bomb fails with an
//! error. All codecs are pure Rust (they also run in the browser).

use crate::{FileFormat, FormatError};
use std::borrow::Cow;
use std::io::{Read, Write};

/// Most bytes a compressed file may expand to.
pub const MAX_DECOMPRESSED_BYTES: usize = 256 << 20;

/// A compression format for Fig files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    /// gzip (`.gz`).
    Gzip,
    /// Unix `compress`, LZW (`.Z`, `.z`).
    Compress,
    /// bzip2 (`.bz2`).
    Bzip2,
    /// xz (`.xz`).
    Xz,
}

impl Compression {
    /// The format of `bytes`, from the magic bytes.
    pub fn detect(bytes: &[u8]) -> Option<Compression> {
        if bytes.starts_with(&[0x1f, 0x8b]) {
            Some(Compression::Gzip)
        } else if bytes.starts_with(&[0x1f, 0x9d]) {
            Some(Compression::Compress)
        } else if bytes.starts_with(b"BZh") {
            Some(Compression::Bzip2)
        } else if bytes.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0]) {
            Some(Compression::Xz)
        } else {
            None
        }
    }

    /// The compression a Fig file name asks for: `name.fig.gz`, `.Z`, `.z`,
    /// `.bz2`, `.xz` (also `.xfig`). Other names, including `name.gz`, are
    /// not compressed Fig files.
    pub fn from_fig_path(path: &str) -> Option<Compression> {
        let lower = path.to_ascii_lowercase();
        let (stem, ext) = lower.rsplit_once('.')?;
        if !(stem.ends_with(".fig") || stem.ends_with(".xfig")) {
            return None;
        }
        match ext {
            "gz" => Some(Compression::Gzip),
            "z" => Some(Compression::Compress),
            "bz2" => Some(Compression::Bzip2),
            "xz" => Some(Compression::Xz),
            _ => None,
        }
    }
}

/// Decompress `bytes` (one layer, at most `cap` bytes out).
pub fn decompress(bytes: &[u8], kind: Compression, cap: usize) -> Result<Vec<u8>, FormatError> {
    let too_large = || FormatError::new("the compressed file expands to too much data");
    let bad = |what: &str, e: &dyn std::fmt::Display| FormatError::new(format!("damaged {what} data: {e}"));
    let limit = (cap as u64).saturating_add(1);
    let mut out = Vec::new();
    match kind {
        Compression::Gzip => {
            flate2::read::MultiGzDecoder::new(bytes)
                .take(limit)
                .read_to_end(&mut out)
                .map_err(|e| bad("gzip", &e))?;
        }
        Compression::Bzip2 => {
            bzip2::read::MultiBzDecoder::new(bytes)
                .take(limit)
                .read_to_end(&mut out)
                .map_err(|e| bad("bzip2", &e))?;
        }
        Compression::Xz => {
            let mut w = CappedWriter {
                out: Vec::new(),
                cap,
                hit: false,
            };
            let r = lzma_rs::xz_decompress(&mut &bytes[..], &mut w);
            if w.hit {
                return Err(too_large());
            }
            r.map_err(|e| bad("xz", &e))?;
            out = w.out;
        }
        Compression::Compress => out = unlzw(bytes, cap)?,
    }
    if out.len() > cap {
        return Err(too_large());
    }
    Ok(out)
}

/// Compress `data`.
pub fn compress(data: &[u8], kind: Compression) -> Result<Vec<u8>, FormatError> {
    let bad = |what: &str, e: &dyn std::fmt::Display| FormatError::new(format!("{what} compression failed: {e}"));
    match kind {
        Compression::Gzip => {
            let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data).map_err(|e| bad("gzip", &e))?;
            enc.finish().map_err(|e| bad("gzip", &e))
        }
        Compression::Bzip2 => {
            let mut enc = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::best());
            enc.write_all(data).map_err(|e| bad("bzip2", &e))?;
            enc.finish().map_err(|e| bad("bzip2", &e))
        }
        Compression::Xz => {
            let mut out = Vec::new();
            lzma_rs::xz_compress(&mut &data[..], &mut out).map_err(|e| bad("xz", &e))?;
            Ok(out)
        }
        Compression::Compress => Ok(lzw_compress(data)),
    }
}

/// The Fig bytes inside a compressed file, or `bytes` themselves when they
/// are not compressed. A compressed file that does not hold a Fig or native
/// document (an `.svgz`, say) comes back unchanged, unless `strict` (its
/// name is a compressed-Fig name): then damage is an error. A stream that
/// expands past [`MAX_DECOMPRESSED_BYTES`] is always an error.
pub fn unpack(bytes: &[u8], strict: bool) -> Result<Cow<'_, [u8]>, FormatError> {
    let Some(kind) = Compression::detect(bytes) else {
        return Ok(Cow::Borrowed(bytes));
    };
    match decompress(bytes, kind, MAX_DECOMPRESSED_BYTES) {
        Ok(inner) if strict || FileFormat::sniff(&inner).is_some() => Ok(Cow::Owned(inner)),
        Ok(_) => Ok(Cow::Borrowed(bytes)),
        Err(e) if strict || e.message.contains("too much") => Err(e),
        Err(_) => Ok(Cow::Borrowed(bytes)),
    }
}

/// [`unpack`] for a file with this name.
pub fn unpack_file<'a>(bytes: &'a [u8], path: &str) -> Result<Cow<'a, [u8]>, FormatError> {
    unpack(bytes, Compression::from_fig_path(path).is_some())
}

struct CappedWriter {
    out: Vec<u8>,
    cap: usize,
    hit: bool,
}

impl Write for CappedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self.out.len().saturating_add(buf.len()) > self.cap {
            self.hit = true;
            return Err(std::io::Error::other("output too large"));
        }
        self.out.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

const CLEAR: usize = 256;

/// Unix `compress` (LZW, LSB-first codes of 9 to `maxbits` bits, block mode
/// with CLEAR = 256; the code stream is realigned to a group of 8 codes at
/// every width change and clear, as `compress` writes it).
fn unlzw(data: &[u8], cap: usize) -> Result<Vec<u8>, FormatError> {
    let bad = |m: &str| FormatError::new(format!("damaged compress data: {m}"));
    let (Some(&flags), Some(body)) = (data.get(2), data.get(3..)) else {
        return Err(bad("truncated header"));
    };
    let maxbits = usize::from(flags & 0x1f);
    let block = flags & 0x80 != 0;
    if !(9..=16).contains(&maxbits) {
        return Err(bad("unsupported code size"));
    }
    let max_entries = 1usize << maxbits;
    let first_free = if block { 257 } else { 256 };
    let mut prefix = vec![0u16; max_entries];
    let mut suffix: Vec<u8> = (0..max_entries).map(|i| i as u8).collect();
    let mut free = first_free;
    let mut n = 9usize;
    let total_bits = body.len() * 8;
    let (mut pos, mut group) = (0usize, 0usize);
    let mut old: Option<usize> = None;
    let mut fin = 0u8;
    let mut out: Vec<u8> = Vec::new();
    let mut stack: Vec<u8> = Vec::new();
    let align = |pos: usize, group: usize, n: usize| group + (pos - group).div_ceil(8 * n) * 8 * n;
    loop {
        if pos + n > total_bits {
            break;
        }
        let mut code = 0usize;
        for i in 0..n {
            let bit = pos + i;
            if body.get(bit / 8).is_some_and(|b| b >> (bit % 8) & 1 == 1) {
                code |= 1 << i;
            }
        }
        pos += n;
        if block && code == CLEAR {
            pos = align(pos, group, n);
            group = pos;
            n = 9;
            free = first_free;
            old = None;
            continue;
        }
        let Some(prev) = old else {
            if code > 255 {
                return Err(bad("bad first code"));
            }
            out.push(code as u8);
            fin = code as u8;
            old = Some(code);
            continue;
        };
        if code > free || (code == free && free >= max_entries) {
            return Err(bad("code out of range"));
        }
        stack.clear();
        let mut cur = code;
        if code == free {
            // KwKwK: the string just being defined.
            stack.push(fin);
            cur = prev;
        }
        while cur >= 256 {
            if cur < first_free || cur >= free {
                return Err(bad("bad code"));
            }
            stack.push(*suffix.get(cur).ok_or_else(|| bad("bad code"))?);
            cur = usize::from(*prefix.get(cur).ok_or_else(|| bad("bad code"))?);
            if stack.len() > max_entries {
                return Err(bad("loop in the code table"));
            }
        }
        fin = cur as u8;
        stack.push(fin);
        if out.len().saturating_add(stack.len()) > cap {
            return Err(FormatError::new("the compressed file expands to too much data"));
        }
        out.extend(stack.iter().rev());
        if free < max_entries {
            if let (Some(p), Some(s)) = (prefix.get_mut(free), suffix.get_mut(free)) {
                *p = prev as u16;
                *s = fin;
            }
            free += 1;
        }
        old = Some(code);
        if free > (1 << n) - 1 && n < maxbits {
            pos = align(pos, group, n);
            group = pos;
            n += 1;
        }
    }
    Ok(out)
}

/// `compress -b 16` for `data`: LZW with the table frozen when full (no
/// CLEAR codes are needed for the result to be valid).
fn lzw_compress(data: &[u8]) -> Vec<u8> {
    const MAXBITS: usize = 16;
    let mut out = vec![0x1f, 0x9d, 0x80 | MAXBITS as u8];
    let mut bits = BitWriter::default();
    let mut table: std::collections::HashMap<(u16, u8), u16> = std::collections::HashMap::new();
    let mut free = 257usize;
    let mut n = 9usize;
    let mut bytes = data.iter();
    let Some(&first) = bytes.next() else {
        return out;
    };
    let mut w = u16::from(first);
    for &c in bytes {
        if let Some(&code) = table.get(&(w, c)) {
            w = code;
            continue;
        }
        bits.put(usize::from(w), n);
        if free > (1 << n) - 1 && n < MAXBITS {
            bits.pad_group(n);
            n += 1;
        }
        if free < (1 << MAXBITS) {
            table.insert((w, c), free as u16);
            free += 1;
        }
        w = u16::from(c);
    }
    bits.put(usize::from(w), n);
    out.extend(bits.finish());
    out
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    nbits: usize,
    group: usize,
}

impl BitWriter {
    fn put(&mut self, code: usize, n: usize) {
        for i in 0..n {
            if self.nbits.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if code >> i & 1 == 1 {
                if let Some(b) = self.bytes.last_mut() {
                    *b |= 1 << (self.nbits % 8);
                }
            }
            self.nbits += 1;
        }
    }

    /// Zero-fill to the end of the current group of 8 codes of `n` bits.
    fn pad_group(&mut self, n: usize) {
        let used = self.nbits - self.group;
        let target = self.group + used.div_ceil(8 * n) * 8 * n;
        while self.nbits < target {
            if self.nbits.is_multiple_of(8) {
                self.bytes.push(0);
            }
            self.nbits += 1;
        }
        self.group = self.nbits;
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Compression; 4] = [
        Compression::Gzip,
        Compression::Compress,
        Compression::Bzip2,
        Compression::Xz,
    ];

    fn sample(len: usize) -> Vec<u8> {
        let mut s = b"#FIG 3.2\nLandscape\nCenter\nInches\nLetter\n100.00\nSingle\n-2\n1200 2\n".to_vec();
        let mut x = 12345u32;
        while s.len() < len {
            x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            s.extend_from_slice(
                format!(
                    "2 1 0 1 0 7 50 -1 -1 0.0 0 0 -1 0 0 2\n\t {} {} {} {}\n",
                    x % 9000,
                    x % 7000,
                    x % 5000,
                    x % 3000
                )
                .as_bytes(),
            );
        }
        s
    }

    #[test]
    fn round_trips_each_codec() {
        for len in [0usize, 1, 100, 5000, 300_000] {
            let data = sample(len);
            for kind in ALL {
                let packed = compress(&data, kind).unwrap();
                assert_eq!(Compression::detect(&packed), Some(kind), "{kind:?}");
                assert_eq!(decompress(&packed, kind, 1 << 30).unwrap(), data, "{kind:?} {len}");
            }
        }
    }

    #[test]
    fn lzw_is_compatible_with_widths_and_full_table() {
        // Random-ish data fills the 16-bit table and crosses every width.
        let mut x = 7u64;
        let data: Vec<u8> = (0..400_000)
            .map(|_| {
                x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (x >> 56) as u8 & 0x3f
            })
            .collect();
        let packed = compress(&data, Compression::Compress).unwrap();
        assert_eq!(decompress(&packed, Compression::Compress, 1 << 30).unwrap(), data);
    }

    #[test]
    fn decompression_bomb_is_capped() {
        let data = vec![b'A'; 3 << 20];
        for kind in ALL {
            let packed = compress(&data, kind).unwrap();
            // lzma-rs's xz encoder stores data in uncompressed LZMA2 chunks.
            assert!(kind == Compression::Xz || packed.len() < data.len() / 4, "{kind:?}");
            let err = decompress(&packed, kind, 1 << 20).unwrap_err();
            assert!(err.message.contains("too much"), "{kind:?}: {err}");
            assert_eq!(decompress(&packed, kind, 3 << 20).unwrap().len(), data.len());
        }
    }

    #[test]
    fn unpack_finds_fig_and_leaves_other_files() {
        let fig = sample(300);
        for kind in ALL {
            let packed = compress(&fig, kind).unwrap();
            assert_eq!(unpack(&packed, false).unwrap().as_ref(), fig.as_slice());
        }
        let other = compress(b"<svg/>", Compression::Gzip).unwrap();
        assert_eq!(unpack(&other, false).unwrap().as_ref(), other.as_slice());
        let mut damaged = compress(&fig, Compression::Gzip).unwrap();
        damaged.truncate(20);
        assert_eq!(unpack(&damaged, false).unwrap().as_ref(), damaged.as_slice());
        assert!(unpack(&damaged, true).is_err());
    }

    #[test]
    fn names() {
        assert_eq!(Compression::from_fig_path("a.fig.gz"), Some(Compression::Gzip));
        assert_eq!(Compression::from_fig_path("a.FIG.Z"), Some(Compression::Compress));
        assert_eq!(Compression::from_fig_path("a.fig.z"), Some(Compression::Compress));
        assert_eq!(Compression::from_fig_path("a.xfig.bz2"), Some(Compression::Bzip2));
        assert_eq!(Compression::from_fig_path("a.fig.xz"), Some(Compression::Xz));
        assert_eq!(Compression::from_fig_path("a.svg.gz"), None);
        assert_eq!(Compression::from_fig_path("a.gz"), None);
        assert_eq!(Compression::from_fig_path("a.fig"), None);
    }

    #[test]
    fn hostile_streams_fail_cleanly() {
        let mut x = 99u32;
        for kind in ALL {
            for len in [0usize, 1, 3, 10, 200, 3000] {
                let head: &[u8] = match kind {
                    Compression::Gzip => &[0x1f, 0x8b, 8],
                    Compression::Compress => &[0x1f, 0x9d, 0x90],
                    Compression::Bzip2 => b"BZh9",
                    Compression::Xz => &[0xfd, b'7', b'z', b'X', b'Z', 0],
                };
                let mut v = head.to_vec();
                for _ in 0..len {
                    x = x.wrapping_mul(1103515245).wrapping_add(12345);
                    v.push((x >> 16) as u8);
                }
                let _ = decompress(&v, kind, 1 << 20);
            }
        }
    }

    /// Real tools read what we write and we read what they write (skipped
    /// where `compress`, `gzip`, `bzip2` or `xz` are not installed).
    #[test]
    fn interoperates_with_the_system_tools() {
        let data = sample(200_000);
        let dir = std::env::temp_dir().join(format!("rustfig-g10-interop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (tool, ext, kind) in [
            ("compress", "Z", Compression::Compress),
            ("gzip", "gz", Compression::Gzip),
            ("bzip2", "bz2", Compression::Bzip2),
            ("xz", "xz", Compression::Xz),
        ] {
            let plain = dir.join("p.txt");
            std::fs::write(&plain, &data).unwrap();
            let packed = dir.join(format!("p.txt.{ext}"));
            let _ = std::fs::remove_file(&packed);
            let Ok(status) = std::process::Command::new(tool).arg("-f").arg(&plain).status() else {
                continue;
            };
            if !status.success() {
                continue;
            }
            let theirs = std::fs::read(&packed).unwrap();
            assert_eq!(decompress(&theirs, kind, 1 << 30).unwrap(), data, "reading {tool}");
            // And back: the tool decompresses ours.
            let ours = dir.join(format!("o.txt.{ext}"));
            std::fs::write(&ours, compress(&data, kind).unwrap()).unwrap();
            let un = match tool {
                "compress" => ("uncompress", vec!["-f"]),
                "gzip" => ("gzip", vec!["-df"]),
                "bzip2" => ("bzip2", vec!["-df"]),
                _ => ("xz", vec!["-df"]),
            };
            let status = std::process::Command::new(un.0)
                .args(&un.1)
                .arg(&ours)
                .status()
                .unwrap();
            assert!(status.success(), "{tool} reading ours");
            assert_eq!(std::fs::read(dir.join("o.txt")).unwrap(), data, "{tool}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
