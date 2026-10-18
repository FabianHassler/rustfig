//! EPS previews and the user bounding box of fig2dev's PostScript driver:
//! `-A` (ASCII, an EPSI `%%BeginPreview` block), `-T` and `-C` (a TIFF
//! preview behind a DOS EPS binary header) and `-B`/`-R` (the
//! `%%BoundingBox`).
//!
//! fig2dev has ghostscript render the preview (`bit`, `tifflzw` or
//! `tiff24nc` at 72 dpi on a page of the bounding box's size plus one point
//! each way). Here the scene rasteriser draws it. The pixels may differ
//! from ghostscript's (no halftoning: the monochrome previews threshold the
//! luminance); the sizes and the containers are fig2dev's.
//
// Ported from fig2dev dev/genps.c (`genps_start`: user bounding box;
// `genps_end`: preview; `appendhex`, `putword`) (xfig licence, see
// LICENSE-xfig).

use crate::gendev::strtod_prefix;
use std::collections::HashMap;

/// The kinds of preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    /// `-A`: hex in the PostScript itself.
    Ascii,
    /// `-T`: a one-bit TIFF.
    TiffMono,
    /// `-C`: a 24-bit TIFF.
    TiffColor,
}

/// `-B` or `-R`: the text of the option.
#[derive(Debug, Clone, Copy)]
pub struct UserBox<'a> {
    pub spec: &'a str,
    /// `-B` (the origin is the figure's own, not the figure's corner).
    pub absolute: bool,
}

/// What `sscanf(s, "%lf %lf %lf %lf", ...)` reads: up to four numbers.
fn scan_four(s: &str) -> Vec<f64> {
    let mut v = Vec::new();
    let mut rest = s;
    while v.len() < 4 {
        let Some((x, used)) = strtod_prefix(rest) else {
            break;
        };
        v.push(x);
        let Some(r) = rest.get(used..) else { break };
        rest = r;
    }
    v
}

/// C's `(int)` of a double: truncation, saturating, 0 for NaN.
fn to_int(v: f64) -> i32 {
    v as i32
}

/// The `%%BoundingBox` of `-B` and `-R` as `genps_start` computes it.
/// `fllx..fury` is the figure's box in points, border included (y down),
/// `border` the integer border in points.
#[allow(clippy::too_many_arguments)]
pub fn user_bbox(b: UserBox<'_>, metric: bool, fllx: f64, flly: f64, furx: f64, fury: f64, border: i32) -> [i32; 4] {
    const PT: f64 = 72.0;
    let n = scan_four(b.spec);
    let mut width_x = n.first().copied().unwrap_or(0.0);
    let mut width_y = n.get(1).copied().unwrap_or(0.0);
    let mut orig_x = n.get(2).copied().unwrap_or(0.0);
    let mut orig_y = n.get(3).copied().unwrap_or(0.0);
    let cm = if metric { 2.54 } else { 1.0 };
    if n.is_empty() {
        width_x = (furx - fllx) / PT * cm;
    }
    if n.len() < 2 {
        width_y = (fury - flly) / PT * cm;
    }
    if width_x <= 0.0 {
        width_x = (furx - fllx) / PT * cm;
    }
    if width_y <= 0.0 {
        width_y = (fury - flly) / PT * cm;
    }
    orig_x *= PT;
    orig_y *= PT;
    width_x *= PT;
    width_y *= PT;
    if metric {
        orig_x /= 2.54;
        orig_y /= 2.54;
        width_x /= 2.54;
        width_y /= 2.54;
    }
    let mut llx = to_int(orig_x.floor());
    let mut lly = to_int(orig_y.floor());
    let mut urx = to_int((orig_x + width_x).ceil());
    let mut ury = to_int((orig_y + width_y).ceil());
    llx = llx.saturating_sub(border);
    lly = lly.saturating_sub(border);
    urx = urx.saturating_add(border);
    ury = ury.saturating_add(border);
    if b.absolute {
        // `userllx += origx` with `origx = -fllx`, in double arithmetic.
        let origx = -fllx;
        llx = to_int(f64::from(llx) + origx);
        urx = to_int(f64::from(urx) + origx);
    }
    [llx, lly, urx, ury]
}

