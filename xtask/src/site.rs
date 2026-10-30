//! `cargo xtask site [--out DIR] [--app DIR]`: the project's web site for
//! GitHub Pages (https://fabianhassler.github.io/rustfig/): a landing page,
//! the manual (`docs/manual/*.md`) as HTML pages, the screenshot, a
//! sitemap, and with `--app` the web app built by `cargo xtask web` under
//! `app/`. Static HTML and CSS only (no scripts besides the app itself), so
//! search engines read every page. The output is deterministic.

use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Where the site is served.
pub const BASE: &str = "https://fabianhassler.github.io/rustfig/";
const REPO: &str = "https://github.com/FabianHassler/rustfig";

/// One manual page: file stem, title (its `# ` heading) and HTML body.
pub struct Page {
    pub stem: String,
    pub title: String,
    pub description: String,
    pub body: String,
}

/// Escapes text for HTML.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The manual's per-system blocks (`<!-- os: mac -->` … `<!-- os: end -->`,
/// which the program shows only on that system) as labelled boxes.
fn os_blocks(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut open = false;
    for line in md.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("<!-- os:").and_then(|r| r.strip_suffix("-->")) {
            if open {
                out.push_str("\n</div>\n\n");
                open = false;
            }
            let systems: Vec<&str> = rest
                .split_whitespace()
                .filter(|s| *s != "end")
                .map(|s| match s {
                    "mac" => "macOS",
                    "windows" => "Windows",
                    "linux" => "Linux",
                    other => other,
                })
                .collect();
            if !systems.is_empty() {
                let _ = write!(
                    out,
                    "<div class=\"os\"><p class=\"os-label\">On {}</p>\n\n",
                    systems.join(" and ")
                );
                open = true;
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    if open {
        out.push_str("\n</div>\n");
    }
    out
}

/// A link to another topic (`keys.md#ctrl`) becomes its page (`keys.html#ctrl`).
fn page_link(dest: &str) -> Option<String> {
    if dest.contains("://") {
        return None;
    }
    let (file, frag) = dest.split_once('#').map_or((dest, None), |(f, h)| (f, Some(h)));
    let stem = file.strip_suffix(".md")?;
    Some(match frag {
        Some(h) => format!("{stem}.html#{h}"),
        None => format!("{stem}.html"),
    })
}

/// One topic's Markdown as a page.
pub fn page(stem: &str, md: &str) -> Page {
    // The program's manual window has a search box; the site doesn't.
    let md = md.replace(
        " This manual is part of the program: it works offline and you can search it with the box above.",
        " The same manual is built into the program (Help > RustFig Manual), works offline and can be searched there.",
    );
    let title = md
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .unwrap_or(stem)
        .trim()
        .to_string();
    // The first paragraph, for the search result snippet.
    let description = md
        .split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty() && !p.starts_with('#') && !p.starts_with("<!--") && !p.starts_with('|'))
        .map(|p| {
            let plain: String = p
                .chars()
                .filter(|c| !matches!(c, '*' | '`' | '[' | ']'))
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let plain = strip_link_targets(&plain);
            if plain.chars().count() > 155 {
                let cut: String = plain.chars().take(152).collect();
                format!("{}…", cut.trim_end())
            } else {
                plain
            }
        })
        .unwrap_or_default();
    let src = os_blocks(&md);
    let parser =
        Parser::new_ext(&src, Options::ENABLE_TABLES | Options::ENABLE_HEADING_ATTRIBUTES).map(|ev| match ev {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                let dest_url = page_link(&dest_url).map_or(dest_url, CowStr::from);
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                })
            }
            other => other,
        });
    let mut body = String::new();
    html::push_html(&mut body, parser);
    Page {
        stem: stem.to_string(),
        title,
        description,
        body,
    }
}

/// `(text)` link targets left after the brackets were dropped.
fn strip_link_targets(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('(') {
        let (head, tail) = rest.split_at(i);
        out.push_str(head);
        match tail.find(')') {
            Some(j) if tail.get(1..j).is_some_and(|t| t.ends_with(".md") || t.contains(".md#")) => {
                rest = tail.get(j + 1..).unwrap_or("");
            }
            _ => {
                out.push('(');
                rest = tail.get(1..).unwrap_or("");
            }
        }
    }
    out.push_str(rest);
    out
}

