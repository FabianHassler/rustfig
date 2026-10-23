//! Editable exports (plan E16): the figure's Fig text stored inside a PDF,
//! SVG or PNG export, and read back from such a file.
//!
//! - PDF: an incremental update appended to fig2dev's file (new objects, a
//!   rewritten catalog, a new xref section with `/Prev`): an embedded file
//!   `figure.fig` in the catalog's `/Names /EmbeddedFiles` tree and `/AF`, so
//!   PDF readers list it among the attachments.
//! - SVG: a `<metadata>` element right after the opening `<svg>` tag holding
//!   a `<rustfig:fig>` element with the text, escaped (base64 when the text
//!   has characters XML cannot carry).
//! - PNG: an `iTXt` chunk with the keyword `rustfig-fig`, deflate-compressed,
//!   before `IEND`.
//!
//! The payload is always the Fig 3.2 text: nothing here needs `.rustfig`.
//! Everything read is untrusted: bounds-checked, sizes capped.

use crate::FormatError;
use std::io::{Read, Write};

/// The most an embedded figure may be, uncompressed (bytes).
pub const MAX_EMBEDDED_BYTES: usize = 128 << 20;
/// PNG keyword of the figure's chunk.
pub const PNG_KEYWORD: &str = "rustfig-fig";
/// Name of the PDF's embedded file.
const PDF_NAME: &str = "figure.fig";
/// The embedded file's MIME type as a PDF name.
const PDF_SUBTYPE: &str = "/application#2Fx-xfig";
const SVG_NS: &str = "https://github.com/FabianHassler/RustFig/ns/fig";
const PNG_SIG: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Whether exports in language `lang` can carry the figure.
pub fn can_embed(lang: &str) -> bool {
    matches!(lang, "pdf" | "svg" | "png")
}

/// `output` (an export in language `lang`) with `fig` stored inside.
pub fn embed_figure(lang: &str, output: Vec<u8>, fig: &str) -> Result<Vec<u8>, FormatError> {
    if fig.len() > MAX_EMBEDDED_BYTES {
        return Err(FormatError::new("the figure is too large to embed"));
    }
    match lang {
        "pdf" => embed_pdf(output, fig),
        "svg" => embed_svg(output, fig),
        "png" => embed_png(output, fig),
        _ => Err(FormatError::new(format!("cannot embed the figure in {lang} files"))),
    }
}

/// The Fig text embedded in a PDF, SVG or PNG file: `Ok(None)` when the file
/// is one of those without an embedded figure, or none of them.
pub fn extract_figure(src: &[u8]) -> Result<Option<String>, FormatError> {
    if src.starts_with(b"%PDF") {
        extract_pdf(src)
    } else if src.starts_with(PNG_SIG) {
        extract_png(src)
    } else if looks_like_svg(src) {
        extract_svg(src)
    } else {
        Ok(None)
    }
}

/// Whether `src` is a PDF, PNG or SVG (so that "no embedded figure" is the
/// right message).
pub fn is_embedding_container(src: &[u8]) -> bool {
    src.starts_with(b"%PDF") || src.starts_with(PNG_SIG) || looks_like_svg(src)
}

fn looks_like_svg(src: &[u8]) -> bool {
    let head = src.get(..src.len().min(4096)).unwrap_or(src);
    find(head, b"<svg").is_some()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).rposition(|w| w == needle)
}

fn deflate(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(data).map_err(|e| FormatError::new(e.to_string()))?;
    enc.finish().map_err(|e| FormatError::new(e.to_string()))
}

fn inflate(data: &[u8]) -> Result<Vec<u8>, FormatError> {
    let mut out = Vec::new();
    let dec = flate2::read::ZlibDecoder::new(data);
    dec.take(MAX_EMBEDDED_BYTES as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|_| FormatError::new("the embedded figure is damaged"))?;
    if out.len() > MAX_EMBEDDED_BYTES {
        return Err(FormatError::new("the embedded figure is too large"));
    }
    Ok(out)
}

fn utf8(bytes: Vec<u8>) -> Result<String, FormatError> {
    String::from_utf8(bytes).map_err(|_| FormatError::new("the embedded figure is not text"))
}

