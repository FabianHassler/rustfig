//! Reading imported pictures for the drivers that embed them (emf, tk,
//! ptk): find the file on disk (also compressed, `name.gz` for `name`),
//! uncompress it, recognise its type, and read its size and pixels into a
//! [`Pic`] the way fig2dev's `read_*` functions fill `F_pic`.
//!
//! - [`open_stream`] (`open_stream`, `file_on_disk`): relative names are
//!   taken relative to [`Ctx::dir`] (the directory of the Fig file; C uses
//!   the working directory). `.gz`, `.Z` and `.z` files are uncompressed
//!   here (gzip and Unix `compress` LZW, recognised by their magic
//!   numbers), where C pipes them through `gunzip -c`. `.zip`, `.bz2`,
//!   `.bz` and `.xz` files are found but reported as unsupported: RustFig
//!   does not run external programs.
//! - [`pic_type`]: the file signatures genps.c recognises.
//! - [`read_pic`]: fills `Pic`. A picture with ≤ 256 colours becomes an
//!   indexed bitmap (one byte per pixel, `cmap`, `numcols`), one with more
//!   an RGB bitmap (three bytes per pixel, `numcols` = 2^24), as
//!   `read_png` does for all formats. An alpha channel is composited over
//!   the background (white, or `-g`) and that colour becomes the
//!   transparent colour (`TRANSP_COLOR`), as in `read_png`. `-N`
//!   (`grayonly`) maps colours to grey. XBM gives the packed bits
//!   (`numcols` 0), EPS and PDF only their bounding box.
//!
//! PNG, JPEG, GIF, TIFF and PNM are decoded by the `image` crate; PCX by
//! a port of readpcx.c; XBM and XPM by small readers written for RustFig
//! (C converts XPM with external programs; its XBM reader derives from
//! X11's `XReadBitmapFromFile`). Sizes are capped ([`MAX_PICFILE_BYTES`],
//! [`MAX_PIXELS`]); hostile files give an error, never a panic.
//
// Ported from fig2dev dev/readpics.c, dev/readpics.h, dev/readeps.c,
// dev/readpcx.c, dev/readpng.c and the picture signatures of dev/genps.c
// (xfig licence, see LICENSE-xfig).

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::gendev::Ctx;
use crate::object::{Pic, Point, P_EPS, P_GIF, P_JPEG, P_PCX, P_PNG, P_PPM, P_TIF, P_XBM, P_XPM};

/// Largest picture file read, after uncompressing (256 MiB).
pub const MAX_PICFILE_BYTES: usize = 256 << 20;
/// Largest picture, in pixels (32 Mi).
pub const MAX_PIXELS: u64 = 1 << 25;
/// Largest side of a picture, in pixels.
pub const MAX_SIDE: u32 = 1 << 16;

/// `num_transp` values (object.h): no transparency, or the transparent
/// colour given in `transp_col`.
pub const NO_TRANSPARENCY: i32 = 0;
pub const TRANSP_COLOR: i32 = -1;
/// `numcols` of an RGB bitmap.
pub const RGB_NUMCOLS: i32 = 1 << 24;

/// How a picture file on disk is compressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    None,
    /// `.gz`, `.Z`, `.z`: gzip or `compress`, told apart by content.
    Gzip,
    Zip,
    Bzip2,
    Xz,
}

/// Compression suffixes, in fig2dev's order (`filetypes`).
const SUFFIXES: [(&str, Compression); 7] = [
    (".gz", Compression::Gzip),
    (".Z", Compression::Gzip),
    (".z", Compression::Gzip),
    (".zip", Compression::Zip),
    (".bz2", Compression::Bzip2),
    (".bz", Compression::Bzip2),
    (".xz", Compression::Xz),
];

/// An opened picture (`struct xfig_stream`), its content uncompressed.
#[derive(Clone, Debug)]
pub struct PicStream {
    /// The name in the Fig file.
    pub name: String,
    /// The file read, e.g. `image.ppm.gz` for `image.ppm`.
    pub path: PathBuf,
    pub compression: Compression,
    /// The uncompressed content.
    pub data: Vec<u8>,
}

/// The path of picture `name`: relative names relative to `ctx.dir`.
pub fn pic_path(ctx: &Ctx, name: &str) -> PathBuf {
    let p = Path::new(name);
    match &ctx.dir {
        Some(dir) if p.is_relative() => dir.join(p),
        _ => p.to_path_buf(),
    }
}

fn compression_of(name: &str) -> Compression {
    let suffix = name.rfind('.').and_then(|i| name.get(i..)).unwrap_or("");
    SUFFIXES
        .iter()
        .find(|(s, _)| *s == suffix)
        .map_or(Compression::None, |&(_, c)| c)
}

/// `file_on_disk`: the file that holds picture `path`: the file itself, or
/// with a compression suffix added, or — if `path` has a compression
/// suffix but only the uncompressed file exists — that one.
pub fn file_on_disk(path: &Path) -> Option<(PathBuf, Compression)> {
    let name = path.to_string_lossy();
    if path.exists() {
        return Some((path.to_path_buf(), compression_of(&name)));
    }
    for (suffix, c) in SUFFIXES {
        let mut s = path.as_os_str().to_os_string();
        s.push(suffix);
        let p = PathBuf::from(s);
        if p.exists() {
            return Some((p, c));
        }
    }
    let i = name.rfind('.')?;
    let suffix = name.get(i..)?;
    if SUFFIXES.iter().any(|(s, _)| *s == suffix) {
        let p = PathBuf::from(name.get(..i)?);
        if p.exists() {
            return Some((p, Compression::None));
        }
    }
    None
}