/// The common head of every page.
fn head(title: &str, description: &str, canonical: &str, css: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{t}</title>
<meta name="description" content="{d}">
<link rel="canonical" href="{c}">
<meta property="og:title" content="{t}">
<meta property="og:description" content="{d}">
<meta property="og:url" content="{c}">
<meta property="og:type" content="website">
<meta property="og:image" content="{BASE}rustfig-window.png">
<link rel="stylesheet" href="{css}">
</head>
"#,
        t = esc(title),
        d = esc(description),
        c = canonical,
    )
}

fn nav(prefix: &str) -> String {
    format!(
        r##"<header class="top"><nav>
<a class="brand" href="{prefix}index.html">RustFig</a>
<a href="{prefix}index.html#download">Download</a>
<a href="{prefix}manual/index.html">Manual</a>
<a href="{prefix}compare.html">Compared</a>
<a href="{prefix}app/">Try it in the browser</a>
<a href="{REPO}">GitHub</a>
</nav></header>
"##
    )
}

fn footer() -> String {
    format!(
        "<footer>RustFig is open source (MIT OR Apache-2.0; parts ported from xfig and fig2dev under the xfig licence). \
         <a href=\"{REPO}\">Source on GitHub</a> · <a href=\"{REPO}/releases\">Releases</a> · \
         <a href=\"https://github.com/FabianHassler/fig2dev\">fig2dev in Rust</a></footer>\n</body>\n</html>\n"
    )
}

/// The questions people search for, answered on the landing page (also as
/// schema.org FAQ data).
const FAQ: &[(&str, &str)] = &[
    (
        "Can I run xfig on Windows?",
        "Yes, with RustFig: it is xfig rebuilt as a native Windows program. Download the Windows zip, unpack it and start rustfig.exe. No X server, Cygwin or WSL is needed.",
    ),
    (
        "Do I need an X server or XQuartz on macOS?",
        "No. RustFig is a native macOS app (Apple silicon), installed from a .dmg or with Homebrew. It needs neither XQuartz nor any other X11 server.",
    ),
    (
        "Does it open my old .fig files?",
        "Yes. RustFig reads every Fig format version (1.3 to 3.2) and saves Fig 3.2 that xfig and fig2dev read unchanged, so you can go back and forth with xfig.",
    ),
    (
        "Is it the same program as xfig?",
        "It works like xfig: the same modes, three-button mouse functions, panels, keys and file format. It is a new implementation in Rust, with modern additions such as undo, native printing, LaTeX typeset on the canvas and a command search.",
    ),
    (
        "How do I export to PDF, EPS, SVG or LaTeX?",
        "File > Export offers every fig2dev language: PDF, EPS, PostScript, SVG, PNG and other bitmaps, pstex/pdftex, TikZ, PSTricks, pict2e and more. fig2dev is built in (ported to Rust); no separate installation is needed.",
    ),
    (
        "Does it run on Linux and in the browser?",
        "Yes: .deb and .tar.gz packages for x86-64 and ARM64 Linux, and a web version that runs in the browser without installing anything.",
    ),
];