// ---------------------------------------------------------------- PNG

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// One PNG chunk: its type, its data and the offset where it starts.
struct Chunk<'a> {
    kind: &'a [u8],
    data: &'a [u8],
    start: usize,
}

/// The chunks of a PNG, up to the first malformed one.
fn png_chunks(src: &[u8]) -> Vec<Chunk<'_>> {
    let mut out = Vec::new();
    let mut pos = PNG_SIG.len();
    while let (Some(len), Some(kind)) = (src.get(pos..pos + 4), src.get(pos + 4..pos + 8)) {
        let len = u32::from_be_bytes([len[0], len[1], len[2], len[3]]) as usize;
        let data_start = pos + 8;
        let Some(end) = data_start.checked_add(len) else { break };
        let Some(data) = src.get(data_start..end) else { break };
        if src.get(end..end + 4).is_none() {
            break;
        }
        out.push(Chunk { kind, data, start: pos });
        pos = end + 4;
    }
    out
}

fn embed_png(output: Vec<u8>, fig: &str) -> Result<Vec<u8>, FormatError> {
    if !output.starts_with(PNG_SIG) {
        return Err(FormatError::new("the export is not a PNG file"));
    }
    let iend = png_chunks(&output)
        .iter()
        .find(|c| c.kind == b"IEND")
        .map(|c| c.start)
        .ok_or_else(|| FormatError::new("the PNG file has no IEND chunk"))?;
    let mut data = Vec::new();
    data.extend_from_slice(PNG_KEYWORD.as_bytes());
    // NUL, compressed flag 1, method 0, empty language tag and translated keyword.
    data.extend_from_slice(&[0, 1, 0, 0, 0]);
    data.extend_from_slice(&deflate(fig.as_bytes())?);
    let len = u32::try_from(data.len()).map_err(|_| FormatError::new("the figure is too large to embed"))?;
    let mut body = b"iTXt".to_vec();
    body.extend_from_slice(&data);
    let mut out = Vec::with_capacity(output.len() + body.len() + 8);
    out.extend_from_slice(output.get(..iend).unwrap_or(&output));
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&body);
    out.extend_from_slice(&crc32(&body).to_be_bytes());
    out.extend_from_slice(output.get(iend..).unwrap_or(&[]));
    Ok(out)
}

fn extract_png(src: &[u8]) -> Result<Option<String>, FormatError> {
    for chunk in png_chunks(src) {
        if chunk.kind != b"iTXt" {
            continue;
        }
        let data = chunk.data;
        let Some(nul) = data.iter().position(|&b| b == 0) else {
            continue;
        };
        if data.get(..nul) != Some(PNG_KEYWORD.as_bytes()) {
            continue;
        }
        let (Some(&flag), Some(&method)) = (data.get(nul + 1), data.get(nul + 2)) else {
            continue;
        };
        // Skip the language tag and the translated keyword (both NUL-ended).
        let rest = data.get(nul + 3..).unwrap_or(&[]);
        let Some(a) = rest.iter().position(|&b| b == 0) else {
            continue;
        };
        let rest = rest.get(a + 1..).unwrap_or(&[]);
        let Some(b) = rest.iter().position(|&b| b == 0) else {
            continue;
        };
        let text = rest.get(b + 1..).unwrap_or(&[]);
        let bytes = if flag == 1 {
            if method != 0 {
                return Err(FormatError::new("the embedded figure uses an unknown compression"));
            }
            inflate(text)?
        } else {
            text.to_vec()
        };
        return utf8(bytes).map(Some);
    }
    Ok(None)
}

// ---------------------------------------------------------------- SVG

fn xml_char_ok(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}')
}

