//! The C library calls `read.c` and `read1_3.c` stand on, over a byte
//! buffer: `getline`, `fgetc`, `fgets`, and the `scanf` conversions they
//! use (`%d`, `%lf`, `%2x`, `%127[^\n]`, `%3hho`), with C's whitespace and
//! end-of-file rules. Integers that overflow saturate; nothing panics.
//! Also the text decoder (the input encoding to Unicode) and C's `%g`.

/// C's `isspace` in the C locale.
pub(crate) fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

/// A read position in the input (`FILE *` or a `sscanf` string).
#[derive(Clone, Debug)]
pub(crate) struct Scan<'a> {
    b: &'a [u8],
    pub pos: usize,
    /// `feof`: a read ran into the end of the input.
    pub eof: bool,
}

/// A scanned value (`%d` or `%lf`).
#[derive(Clone, Copy, Debug)]
enum Val {
    I(i32),
    F(f64),
}

/// The values a `scanf` call assigned, in order (`n` is its result).
#[derive(Clone, Debug, Default)]
pub(crate) struct Fields {
    vals: Vec<Val>,
}

impl Fields {
    /// The number of assigned values (`scanf`'s return value).
    pub fn n(&self) -> usize {
        self.vals.len()
    }
    /// The `k`-th value as an `int` (0 if missing).
    pub fn i(&self, k: usize) -> i32 {
        match self.vals.get(k) {
            Some(Val::I(v)) => *v,
            Some(Val::F(v)) => *v as i32,
            None => 0,
        }
    }
    /// The `k`-th value as a `double` (0 if missing).
    pub fn f(&self, k: usize) -> f64 {
        match self.vals.get(k) {
            Some(Val::F(v)) => *v,
            Some(Val::I(v)) => f64::from(*v),
            None => 0.0,
        }
    }
}

impl<'a> Scan<'a> {
    pub fn new(b: &'a [u8]) -> Scan<'a> {
        Scan { b, pos: 0, eof: false }
    }

    pub fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    /// The rest of the input.
    pub fn rest(&self) -> &'a [u8] {
        self.b.get(self.pos..).unwrap_or_default()
    }

    /// `fgetc`.
    pub fn getc(&mut self) -> Option<u8> {
        match self.peek() {
            Some(c) => {
                self.pos += 1;
                Some(c)
            }
            None => {
                self.eof = true;
                None
            }
        }
    }

    /// `ungetc` of the character just read.
    pub fn unget(&mut self) {
        self.pos = self.pos.saturating_sub(1);
    }