/// The landing page.
pub fn landing() -> String {
    let mut s = head(
        "RustFig – a modern xfig for Windows, macOS, Linux and the browser",
        "RustFig is xfig rebuilt in Rust: the classic vector drawing program, native on Windows, macOS and Linux and in the browser, with no X server or XQuartz, fig2dev and LaTeX built in.",
        BASE,
        "style.css",
    );
    let faq_ld: Vec<String> = FAQ
        .iter()
        .map(|(q, a)| {
            format!(
                r#"{{"@type": "Question", "name": "{}", "acceptedAnswer": {{"@type": "Answer", "text": "{}"}}}}"#,
                q.replace('"', "\\\""),
                a.replace('"', "\\\"")
            )
        })
        .collect();
    // Google Search Console's ownership check for the URL prefix BASE.
    let mut ld = String::from(
        "<meta name=\"google-site-verification\" content=\"VhJxcmEGe9HeblW2S-U6HkukDuyYXq5AJIMqeByEZhk\">\n",
    );
    let _ = write!(
        ld,
        r#"<script type="application/ld+json">
{{"@context": "https://schema.org", "@graph": [
 {{"@type": "SoftwareApplication", "name": "RustFig", "url": "{BASE}", "sameAs": "{REPO}",
  "description": "A modern xfig: the vector drawing program rebuilt in Rust for Windows, macOS, Linux and the browser, without an X server, with fig2dev and LaTeX built in.",
  "applicationCategory": "DesignApplication", "operatingSystem": "Windows, macOS, Linux, Web browser",
  "offers": {{"@type": "Offer", "price": "0", "priceCurrency": "EUR"}}, "image": "{BASE}rustfig-window.png"}},
 {{"@type": "FAQPage", "mainEntity": [{faq}]}}
]}}
</script>
</head>
<body>
"#,
        faq = faq_ld.join(", ")
    );
    s = s.replacen("</head>\n", &ld, 1);
    s.push_str(&nav(""));
    let _ = write!(
        s,
        r##"<main>
<section class="hero">
<h1>RustFig: xfig for today's computers</h1>
<p class="lead">The classic vector drawing program, rebuilt in Rust. It runs natively on <strong>Windows</strong>, <strong>macOS</strong> and <strong>Linux</strong> and in the browser, with <strong>no X server, XQuartz or WSL</strong>, and has fig2dev and LaTeX built in.</p>
<p class="buttons"><a class="button primary" href="#download">Download</a> <a class="button" href="app/">Try it in the browser</a> <a class="button" href="manual/index.html">Read the manual</a> <a class="button" href="compare.html">Compare with xfig and others</a></p>
<img src="rustfig-window.png" width="1600" height="1033" alt="RustFig's window in xfig's layout: a plot with LaTeX labels, LaTeX formulas typeset on the canvas, and a block diagram">
</section>

<section>
<h2>Why RustFig</h2>
<ul class="features">
<li><strong>It is xfig.</strong> The same modes, mouse buttons, panels and keys, and the same <code>.fig</code> files: RustFig opens every Fig version and saves Fig 3.2 that xfig and fig2dev read unchanged.</li>
<li><strong>It runs everywhere.</strong> Native on Windows, macOS and Linux, no X11 needed, and in the browser.</li>
<li><strong>LaTeX on the canvas.</strong> Text with the TeX flag is typeset by your TeX installation and shown as it will print, formulas included; plots of functions and data with LaTeX labels.</li>
<li><strong>Every fig2dev export, built in.</strong> PDF, SVG, EPS, PNG, pstex/pdftex, TikZ, PSTricks, pict2e and all of fig2dev's other languages, through fig2dev's drivers ported to Rust. The <code>fig2dev</code> command line comes along.</li>
<li><strong>Modern where it helps.</strong> Native printing, undo, command search, F1 help everywhere, the system's keys next to xfig's, figure libraries (Feynman diagrams, quantum circuits, optics, circuits), and automation through a JSON control channel and MCP.</li>
</ul>
</section>

<section id="download">
<h2>Download and install</h2>
<table>
<tr><th>System</th><th>How</th></tr>
<tr><td>Windows</td><td><a href="{REPO}/releases/latest">rustfig-VERSION-windows.zip</a>: unpack anywhere and start <code>rustfig.exe</code>; <code>fig-association.reg</code> makes <code>.fig</code> files open in RustFig. Or with <a href="https://scoop.sh">Scoop</a>: <code>scoop install https://raw.githubusercontent.com/FabianHassler/rustfig/main/packaging/scoop/rustfig.json</code></td></tr>
<tr><td>macOS (Apple silicon)</td><td><a href="{REPO}/releases/latest">RustFig-VERSION-macos.dmg</a>: drag RustFig to Applications (not yet notarized: allow it once under System Settings › Privacy &amp; Security). Or with Homebrew: <code>brew install fabianhassler/tap/rustfig</code></td></tr>
<tr><td>Linux (x86-64, ARM64)</td><td><a href="{REPO}/releases/latest">.deb or .tar.gz</a>: <code>sudo apt install ./rustfig_VERSION_amd64.deb</code>, or unpack the .tar.gz and run <code>install.sh</code>.</td></tr>
<tr><td>Browser</td><td><a href="app/">Open the web version</a>: nothing to install (drawings stay in your browser until you save them as a download).</td></tr>
<tr><td>From source</td><td><code>cargo install --locked --git {REPO} rustfig rustfig-cli</code> (Rust 1.92 or newer).</td></tr>
</table>
<p>The Rust fig2dev on its own: <code>cargo install fig2dev-rs</code> or <code>brew install fabianhassler/tap/fig2dev-rs</code>.</p>
</section>

<section>
<h2>Questions</h2>
"##
    );
    for (q, a) in FAQ {
        let _ = write!(s, "<h3>{}</h3>\n<p>{}</p>\n", esc(q), esc(a));
    }
    s.push_str("</section>\n</main>\n");
    s.push_str(&footer());
    s
}