/// `open_stream`: finds, reads and uncompresses picture `name`.
pub fn open_stream(ctx: &Ctx, name: &str) -> Result<PicStream, String> {
    let wanted = pic_path(ctx, name);
    let (path, compression) = file_on_disk(&wanted).ok_or_else(|| format!("{name}: No such picture file"))?;
    let file = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut raw = Vec::new();
    file.take(MAX_PICFILE_BYTES as u64 + 1)
        .read_to_end(&mut raw)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if raw.len() > MAX_PICFILE_BYTES {
        return Err(format!("{}: picture file too large", path.display()));
    }
    let data = uncompress(&raw, compression).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(PicStream {
        name: name.to_string(),
        path,
        compression,
        data,
    })
}

/// Uncompresses `data`.
pub fn uncompress(data: &[u8], c: Compression) -> Result<Vec<u8>, String> {
    match c {
        Compression::None => Ok(data.to_vec()),
        Compression::Gzip => match data {
            [0x1f, 0x8b, ..] => gunzip(data),
            [0x1f, 0x9d, ..] => uncompress_lzw(data),
            _ => Err("not in gzip or compress format".into()),
        },
        Compression::Zip => Err("zip-compressed pictures are not supported".into()),
        Compression::Bzip2 => Err("bzip2-compressed pictures are not supported".into()),
        Compression::Xz => Err("xz-compressed pictures are not supported".into()),
    }
}

/// Uncompresses a gzip file (its first member).
pub fn gunzip(data: &[u8]) -> Result<Vec<u8>, String> {
    const FHCRC: u8 = 2;
    const FEXTRA: u8 = 4;
    const FNAME: u8 = 8;
    const FCOMMENT: u8 = 16;
    let bad = || "corrupt gzip header".to_string();
    if data.get(..3) != Some(&[0x1f, 0x8b, 8][..]) {
        return Err(bad());
    }
    let flags = *data.get(3).ok_or_else(bad)?;
    let mut pos = 10usize;
    if flags & FEXTRA != 0 {
        let lo = *data.get(pos).ok_or_else(bad)?;
        let hi = *data.get(pos + 1).ok_or_else(bad)?;
        pos += 2 + usize::from(u16::from_le_bytes([lo, hi]));
    }
    for flag in [FNAME, FCOMMENT] {
        if flags & flag != 0 {
            let len = data
                .get(pos..)
                .ok_or_else(bad)?
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(bad)?;
            pos += len + 1;
        }
    }
    if flags & FHCRC != 0 {
        pos += 2;
    }
    let body = data.get(pos..).ok_or_else(bad)?;
    miniz_oxide::inflate::decompress_to_vec_with_limit(body, MAX_PICFILE_BYTES)
        .map_err(|e| format!("corrupt gzip data ({:?})", e.status))
}

/// Uncompresses a Unix `compress` (`.Z`) file, as gzip's `unlzw` does.
pub fn uncompress_lzw(data: &[u8]) -> Result<Vec<u8>, String> {
    const INIT_BITS: u32 = 9;
    const CLEAR: usize = 256;
    const FIRST: usize = 257;
    let corrupt = || "corrupt compress data".to_string();
    if data.get(..2) != Some(&[0x1f, 0x9d][..]) {
        return Err("not in compress format".into());
    }
    let flags = *data.get(2).ok_or_else(corrupt)?;
    let maxbits = u32::from(flags & 0x1f);
    let block_mode = flags & 0x80 != 0;
    if !(INIT_BITS..=16).contains(&maxbits) {
        return Err(format!("compressed with {maxbits} bits, can only handle 9 to 16"));
    }
    let body = data.get(3..).unwrap_or(&[]);
    let total_bits = body.len() as u64 * 8;
    let maxmaxcode = 1usize << maxbits;

    let mut prefix = vec![0u16; maxmaxcode];
    let mut suffix = vec![0u8; maxmaxcode];
    for (i, s) in suffix.iter_mut().enumerate().take(256) {
        *s = i as u8;
    }
    let mut out = Vec::new();
    let mut stack = Vec::new();
    let mut n_bits = INIT_BITS;
    let mut maxcode = (1usize << n_bits) - 1;
    let mut free_ent = if block_mode { FIRST } else { 256 };
    let mut oldcode: Option<usize> = None;
    let mut finchar = 0u8;
    let mut pos: u64 = 0;
    let mut group_start: u64 = 0;

    // Codes come in groups of eight; a new code width or a clear code
    // skips the rest of the group.
    let align = |pos: u64, group_start: u64, n_bits: u32| -> u64 {
        let n = u64::from(n_bits) * 8;
        let rel = pos - group_start;
        group_start + rel.div_ceil(n) * n
    };

    while pos + u64::from(n_bits) <= total_bits {
        if free_ent > maxcode {
            pos = align(pos, group_start, n_bits);
            group_start = pos;
            n_bits += 1;
            maxcode = if n_bits == maxbits {
                maxmaxcode
            } else {
                (1usize << n_bits) - 1
            };
            continue;
        }
        let byte = (pos / 8) as usize;
        let mut word = 0u32;
        for k in 0..3 {
            word |= u32::from(body.get(byte + k).copied().unwrap_or(0)) << (8 * k);
        }
        let code = ((word >> (pos % 8)) & ((1 << n_bits) - 1)) as usize;
        pos += u64::from(n_bits);

        let Some(old) = oldcode else {
            if code >= 256 {
                return Err(corrupt());
            }
            finchar = code as u8;
            out.push(finchar);
            oldcode = Some(code);
            continue;
        };
        if code == CLEAR && block_mode {
            free_ent = FIRST - 1;
            pos = align(pos, group_start, n_bits);
            group_start = pos;
            n_bits = INIT_BITS;
            maxcode = (1usize << n_bits) - 1;
            continue;
        }
        let incode = code;
        let mut code = code;
        stack.clear();
        if code >= free_ent {
            // The KwKwK case.
            if code > free_ent {
                return Err(corrupt());
            }
            stack.push(finchar);
            code = old;
        }
        while code >= 256 {
            stack.push(*suffix.get(code).ok_or_else(corrupt)?);
            code = usize::from(*prefix.get(code).ok_or_else(corrupt)?);
            if stack.len() > maxmaxcode {
                return Err(corrupt());
            }
        }
        finchar = *suffix.get(code).ok_or_else(corrupt)?;
        stack.push(finchar);
        if out.len() + stack.len() > MAX_PICFILE_BYTES {
            return Err("uncompressed picture too large".into());
        }
        out.extend(stack.iter().rev());
        if free_ent < maxmaxcode {
            if let (Some(p), Some(s)) = (prefix.get_mut(free_ent), suffix.get_mut(free_ent)) {
                *p = old as u16;
                *s = finchar;
            }
            free_ent += 1;
        }
        oldcode = Some(incode);
    }
    Ok(out)
}