    /// `getline`: the next line with its newline, `None` at the end.
    pub fn getline(&mut self) -> Option<&'a [u8]> {
        let rest = self.rest();
        if rest.is_empty() {
            self.eof = true;
            return None;
        }
        let n = rest.iter().position(|&c| c == b'\n').map_or(rest.len(), |i| i + 1);
        self.pos += n;
        if self.pos >= self.b.len() && rest.last() != Some(&b'\n') {
            self.eof = true;
        }
        rest.get(..n)
    }

    /// `fgets(buf, 17, fp)`: at most 16 bytes, up to and including a
    /// newline; `None` at the end.
    pub fn fgets16(&mut self) -> Option<&'a [u8]> {
        let rest = self.rest();
        if rest.is_empty() {
            self.eof = true;
            return None;
        }
        let max = rest.len().min(16);
        let head = rest.get(..max).unwrap_or_default();
        let n = head.iter().position(|&c| c == b'\n').map_or(max, |i| i + 1);
        self.pos += n;
        rest.get(..n)
    }

    /// Skip to just after the next newline, or to the end (`skip_line`).
    pub fn skip_line(&mut self) {
        while let Some(c) = self.getc() {
            if c == b'\n' {
                return;
            }
        }
    }

    /// A whitespace directive in a format: skip any whitespace.
    pub fn skip_ws(&mut self) {
        while self.peek().is_some_and(is_space) {
            self.pos += 1;
        }
    }

    /// A literal character in a format.
    pub fn lit(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            if self.peek().is_none() {
                self.eof = true;
            }
            false
        }
    }

    /// Read decimal digits from `pos`, saturating; the number of digits.
    fn digits(&mut self, neg: bool) -> (i64, usize) {
        let mut v: i64 = 0;
        let mut n = 0;
        while let Some(c) = self.peek().filter(u8::is_ascii_digit) {
            v = v.saturating_mul(10).saturating_add(i64::from(c - b'0'));
            self.pos += 1;
            n += 1;
        }
        (if neg { -v } else { v }, n)
    }

    /// `%d`. Out-of-range values saturate.
    pub fn int(&mut self) -> Option<i32> {
        self.skip_ws();
        let mut neg = false;
        match self.peek() {
            Some(b'-') => {
                neg = true;
                self.pos += 1;
            }
            Some(b'+') => self.pos += 1,
            _ => {}
        }
        let (v, n) = self.digits(neg);
        if n == 0 {
            if self.peek().is_none() {
                self.eof = true;
            }
            return None;
        }
        Some(v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32)
    }

    /// `%lf`, as `strtod` reads it: decimal numbers, `inf`, `infinity`
    /// and `nan`, ignoring case.
    pub fn float(&mut self) -> Option<f64> {
        self.skip_ws();
        let start = self.pos;
        let mut neg = false;
        match self.peek() {
            Some(b'-') => {
                neg = true;
                self.pos += 1;
            }
            Some(b'+') => self.pos += 1,
            _ => {}
        }
        let rest = self.rest();
        let word = |w: &[u8]| rest.len() >= w.len() && rest.get(..w.len()).is_some_and(|r| r.eq_ignore_ascii_case(w));
        if word(b"infinity") || word(b"inf") {
            self.pos += if word(b"infinity") { 8 } else { 3 };
            return Some(if neg { f64::NEG_INFINITY } else { f64::INFINITY });
        }
        if word(b"nan") {
            self.pos += 3;
            return Some(f64::NAN);
        }
        let num_start = self.pos;
        let mut nd = 0;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.pos += 1;
            nd += 1;
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.pos += 1;
                nd += 1;
            }
        }
        if nd == 0 {
            if self.peek().is_none() {
                self.eof = true;
            }
            self.pos = start;
            return None;
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            let save = self.pos;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                }
            } else {
                self.pos = save;
            }
        }
        let text = std::str::from_utf8(self.b.get(num_start..self.pos)?).ok()?;
        let v = text.parse::<f64>().ok()?;
        Some(if neg { -v } else { v })
    }

    /// `%2x`: at most two characters, a sign and hexadecimal digits.
    /// Negative values wrap as C's `unsigned int` does, so they come back
    /// out of the 0..255 range.
    pub fn hex2(&mut self) -> Option<i64> {
        self.skip_ws();
        let mut width = 2;
        let mut neg = false;
        if let Some(c @ (b'-' | b'+')) = self.peek() {
            neg = c == b'-';
            self.pos += 1;
            width -= 1;
        }
        let mut v: i64 = 0;
        let mut n = 0;
        while n < width {
            let Some(d) = self.peek().and_then(|c| char::from(c).to_digit(16)) else {
                break;
            };
            v = v * 16 + i64::from(d);
            self.pos += 1;
            n += 1;
        }
        if n == 0 {
            if self.peek().is_none() {
                self.eof = true;
            }
            return None;
        }
        Some(if neg && v != 0 { i64::from(u32::MAX) + 1 - v } else { v })
    }

    /// `%127[^\n]`: up to 127 bytes other than a newline, at least one.
    pub fn line127(&mut self) -> Option<&'a [u8]> {
        let rest = self.rest();
        let n = rest.iter().take(127).take_while(|&&c| c != b'\n').count();
        if n == 0 {
            if rest.is_empty() {
                self.eof = true;
            }
            return None;
        }
        self.pos += n;
        rest.get(..n)
    }

    /// A run of `%d` (`d`), `%lf` (`f`) and `%*d` (`*`) conversions, as
    /// `sscanf` does them: it stops at the first that fails.
    pub fn fields(&mut self, fmt: &str) -> Fields {
        let mut out = Fields::default();
        for c in fmt.bytes() {
            match c {
                b'*' => {
                    if self.int().is_none() {
                        break;
                    }
                }
                b'd' => match self.int() {
                    Some(v) => out.vals.push(Val::I(v)),
                    None => break,
                },
                b'f' => match self.float() {
                    Some(v) => out.vals.push(Val::F(v)),
                    None => break,
                },
                _ => {}
            }
        }
        out
    }
}

/// `sscanf(line, fmt)` with the conversions of [`Scan::fields`].
pub(crate) fn sscanf(line: &[u8], fmt: &str) -> Fields {
    Scan::new(line).fields(fmt)
}