/// One row of the comparison: program, platforms, X server, `.fig`, LaTeX,
/// licence, notes.
type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// What the comparison page says about each program (as of October 2026;
/// the others' facts from their own sites and package listings).
const COMPARE: &[Row] = &[
    (
        "RustFig",
        "Windows, macOS, Linux, web browser",
        "No",
        "Opens every Fig version (1.3–3.2), saves Fig 3.2 that xfig reads",
        "Typeset on the canvas; every fig2dev export built in (pstex/pdftex, TikZ, PSTricks, pict2e, …)",
        "Open source (MIT or Apache-2.0)",
        "xfig's modes, mouse buttons, panels and keys; beta",
    ),
    (
        "xfig 3.2.9a",
        "Linux and Unix; macOS with XQuartz; Windows with Cygwin/X or WSL",
        "Yes (X11)",
        "Its own format",
        "Through fig2dev, installed separately",
        "Open source (xfig licence)",
        "The original, still maintained",
    ),
    (
        "xfig-qt",
        "macOS",
        "No",
        "Its own format (it is xfig 3.2.9a)",
        "Through fig2dev, installed separately",
        "Open source (xfig licence, MIT)",
        "xfig's own code on Qt instead of X11",
    ),
    (
        "WinFIG",
        "Windows, macOS, Linux",
        "No",
        "Aims at xfig's format and rendering",
        "Exports through fig2dev",
        "Shareware",
        "An independent program modelled on xfig",
    ),
    (
        "jfig",
        "Any system with Java",
        "No",
        "Reads and writes Fig files",
        "Several export formats",
        "Shareware (registration)",
        "Java editor modelled on xfig; last releases are years old",
    ),
    (
        "Ipe",
        "Windows, macOS, Linux",
        "No",
        "No (own XML/PDF format; the figtoipe tool converts .fig)",
        "LaTeX is its native text system",
        "Open source (GPL)",
        "Built for figures in LaTeX documents and slides",
    ),
    (
        "Inkscape",
        "Windows, macOS, Linux",
        "No",
        "Import only, through an extension that needs fig2dev",
        "PDF/EPS with a LaTeX text overlay",
        "Open source (GPL)",
        "A general SVG editor, not xfig-like",
    ),
];