/// Picture file types fig2dev recognises (genps.c `headers[]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PicType {
    Gif,
    Pcx,
    Eps,
    /// EPS with a DOS binary header.
    Epsi,
    Pdf,
    Ppm,
    Tiff,
    Xbm,
    Png,
    Jpeg,
    Xpm,
}

impl PicType {
    /// The name fig2dev gives the type.
    pub fn name(self) -> &'static str {
        match self {
            PicType::Gif => "GIF",
            PicType::Pcx => "PCX",
            PicType::Eps => "EPS",
            PicType::Epsi => "EPSI",
            PicType::Pdf => "PDF",
            PicType::Ppm => "PPM",
            PicType::Tiff => "TIFF",
            PicType::Xbm => "XBM",
            PicType::Png => "PNG",
            PicType::Jpeg => "JPEG",
            PicType::Xpm => "XPM",
        }
    }

    /// The `F_pic` subtype a picture of this type gets.
    pub fn subtype(self) -> i32 {
        match self {
            PicType::Gif => P_GIF,
            PicType::Pcx => P_PCX,
            PicType::Eps | PicType::Epsi | PicType::Pdf => P_EPS,
            PicType::Ppm => P_PPM,
            PicType::Tiff => P_TIF,
            PicType::Xbm => P_XBM,
            PicType::Png => P_PNG,
            PicType::Jpeg => P_JPEG,
            PicType::Xpm => P_XPM,
        }
    }
}

/// The type of a picture from its first bytes. PPM also accepts the other
/// PNM variants (P1–P5), which C rejects.
pub fn pic_type(data: &[u8]) -> Option<PicType> {
    const HEADERS: [(&[u8], PicType); 15] = [
        (b"GIF", PicType::Gif),
        (b"\x0a\x05\x01", PicType::Pcx),
        (b"%!", PicType::Eps),
        (b"\xc5\xd0\xd3\xc6", PicType::Epsi),
        (b"%PDF", PicType::Pdf),
        (b"P3", PicType::Ppm),
        (b"P6", PicType::Ppm),
        (b"II*\0", PicType::Tiff),
        (b"MM\0*", PicType::Tiff),
        (b"#define", PicType::Xbm),
        (b"\x89PNG\r\n\x1a\n", PicType::Png),
        (b"\xff\xd8\xff\xe0", PicType::Jpeg),
        (b"\xff\xd8\xff\xe1", PicType::Jpeg),
        (b"/* XPM */", PicType::Xpm),
        (b"\xff\xd8\xff", PicType::Jpeg),
    ];
    if let Some(&(_, t)) = HEADERS.iter().find(|(h, _)| data.starts_with(h)) {
        return Some(t);
    }
    match data {
        [b'P', b'1'..=b'5', ws, ..] if ws.is_ascii_whitespace() => Some(PicType::Ppm),
        _ => None,
    }
}