/// Whether `s` starts with `pat`, ignoring ASCII case (`strncasecmp`).
pub(crate) fn starts_ci(s: &[u8], pat: &[u8]) -> bool {
    s.get(..pat.len()).is_some_and(|h| h.eq_ignore_ascii_case(pat))
}

/// `atoi`: leading whitespace, a sign and digits; 0 if none; saturating.
pub(crate) fn atoi(s: &[u8]) -> i32 {
    Scan::new(s).int().unwrap_or(0)
}

/// `atof`: the leading number; 0 if none.
pub(crate) fn atof(s: &[u8]) -> f64 {
    Scan::new(s).float().unwrap_or(0.0)
}

/// The encoding of the strings in a Fig file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Latin1,
    Latin2,
}

impl Encoding {
    /// fig2dev's `-E` name (or the file's `#encoding:` line), else, as
    /// RustFig's default for the locale fig2dev would use: UTF-8 if the
    /// whole file is valid UTF-8, otherwise ISO-8859-1. Names fig2dev
    /// could only convert with iconv keep their bytes as ISO-8859-1.
    pub fn resolve(name: Option<&str>, file_is_utf8: bool) -> Encoding {
        let Some(name) = name else {
            return if file_is_utf8 { Encoding::Utf8 } else { Encoding::Latin1 };
        };
        Encoding::from_name(name).unwrap_or(Encoding::Latin1)
    }

    /// The encodings RustFig converts itself (what iconv would do for
    /// `-E name`): UTF-8, ISO-8859-1 and -2 under their usual names.
    pub fn from_name(name: &str) -> Option<Encoding> {
        let key: String = name
            .chars()
            .filter(|c| !matches!(c, '-' | '_' | ' '))
            .map(|c| c.to_ascii_lowercase())
            .collect();
        match key.as_str() {
            "utf8" | "0" => Some(Encoding::Utf8),
            "iso88592" | "iso88592:1987" | "latin2" | "l2" | "2" => Some(Encoding::Latin2),
            "iso88591" | "iso88591:1987" | "latin1" | "l1" | "1" => Some(Encoding::Latin1),
            _ => None,
        }
    }

    /// Decode `bytes` to Unicode. Bytes that are not valid UTF-8 in a
    /// UTF-8 file are taken as ISO-8859-1.
    pub fn decode(self, bytes: &[u8]) -> String {
        match self {
            Encoding::Latin1 => bytes.iter().map(|&b| char::from(b)).collect(),
            Encoding::Latin2 => bytes.iter().map(|&b| latin2(b)).collect(),
            Encoding::Utf8 => {
                let mut s = String::with_capacity(bytes.len());
                for chunk in bytes.utf8_chunks() {
                    s.push_str(chunk.valid());
                    s.extend(chunk.invalid().iter().map(|&b| char::from(b)));
                }
                s
            }
        }
    }
}

/// ISO-8859-2, 0xA0..=0xFF (the Unicode mapping of the standard).
const LATIN2_HIGH: [u16; 96] = [
    0x00A0, 0x0104, 0x02D8, 0x0141, 0x00A4, 0x013D, 0x015A, 0x00A7, 0x00A8, 0x0160, 0x015E, 0x0164, 0x0179, 0x00AD,
    0x017D, 0x017B, 0x00B0, 0x0105, 0x02DB, 0x0142, 0x00B4, 0x013E, 0x015B, 0x02C7, 0x00B8, 0x0161, 0x015F, 0x0165,
    0x017A, 0x02DD, 0x017E, 0x017C, 0x0154, 0x00C1, 0x00C2, 0x0102, 0x00C4, 0x0139, 0x0106, 0x00C7, 0x010C, 0x00C9,
    0x0118, 0x00CB, 0x011A, 0x00CD, 0x00CE, 0x010E, 0x0110, 0x0143, 0x0147, 0x00D3, 0x00D4, 0x0150, 0x00D6, 0x00D7,
    0x0158, 0x016E, 0x00DA, 0x0170, 0x00DC, 0x00DD, 0x0162, 0x00DF, 0x0155, 0x00E1, 0x00E2, 0x0103, 0x00E4, 0x013A,
    0x0107, 0x00E7, 0x010D, 0x00E9, 0x0119, 0x00EB, 0x011B, 0x00ED, 0x00EE, 0x010F, 0x0111, 0x0144, 0x0148, 0x00F3,
    0x00F4, 0x0151, 0x00F6, 0x00F7, 0x0159, 0x016F, 0x00FA, 0x0171, 0x00FC, 0x00FD, 0x0163, 0x02D9,
];

