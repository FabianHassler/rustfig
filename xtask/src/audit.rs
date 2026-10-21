//! `cargo xtask source-audit`: lists every C function definition in
//! `old/xfig-3.2.9a/src/*.c` into `docs/source-audit.md` and keeps the status and
//! notes columns of rows already there (merged by file, name and occurrence).
//! `old/` is gitignored; without it the task prints a note and succeeds, so CI
//! never fails on it.

use std::collections::BTreeMap;
use std::path::Path;

const SRC: &str = "old/xfig-3.2.9a/src";
const DOC: &str = "docs/source-audit.md";
/// Marks the start of the generated table in the document.
const TABLE_MARK: &str = "<!-- audit-table -->";
const HEADER: &str = "| File | Function | Line | Status | RustFig item or reason | Task |\n|---|---|---:|---|---|---|";

/// One function definition found in a C file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Func {
    pub name: String,
    pub line: usize,
}

/// Blanks comments, string and character literals and preprocessor lines
/// (newlines stay, so line numbers survive).
fn strip(src: &str) -> Vec<u8> {
    let b = src.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    let mut line_start = true;
    while i < b.len() {
        let c = b[i];
        let next = b.get(i + 1).copied().unwrap_or(0);
        if line_start && (c == b' ' || c == b'\t') {
            out.push(c);
            i += 1;
            continue;
        }
        if line_start && c == b'#' {
            // preprocessor line with continuations
            while i < b.len() && b[i] != b'\n' {
                if b[i] == b'\\' && b.get(i + 1) == Some(&b'\n') {
                    out.push(b' ');
                    out.push(b'\n');
                    i += 2;
                    continue;
                }
                out.push(b' ');
                i += 1;
            }
            continue;
        }
        line_start = false;
        if c == b'\n' {
            out.push(c);
            line_start = true;
            i += 1;
        } else if c == b'/' && next == b'*' {
            i += 2;
            while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                out.push(if b[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            i += 2;
            out.push(b' ');
        } else if c == b'/' && next == b'/' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'"' || c == b'\'' {
            out.push(c);
            i += 1;
            while i < b.len() && b[i] != c && b[i] != b'\n' {
                if b[i] == b'\\' {
                    i += 1;
                    if b.get(i) == Some(&b'\n') {
                        out.push(b'\n'); // string continuation keeps the line count
                    }
                }
                i += 1;
            }
            out.push(c);
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// The identifier right before the first parenthesis of a header, with the
/// offset of that parenthesis, or `None` if the header is not a function's.
fn header_name(h: &[u8]) -> Option<(String, usize)> {
    let open = h.iter().position(|&c| c == b'(')?;
    // `void (*handler) ();`: a pointer declaration, not a function
    // (`(* name(args))(args)` is a function returning a function pointer)
    if h.get(open + 1..)?.iter().find(|c| !c.is_ascii_whitespace()) == Some(&b'*') {
        let (name, o) = header_name(h.get(open + 1..)?)?;
        return Some((name, open + 1 + o));
    }
    let mut e = open;
    while e > 0 && h[e - 1].is_ascii_whitespace() {
        e -= 1;
    }
    let mut s = e;
    while s > 0 && is_ident(h[s - 1]) {
        s -= 1;
    }
    if s == e {
        return None;
    }
    Some((String::from_utf8_lossy(&h[s..e]).into_owned(), open))
}

/// Index just after the parenthesis group opening at `open`.
fn after_group(h: &[u8], open: usize) -> Option<usize> {
    let mut d = 0usize;
    for (k, &c) in h.iter().enumerate().skip(open) {
        if c == b'(' {
            d += 1;
        } else if c == b')' {
            d = d.checked_sub(1)?;
            if d == 0 {
                return Some(k + 1);
            }
        }
    }
    None
}

/// Every function definition in `src`: a `{` at brace depth 0 whose header
/// (K&R parameter declarations included) names a function.
pub fn functions(src: &str) -> Vec<Func> {
    let t = strip(src);
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut header: Vec<u8> = Vec::new();
    let mut line = 1usize;
    let mut header_line = 1usize;
    let mut header_col0 = false;
    let mut col = 0usize;
    for &c in &t {
        let at_col0 = col == 0;
        if c == b'\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
        // `#if`/`#else` branches that both open a block unbalance the count; xfig closes
        // every function with a `}` in column 0, so that resynchronises.
        if c == b'}' && at_col0 && depth > 0 {
            depth = 0;
            header.clear();
            continue;
        }
        if depth > 0 {
            match c {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            continue;
        }
        match c {
            b'{' => {
                let text = String::from_utf8_lossy(&header).into_owned();
                let first = text.split_whitespace().next().unwrap_or("");
                let skip = text.contains('=')
                    || matches!(first, "typedef" | "struct" | "enum" | "union") && !text.contains(')')
                    || first == "extern" && text.contains("\"C\"");
                if !skip && header_col0 {
                    if let Some((name, open)) = header_name(&header) {
                        if after_group(&header, open).is_some() {
                            // the name's own line: the header may start on an earlier line
                            let nl = header[..open].iter().filter(|&&x| x == b'\n').count();
                            out.push(Func {
                                name,
                                line: header_line + nl,
                            });
                        }
                    }
                }
                header.clear();
                depth = 1;
            }
            b'}' => header.clear(),
            b';' => {
                // a prototype or declaration ends here; K&R parameter
                // declarations (text after the closing parenthesis) continue the header
                let keep = match header_name(&header) {
                    Some((_, open)) if !header.contains(&b'=') => after_group(&header, open).is_some_and(|e| {
                        header[e..]
                            .iter()
                            .find(|x| !x.is_ascii_whitespace())
                            .is_some_and(|&x| is_ident(x))
                    }),
                    _ => false,
                };
                if keep {
                    header.push(c);
                } else {
                    header.clear();
                }
            }
            _ => {
                if header.iter().all(|x| x.is_ascii_whitespace()) {
                    if c.is_ascii_whitespace() {
                        continue;
                    }
                    header_line = line;
                    header_col0 = at_col0;
                }
                header.push(c);
            }
        }
    }
    out
}

/// A row of the table: key columns plus the kept ones.
#[derive(Debug, Default, Clone)]
struct Row {
    status: String,
    note: String,
    task: String,
}

fn parse_table(doc: &str) -> BTreeMap<String, Row> {
    let mut rows = BTreeMap::new();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for l in doc.lines() {
        if !l.starts_with("| `") {
            continue;
        }
        let cells: Vec<&str> = l.trim().trim_matches('|').split(" | ").map(str::trim).collect();
        if cells.len() < 6 {
            continue;
        }
        let file = cells[0].trim_matches('`');
        let name = cells[1].trim_matches('`');
        let n = seen.entry(format!("{file}|{name}")).or_insert(0);
        *n += 1;
        rows.insert(
            format!("{file}|{name}|{n}"),
            Row {
                status: cells[3].into(),
                note: cells[4].into(),
                task: cells[5].into(),
            },
        );
    }
    rows
}

fn cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

const COUNTS_START: &str = "<!-- audit-counts -->";
const COUNTS_END: &str = "<!-- /audit-counts -->";

/// The counts block: functions per status, and the open ones (missing, partial) per task.
fn counts_block(rows: &[(String, String, String, String)]) -> String {
    let mut by_status: BTreeMap<&str, usize> = BTreeMap::new();
    // task -> file -> names
    let mut by_task: BTreeMap<String, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
    let mut copy = 0usize;
    for (file, name, status, task) in rows {
        if file == "xfigmain.c" {
            copy += 1;
            continue;
        }
        *by_status.entry(status.as_str()).or_insert(0) += 1;
        if status == "missing" || status == "partial" {
            for t in task.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                by_task
                    .entry(t.to_string())
                    .or_default()
                    .entry(file.as_str())
                    .or_default()
                    .push(name.as_str());
            }
        }
    }
    let total: usize = by_status.values().sum();
    let mut out = format!("{COUNTS_START}\n\n| Status | Functions |\n|---|---:|\n");
    for st in ["ported", "replaced", "not needed", "partial", "missing", "?"] {
        if let Some(n) = by_status.get(st) {
            out.push_str(&format!("| {st} | {n} |\n"));
        }
    }
    out.push_str(&format!(
        "| **total** | **{total}** (plus {copy} in `xfigmain.c`, a generated copy of `main.c`) |\n\n"
    ));
    out.push_str("Missing and partial functions by task (a function counts under each task it names):\n\n");
    let mut tasks: Vec<_> = by_task.into_iter().collect();
    tasks.sort_by_key(|(t, files)| {
        (
            std::cmp::Reverse(files.values().map(Vec::len).sum::<usize>()),
            t.clone(),
        )
    });
    for (t, files) in tasks {
        let n: usize = files.values().map(Vec::len).sum();
        let list: Vec<String> = files
            .iter()
            .map(|(f, names)| {
                format!(
                    "`{f}`: {}",
                    names.iter().map(|x| format!("`{x}`")).collect::<Vec<_>>().join(", ")
                )
            })
            .collect();
        out.push_str(&format!("- **{t}** ({n}): {}\n", list.join("; ")));
    }
    out.push_str(&format!("\n{COUNTS_END}"));
    out
}

/// Rebuilds the document: everything before the table mark is kept, the table is regenerated.
pub fn run(root: &Path) -> Result<(), String> {
    let src = root.join(SRC);
    if !src.is_dir() {
        eprintln!("source-audit: {SRC} not found (old/ is gitignored); skipped");
        return Ok(());
    }
    let doc_path = root.join(DOC);
    let doc = std::fs::read_to_string(&doc_path).unwrap_or_default();
    let (head, table) = match doc.find(TABLE_MARK) {
        Some(i) => (doc[..i].to_string(), &doc[i..]),
        None => (format!("{doc}\n"), ""),
    };
    let old = parse_table(table);
    let mut files: Vec<_> = std::fs::read_dir(&src)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "c"))
        .collect();
    files.sort();
    let mut body = format!("{TABLE_MARK}\n\n{HEADER}\n");
    let mut kept: Vec<(String, String, String, String)> = Vec::new();
    let (mut total, mut fresh) = (0usize, 0usize);
    for p in files {
        let file = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        // C sources of xfig are Latin-1 in places
        let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        let text: String = bytes.iter().map(|&b| b as char).collect();
        let mut seen: BTreeMap<String, usize> = BTreeMap::new();
        for f in functions(&text) {
            let n = seen.entry(f.name.clone()).or_insert(0);
            *n += 1;
            let row = old.get(&format!("{file}|{}|{n}", f.name)).cloned();
            if row.is_none() {
                fresh += 1;
            }
            let r = row.unwrap_or_else(|| Row {
                status: "?".into(),
                ..Row::default()
            });
            kept.push((file.clone(), f.name.clone(), r.status.clone(), r.task.clone()));
            body.push_str(&format!(
                "| `{file}` | `{}` | {} | {} | {} | {} |\n",
                f.name,
                f.line,
                cell(&r.status),
                cell(&r.note),
                cell(&r.task)
            ));
            total += 1;
        }
    }
    let head = match (head.find(COUNTS_START), head.find(COUNTS_END)) {
        (Some(a), Some(b)) if a < b => {
            format!("{}{}{}", &head[..a], counts_block(&kept), &head[b + COUNTS_END.len()..])
        }
        _ => head,
    };
    std::fs::write(&doc_path, format!("{head}{body}")).map_err(|e| e.to_string())?;
    eprintln!("source-audit: {total} functions, {fresh} without a status yet");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_ansi_and_kr_functions() {
        let s = "#define X(a) { a }\nstatic int t[] = { 1 };\nstruct s { int a; };\nint proto(int);\n/* f() { } */\nvoid\nfoo(a, b)\n    int a;\n    char *b;\n{\n    if (a) { b = \"}\"; }\n}\nstatic void bar(void) {}\n";
        let f = functions(s);
        let names: Vec<_> = f.iter().map(|f| (f.name.as_str(), f.line)).collect();
        assert_eq!(names, vec![("foo", 7), ("bar", 13)]);
    }
}