/// Reads picture `stream` into `pic` (the `read_*` functions): sets
/// `subtype`, `bit_size`, `hw_ratio` and, for bitmaps, `bitmap`, `cmap`,
/// `numcols`, `num_transp`, `transp_col`. Returns the lower left corner of
/// the bounding box (`llx`, `lly`; 0 except for EPS and PDF).
pub fn read_pic(ctx: &Ctx, stream: &PicStream, pic: &mut Pic) -> Result<(i32, i32), String> {
    let t = pic_type(&stream.data).ok_or_else(|| format!("{}: Unknown image format", stream.name))?;
    let r = match t {
        PicType::Eps | PicType::Epsi => read_eps(&stream.data, pic),
        PicType::Pdf => read_pdf(&stream.data, pic),
        PicType::Xbm => read_xbm(&stream.data, pic).map(|()| (0, 0)),
        PicType::Xpm => read_xpm(ctx, &stream.data, pic).map(|()| (0, 0)),
        PicType::Pcx => read_pcx(ctx, &stream.data, pic).map(|()| (0, 0)),
        PicType::Gif => read_raster(ctx, &stream.data, image::ImageFormat::Gif, t, pic),
        PicType::Ppm => read_raster(ctx, &stream.data, image::ImageFormat::Pnm, t, pic),
        PicType::Tiff => read_raster(ctx, &stream.data, image::ImageFormat::Tiff, t, pic),
        PicType::Png => read_raster(ctx, &stream.data, image::ImageFormat::Png, t, pic),
        PicType::Jpeg => read_raster(ctx, &stream.data, image::ImageFormat::Jpeg, t, pic),
    };
    r.map_err(|e| format!("{}: {e}", stream.name))
}

/// `open_stream` and `read_pic` in one.
pub fn read_picture(ctx: &Ctx, pic: &mut Pic) -> Result<(i32, i32), String> {
    let stream = open_stream(ctx, &pic.file)?;
    read_pic(ctx, &stream, pic)
}

fn check_size(w: u64, h: u64) -> Result<(), String> {
    if w == 0 || h == 0 {
        return Err("empty image".into());
    }
    if w > u64::from(MAX_SIDE) || h > u64::from(MAX_SIDE) || w * h > MAX_PIXELS {
        return Err(format!("image of {w}×{h} pixels is too large"));
    }
    Ok(())
}

fn set_size(pic: &mut Pic, w: u32, h: u32) {
    pic.bit_size = Point::new(w as i32, h as i32);
    pic.hw_ratio = h as f32 / w as f32;
}

/// `rgb2luminance` (colors.c) on 8-bit channels.
fn luminance(c: [u8; 3]) -> u8 {
    let [r, g, b] = c.map(|v| f64::from(v) / 255.0);
    ((0.3 * r + 0.59 * g + 0.11 * b) * 255.0) as u8
}

/// The background alpha is composited over: `-g`, or white.
fn background(ctx: &Ctx) -> [u8; 3] {
    if ctx.bgspec {
        let b = ctx.background;
        [(b.red >> 8) as u8, (b.green >> 8) as u8, (b.blue >> 8) as u8]
    } else {
        [255; 3]
    }
}

/// Fills `pic`'s bitmap from RGBA pixels: indexed with ≤ 256 colours, RGB
/// otherwise; alpha composited over the background.
pub fn fill_from_rgba(ctx: &Ctx, pic: &mut Pic, w: u32, h: u32, rgba: &[u8]) -> Result<(), String> {
    check_size(u64::from(w), u64::from(h))?;
    let n = (w as usize) * (h as usize);
    if rgba.len() < n * 4 {
        return Err("short image data".into());
    }
    let bg = background(ctx);
    let mut transparent = false;
    let mut pixels: Vec<[u8; 3]> = Vec::with_capacity(n);
    for px in rgba.as_chunks::<4>().0.iter().take(n) {
        let (c, a) = ([px[0], px[1], px[2]], u32::from(px[3]));
        let c = if a == 255 {
            c
        } else {
            transparent = true;
            let mix = |v: u8, b: u8| ((u32::from(v) * a + u32::from(b) * (255 - a) + 127) / 255) as u8;
            [mix(c[0], bg[0]), mix(c[1], bg[1]), mix(c[2], bg[2])]
        };
        pixels.push(if ctx.grayonly { [luminance(c); 3] } else { c });
    }
    pic.num_transp = if transparent { TRANSP_COLOR } else { NO_TRANSPARENCY };
    pic.transp_col = if transparent {
        if ctx.grayonly {
            [luminance(bg); 3]
        } else {
            bg
        }
    } else {
        [0; 3]
    };

    let mut index: HashMap<[u8; 3], u8> = HashMap::new();
    let mut cmap: Vec<[u8; 3]> = Vec::new();
    let mut indexed = Vec::with_capacity(n);
    for &c in &pixels {
        let i = match index.get(&c) {
            Some(&i) => i,
            None => {
                if cmap.len() == 256 {
                    cmap.clear();
                    break;
                }
                let i = cmap.len() as u8;
                index.insert(c, i);
                cmap.push(c);
                i
            }
        };
        indexed.push(i);
    }
    if indexed.len() == n {
        pic.bitmap = indexed;
        pic.numcols = cmap.len() as i32;
        pic.cmap = cmap;
    } else {
        pic.bitmap = pixels.concat();
        pic.cmap = Vec::new();
        pic.numcols = RGB_NUMCOLS;
    }
    set_size(pic, w, h);
    Ok(())
}