/// Replace the `%%BoundingBox` line of EPS text (and drop the hi-res one,
/// which would contradict it).
pub fn set_bbox(ps: &str, bb: [i32; 4]) -> String {
    let mut out = String::with_capacity(ps.len());
    let mut done = false;
    for line in ps.split_inclusive('\n') {
        if line.starts_with("%%HiResBoundingBox:") {
            continue;
        }
        if !done && line.starts_with("%%BoundingBox:") {
            done = true;
            out.push_str(&format!("%%BoundingBox: {} {} {} {}\n", bb[0], bb[1], bb[2], bb[3]));
            continue;
        }
        out.push_str(line);
    }
    out
}

/// The rendered preview: `width` x `height` pixels, as pixels (RGB, three
/// bytes each) over white.
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

impl Canvas {
    /// A white canvas.
    pub fn white(width: u32, height: u32) -> Option<Canvas> {
        let n = (width as usize).checked_mul(height as usize)?.checked_mul(3)?;
        Some(Canvas {
            width,
            height,
            rgb: vec![255; n],
        })
    }

    /// Paste `src` (RGB, `w` x `h`) with its lower left corner `x` pixels
    /// from the left and `y` pixels from the bottom (PostScript's origin).
    pub fn paste(&mut self, src: &[u8], w: u32, h: u32, x: i64, y: i64) {
        let top = i64::from(self.height) - y - i64::from(h);
        for sy in 0..i64::from(h) {
            let dy = top + sy;
            if dy < 0 || dy >= i64::from(self.height) {
                continue;
            }
            for sx in 0..i64::from(w) {
                let dx = x + sx;
                if dx < 0 || dx >= i64::from(self.width) {
                    continue;
                }
                let s = (sy as usize * w as usize + sx as usize) * 3;
                let d = (dy as usize * self.width as usize + dx as usize) * 3;
                if let (Some(from), Some(to)) = (src.get(s..s + 3), self.rgb.get_mut(d..d + 3)) {
                    to.copy_from_slice(from);
                }
            }
        }
    }

    /// One bit per pixel, rows padded to whole bytes, most significant bit
    /// first, 1 for black (ghostscript's `bit` device): luminance below
    /// the middle is black.
    pub fn mono_bits(&self) -> Vec<u8> {
        let stride = (self.width as usize).div_ceil(8);
        let mut out = vec![0u8; stride * self.height as usize];
        for (y, row) in self.rgb.chunks_exact((self.width as usize * 3).max(1)).enumerate() {
            for (x, p) in row.as_chunks::<3>().0.iter().enumerate() {
                let [r, g, b] = p.map(f64::from);
                if 0.3 * r + 0.59 * g + 0.11 * b < 127.5 {
                    if let Some(byte) = out.get_mut(y * stride + x / 8) {
                        *byte |= 0x80 >> (x % 8);
                    }
                }
            }
        }
        out
    }
}

/// `appendhex`: `height` lines of `(width + 7) / 8` bytes as hex, each
/// behind `% `, read one after the other from the raw rows. (`width` and
/// `height` are one less than the page's, as in `genps_end`, so with
/// ghostscript's row padding the reading drifts when the two strides
/// differ, as it does in fig2dev.)
pub fn append_hex(raw: &[u8], width: i32, height: i32) -> String {
    use std::fmt::Write as _;
    let len = (i64::from(width) + 7).div_euclid(8).max(0) as usize;
    let mut out = String::new();
    let mut pos = 0usize;
    for _ in 0..height.max(0) {
        out.push_str("% ");
        for _ in 0..len {
            let Some(&byte) = raw.get(pos) else { break };
            pos += 1;
            let _ = write!(out, "{byte:02X}"); // writing to a String cannot fail
        }
        out.push('\n');
    }
    out
}

/// A 32-bit value, least significant byte first (`putword`).
fn putword(out: &mut Vec<u8>, w: u32) {
    out.extend_from_slice(&w.to_le_bytes());
}