/// The comparison page: RustFig beside xfig and the other programs people
/// look at instead of it.
pub fn compare() -> String {
    let mut s = head(
        "RustFig, xfig, WinFIG, jfig, Ipe, Inkscape compared – xfig alternatives",
        "Which program for xfig figures on Windows, macOS and Linux? RustFig, xfig, xfig-qt, WinFIG, jfig, Ipe and Inkscape compared: platforms, X server, .fig files, LaTeX, licence.",
        &format!("{BASE}compare.html"),
        "style.css",
    );
    s.push_str("<body>\n");
    s.push_str(&nav(""));
    s.push_str(
        "<main>\n<h1>xfig and its alternatives compared</h1>\n\
         <p class=\"lead\">xfig is the classic drawing program for figures in papers and LaTeX documents, \
         but it needs an X server: XQuartz on macOS, Cygwin/X or WSL on Windows. \
         These programs open xfig's <code>.fig</code> files or do a similar job.</p>\n<table class=\"compare\">\n\
         <tr><th>Program</th><th>Systems</th><th>Needs an X server</th><th>xfig's <code>.fig</code> files</th>\
         <th>LaTeX</th><th>Licence</th><th>Notes</th></tr>\n",
    );
    // The programs' own sites (none listed where unsure).
    let site = |name: &str| match name {
        "RustFig" => Some(BASE),
        "xfig 3.2.9a" => Some("https://mcj.sourceforge.net/"),
        "xfig-qt" => Some("https://github.com/FabianHassler/xfig-qt"),
        "WinFIG" => Some("https://www.winfig.com/"),
        "Ipe" => Some("https://ipe.otfried.org/"),
        "Inkscape" => Some("https://inkscape.org/"),
        _ => None,
    };
    for (name, systems, x, fig, tex, licence, notes) in COMPARE {
        let name_html = match site(name) {
            Some(url) => format!("<a href=\"{url}\">{}</a>", esc(name)),
            None => esc(name),
        };
        let _ = writeln!(
            s,
            "<tr><th>{}</th><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            name_html,
            esc(systems),
            esc(x),
            esc(fig),
            esc(tex),
            esc(licence),
            esc(notes)
        );
    }
    s.push_str(
        "</table>\n<p class=\"note\">As of October 2026; the other programs' details come from their own \
         sites and package listings and may have changed. Corrections are welcome as GitHub issues.</p>\n\
         <h2>Which one to choose</h2>\n<ul class=\"features\">\n\
         <li><strong>You have xfig figures and want to keep working the xfig way on Windows or macOS:</strong> \
         RustFig or WinFIG. RustFig is open source, also runs in the browser, and has fig2dev's exports and \
         LaTeX typesetting built in.</li>\n\
         <li><strong>You are on Linux and xfig works for you:</strong> keep xfig; RustFig reads and writes the \
         same files, so you can switch back and forth.</li>\n\
         <li><strong>You want xfig exactly, without X11, on a Mac:</strong> xfig-qt runs xfig's own code on Qt.</li>\n\
         <li><strong>You draw mainly for LaTeX and don't need <code>.fig</code>:</strong> Ipe, or TikZ directly; \
         RustFig also exports TikZ, pict2e and pstex/pdftex.</li>\n\
         <li><strong>You want a general illustration program:</strong> Inkscape.</li>\n</ul>\n\
         <p class=\"buttons\"><a class=\"button primary\" href=\"index.html#download\">Download RustFig</a> \
         <a class=\"button\" href=\"app/\">Try it in the browser</a></p>\n</main>\n",
    );
    s.push_str(&footer());
    s
}

/// A manual page in the site's frame, with the list of topics beside it.
fn manual_html(p: &Page, toc: &str) -> String {
    let title = if p.stem == "index" {
        "RustFig manual – xfig for Windows, macOS, Linux and the browser".to_string()
    } else {
        format!("{} – RustFig manual (xfig without an X server)", p.title)
    };
    let mut s = head(
        &title,
        &p.description,
        &format!("{BASE}manual/{}.html", p.stem),
        "../style.css",
    );
    s.push_str("<body>\n");
    s.push_str(&nav("../"));
    let _ = write!(
        s,
        "<div class=\"manual\"><aside><p><a href=\"index.html\">Manual contents</a></p>\n{toc}</aside>\n<main class=\"topic\">\n{}</main></div>\n",
        p.body
    );
    s.push_str(&footer());
    s
}