/// GIF, PNM, TIFF, PNG, JPEG through the `image` crate.
fn read_raster(
    ctx: &Ctx,
    data: &[u8],
    format: image::ImageFormat,
    t: PicType,
    pic: &mut Pic,
) -> Result<(i32, i32), String> {
    let mut reader = image::ImageReader::with_format(Cursor::new(data), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(MAX_PIXELS * 8);
    reader.limits(limits);
    let img = reader.decode().map_err(|e| e.to_string())?;
    check_size(u64::from(img.width()), u64::from(img.height()))?;
    let rgba = img.to_rgba8();
    fill_from_rgba(ctx, pic, rgba.width(), rgba.height(), rgba.as_raw())?;
    pic.subtype = t.subtype();
    Ok((0, 0))
}

/// The PostScript part of an EPS file, without a DOS binary header.
fn eps_section(data: &[u8]) -> &[u8] {
    if data.starts_with(b"\xc5\xd0\xd3\xc6") {
        let word = |i: usize| -> Option<usize> {
            let b = data.get(i..i + 4)?;
            usize::try_from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok()
        };
        if let (Some(start), Some(len)) = (word(4), word(8)) {
            let end = start.saturating_add(len).min(data.len());
            return data.get(start..end).unwrap_or(&[]);
        }
        return &[];
    }
    data
}

/// The first `n` numbers in `s` (as `sscanf("%lf %lf …")`).
fn numbers(s: &str, n: usize) -> Option<Vec<f64>> {
    let v: Vec<f64> = s
        .split_whitespace()
        .take(n)
        .map_while(|t| t.parse::<f64>().ok())
        .collect();
    (v.len() == n && v.iter().all(|x| x.is_finite())).then_some(v)
}

/// `read_eps`: the bounding box of an EPS file (or EPSI). Without a
/// `%%BoundingBox`, the picture is 10 × 10 points at the origin.
pub fn read_eps(data: &[u8], pic: &mut Pic) -> Result<(i32, i32), String> {
    let ps = eps_section(data);
    pic.subtype = P_EPS;
    pic.bit_size = Point::new(10, 10);
    let mut ll = (0, 0);
    let mut nested = 0i32;
    for line in ps.split(|&b| b == b'\n') {
        if nested == 0 && line.starts_with(b"%%BoundingBox:") {
            let s = String::from_utf8_lossy(line);
            if s.contains("(atend)") {
                continue;
            }
            let v = s
                .split_once(':')
                .and_then(|(_, r)| numbers(r, 4))
                .ok_or("Bad EPS file")?;
            ll = (v[0].floor() as i32, v[1].floor() as i32);
            pic.bit_size = Point::new((v[2] - v[0]) as i32, (v[3] - v[1]) as i32);
            break;
        } else if line.starts_with(b"%%Begin") {
            nested += 1;
        } else if nested > 0 && line.starts_with(b"%%End") {
            nested -= 1;
        }
    }
    Ok(ll)
}

/// `read_pdf`: the first `/MediaBox` of a PDF. C asks ghostscript when
/// none is found; RustFig runs no external programs and fails.
pub fn read_pdf(data: &[u8], pic: &mut Pic) -> Result<(i32, i32), String> {
    const KEY: &[u8] = b"/MediaBox";
    pic.subtype = P_EPS;
    let fail = || "no /MediaBox found in the PDF".to_string();
    let at = data.windows(KEY.len()).position(|w| w == KEY).ok_or_else(fail)?;
    let rest = data.get(at + KEY.len()..).ok_or_else(fail)?;
    let rest = rest.get(..rest.len().min(512)).unwrap_or(rest);
    let s = String::from_utf8_lossy(rest);
    let s = s.trim_start().strip_prefix('[').ok_or_else(fail)?;
    let inner = s.split(']').next().unwrap_or("");
    let v = numbers(inner, 4).ok_or_else(fail)?;
    let (llx, lly) = (v[0].floor() as i32, v[1].floor() as i32);
    pic.bit_size = Point::new(
        (v[2].ceil() as i32).saturating_sub(llx),
        (v[3].ceil() as i32).saturating_sub(lly),
    );
    Ok((llx, lly))
}

/// An X bitmap: `height` rows of `(width + 7) / 8` bytes, leftmost pixel
/// in the most significant bit (C reverses the bits of the file's bytes).
/// Unlike C, X10 bitmaps (16-bit values) keep their high bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct XBitmap {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

/// `ReadFromBitmapFile`: reads an X11 (or X10) bitmap file.
pub fn read_bitmap_file(data: &[u8]) -> Option<XBitmap> {
    let text = String::from_utf8_lossy(data);
    let (mut w, mut h) = (0u32, 0u32);
    let mut body: Option<(usize, bool)> = None;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let mut tok = line.split_whitespace();
        if tok.next() == Some("#define") {
            let (Some(name), Some(value)) = (tok.next(), tok.next()) else {
                continue;
            };
            let kind = name.rsplit('_').next().unwrap_or(name);
            let Ok(v) = value.parse::<u32>() else {
                continue;
            };
            match kind {
                "width" => w = v,
                "height" => h = v,
                _ => {}
            }
            continue;
        }
        let Some(brace) = line.find('{') else {
            continue;
        };
        let decl = line.get(..brace).unwrap_or("");
        if decl.contains("static") && decl.contains("bits[]") {
            body = Some((start + brace + 1, decl.contains("short")));
            break;
        }
    }
    let (at, version10) = body?;
    if w == 0 || h == 0 || check_size(u64::from(w), u64::from(h)).is_err() {
        return None;
    }
    let padding = usize::from(version10 && w % 16 != 0 && w % 16 < 9);
    let bytes_per_line = (w as usize).div_ceil(8) + padding;
    let size = bytes_per_line * h as usize;
    let mut values = text
        .get(at..)?
        .split([' ', ',', '}', '\n', '\t', '\r', ';'])
        .filter(|t| !t.is_empty())
        .map(|t| {
            let digits = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
            u16::from_str_radix(digits, 16).ok()
        });
    let mut out = Vec::with_capacity(size);
    if version10 {
        // C's loop: two bytes per value; the padding byte at the end of a
        // line is dropped, so rows are (width + 7) / 8 bytes either way.
        let mut bytes = 0;
        while bytes < size {
            let v = values.next()??;
            out.push((v as u8).reverse_bits());
            if padding == 0 || (bytes + 2) % bytes_per_line != 0 {
                out.push(((v >> 8) as u8).reverse_bits());
            }
            bytes += 2;
        }
    } else {
        while out.len() < size {
            let v = values.next()??;
            out.push((v as u8).reverse_bits());
        }
    }
    Some(XBitmap {
        width: w,
        height: h,
        data: out,
    })
}