/// TIFF's LZW (most significant bit first, early change), as libtiff
/// writes it.
fn lzw(data: &[u8]) -> Vec<u8> {
    const CLEAR: u32 = 256;
    const EOI: u32 = 257;
    struct Bits {
        out: Vec<u8>,
        acc: u64,
        n: u32,
    }
    impl Bits {
        fn put(&mut self, code: u32, width: u32) {
            self.acc = (self.acc << width) | u64::from(code);
            self.n += width;
            while self.n >= 8 {
                self.n -= 8;
                self.out.push((self.acc >> self.n) as u8);
            }
            self.acc &= (1u64 << self.n) - 1;
        }
    }
    let mut w = Bits {
        out: Vec::new(),
        acc: 0,
        n: 0,
    };
    let mut table: HashMap<(u32, u8), u32> = HashMap::new();
    let (mut free, mut width) = (258u32, 9u32);
    w.put(CLEAR, width);
    let mut bytes = data.iter().copied();
    let Some(first) = bytes.next() else {
        w.put(EOI, width);
        if w.n > 0 {
            w.put(0, 8 - w.n);
        }
        return w.out;
    };
    let mut cur = u32::from(first);
    for b in bytes {
        if let Some(&code) = table.get(&(cur, b)) {
            cur = code;
            continue;
        }
        w.put(cur, width);
        table.insert((cur, b), free);
        free += 1;
        if free == 4094 {
            w.put(CLEAR, width);
            table.clear();
            free = 258;
            width = 9;
        } else if free > (1 << width) - 1 {
            width += 1;
        }
        cur = u32::from(b);
    }
    w.put(cur, width);
    // The decoder adds one more entry before it reads the end code.
    free += 1;
    if free > (1 << width) - 1 && width < 12 {
        width += 1;
    }
    w.put(EOI, width);
    if w.n > 0 {
        w.put(0, 8 - w.n);
    }
    w.out
}

/// A baseline TIFF as ghostscript's `tifflzw` (one bit, black is 1,
/// `min-is-white`) and `tiff24nc` (RGB) write it: one image, strips of
/// about 8 KiB, 72 dpi.
fn tiff(width: u32, height: u32, color: bool, pixels: &[u8]) -> Vec<u8> {
    let stride = if color {
        width as usize * 3
    } else {
        (width as usize).div_ceil(8)
    };
    let rows_per_strip = (8192 / stride.max(1)).max(1);
    let mut strips: Vec<Vec<u8>> = Vec::new();
    for rows in pixels.chunks((rows_per_strip * stride).max(1)) {
        strips.push(if color { rows.to_vec() } else { lzw(rows) });
    }
    let nstrips = strips.len().max(1) as u32;
    let software = b"RustFig fig2dev\0";
    // (tag, type, count, value); values over four bytes live after the
    // directory, at `extra[i]`.
    struct Entry {
        tag: u16,
        typ: u16,
        count: u32,
        value: Vec<u8>,
    }
    let short = |v: u16| v.to_le_bytes().to_vec();
    let long = |v: u32| v.to_le_bytes().to_vec();
    let mut entries = vec![
        Entry {
            tag: 254,
            typ: 4,
            count: 1,
            value: long(2),
        },
        Entry {
            tag: 256,
            typ: 3,
            count: 1,
            value: short(width.min(65535) as u16),
        },
        Entry {
            tag: 257,
            typ: 3,
            count: 1,
            value: short(height.min(65535) as u16),
        },
        Entry {
            tag: 258,
            typ: 3,
            count: if color { 3 } else { 1 },
            value: if color {
                [8u16, 8, 8].iter().flat_map(|v| v.to_le_bytes()).collect()
            } else {
                short(1)
            },
        },
        Entry {
            tag: 259,
            typ: 3,
            count: 1,
            value: short(if color { 1 } else { 5 }),
        },
        Entry {
            tag: 262,
            typ: 3,
            count: 1,
            value: short(if color { 2 } else { 0 }),
        },
        Entry {
            tag: 266,
            typ: 3,
            count: 1,
            value: short(1),
        },
        Entry {
            tag: 273,
            typ: 4,
            count: nstrips,
            value: Vec::new(),
        },
        Entry {
            tag: 274,
            typ: 3,
            count: 1,
            value: short(1),
        },
        Entry {
            tag: 277,
            typ: 3,
            count: 1,
            value: short(if color { 3 } else { 1 }),
        },
        Entry {
            tag: 278,
            typ: 3,
            count: 1,
            value: short(rows_per_strip.min(65535) as u16),
        },
        Entry {
            tag: 279,
            typ: 4,
            count: nstrips,
            value: Vec::new(),
        },
        Entry {
            tag: 282,
            typ: 5,
            count: 1,
            value: [72u32, 1].iter().flat_map(|v| v.to_le_bytes()).collect(),
        },
        Entry {
            tag: 283,
            typ: 5,
            count: 1,
            value: [72u32, 1].iter().flat_map(|v| v.to_le_bytes()).collect(),
        },
        Entry {
            tag: 284,
            typ: 3,
            count: 1,
            value: short(1),
        },
        Entry {
            tag: 296,
            typ: 3,
            count: 1,
            value: short(2),
        },
        Entry {
            tag: 297,
            typ: 3,
            count: 2,
            value: [0u16, 0].iter().flat_map(|v| v.to_le_bytes()).collect(),
        },
        Entry {
            tag: 305,
            typ: 2,
            count: software.len() as u32,
            value: software.to_vec(),
        },
    ];
    let dir_end = 8 + 2 + entries.len() * 12 + 4;
    // Where each out-of-line value goes.
    let mut at = dir_end;
    let mut offsets: Vec<Option<usize>> = Vec::new();
    for e in &entries {
        let size = match e.tag {
            273 | 279 => 4 * nstrips as usize,
            _ => e.value.len(),
        };
        if size > 4 {
            offsets.push(Some(at));
            at += size;
        } else {
            offsets.push(None);
        }
    }
    let mut strip_at = at;
    let mut strip_offsets = Vec::new();
    let mut strip_counts = Vec::new();
    for s in &strips {
        strip_offsets.push(strip_at as u32);
        strip_counts.push(s.len() as u32);
        strip_at += s.len();
    }
    for e in &mut entries {
        let list = match e.tag {
            273 => &strip_offsets,
            279 => &strip_counts,
            _ => continue,
        };
        e.value = list.iter().flat_map(|v| v.to_le_bytes()).collect();
    }
    let mut out = vec![b'I', b'I', 42, 0];
    putword(&mut out, 8);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (e, off) in entries.iter().zip(&offsets) {
        out.extend_from_slice(&e.tag.to_le_bytes());
        out.extend_from_slice(&e.typ.to_le_bytes());
        putword(&mut out, e.count);
        match off {
            Some(o) => putword(&mut out, *o as u32),
            None => {
                let mut v = e.value.clone();
                v.resize(4, 0);
                out.extend_from_slice(&v);
            }
        }
    }
    putword(&mut out, 0);
    for (e, off) in entries.iter().zip(&offsets) {
        if off.is_some() {
            out.extend_from_slice(&e.value);
        }
    }
    for s in &strips {
        out.extend_from_slice(s);
    }
    out
}