fn embed_svg(output: Vec<u8>, fig: &str) -> Result<Vec<u8>, FormatError> {
    let text = String::from_utf8(output).map_err(|_| FormatError::new("the SVG export is not UTF-8"))?;
    let bytes = text.as_bytes();
    let start = find(bytes, b"<svg").ok_or_else(|| FormatError::new("the SVG export has no <svg> element"))?;
    // End of the start tag: the first '>' outside quotes.
    let mut quote = 0u8;
    let mut end = None;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match (quote, b) {
            (0, b'"' | b'\'') => quote = b,
            (q, c) if q != 0 && q == c => quote = 0,
            (0, b'>') => {
                end = Some(i + 1);
                break;
            }
            _ => {}
        }
    }
    let end = end.ok_or_else(|| FormatError::new("the SVG export is damaged"))?;
    if end >= 2 && bytes.get(end - 2) == Some(&b'/') {
        return Err(FormatError::new("the SVG export is empty"));
    }
    let payload = if fig.chars().all(xml_char_ok) {
        let mut esc = String::with_capacity(fig.len() + fig.len() / 8);
        for c in fig.chars() {
            match c {
                '&' => esc.push_str("&amp;"),
                '<' => esc.push_str("&lt;"),
                '>' => esc.push_str("&gt;"),
                '\r' => esc.push_str("&#13;"),
                c => esc.push(c),
            }
        }
        format!("<rustfig:fig xmlns:rustfig=\"{SVG_NS}\">{esc}</rustfig:fig>")
    } else {
        format!(
            "<rustfig:fig xmlns:rustfig=\"{SVG_NS}\" encoding=\"base64\">{}</rustfig:fig>",
            base64(fig.as_bytes())
        )
    };
    let mut out = String::with_capacity(text.len() + payload.len() + 32);
    out.push_str(text.get(..end).unwrap_or(""));
    out.push_str("\n<metadata>");
    out.push_str(&payload);
    out.push_str("</metadata>");
    out.push_str(text.get(end..).unwrap_or(""));
    Ok(out.into_bytes())
}