/// `read_xbm`.
pub fn read_xbm(data: &[u8], pic: &mut Pic) -> Result<(), String> {
    let b = read_bitmap_file(data).ok_or("not a valid X bitmap")?;
    pic.subtype = P_XBM;
    pic.numcols = 0;
    pic.bitmap = b.data;
    set_size(pic, b.width, b.height);
    Ok(())
}

/// The string literals of a C source (XPM), comments skipped.
fn c_strings(text: &str, max: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = ' ';
                for c in chars.by_ref() {
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
            }
            '"' => {
                let mut s = String::new();
                while let Some(c) = chars.next() {
                    match c {
                        '"' => break,
                        '\\' => {
                            if let Some(n) = chars.next() {
                                s.push(n);
                            }
                        }
                        _ => s.push(c),
                    }
                }
                out.push(s);
                if out.len() >= max {
                    break;
                }
            }
            _ => {}
        }
    }
    out
}

/// The colour of an XPM colour definition (after the pixel characters):
/// the `c` key, else `g`, `g4`, `m`; `None` is transparent.
fn xpm_color(def: &str) -> Option<[u8; 4]> {
    let mut values: Vec<(&str, String)> = Vec::new();
    for t in def.split_whitespace() {
        if matches!(t, "c" | "m" | "g" | "g4" | "s") {
            values.push((t, String::new()));
        } else if let Some((_, v)) = values.last_mut() {
            if !v.is_empty() {
                v.push(' ');
            }
            v.push_str(t);
        }
    }
    let value = ["c", "g", "g4", "m"].iter().find_map(|k| {
        values
            .iter()
            .find(|(key, v)| key == k && !v.is_empty())
            .map(|(_, v)| v.as_str())
    })?;
    if value.eq_ignore_ascii_case("none") {
        return Some([0, 0, 0, 0]);
    }
    let c = rustfig_model::x_color(value)?;
    Some([c.r, c.g, c.b, 255])
}

/// Reads an XPM (version 3) image (C converts it with netpbm or
/// ImageMagick). Unknown colour names are black.
pub fn read_xpm(ctx: &Ctx, data: &[u8], pic: &mut Pic) -> Result<(), String> {
    let text = String::from_utf8_lossy(data);
    let header = c_strings(&text, 1).into_iter().next().ok_or("not an XPM file")?;
    let v: Vec<u64> = header
        .split_whitespace()
        .take(4)
        .map_while(|t| t.parse().ok())
        .collect();
    let &[w, h, ncolors, cpp] = v.as_slice() else {
        return Err("bad XPM header".into());
    };
    check_size(w, h)?;
    if ncolors == 0 || ncolors > MAX_PIXELS || !(1..=8).contains(&cpp) {
        return Err("bad XPM header".into());
    }
    let (w, h, ncolors, cpp) = (w as usize, h as usize, ncolors as usize, cpp as usize);
    let strings = c_strings(&text, 1 + ncolors + h);
    if strings.len() < 1 + ncolors + h {
        return Err("XPM file too short".into());
    }
    let mut colors: HashMap<&[u8], [u8; 4]> = HashMap::with_capacity(ncolors);
    for def in strings.get(1..1 + ncolors).unwrap_or(&[]) {
        let key = def.as_bytes().get(..cpp).ok_or("bad XPM colour")?;
        let rest = def.get(cpp..).unwrap_or("");
        colors.insert(key, xpm_color(rest).unwrap_or([0, 0, 0, 255]));
    }
    let mut rgba = Vec::with_capacity(w * h * 4);
    for row in strings.get(1 + ncolors..1 + ncolors + h).unwrap_or(&[]) {
        let row = row.as_bytes();
        for x in 0..w {
            let key = row.get(x * cpp..(x + 1) * cpp).ok_or("XPM row too short")?;
            let c = colors.get(key).ok_or("undefined XPM pixel")?;
            rgba.extend_from_slice(c);
        }
    }
    fill_from_rgba(ctx, pic, w as u32, h as u32, &rgba)?;
    pic.subtype = P_XPM;
    Ok(())
}