fn latin2(b: u8) -> char {
    if b < 0xA0 {
        return char::from(b);
    }
    LATIN2_HIGH
        .get(usize::from(b - 0xA0))
        .and_then(|&u| char::from_u32(u32::from(u)))
        .unwrap_or(char::from(b))
}

/// C's `%g` (six significant digits).
pub(crate) fn fmt_g(x: f64) -> String {
    if x.is_nan() {
        return if x.is_sign_negative() {
            "-nan".into()
        } else {
            "nan".into()
        };
    }
    if x.is_infinite() {
        return if x < 0.0 { "-inf".into() } else { "inf".into() };
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let e = format!("{x:.5e}");
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let trim = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if !(-4..6).contains(&exp) {
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim(mant), exp.abs())
    } else {
        let prec = usize::try_from(5 - exp).unwrap_or(0);
        trim(&format!("{x:.prec$}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanf_conversions() {
        let f = sscanf(b"2 1 0 1 -1 -1 50 -1 -1 0.000 0 0 -1 0 0 5", "*ddddddddfdddddd");
        assert_eq!(f.n(), 15);
        assert_eq!((f.i(0), f.i(5), f.f(8), f.i(14)), (1, 50, 0.0, 5));
        let f = sscanf(b"3 0 0 0 0 0 0 0 123456789 0 0-0-0 00", "*ddddddddfdddd");
        assert_eq!(f.n(), 13);
        assert_eq!(f.i(7), 123456789);
        assert_eq!(sscanf(b"10000000000000 0", "dd").i(0), i32::MAX);
        assert_eq!(sscanf(b"71111111iNf7-3", "dfdf").n(), 4);
        assert!(sscanf(b"1 9e999", "df").f(1).is_infinite());
        assert_eq!(sscanf(b"1.e2x", "f").f(0), 100.0);
        assert_eq!(sscanf(b"1e+x", "f").f(0), 1.0);
        assert_eq!(sscanf(b"- 1", "d").n(), 0);
        let mut s = Scan::new(b" #ff0a 1");
        s.skip_ws();
        assert!(s.lit(b'#'));
        assert_eq!((s.hex2(), s.hex2(), s.hex2()), (Some(255), Some(10), Some(1)));
        assert!(Scan::new(b"-1").hex2().is_some_and(|v| v > 255));
    }

    #[test]
    fn lines_and_eof() {
        let mut s = Scan::new(b"a\n\nb");
        assert_eq!(s.getline(), Some(&b"a\n"[..]));
        assert_eq!(s.getline(), Some(&b"\n"[..]));
        assert!(!s.eof);
        assert_eq!(s.getline(), Some(&b"b"[..]));
        assert!(s.eof);
        assert_eq!(s.getline(), None);
        let mut s = Scan::new(b"#FIG 3.2  Produced by xfig\nx");
        assert_eq!(s.fgets16(), Some(&b"#FIG 3.2  Produc"[..]));
        let mut s = Scan::new(b"12 ");
        assert_eq!(s.int(), Some(12));
        assert_eq!(s.int(), None);
        assert!(s.eof);
    }

    #[test]
    fn percent_g() {
        assert_eq!(fmt_g(1.0), "1");
        assert_eq!(fmt_g(0.5), "0.5");
        assert_eq!(fmt_g(-3.25), "-3.25");
        assert_eq!(fmt_g(1e99), "1e+99");
        assert_eq!(fmt_g(123456.0), "123456");
        assert_eq!(fmt_g(1234567.0), "1.23457e+06");
        assert_eq!(fmt_g(0.0001), "0.0001");
        assert_eq!(fmt_g(0.00001), "1e-05");
        assert_eq!(fmt_g(f64::NAN), "nan");
    }

    #[test]
    fn decoding() {
        assert_eq!(Encoding::resolve(None, true), Encoding::Utf8);
        assert_eq!(Encoding::resolve(None, false), Encoding::Latin1);
        assert_eq!(Encoding::resolve(Some("ISO-8859-2"), true), Encoding::Latin2);
        assert_eq!(Encoding::resolve(Some("KOI8-R"), true), Encoding::Latin1);
        assert_eq!(Encoding::Utf8.decode("é\u{1}".as_bytes()), "é\u{1}");
        assert_eq!(Encoding::Utf8.decode(b"a\xe9"), "aé");
        assert_eq!(Encoding::Latin1.decode(b"\xe9"), "é");
        assert_eq!(Encoding::Latin2.decode(b"\xb1"), "ą");
    }
}