fn extract_svg(src: &[u8]) -> Result<Option<String>, FormatError> {
    if src.len() > 2 * MAX_EMBEDDED_BYTES {
        return Err(FormatError::new("the embedded figure is too large"));
    }
    let Ok(text) = std::str::from_utf8(src) else {
        return Ok(None);
    };
    let Some(open) = text.find("<rustfig:fig") else {
        return Ok(None);
    };
    let Some(gt) = text.get(open..).and_then(|t| t.find('>')) else {
        return Ok(None);
    };
    let tag = text.get(open..open + gt).unwrap_or("");
    let body_start = open + gt + 1;
    if tag.ends_with('/') {
        return Ok(None);
    }
    let Some(close) = text.get(body_start..).and_then(|t| t.find("</rustfig:fig>")) else {
        return Ok(None);
    };
    let body = text.get(body_start..body_start + close).unwrap_or("");
    if tag.contains("encoding=\"base64\"") {
        let bytes = unbase64(body).ok_or_else(|| FormatError::new("the embedded figure is damaged"))?;
        return utf8(bytes).map(Some);
    }
    let body = body
        .strip_prefix("<![CDATA[")
        .and_then(|b| b.strip_suffix("]]>"))
        .map(str::to_string)
        .unwrap_or_else(|| unescape(body));
    Ok(Some(body))
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(rest.get(..i).unwrap_or(""));
        rest = rest.get(i..).unwrap_or("");
        let semi = rest.find(';').filter(|&j| j <= 10);
        let decoded = semi.and_then(|j| {
            let name = rest.get(1..j)?;
            match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => name
                    .strip_prefix("#x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| name.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            }
        });
        match (decoded, semi) {
            (Some(c), Some(j)) => {
                out.push(c);
                rest = rest.get(j + 1..).unwrap_or("");
            }
            _ => {
                out.push('&');
                rest = rest.get(1..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let b = |i: usize| u32::from(c.get(i).copied().unwrap_or(0));
        let n = (b(0) << 16) | (b(1) << 8) | b(2);
        for i in 0..4 {
            if i <= c.len() {
                out.push(B64.get(((n >> (18 - 6 * i)) & 63) as usize).copied().unwrap_or(b'A') as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn unbase64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0);
    for b in s.bytes() {
        if b.is_ascii_whitespace() || b == b'=' {
            continue;
        }
        let v = B64.iter().position(|&c| c == b)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

// ---------------------------------------------------------------- PDF

/// The unsigned number after `key` in `hay`.
fn num_after(hay: &[u8], key: &[u8]) -> Option<usize> {
    let at = find(hay, key)? + key.len();
    let rest = hay.get(at..)?;
    let rest = rest.get(rest.iter().position(|b| !b.is_ascii_whitespace())?..)?;
    let digits = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    std::str::from_utf8(rest.get(..digits)?).ok()?.parse().ok()
}

/// The indirect reference `N G R` after `key` in `hay`: the object number
/// and the reference as text.
fn ref_after(hay: &[u8], key: &[u8]) -> Option<(usize, String)> {
    let at = find(hay, key)? + key.len();
    let rest = hay.get(at..)?;
    let rest = rest.get(..rest.len().min(40))?;
    let text: String = rest.iter().map(|&b| b as char).collect();
    let mut it = text.split_whitespace();
    let n: usize = it.next()?.parse().ok()?;
    let g: usize = it.next()?.parse().ok()?;
    (it.next()? == "R").then(|| (n, format!("{n} {g} R")))
}

/// The offset and generation of object `num` in the classic xref section at
/// `at`, following `/Prev` sections.
fn xref_lookup(src: &[u8], mut at: usize, num: usize) -> Option<(usize, usize)> {
    for _ in 0..64 {
        let sect = src.get(at..)?.strip_prefix(b"xref")?;
        let trailer_at = find(sect, b"trailer")?;
        let table = std::str::from_utf8(sect.get(..trailer_at)?).ok()?;
        let mut words = table.split_whitespace();
        while let (Some(first), Some(count)) = (words.next(), words.next()) {
            let (first, count): (usize, usize) = (first.parse().ok()?, count.parse().ok()?);
            for i in 0..count.min(1 << 24) {
                let off = words.next()?;
                let gen = words.next()?;
                let kind = words.next()?;
                if first + i == num {
                    return (kind == "n").then(|| (off.parse().ok().unwrap_or(0), gen.parse().unwrap_or(0)));
                }
            }
        }
        at = num_after(sect.get(trailer_at..)?, b"/Prev")?;
    }
    None
}

fn embed_pdf(output: Vec<u8>, fig: &str) -> Result<Vec<u8>, FormatError> {
    let bad = || FormatError::new("the PDF export has a structure that cannot take an embedded file");
    let tail = output.get(output.len().saturating_sub(2048)..).ok_or_else(bad)?;
    let sx = rfind(tail, b"startxref").ok_or_else(bad)?;
    let prev = num_after(tail.get(sx..).ok_or_else(bad)?, b"startxref").ok_or_else(bad)?;
    let xref = output.get(prev..).ok_or_else(bad)?;
    if !xref.starts_with(b"xref") {
        return Err(bad());
    }
    let trailer = xref.get(find(xref, b"trailer").ok_or_else(bad)?..).ok_or_else(bad)?;
    let trailer = trailer
        .get(..find(trailer, b"startxref").unwrap_or(trailer.len()))
        .ok_or_else(bad)?;
    let size = num_after(trailer, b"/Size").ok_or_else(bad)?;
    if !(1..=10_000_000).contains(&size) {
        return Err(bad());
    }
    let (root_num, root_ref) = ref_after(trailer, b"/Root").ok_or_else(bad)?;
    let (cat_off, cat_gen) = xref_lookup(&output, prev, root_num).ok_or_else(bad)?;
    let cat = output.get(cat_off..).ok_or_else(bad)?;
    let cat = cat.get(..find(cat, b"endobj").ok_or_else(bad)?).ok_or_else(bad)?;
    let d0 = find(cat, b"<<").ok_or_else(bad)?;
    let d1 = rfind(cat, b">>").ok_or_else(bad)?;
    if d1 < d0 + 2 || find(cat, b"/Names").is_some() || find(cat, b"/AF").is_some() {
        return Err(bad());
    }
    let inner = cat.get(d0 + 2..d1).ok_or_else(bad)?.to_vec();

    let (fs_num, ef_num) = (size, size + 1);
    let data = deflate(fig.as_bytes())?;
    let trailer = trailer.to_vec();
    let trailer = &trailer[..];
    let mut out = output;
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    // (object, generation, offset) of what the update adds or replaces.
    let mut offsets: Vec<(usize, usize, usize)> = Vec::new();

    offsets.push((fs_num, 0, out.len()));
    out.extend_from_slice(
        format!(
            "{fs_num} 0 obj\n<< /Type /Filespec /F ({PDF_NAME}) /UF ({PDF_NAME}) /Desc (RustFig figure) \
             /AFRelationship /Source /EF << /F {ef_num} 0 R /UF {ef_num} 0 R >> >>\nendobj\n"
        )
        .as_bytes(),
    );
    offsets.push((ef_num, 0, out.len()));
    out.extend_from_slice(
        format!(
            "{ef_num} 0 obj\n<< /Type /EmbeddedFile /Subtype {PDF_SUBTYPE} /Filter /FlateDecode /Length {} >>\nstream\n",
            data.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(&data);
    out.extend_from_slice(b"\nendstream\nendobj\n");

    offsets.push((root_num, cat_gen, out.len()));
    out.extend_from_slice(format!("{root_num} {cat_gen} obj\n<<").as_bytes());
    out.extend_from_slice(&inner);
    out.extend_from_slice(
        format!("/Names << /EmbeddedFiles << /Names [({PDF_NAME}) {fs_num} 0 R] >> >> /AF [{fs_num} 0 R] >>\nendobj\n")
            .as_bytes(),
    );

    offsets.sort_unstable();
    let xref_at = out.len();
    out.extend_from_slice(b"xref\n");
    for (num, gen, off) in &offsets {
        out.extend_from_slice(format!("{num} 1\n{off:010} {gen:05} n \n").as_bytes());
    }
    let mut t = format!("trailer\n<< /Size {} /Root {root_ref} /Prev {prev}", size + 2);
    if let Some((_, info)) = ref_after(trailer, b"/Info") {
        t.push_str(&format!(" /Info {info}"));
    }
    if let Some(rest) = find(trailer, b"/ID").and_then(|i| trailer.get(i + 3..)) {
        if let (Some(a), Some(b)) = (find(rest, b"["), find(rest, b"]")) {
            if a < b {
                t.push_str(" /ID ");
                t.push_str(&String::from_utf8_lossy(rest.get(a..=b).unwrap_or(&[])));
            }
        }
    }
    t.push_str(&format!(" >>\nstartxref\n{xref_at}\n%%EOF\n"));
    out.extend_from_slice(t.as_bytes());
    Ok(out)
}

fn extract_pdf(src: &[u8]) -> Result<Option<String>, FormatError> {
    let marker = format!("/Subtype {PDF_SUBTYPE}");
    let Some(at) = rfind(src, marker.as_bytes()) else {
        return Ok(None);
    };
    let rest = src.get(at..).unwrap_or(&[]);
    let near = rest.get(..rest.len().min(1024)).unwrap_or(rest);
    let (Some(len), Some(s)) = (num_after(near, b"/Length"), find(near, b"stream\n")) else {
        return Ok(None);
    };
    let Some(data) = rest.get(s + 7..).and_then(|d| d.get(..len)) else {
        return Err(FormatError::new("the embedded figure is damaged"));
    };
    utf8(inflate(data)?).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        for n in 0..20 {
            let d: Vec<u8> = (0..n).map(|i| (i as u8).wrapping_mul(37)).collect();
            assert_eq!(unbase64(&base64(&d)).unwrap(), d);
        }
    }

    #[test]
    fn svg_round_trip_with_odd_text() {
        let svg = b"<?xml version=\"1.0\"?>\n<svg width=\"1\" height=\"1\"><g/></svg>\n".to_vec();
        for fig in ["#FIG 3.2\n4 0 0 <&>]]>\r\n", "bad \u{1} char"] {
            let out = embed_svg(svg.clone(), fig).unwrap();
            assert_eq!(extract_svg(&out).unwrap().as_deref(), Some(fig));
        }
    }

    #[test]
    fn hostile_input_is_refused() {
        let pdf = b"%PDF-1.4 /Subtype /application#2Fx-xfig /Length 99999 stream\n";
        for src in [
            &pdf[..],
            b"\x89PNG\r\n\x1a\n\xff\xff\xff\xff",
            b"<svg><rustfig:fig>&#99999999;",
        ] {
            let _ = extract_figure(src);
        }
        assert!(embed_pdf(b"%PDF-1.4\n".to_vec(), "x").is_err());
    }
}