/// `_read_pcx`: reads a PCX image, 1–4 bit planar, 8 bit indexed or 24
/// bit RGB. Unlike C, the scanline length is the header's (C uses the
/// width for 8 and 24 bit images, misreading padded lines).
pub fn read_pcx(ctx: &Ctx, data: &[u8], pic: &mut Pic) -> Result<(), String> {
    let hdr = data.get(..128).ok_or("PCX file too short")?;
    let at = |i: usize| usize::from(hdr.get(i).copied().unwrap_or(0));
    let word = |i: usize| at(i) + 256 * at(i + 1);
    if at(0) != 10 || at(2) != 1 {
        return Err("not a PCX file".into());
    }
    let (bpp, nplanes) = (at(3), at(65));
    let real_bpp = match (bpp, nplanes) {
        (1, 1..=4) => nplanes,
        (8, 1) => 8,
        (8, 3) => 24,
        _ => return Err("unsupported PCX format".into()),
    };
    let w = (word(8) as i64) - (word(4) as i64) + 1;
    let h = (word(10) as i64) - (word(6) as i64) + 1;
    if w <= 0 || h <= 0 {
        return Err("empty image".into());
    }
    let (w, h) = (w as usize, h as usize);
    check_size(w as u64, h as u64)?;
    let min_line = if bpp == 1 { w.div_ceil(8) } else { w };
    let byteline = word(66).max(min_line);
    let line = byteline * nplanes;

    // Uncompress the run-length encoded scanlines.
    let total = line * h;
    let mut raw = Vec::with_capacity(total);
    let mut pos = 128usize;
    while raw.len() < total {
        let Some(&b) = data.get(pos) else {
            break; // truncated: the rest stays 0
        };
        pos += 1;
        if b >= 192 {
            let v = data.get(pos).copied().unwrap_or(0);
            pos += 1;
            let n = usize::from(b & 63).min(total - raw.len());
            raw.extend(std::iter::repeat_n(v, n));
        } else {
            raw.push(b);
        }
    }
    raw.resize(total, 0);

    let gray = |c: [u8; 3]| if ctx.grayonly { [luminance(c); 3] } else { c };
    let mut bitmap = Vec::with_capacity(w * h * if real_bpp == 24 { 3 } else { 1 });
    for row in raw.chunks_exact(line) {
        for x in 0..w {
            match real_bpp {
                24 => {
                    for p in 0..3 {
                        bitmap.push(row.get(p * byteline + x).copied().unwrap_or(0));
                    }
                }
                8 => bitmap.push(row.get(x).copied().unwrap_or(0)),
                _ => {
                    let mut v = 0u8;
                    for p in 0..nplanes {
                        let byte = row.get(p * byteline + x / 8).copied().unwrap_or(0);
                        if byte & (0x80 >> (x % 8)) != 0 {
                            v |= 1 << p;
                        }
                    }
                    bitmap.push(v);
                }
            }
        }
    }
    match real_bpp {
        24 => {
            if ctx.grayonly {
                for px in bitmap.as_chunks_mut::<3>().0 {
                    let l = luminance([px[0], px[1], px[2]]);
                    px.fill(l);
                }
            }
            pic.cmap = Vec::new();
            pic.numcols = RGB_NUMCOLS;
        }
        8 => {
            // The palette follows the image data after a 0x0c byte; it is
            // also the last 769 bytes of the file.
            let tail = data
                .len()
                .checked_sub(769)
                .and_then(|i| data.get(i..))
                .filter(|t| t.first() == Some(&0x0c));
            let pal = tail
                .or_else(|| data.get(pos..))
                .map_or(&[][..], |p| p.get(1..).unwrap_or(&[]));
            pic.cmap = (0..256)
                .map(|i| {
                    let c = |k: usize| pal.get(i * 3 + k).copied().unwrap_or(0);
                    gray([c(0), c(1), c(2)])
                })
                .collect();
            pic.numcols = 256;
        }
        _ => {
            let n = 1usize << real_bpp;
            pic.cmap = (0..n)
                .map(|i| gray([at(16 + i * 3) as u8, at(17 + i * 3) as u8, at(18 + i * 3) as u8]))
                .collect();
            pic.numcols = n as i32;
        }
    }
    pic.bitmap = bitmap;
    pic.num_transp = NO_TRANSPARENCY;
    pic.subtype = P_PCX;
    set_size(pic, w as u32, h as u32);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn types() {
        assert_eq!(pic_type(b"GIF89a"), Some(PicType::Gif));
        assert_eq!(pic_type(b"%!PS-Adobe-3.0 EPSF"), Some(PicType::Eps));
        assert_eq!(pic_type(b"%PDF-1.4"), Some(PicType::Pdf));
        assert_eq!(pic_type(b"P5\n1 1\n255\n\0"), Some(PicType::Ppm));
        assert_eq!(pic_type(b"\xff\xd8\xff\xdb"), Some(PicType::Jpeg));
        assert_eq!(pic_type(b"hello"), None);
        assert_eq!(pic_type(b""), None);
    }

    #[test]
    fn eps_bounding_box() {
        let mut pic = Pic::default();
        let eps = b"%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: (atend)\n%%BeginDocument: x\n%%BoundingBox: 1 1 2 2\n%%EndDocument\n%%BoundingBox: 10.5 20 110 220\n";
        assert_eq!(read_eps(eps, &mut pic), Ok((10, 20)));
        assert_eq!(pic.bit_size, Point::new(99, 200));
        assert!(read_eps(b"%!\n%%BoundingBox: 1 2 x\n", &mut pic).is_err());
        assert_eq!(read_eps(b"%!\n", &mut pic), Ok((0, 0)));
        assert_eq!(pic.bit_size, Point::new(10, 10));
    }

    #[test]
    fn pdf_media_box() {
        let mut pic = Pic::default();
        assert_eq!(
            read_pdf(b"%PDF-1.4\n1 0 obj << /MediaBox [0 0 595.3 841.9] >>", &mut pic),
            Ok((0, 0))
        );
        assert_eq!(pic.bit_size, Point::new(596, 842));
        assert!(read_pdf(b"%PDF-1.4\n", &mut pic).is_err());
        assert!(read_pdf(b"%PDF /MediaBox [1 2", &mut pic).is_err());
    }

    #[test]
    fn xbm() {
        let src =
            b"#define t_width 10\n#define t_height 2\nstatic unsigned char t_bits[] = {\n 0x01, 0x02, 0xff, 0x03 };\n";
        let b = read_bitmap_file(src).unwrap();
        assert_eq!((b.width, b.height), (10, 2));
        assert_eq!(b.data, vec![0x80, 0x40, 0xff, 0xc0]);
        assert!(read_bitmap_file(b"#define t_width 10\nstatic char t_bits[] = { 0x01 };").is_none());
        assert!(
            read_bitmap_file(b"#define a_width 99999999\n#define a_height 99999999\nstatic char a_bits[] = {")
                .is_none()
        );
    }

    #[test]
    fn xpm() {
        let src = b"/* XPM */\nstatic char *x[] = {\n/* w h n c */\n\"2 2 2 1\",\n\"a c #ff0000\",\n\"b c None\",\n\"ab\",\n\"ba\"};\n";
        let mut pic = Pic::default();
        read_xpm(&Ctx::default(), src, &mut pic).unwrap();
        assert_eq!(pic.bit_size, Point::new(2, 2));
        assert_eq!(pic.numcols, 2);
        assert_eq!(pic.cmap, vec![[255, 0, 0], [255, 255, 255]]);
        assert_eq!(pic.bitmap, vec![0, 1, 1, 0]);
        assert_eq!(pic.num_transp, TRANSP_COLOR);
        assert!(read_xpm(&Ctx::default(), b"/* XPM */ \"2 2 1 1\" \"a c red\" \"aa\"", &mut pic).is_err());
        assert!(read_xpm(&Ctx::default(), b"/* XPM */ \"9999999 9999999 1 1\"", &mut pic).is_err());
    }

    #[test]
    fn rgb_beyond_256_colours() {
        let w = 20u32;
        let h = 20u32;
        let rgba: Vec<u8> = (0..w * h)
            .flat_map(|i| [(i % 256) as u8, (i / 256) as u8, 7, 255])
            .collect();
        let mut pic = Pic::default();
        fill_from_rgba(&Ctx::default(), &mut pic, w, h, &rgba).unwrap();
        assert_eq!(pic.numcols, RGB_NUMCOLS);
        assert_eq!(pic.bitmap.len(), (w * h * 3) as usize);
        let mut ctx = Ctx::default();
        ctx.grayonly = true;
        fill_from_rgba(&ctx, &mut pic, w, h, &rgba).unwrap();
        assert!(pic.numcols <= 256);
    }

    #[test]
    fn lzw_round_trip() {
        // "abababab" compressed by `compress -c` (block mode, 16 bits).
        let z = [0x1f, 0x9d, 0x90, 0x61, 0xc4, 0x04, 0x1c, 0x28, 0x06];
        assert_eq!(uncompress_lzw(&z).unwrap(), b"abababab");
        assert!(uncompress_lzw(&[0x1f, 0x9d, 0x90, 0xff, 0xff, 0xff]).is_err());
        assert!(uncompress_lzw(&[0x1f, 0x9d, 0x99]).is_err());
    }

    #[test]
    fn junk_does_not_panic() {
        let ctx = Ctx::default();
        let mut seed = 12345u32;
        for len in 0..300usize {
            let mut junk: Vec<u8> = (0..len)
                .map(|_| {
                    seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    (seed >> 16) as u8
                })
                .collect();
            for prefix in [
                &b"GIF89a"[..],
                b"\x0a\x05\x01",
                b"%!",
                b"\xc5\xd0\xd3\xc6",
                b"%PDF",
                b"P6",
                b"II*\0",
                b"#define",
                b"\x89PNG\r\n\x1a\n",
                b"\xff\xd8\xff\xe0",
                b"/* XPM */",
            ] {
                let mut data = prefix.to_vec();
                data.append(&mut junk.clone());
                let stream = PicStream {
                    name: "junk".into(),
                    path: PathBuf::new(),
                    compression: Compression::None,
                    data,
                };
                let mut pic = Pic::default();
                let _ = read_pic(&ctx, &stream, &mut pic);
            }
            junk.insert(0, 0x1f);
            junk.insert(1, 0x9d);
            let _ = uncompress(&junk, Compression::Gzip);
            junk[1] = 0x8b;
            let _ = uncompress(&junk, Compression::Gzip);
        }
    }
}