const CSS: &str = r#":root { --bg: #ffffff; --fg: #1f2328; --muted: #59636e; --line: #d1d9e0; --accent: #0b5cad; --box: #f6f8fa; }
@media (prefers-color-scheme: dark) { :root { --bg: #16181d; --fg: #e6e8eb; --muted: #9aa3ad; --line: #30363d; --accent: #6cb0ff; --box: #1f232a; } }
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--fg); font: 17px/1.55 system-ui, -apple-system, "Segoe UI", sans-serif; }
a { color: var(--accent); }
header.top { border-bottom: 1px solid var(--line); }
header.top nav { max-width: 1100px; margin: 0 auto; padding: 0.7em 16px; display: flex; flex-wrap: wrap; gap: 0.4em 1.2em; align-items: baseline; }
header.top a { text-decoration: none; }
header.top .brand { font-weight: 600; font-size: 1.15em; color: var(--fg); margin-right: auto; }
main { max-width: 1100px; margin: 0 auto; padding: 0 16px 2em; }
.hero { text-align: center; padding-top: 2em; }
.hero h1 { font-size: 2.1em; line-height: 1.2; margin: 0.2em 0; }
.lead { font-size: 1.2em; color: var(--muted); max-width: 46em; margin: 0.5em auto 1em; }
.hero img { max-width: 100%; height: auto; border: 1px solid var(--line); border-radius: 6px; margin-top: 1.5em; }
.buttons { display: flex; flex-wrap: wrap; justify-content: center; gap: 0.6em; }
.button { display: inline-block; padding: 0.55em 1.1em; border: 1px solid var(--line); border-radius: 6px; text-decoration: none; }
.button.primary { background: var(--accent); color: var(--bg); border-color: var(--accent); }
section { margin-top: 2.5em; }
.features li { margin: 0.5em 0; }
table { border-collapse: collapse; width: 100%; display: block; overflow-x: auto; }
th, td { border: 1px solid var(--line); padding: 0.4em 0.6em; text-align: left; vertical-align: top; }
code { background: var(--box); padding: 0.1em 0.3em; border-radius: 4px; font-size: 0.92em; overflow-wrap: anywhere; }
pre { background: var(--box); padding: 0.8em; overflow-x: auto; border-radius: 6px; }
pre code { padding: 0; }
.manual { max-width: 1200px; margin: 0 auto; display: flex; gap: 2em; padding: 0 16px; }
.manual aside { flex: 0 0 15em; font-size: 0.93em; padding-top: 1.5em; }
.manual aside ul { list-style: none; padding: 0; margin: 0; }
.manual aside li { margin: 0.25em 0; }
.manual main.topic { flex: 1; min-width: 0; padding: 0 0 2em; margin: 0; }
.os { border-left: 3px solid var(--line); padding: 0.1em 0 0.1em 1em; margin: 1em 0; }
.os-label { font-size: 0.85em; color: var(--muted); margin: 0.3em 0; text-transform: uppercase; letter-spacing: 0.04em; }
table.compare th[scope], table.compare tr th:first-child { white-space: nowrap; }
table.compare td, table.compare th { font-size: 0.93em; }
.note { color: var(--muted); font-size: 0.9em; }
main > p.lead { margin-left: 0; }
footer { border-top: 1px solid var(--line); color: var(--muted); font-size: 0.9em; padding: 1.2em 16px; text-align: center; }
@media (max-width: 760px) { .manual { flex-direction: column; gap: 0; } .manual aside { flex: none; } .hero h1 { font-size: 1.6em; } }
"#;

/// The manual's topics in the order of its contents page: `index` first,
/// then the topics as `index.md` links them, then any left over.
pub fn manual_pages(dir: &Path) -> Result<Vec<Page>, String> {
    let read = |stem: &str| {
        std::fs::read_to_string(dir.join(format!("{stem}.md"))).map_err(|e| format!("docs/manual/{stem}.md: {e}"))
    };
    let index = read("index")?;
    let mut order = vec!["index".to_string()];
    for piece in index.split("](").skip(1) {
        if let Some(stem) = piece.split(')').next().and_then(|d| d.strip_suffix(".md")) {
            if !order.iter().any(|o| o == stem) {
                order.push(stem.to_string());
            }
        }
    }
    let mut rest: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| format!("docs/manual: {e}"))?
        .filter_map(Result::ok)
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|n| n.strip_suffix(".md"))
                .map(str::to_string)
        })
        .filter(|s| !order.contains(s))
        .collect();
    rest.sort();
    order.extend(rest);
    order.iter().map(|stem| Ok(page(stem, &read(stem)?))).collect()
}

fn write(path: &Path, text: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for e in std::fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))? {
        let e = e.map_err(|e| e.to_string())?;
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst).map_err(|e| format!("{}: {e}", src.display()))?;
        }
    }
    Ok(())
}