/// Put the preview into the PostScript `ps` (an EPS, or a PostScript page
/// document) as `genps_end` does. The TIFF previews come with the DOS EPS
/// header (`C5D0D3C6`, the PostScript part at byte 30), the PostScript up
/// to its `%%Trailer`, the TIFF, a newline and then the trailer.
pub fn attach(kind: PreviewKind, ps: &str, canvas: &Canvas, eps_flag: bool) -> Result<Vec<u8>, String> {
    match kind {
        PreviewKind::Ascii => {
            // After the header comments (after the setup of a PostScript
            // page document), ahead of the rest.
            let marker = if eps_flag { "%%EndComments\n" } else { "%%EndSetup\n" };
            let Some((head, rest)) = ps.find(marker).and_then(|i| ps.split_at_checked(i + marker.len())) else {
                return Err("no place for the preview in the PostScript".into());
            };
            let (w, h) = (canvas.width as i32 - 1, canvas.height as i32 - 1);
            let mut out = String::with_capacity(ps.len() + canvas.rgb.len() / 8);
            out.push_str(head);
            out.push_str(&format!("%%BeginPreview: {w} {h} 1 {h}\n"));
            out.push_str(&append_hex(&canvas.mono_bits(), w, h));
            out.push_str("%%EndPreview\n");
            out.push_str(rest);
            Ok(out.into_bytes())
        }
        PreviewKind::TiffMono | PreviewKind::TiffColor => {
            let color = kind == PreviewKind::TiffColor;
            let tif = tiff(
                canvas.width,
                canvas.height,
                color,
                &if color { canvas.rgb.clone() } else { canvas.mono_bits() },
            );
            let cut = ps.rfind("%%Trailer\n").unwrap_or(ps.len());
            let (body, trailer) = ps.split_at(cut);
            let (epslen, tiflen) = (body.len() as u32, tif.len() as u32);
            let mut out = vec![0xC5, 0xD0, 0xD3, 0xC6];
            putword(&mut out, 30);
            putword(&mut out, epslen);
            putword(&mut out, 0);
            putword(&mut out, 0);
            putword(&mut out, epslen.saturating_add(30));
            putword(&mut out, tiflen);
            out.extend_from_slice(&[0xFF, 0xFF]);
            out.extend_from_slice(body.as_bytes());
            out.extend_from_slice(&tif);
            out.push(b'\n');
            out.extend_from_slice(trailer.as_bytes());
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_up_to_four_numbers() {
        assert_eq!(scan_four("2 1.5 x 3"), vec![2.0, 1.5]);
        assert_eq!(scan_four(""), Vec::<f64>::new());
        assert_eq!(scan_four("1 2 3 4 5").len(), 4);
    }

    #[test]
    fn user_box_defaults_to_the_figure() {
        let b = UserBox {
            spec: "",
            absolute: false,
        };
        assert_eq!(user_bbox(b, false, 0.0, 0.0, 30.5, 12.2, 0), [0, 0, 31, 13]);
        let b = UserBox {
            spec: "1 2 0.5 0.25",
            absolute: false,
        };
        assert_eq!(user_bbox(b, false, 0.0, 0.0, 30.5, 12.2, 2), [34, 16, 110, 164]);
    }

    #[test]
    fn hex_reads_rows_in_turn() {
        assert_eq!(
            append_hex(&[0x3f, 0xff, 0x00, 0x0e, 1, 2, 3, 4], 31, 2),
            "% 3FFF000E\n% 01020304\n"
        );
        assert_eq!(append_hex(&[1], 16, 2), "% 01\n% \n");
    }

    #[test]
    fn lzw_round_trips() {
        // A minimal TIFF LZW decoder.
        fn decode(data: &[u8]) -> Vec<u8> {
            let mut table: Vec<Vec<u8>> = Vec::new();
            let reset = |t: &mut Vec<Vec<u8>>| {
                t.clear();
                for i in 0..=255u8 {
                    t.push(vec![i]);
                }
                t.push(vec![]);
                t.push(vec![]);
            };
            reset(&mut table);
            let (mut out, mut acc, mut n, mut width) = (Vec::new(), 0u32, 0u32, 9u32);
            let mut prev: Option<Vec<u8>> = None;
            for &b in data {
                acc = (acc << 8) | u32::from(b);
                n += 8;
                while n >= width {
                    let code = (acc >> (n - width)) & ((1 << width) - 1);
                    n -= width;
                    acc &= (1 << n) - 1;
                    if code == 256 {
                        reset(&mut table);
                        width = 9;
                        prev = None;
                        continue;
                    }
                    if code == 257 {
                        return out;
                    }
                    let entry = match table.get(code as usize) {
                        Some(e) => e.clone(),
                        None => {
                            let mut e = prev.clone().unwrap();
                            e.push(e[0]);
                            e
                        }
                    };
                    out.extend_from_slice(&entry);
                    if let Some(mut p) = prev.take() {
                        p.push(entry[0]);
                        table.push(p);
                    }
                    prev = Some(entry);
                    let len = table.len() as u32;
                    width = if len + 1 > 2047 {
                        12
                    } else if len + 1 > 1023 {
                        11
                    } else if len + 1 > 511 {
                        10
                    } else {
                        9
                    };
                }
            }
            out
        }
        let mut data = Vec::new();
        for i in 0..30_000u32 {
            data.push(((i * i / 7) % 251) as u8);
            if i % 5 == 0 {
                data.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
        assert_eq!(decode(&lzw(&data)), data);
        assert_eq!(decode(&lzw(&[])), Vec::<u8>::new());
        assert_eq!(decode(&lzw(&[7])), vec![7]);
    }

    #[test]
    fn dos_header_points_at_the_parts() {
        let mut c = Canvas::white(32, 14).unwrap();
        c.paste(&[0; 3], 1, 1, 5, 5);
        let ps = "%!PS-Adobe-3.0 EPSF-3.0\n%%EndComments\nshowpage\n%%Trailer\n%%EOF\n";
        let out = attach(PreviewKind::TiffMono, ps, &c, true).unwrap();
        let w = |i: usize| u32::from_le_bytes([out[i], out[i + 1], out[i + 2], out[i + 3]]) as usize;
        assert_eq!(&out[..4], &[0xC5, 0xD0, 0xD3, 0xC6]);
        assert_eq!(w(4), 30);
        assert_eq!(&out[30..30 + w(8)], &ps.as_bytes()[..w(8)]);
        assert_eq!(w(20), w(8) + 30);
        assert_eq!(&out[w(20)..w(20) + 4], b"II*\0");
        assert_eq!(&out[w(20) + w(24)..], b"\n%%Trailer\n%%EOF\n");
    }
}