/// Writes the site into `out` (and the web app from `app` into `out/app`).
pub fn build(root: &Path, out: &Path, app: Option<&Path>) -> Result<usize, String> {
    let pages = manual_pages(&root.join("docs/manual"))?;
    let mut toc = String::from("<ul>\n");
    for p in pages.iter().skip(1) {
        let _ = writeln!(toc, "<li><a href=\"{}.html\">{}</a></li>", p.stem, esc(&p.title));
    }
    toc.push_str("</ul>\n");
    write(&out.join("index.html"), landing())?;
    write(&out.join("compare.html"), compare())?;
    write(&out.join("style.css"), CSS)?;
    std::fs::copy(
        root.join("docs/images/rustfig-window.png"),
        out.join("rustfig-window.png"),
    )
    .map_err(|e| format!("docs/images/rustfig-window.png: {e}"))?;
    let mut urls = vec![BASE.to_string(), format!("{BASE}compare.html"), format!("{BASE}app/")];
    for p in &pages {
        write(&out.join(format!("manual/{}.html", p.stem)), manual_html(p, &toc))?;
        urls.push(format!("{BASE}manual/{}.html", p.stem));
    }
    let mut sitemap = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for u in &urls {
        let _ = writeln!(sitemap, "<url><loc>{u}</loc></url>");
    }
    sitemap.push_str("</urlset>\n");
    write(&out.join("sitemap.xml"), sitemap)?;
    if let Some(app) = app {
        if !app.join("index.html").is_file() {
            return Err(format!(
                "{}: no index.html (run `cargo xtask web` first)",
                app.display()
            ));
        }
        copy_dir(app, &out.join("app"))?;
    }
    Ok(pages.len())
}

/// `cargo xtask site [--out DIR] [--app DIR]`.
pub fn run(root: &Path) -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let arg = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
    };
    let out = arg("--out").unwrap_or_else(|| root.join("target/site"));
    let app = arg("--app");
    let n = build(root, &out, app.as_deref())?;
    println!(
        "site: wrote {} ({n} manual pages{})",
        out.display(),
        if app.is_some() { ", web app in app/" } else { "" }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
    }

    #[test]
    fn topic_links_become_pages() {
        assert_eq!(page_link("keys.md").as_deref(), Some("keys.html"));
        assert_eq!(page_link("keys.md#ctrl").as_deref(), Some("keys.html#ctrl"));
        assert_eq!(page_link("https://x.org/a.md"), None);
        assert_eq!(page_link("../LICENSE"), None);
    }

    #[test]
    fn os_blocks_are_labelled_and_closed() {
        let s = os_blocks("a\n<!-- os: mac -->\nm\n<!-- os: linux windows -->\nw\n<!-- os: end -->\nb\n");
        assert!(s.contains("On macOS</p>") && s.contains("On Linux and Windows</p>"));
        assert_eq!(s.matches("<div class=\"os\">").count(), s.matches("</div>").count());
        assert!(!s.contains("<!--"));
    }

    #[test]
    fn every_manual_page_converts_with_a_title_and_links_to_pages() {
        let pages = manual_pages(&root().join("docs/manual")).unwrap();
        assert!(pages.len() >= 30);
        assert_eq!(pages.first().map(|p| p.stem.as_str()), Some("index"));
        let stems: Vec<&str> = pages.iter().map(|p| p.stem.as_str()).collect();
        for p in &pages {
            assert!(!p.title.is_empty() && !p.description.is_empty(), "{}", p.stem);
            for piece in p.body.split("href=\"").skip(1) {
                let target = piece.split('"').next().unwrap_or("");
                assert!(!target.contains(".md"), "{}: link to Markdown {target}", p.stem);
            }
            assert!(!p.body.contains("<!-- os:"), "{}", p.stem);
            // Every link to another page names an existing topic.
            for piece in p.body.split("href=\"").skip(1) {
                let target = piece.split('"').next().unwrap_or("");
                if let Some(stem) = target.split('#').next().and_then(|t| t.strip_suffix(".html")) {
                    assert!(stems.contains(&stem), "{}: broken link {target}", p.stem);
                }
            }
        }
    }

    #[test]
    fn site_builds_and_is_deterministic() {
        let dir = std::env::temp_dir().join(format!("rustfig-site-{}", std::process::id()));
        let n = build(&root(), &dir.join("a"), None).unwrap();
        build(&root(), &dir.join("b"), None).unwrap();
        for f in [
            "index.html",
            "manual/index.html",
            "manual/keys.html",
            "sitemap.xml",
            "style.css",
        ] {
            let a = std::fs::read(dir.join("a").join(f)).unwrap();
            assert_eq!(a, std::fs::read(dir.join("b").join(f)).unwrap(), "{f}");
        }
        let sitemap = std::fs::read_to_string(dir.join("a/sitemap.xml")).unwrap();
        assert_eq!(sitemap.matches("<url>").count(), n + 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
