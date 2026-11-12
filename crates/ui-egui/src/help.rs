//! The bundled manual (plans W10-28 and W10-2): Markdown topics from
//! `docs/manual/`, compiled into the program (no file access at run time, so
//! it works offline and on the web), a topic list, a search box and a small
//! Markdown renderer for the subset the topics use: headings, paragraphs,
//! bullet and numbered lists, **bold**, *italic*, `code`, code blocks, simple
//! tables, links between topics (`[text](topic.md)`) and pictures on a line
//! of their own (`![alt](../images/….png)`, compiled in through [`IMAGES`];
//! a `-light.png` picture shows its `-dark.png` twin in the dark theme; a
//! `.mp4` movie shows its `-poster.jpg` and a link to play it on the website).

use crate::theme::Tokens;
use crate::App;

/// One manual page: its file name (the link target) and Markdown text.
pub struct Topic {
    pub file: &'static str,
    pub text: &'static str,
}

macro_rules! topic {
    ($f:literal) => {
        Topic {
            file: $f,
            text: include_str!(concat!("../../../docs/manual/", $f)),
        }
    };
}

/// Every topic, in the order of the topic list.
pub const TOPICS: &[Topic] = &[
    topic!("index.md"),
    topic!("getting-started.md"),
    topic!("window.md"),
    topic!("mouse.md"),
    topic!("keys.md"),
    topic!("menus.md"),
    topic!("draw-shapes.md"),
    topic!("draw-splines.md"),
    topic!("draw-text-pictures.md"),
    topic!("select-move.md"),
    topic!("transform.md"),
    topic!("points.md"),
    topic!("compounds.md"),
    topic!("join-split.md"),
    topic!("chop.md"),
    topic!("trim.md"),
    topic!("tangent-normal.md"),
    topic!("measure.md"),
    topic!("libraries.md"),
    topic!("plot.md"),
    topic!("attribute-modes.md"),
    topic!("attributes.md"),
    topic!("line-styles.md"),
    topic!("angle-geom.md"),
    topic!("smart-links.md"),
    topic!("snap-grid.md"),
    topic!("view.md"),
    topic!("files.md"),
    topic!("export.md"),
    topic!("print.md"),
    topic!("preferences.md"),
    topic!("automation.md"),
    topic!("ai-assistants.md"),
    topic!("help-tools.md"),
];

macro_rules! image {
    ($f:literal) => {
        (
            concat!("../images/", $f),
            include_bytes!(concat!("../../../docs/images/", $f)).as_slice(),
        )
    };
}

/// The pictures the topics show, by the path they are written with.
pub const IMAGES: &[(&str, &[u8])] = &[
    image!("window/window-light.png"),
    image!("window/window-dark.png"),
    image!("window/mouse-indicator-polyline-light.png"),
    image!("window/mouse-indicator-polyline-shift-light.png"),
    image!("window/mouse-indicator-move-light.png"),
    image!("movie/inclined-plane-poster.jpg"),
];

/// The bytes of a topic's picture, the dark twin in the dark theme when there is one.
fn image_bytes(src: &str, dark: bool) -> Option<(&'static str, &'static [u8])> {
    let find = |p: &str| IMAGES.iter().find(|(k, _)| *k == p).copied();
    let twin = dark.then(|| src.strip_suffix("-light.png")).flatten();
    twin.and_then(|stem| find(&format!("{stem}-dark.png")))
        .or_else(|| find(src))
}

/// The title of a topic: its first `# ` heading.
pub fn title_of(t: &Topic) -> &'static str {
    t.text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(str::trim)
        .unwrap_or(t.file)
}

/// The topic a name stands for: the file name (with or without `.md`) or the
/// title, ignoring case.
pub fn find_topic(name: &str) -> Option<usize> {
    let n = name.trim().to_lowercase();
    let n = n.strip_suffix(".md").unwrap_or(&n);
    TOPICS
        .iter()
        .position(|t| t.file.strip_suffix(".md").unwrap_or(t.file) == n)
        .or_else(|| TOPICS.iter().position(|t| title_of(t).to_lowercase() == n))
}

// ---- search ----------------------------------------------------------------

/// A search result: the topic and the first line that matches.
pub struct Hit {
    pub topic: usize,
    pub snippet: String,
}

/// Topics whose title or text contain every word of `query` (ignoring case),
/// titles first.
pub fn search(query: &str) -> Vec<Hit> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let Some(first) = words.first() else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for (i, t) in TOPICS.iter().enumerate() {
        let title = title_of(t).to_lowercase();
        let text = t.text.to_lowercase();
        if !words.iter().all(|w| title.contains(w) || text.contains(w)) {
            continue;
        }
        let line = t
            .text
            .lines()
            .find(|l| l.to_lowercase().contains(first.as_str()))
            .unwrap_or("");
        let snippet: String = line
            .trim()
            .trim_start_matches(['#', '-', '*', '|', ' '])
            .chars()
            .filter(|c| !matches!(c, '*' | '`'))
            .take(110)
            .collect();
        hits.push((words.iter().all(|w| title.contains(w)), Hit { topic: i, snippet }));
    }
    hits.sort_by_key(|(in_title, _)| !*in_title);
    hits.into_iter().map(|(_, h)| h).collect()
}

// ---- Markdown ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Style {
    Plain,
    Bold,
    Italic,
    Code,
    Link(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading(u8, Vec<Span>),
    Para(Vec<Span>),
    List {
        ordered: bool,
        items: Vec<Vec<Span>>,
    },
    Code(String),
    Table {
        head: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
    /// A picture on a line of its own: alt text and path.
    Image {
        alt: String,
        src: String,
    },
}

/// `![alt](src)` filling the whole line.
fn image_line(line: &str) -> Option<(String, String)> {
    let rest = line.trim().strip_prefix("![")?.strip_suffix(')')?;
    let (alt, src) = rest.split_once("](")?;
    (!src.is_empty() && !src.contains(' ')).then(|| (alt.to_string(), src.to_string()))
}

fn find_seq(chars: &[char], from: usize, pat: &[char]) -> Option<usize> {
    let n = pat.len();
    let last = chars.len().checked_sub(n)?;
    (from..=last).find(|&i| chars.get(i..i + n) == Some(pat))
}

fn collect(chars: &[char], a: usize, b: usize) -> String {
    chars.get(a..b).map(|s| s.iter().collect()).unwrap_or_default()
}

/// Inline Markdown: `**bold**`, `*italic*`, `` `code` ``, `[text](target)`.
pub fn parse_inline(src: &str) -> Vec<Span> {
    let c: Vec<char> = src.chars().collect();
    let mut out: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    let flush = |plain: &mut String, out: &mut Vec<Span>| {
        if !plain.is_empty() {
            out.push(Span {
                text: std::mem::take(plain),
                style: Style::Plain,
            });
        }
    };
    while let Some(&ch) = c.get(i) {
        let rest = i + 1;
        match ch {
            '`' => {
                if let Some(e) = find_seq(&c, rest, &['`']) {
                    flush(&mut plain, &mut out);
                    out.push(Span {
                        text: collect(&c, rest, e),
                        style: Style::Code,
                    });
                    i = e + 1;
                    continue;
                }
            }
            '*' if c.get(rest) == Some(&'*') => {
                if let Some(e) = find_seq(&c, i + 2, &['*', '*']) {
                    flush(&mut plain, &mut out);
                    out.push(Span {
                        text: collect(&c, i + 2, e),
                        style: Style::Bold,
                    });
                    i = e + 2;
                    continue;
                }
            }
            '*' if c.get(rest).is_some_and(|n| !n.is_whitespace()) => {
                if let Some(e) = find_seq(&c, rest, &['*']) {
                    flush(&mut plain, &mut out);
                    out.push(Span {
                        text: collect(&c, rest, e),
                        style: Style::Italic,
                    });
                    i = e + 1;
                    continue;
                }
            }
            '[' => {
                if let Some(m) = find_seq(&c, rest, &[']', '(']) {
                    if let Some(e) = find_seq(&c, m + 2, &[')']) {
                        flush(&mut plain, &mut out);
                        out.push(Span {
                            text: collect(&c, rest, m),
                            style: Style::Link(collect(&c, m + 2, e)),
                        });
                        i = e + 1;
                        continue;
                    }
                }
            }
            '\\' if c.get(rest).is_some_and(|n| "*`[]\\|".contains(*n)) => {
                if let Some(n) = c.get(rest) {
                    plain.push(*n);
                }
                i += 2;
                continue;
            }
            _ => {}
        }
        plain.push(ch);
        i += 1;
    }
    flush(&mut plain, &mut out);
    out
}

fn table_cells(line: &str) -> Vec<Vec<Span>> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    // `\|` is a bar inside a cell.
    t.replace("\\|", "\u{1}")
        .split('|')
        .map(|c| parse_inline(c.trim().replace('\u{1}', "\\|").as_str()))
        .collect()
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

fn list_item(line: &str) -> Option<(bool, &str)> {
    let t = line.trim_start();
    if let Some(r) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
        return Some((false, r));
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 {
        let r = t.get(digits..)?;
        return r.strip_prefix(". ").map(|r| (true, r));
    }
    None
}

/// Parse the Markdown subset into blocks. Never fails: what isn't
/// recognized is a paragraph.
pub fn parse(src: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut para = String::new();
    let mut list: Option<(bool, Vec<String>)> = None;
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    macro_rules! flush {
        () => {
            if !para.is_empty() {
                blocks.push(Block::Para(parse_inline(&std::mem::take(&mut para))));
            }
            if let Some((ordered, items)) = list.take() {
                blocks.push(Block::List {
                    ordered,
                    items: items.iter().map(|s| parse_inline(s)).collect(),
                });
            }
        };
    }
    while let Some(&line) = lines.get(i) {
        i += 1;
        let t = line.trim_end();
        if t.trim_start().starts_with("```") {
            flush!();
            let mut code = Vec::new();
            while let Some(&l) = lines.get(i) {
                i += 1;
                if l.trim_start().starts_with("```") {
                    break;
                }
                code.push(l);
            }
            blocks.push(Block::Code(code.join("\n")));
        } else if t.trim().is_empty() {
            flush!();
        } else if t.trim_start().starts_with("<!--") {
            // A comment line.
        } else if let Some((alt, src)) = image_line(t) {
            flush!();
            blocks.push(Block::Image { alt, src });
        } else if let Some(h) = t.strip_prefix('#') {
            flush!();
            let extra = h.chars().take_while(|c| *c == '#').count();
            let text = h.get(extra..).unwrap_or("").trim();
            blocks.push(Block::Heading(
                u8::try_from(extra + 1).unwrap_or(4).min(4),
                parse_inline(text),
            ));
        } else if t.trim_start().starts_with('|') {
            flush!();
            let head = table_cells(t);
            let mut rows = Vec::new();
            while let Some(&l) = lines.get(i) {
                if !l.trim_start().starts_with('|') {
                    break;
                }
                i += 1;
                if !is_rule(l) {
                    rows.push(table_cells(l));
                }
            }
            blocks.push(Block::Table { head, rows });
        } else if let Some((ordered, item)) = list_item(t) {
            if !para.is_empty() {
                blocks.push(Block::Para(parse_inline(&std::mem::take(&mut para))));
            }
            match &mut list {
                Some((o, items)) if *o == ordered => items.push(item.trim().to_string()),
                _ => {
                    flush!();
                    list = Some((ordered, vec![item.trim().to_string()]));
                }
            }
        } else if line.starts_with("  ") && list.is_some() {
            // A continuation of the last list item.
            if let Some((_, items)) = &mut list {
                if let Some(last) = items.last_mut() {
                    last.push(' ');
                    last.push_str(t.trim());
                }
            }
        } else {
            if list.is_some() {
                flush!();
            }
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(t.trim());
        }
    }
    flush!();
    blocks
}

/// The links of a topic (targets as written).
pub fn links(text: &str) -> Vec<String> {
    fn spans(out: &mut Vec<String>, s: &[Span]) {
        for sp in s {
            if let Style::Link(t) = &sp.style {
                out.push(t.clone());
            }
        }
    }
    let mut out = Vec::new();
    for b in parse(text) {
        match b {
            Block::Heading(_, s) | Block::Para(s) => spans(&mut out, &s),
            Block::List { items, .. } => items.iter().for_each(|s| spans(&mut out, s)),
            Block::Table { head, rows } => {
                head.iter().for_each(|s| spans(&mut out, s));
                rows.iter().flatten().for_each(|s| spans(&mut out, s));
            }
            Block::Code(_) | Block::Image { .. } => {}
        }
    }
    out
}

/// The pictures of a topic (paths as written).
pub fn images(text: &str) -> Vec<String> {
    parse(text)
        .into_iter()
        .filter_map(|b| match b {
            Block::Image { src, .. } => Some(src),
            _ => None,
        })
        .collect()
}

// ---- window --------------------------------------------------------------------

/// The manual window's state.
pub struct HelpState {
    pub open: bool,
    /// Index into [`TOPICS`].
    pub topic: usize,
    pub query: String,
    scroll_top: bool,
    /// Pictures already decoded, by their compiled-in path.
    textures: std::collections::HashMap<&'static str, egui::TextureHandle>,
}

impl Default for HelpState {
    fn default() -> Self {
        HelpState {
            open: false,
            topic: 0,
            query: String::new(),
            scroll_top: true,
            textures: std::collections::HashMap::new(),
        }
    }
}

impl HelpState {
    /// Open the manual at `topic` (the first page when `None`).
    pub fn show(&mut self, topic: Option<&str>) -> Result<(), String> {
        if let Some(name) = topic.filter(|t| !t.trim().is_empty()) {
            let i = find_topic(name).ok_or_else(|| format!("unknown help topic {name:?}"))?;
            self.go(i);
        }
        self.open = true;
        Ok(())
    }

    fn go(&mut self, topic: usize) {
        self.topic = topic;
        self.query.clear();
        self.scroll_top = true;
    }
}

fn inline(ui: &mut egui::Ui, tk: &Tokens, spans: &[Span], base: Option<f32>, nav: &mut Option<String>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for s in spans {
            let mut rt = egui::RichText::new(&s.text);
            if let Some(size) = base {
                rt = rt.size(size).strong();
            }
            match &s.style {
                Style::Plain => {
                    ui.label(rt);
                }
                Style::Bold => {
                    ui.label(rt.strong());
                }
                Style::Italic => {
                    ui.label(rt.italics());
                }
                Style::Code => {
                    ui.label(rt.monospace().background_color(tk.inset_bg));
                }
                Style::Link(target) => {
                    if ui.link(rt.color(tk.selection)).clicked() {
                        if target.contains("://") {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(target));
                        } else {
                            *nav = Some(target.clone());
                        }
                    }
                }
            }
        }
    });
}

/// A topic's picture, at most half its pixel size (the screenshots are taken at 2x) and no
/// wider than the page; the alt text when it cannot be shown.
/// The website, where a topic's movie plays (with its captions).
const SITE: &str = "https://fabianhassler.github.io/rustfig/";

/// A movie's poster picture: `x.mp4` shows `x-poster.jpg`.
fn poster_of(src: &str) -> Option<String> {
    src.strip_suffix(".mp4").map(|stem| format!("{stem}-poster.jpg"))
}

fn show_image(ui: &mut egui::Ui, tk: &Tokens, textures: &mut Textures, alt: &str, src: &str) {
    // A movie: its poster here, played on the website.
    let poster = poster_of(src);
    let src = poster.as_deref().unwrap_or(src);
    let tex = image_bytes(src, ui.visuals().dark_mode).and_then(|(key, bytes)| {
        if let Some(t) = textures.get(key) {
            return Some(t.clone());
        }
        let pic = rustfig_engine::import::decode_picture(bytes).ok()?;
        let size = [usize::try_from(pic.width).ok()?, usize::try_from(pic.height).ok()?];
        let img = egui::ColorImage::from_rgba_unmultiplied(size, &pic.rgba);
        let t = ui
            .ctx()
            .load_texture(format!("help-{key}"), img, egui::TextureOptions::LINEAR);
        textures.insert(key, t.clone());
        Some(t)
    });
    let Some(tex) = tex else {
        ui.label(egui::RichText::new(alt).italics().color(tk.muted));
        return;
    };
    let [w, h] = tex.size_vec2().into();
    let width = (w / 2.0).min(ui.available_width()).max(1.0);
    let size = egui::vec2(width, h * width / w.max(1.0));
    let r = ui.add(egui::Image::new((tex.id(), size)));
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, alt));
    if poster.is_some() {
        let url = format!("{SITE}manual/getting-started.html");
        if ui
            .link("\u{25B6} Play the movie, with captions, on the website")
            .clicked()
        {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
    }
    ui.add_space(6.0);
}

/// The mode a heading names (its text is the mode's name, ignoring case).
fn heading_mode(spans: &[Span]) -> Option<rustfig_engine::ToolKind> {
    let text = heading_text(spans);
    rustfig_engine::ToolKind::ALL
        .into_iter()
        .find(|t| t.label().eq_ignore_ascii_case(&text))
}

fn heading_text(spans: &[Span]) -> String {
    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
    text.trim().to_string()
}

/// A heading's picture: an xfig bitmap (or its vector tracing) or a vector icon.
#[derive(Clone, Copy)]
enum HeadingIcon {
    Xbm(&'static crate::xfig_icons::Xbm),
    Vector(crate::vector_icons::VIcon),
}

/// Headings that are not a mode's name but explain something with its own button picture.
const HEADING_ICONS: &[(&str, HeadingIcon)] = {
    use crate::xfig_icons as xi;
    use HeadingIcon::{Vector, Xbm};
    &[
        ("Library mode", Xbm(&xi::LIBRARY)),
        ("Libraries", Xbm(&xi::LIBRARY)),
        ("Smart links", Xbm(&xi::SMARTMOVE)),
        ("Angle Geom", Xbm(&xi::MOUNTHATTAN)),
        ("Point Posn and Grid Mode", Xbm(&xi::GRID1)),
        ("Snap and grid", Xbm(&xi::GRID1)),
        ("Line styles", Xbm(&xi::DASHLINE)),
        (
            "Arrowhead in the middle of a line",
            Vector(crate::vector_icons::VIcon::ArrowMid),
        ),
        ("Join and split", Xbm(&xi::JOIN_SPLIT)),
        ("Tangent and normal", Xbm(&xi::TANGENT)),
        ("Trim mode", Xbm(&xi::TRIM)),
        ("Compounds", Xbm(&xi::GLUE)),
        ("Edit a compound", Xbm(&xi::OPEN_COMP)),
        ("Plotting a curve", Xbm(&xi::PLOT)),
        ("Splines", Xbm(&xi::SPL)),
        ("Measuring", Xbm(&xi::LENMEAS)),
    ]
};

/// The picture beside a heading: the mode's icon for a mode's name, else [`HEADING_ICONS`].
fn heading_icon(spans: &[Span]) -> Option<HeadingIcon> {
    if let Some(icon) = heading_mode(spans).and_then(crate::panels::icon_of) {
        return Some(HeadingIcon::Xbm(icon));
    }
    let text = heading_text(spans);
    HEADING_ICONS
        .iter()
        .find(|(h, _)| h.eq_ignore_ascii_case(&text))
        .map(|(_, i)| *i)
}

/// The file name of a heading picture in `docs/images/icons/`.
fn heading_icon_file(icon: HeadingIcon) -> String {
    match icon {
        HeadingIcon::Xbm(x) => format!("{}.svg", x.name.replace('_', "-")),
        HeadingIcon::Vector(_) => "arrow-mid.svg".to_string(),
    }
}

/// A heading picture as SVG: the vector drawing where the icon has one, else
/// xfig's bitmap as squares.
fn heading_icon_svg(icon: HeadingIcon) -> Option<String> {
    match icon {
        HeadingIcon::Vector(v) => crate::vector_icons::svg(v),
        HeadingIcon::Xbm(x) if crate::mode_icons::traced(x.name) => {
            crate::vector_icons::svg(crate::vector_icons::VIcon::Traced(x.name))
        }
        HeadingIcon::Xbm(x) => {
            let row = x.width.div_ceil(8);
            let mut out = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">\n<path fill=\"#000\" d=\"",
                w = x.width,
                h = x.height
            );
            for y in 0..x.height {
                for c in 0..x.width {
                    let byte = x.bits.get(y * row + c / 8).copied().unwrap_or(0);
                    if byte & (1 << (c % 8)) != 0 {
                        out.push_str(&format!("M{c} {y}h1v1h-1z"));
                    }
                }
            }
            out.push_str("\"/>\n</svg>\n");
            Some(out)
        }
    }
}

/// The pictures beside the manual's headings for the website: each file
/// (name, SVG) once, and the index of `heading<TAB>file` lines it reads
/// (`docs/images/icons/`, kept current by `heading_icon_pictures_are_current`).
pub fn heading_icon_pictures() -> (Vec<(String, String)>, String) {
    let mut files: Vec<(String, String)> = Vec::new();
    let mut index = String::new();
    for t in TOPICS {
        for b in parse(t.text) {
            let Block::Heading(_, spans) = b else { continue };
            let Some(icon) = heading_icon(&spans) else { continue };
            let file = heading_icon_file(icon);
            let line = format!("{}\t{file}\n", heading_text(&spans));
            if !index.contains(&line) {
                index.push_str(&line);
            }
            if !files.iter().any(|(f, _)| *f == file) {
                if let Some(svg) = heading_icon_svg(icon) {
                    files.push((file, svg));
                }
            }
        }
    }
    files.sort();
    (files, index)
}

type Textures = std::collections::HashMap<&'static str, egui::TextureHandle>;

fn show_blocks(ui: &mut egui::Ui, tk: &Tokens, blocks: &[Block], textures: &mut Textures, nav: &mut Option<String>) {
    for b in blocks {
        match b {
            Block::Image { alt, src } => show_image(ui, tk, textures, alt, src),
            Block::Heading(level, spans) => {
                ui.add_space(if *level <= 2 { 10.0 } else { 6.0 });
                let size = match level {
                    1 => 24.0,
                    2 => 18.0,
                    3 => 15.0,
                    _ => 13.0,
                };
                // A mode's heading carries the mode's icon from the mode panel, a few others
                // their button's picture.
                match heading_icon(spans) {
                    Some(icon) => {
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(egui::vec2(size * 1.9, size * 1.7), egui::Sense::hover());
                            crate::bevel::paint(ui.painter(), r, tk, crate::bevel::State::Normal);
                            match icon {
                                HeadingIcon::Xbm(x) => crate::panels::paint_mode_icon(ui, x, r.shrink(1.0), tk.icon),
                                HeadingIcon::Vector(v) => crate::vector_icons::paint(ui, v, r.shrink(3.0), tk.icon),
                            }
                            ui.add_space(4.0);
                            inline(ui, tk, spans, Some(size), nav);
                        });
                    }
                    None => inline(ui, tk, spans, Some(size), nav),
                }
                ui.add_space(2.0);
            }
            Block::Para(spans) => {
                inline(ui, tk, spans, None, nav);
                ui.add_space(5.0);
            }
            Block::List { ordered, items } => {
                for (k, item) in items.iter().enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.add_space(8.0);
                        let mark = if *ordered {
                            format!("{}.", k + 1)
                        } else {
                            "\u{2022}".to_string()
                        };
                        ui.label(mark);
                        ui.add_space(4.0);
                        ui.vertical(|ui| inline(ui, tk, item, None, nav));
                    });
                }
                ui.add_space(5.0);
            }
            Block::Code(code) => {
                egui::Frame::new()
                    .fill(tk.inset_bg)
                    .inner_margin(6.0)
                    .corner_radius(3.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(code).monospace());
                    });
                ui.add_space(5.0);
            }
            Block::Table { head, rows } => {
                // Column widths follow the longest text in each column.
                let len = |cell: &Vec<Span>| cell.iter().map(|s| s.text.chars().count()).sum::<usize>();
                let cols = head.len().max(1);
                let weights: Vec<f32> = (0..cols)
                    .map(|c| {
                        let longest = std::iter::once(head)
                            .chain(rows.iter())
                            .filter_map(|r| r.get(c))
                            .map(len)
                            .max()
                            .unwrap_or(1);
                        (longest as f32).clamp(6.0, 60.0)
                    })
                    .collect();
                let total: f32 = weights.iter().sum();
                let gap = 12.0;
                let avail = (ui.available_width() - gap * cols as f32).max(60.0);
                let strong: Vec<Vec<Span>> = head
                    .iter()
                    .map(|cell| {
                        cell.iter()
                            .map(|s| Span {
                                text: s.text.clone(),
                                style: if s.style == Style::Plain {
                                    Style::Bold
                                } else {
                                    s.style.clone()
                                },
                            })
                            .collect()
                    })
                    .collect();
                for (r, row) in std::iter::once(&strong).chain(rows.iter()).enumerate() {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for (c, w) in weights.iter().enumerate() {
                            let width = avail * w / total;
                            ui.allocate_ui_with_layout(
                                egui::vec2(width, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_width(width);
                                    if let Some(cell) = row.get(c) {
                                        inline(ui, tk, cell, None, nav);
                                    }
                                },
                            );
                        }
                    });
                    if r == 0 {
                        ui.separator();
                    }
                }
                ui.add_space(5.0);
            }
        }
    }
}

/// The manual window (`help.open`).
pub fn help_dialog(app: &mut App, ctx: &egui::Context) {
    if !app.dialogs.help.open {
        return;
    }
    let tk = Tokens::for_dark(ctx.style().visuals.dark_mode);
    let mut open = true;
    let mut nav: Option<String> = None;
    let os = crate::platform::Os::current(ctx);
    let st = &mut app.dialogs.help;
    egui::Window::new("Manual")
        .open(&mut open)
        .default_size([860.0, 580.0])
        .show(ctx, |ui| {
            egui::TopBottomPanel::top("help_search").show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search");
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut st.query)
                            .hint_text("words in a title or text")
                            .desired_width(280.0),
                    );
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "Search the manual"));
                    if ui.button("Clear").clicked() {
                        st.query.clear();
                    }
                });
                ui.add_space(2.0);
            });
            egui::SidePanel::left("help_topics")
                .resizable(false)
                .exact_width(190.0)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().id_salt("help_topic_list").show(ui, |ui| {
                        for (i, t) in TOPICS.iter().enumerate() {
                            let on = i == st.topic && st.query.trim().is_empty();
                            if ui.selectable_label(on, title_of(t)).clicked() {
                                st.go(i);
                            }
                        }
                    });
                });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                let mut scroll = egui::ScrollArea::vertical()
                    .id_salt("help_page")
                    .auto_shrink([false, false]);
                if st.scroll_top {
                    scroll = scroll.vertical_scroll_offset(0.0);
                    st.scroll_top = false;
                }
                scroll.show(ui, |ui| {
                    if st.query.trim().is_empty() {
                        let Some(t) = TOPICS.get(st.topic) else { return };
                        let blocks = parse(&crate::platform::for_os(t.text, os));
                        show_blocks(ui, &tk, &blocks, &mut st.textures, &mut nav);
                        return;
                    }
                    let hits = search(&st.query);
                    ui.heading(format!(
                        "{} result(s) for \u{201c}{}\u{201d}",
                        hits.len(),
                        st.query.trim()
                    ));
                    for h in hits {
                        let Some(t) = TOPICS.get(h.topic) else { continue };
                        if ui.link(egui::RichText::new(title_of(t)).color(tk.selection)).clicked() {
                            nav = Some(t.file.to_string());
                        }
                        ui.label(egui::RichText::new(h.snippet).small().color(tk.muted));
                        ui.add_space(4.0);
                    }
                });
            });
        });
    if let Some(target) = nav {
        if let Some(i) = find_topic(&target) {
            st.go(i);
        }
    }
    st.open = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_styles() {
        let s = parse_inline("a **b** *c* `d` [e](f.md) \\* g");
        let styles: Vec<&Style> = s.iter().map(|x| &x.style).collect();
        assert_eq!(
            styles,
            [
                &Style::Plain,
                &Style::Bold,
                &Style::Plain,
                &Style::Italic,
                &Style::Plain,
                &Style::Code,
                &Style::Plain,
                &Style::Link("f.md".into()),
                &Style::Plain
            ]
        );
        // Unterminated markers stay text; hostile input does not panic.
        for src in ["**x", "`x", "[x](", "[x]", "*", "\\", "]( )", "***"] {
            let _ = parse_inline(src);
        }
    }

    #[test]
    fn blocks() {
        let b = parse("# T\n\ntext\nmore\n\n- a\n- b\n  cont\n\n1. x\n2. y\n\n| h | i |\n|---|---|\n| 1 | 2 |\n\n```\ncode\n```\n");
        assert!(matches!(b.first(), Some(Block::Heading(1, _))));
        assert!(
            matches!(b.get(1), Some(Block::Para(p)) if p.iter().map(|s| s.text.as_str()).collect::<String>() == "text more")
        );
        assert!(matches!(b.get(2), Some(Block::List { ordered: false, items }) if items.len() == 2));
        assert!(matches!(b.get(3), Some(Block::List { ordered: true, items }) if items.len() == 2));
        assert!(matches!(b.get(4), Some(Block::Table { rows, .. }) if rows.len() == 1));
        assert!(matches!(b.get(5), Some(Block::Code(c)) if c == "code"));
    }

    #[test]
    fn search_finds_titles_and_text() {
        assert!(!search("chop").is_empty());
        assert!(search("CHOP")
            .iter()
            .any(|h| title_of(&TOPICS[h.topic]).to_lowercase().contains("chop")));
        assert!(search("zzzzqqqq").is_empty());
        assert!(search("   ").is_empty());
    }

    /// Every feature is explained in the help: each menu command (engine
    /// commands with a menu path, and the UI's own commands) appears in some
    /// manual topic by its label or its id.
    #[test]
    fn every_menu_command_is_in_the_manual() {
        let manual: String = TOPICS
            .iter()
            .map(|t| t.text.to_lowercase())
            .collect::<Vec<_>>()
            .join("\n");
        let norm = |l: &str| l.trim_end_matches('…').trim_end_matches("...").trim().to_lowercase();
        let mut missing: Vec<String> = rustfig_engine::cmd::all()
            .iter()
            .filter(|c| !c.menu.is_empty())
            .map(|c| (c.id, c.label))
            .chain(crate::menus::UI_COMMANDS.iter().map(|c| (c.id, c.label)))
            .filter(|(id, l)| !manual.contains(&norm(l)) && !manual.contains(id))
            .map(|(id, l)| format!("{id} ({l})"))
            .collect();
        missing.sort();
        assert!(missing.is_empty(), "not in the manual: {}", missing.join(", "));
    }

    #[test]
    fn every_manual_link_resolves() {
        for t in TOPICS {
            for l in links(t.text) {
                if l.contains("://") {
                    continue;
                }
                assert!(find_topic(&l).is_some(), "{}: link {l:?} has no topic", t.file);
            }
        }
    }

    #[test]
    fn every_manual_picture_is_compiled_in() {
        for t in TOPICS {
            for src in images(t.text) {
                let src = poster_of(&src).unwrap_or(src);
                assert!(
                    image_bytes(&src, false).is_some(),
                    "{}: picture {src:?} is not in IMAGES",
                    t.file
                );
            }
        }
        for (src, bytes) in IMAGES {
            assert!(
                rustfig_engine::import::decode_picture(bytes).is_ok(),
                "{src} does not decode"
            );
        }
    }

    /// Every mode of the mode panel is explained under a heading with its name, which shows
    /// its icon.
    #[test]
    fn every_mode_has_a_heading_with_its_icon() {
        let mut found = Vec::new();
        for t in TOPICS {
            for b in parse(t.text) {
                if let Block::Heading(_, s) = b {
                    if let Some(m) = heading_mode(&s) {
                        found.push(m);
                    }
                }
            }
        }
        use rustfig_engine::ToolKind;
        for t in ToolKind::ALL {
            // The three modes without an xfig picture have no icon (yet).
            let iconless = matches!(
                t,
                ToolKind::None | ToolKind::OpenClose | ToolKind::Find | ToolKind::BoundingBox
            );
            assert!(
                iconless || crate::panels::icon_of(t).is_some(),
                "the mode {:?} has no icon",
                t.label()
            );
            if t != ToolKind::None {
                assert!(found.contains(&t), "no manual heading names the mode {:?}", t.label());
            }
        }
    }

    /// The Keys topic lists every mode key, and says for the Control-key modes whether they work
    /// with xfig's keys off, as the key handling decides it.
    #[test]
    fn keys_topic_lists_every_mode_key() {
        use rustfig_engine::ToolKind;
        let keys = TOPICS
            .iter()
            .find(|t| t.file == "keys.md")
            .map(|t| t.text)
            .unwrap_or("");
        for t in ToolKind::ALL {
            let k = t.shortcut();
            if k.is_empty() {
                continue;
            }
            let k = k.to_ascii_uppercase().replace("SHIFT", "Shift");
            let row = format!("| {} | {k} |", t.label());
            assert!(keys.contains(&row), "keys.md lacks the row {row:?}");
        }
        for (spec, id) in crate::menus::XFIG_KEYS
            .iter()
            .filter(|(s, i)| s.starts_with("PhysCtrl+") && i.starts_with("tool."))
        {
            let Some(t) = ToolKind::from_id(id) else { panic!("{id}") };
            let letter = spec.trim_start_matches("PhysCtrl+");
            assert!(
                keys.contains(&format!("| {} | ⌃ {letter} |", t.label())),
                "macOS row of {id}"
            );
            let always = crate::menus::xfig_key_always(spec, id, false);
            let row = format!(
                "| {} | Ctrl+{letter} | {}",
                t.label(),
                if always { "always" } else { "with xfig's keys on" }
            );
            assert!(keys.contains(&row), "keys.md lacks {row:?}");
        }
    }

    /// `docs/images/icons/` holds every heading picture and its index as made
    /// now; `RUSTFIG_UPDATE_ICONS=1` (or `cargo xtask icons`) writes them.
    #[test]
    fn heading_icon_pictures_are_current() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/images/icons");
        let (files, index) = heading_icon_pictures();
        assert!(files.len() > 40, "{} pictures", files.len());
        let update = std::env::var_os("RUSTFIG_UPDATE_ICONS").is_some();
        if update {
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            for (f, svg) in &files {
                std::fs::write(dir.join(f), svg).unwrap();
            }
            std::fs::write(dir.join("index.tsv"), &index).unwrap();
        }
        let stale = "docs/images/icons is stale: run cargo xtask icons";
        assert_eq!(
            std::fs::read_to_string(dir.join("index.tsv")).unwrap_or_default(),
            index,
            "{stale}"
        );
        for (f, svg) in &files {
            assert_eq!(
                &std::fs::read_to_string(dir.join(f)).unwrap_or_default(),
                svg,
                "{f}: {stale}"
            );
        }
        let svgs = std::fs::read_dir(&dir)
            .map(|d| {
                d.flatten()
                    .filter(|e| e.path().extension().is_some_and(|x| x == "svg"))
                    .count()
            })
            .unwrap_or(0);
        assert_eq!(svgs, files.len(), "{stale} (pictures no heading uses)");
    }

    #[test]
    fn heading_icons_name_real_headings() {
        let headings: Vec<String> = TOPICS
            .iter()
            .flat_map(|t| parse(t.text))
            .filter_map(|b| match b {
                Block::Heading(_, s) => Some(heading_text(&s).to_lowercase()),
                _ => None,
            })
            .collect();
        for (h, _) in HEADING_ICONS {
            assert!(headings.contains(&h.to_lowercase()), "no manual heading {h:?}");
        }
    }

    #[test]
    fn image_lines() {
        assert_eq!(
            parse("a\n![The window](../images/w-light.png)\nb"),
            vec![
                Block::Para(parse_inline("a")),
                Block::Image {
                    alt: "The window".into(),
                    src: "../images/w-light.png".into()
                },
                Block::Para(parse_inline("b")),
            ]
        );
        assert_eq!(image_line("![x](a b.png)"), None);
        assert_eq!(image_line("see ![x](a.png)"), None);
    }

    #[test]
    fn topic_files_are_unique_and_titled() {
        for (i, t) in TOPICS.iter().enumerate() {
            assert!(t.text.starts_with("# "), "{} must start with a title", t.file);
            assert!(TOPICS.iter().skip(i + 1).all(|o| o.file != t.file), "{} twice", t.file);
        }
    }

    /// Each block written for some systems (`<!-- os: … -->`) is closed
    /// with `<!-- os: end -->`, and every system still gets the title.
    #[test]
    fn system_blocks_are_closed() {
        use crate::platform::{for_os, Os};
        for t in TOPICS {
            let mut open = false;
            for line in t.text.lines().map(str::trim) {
                if line == "<!-- os: end -->" {
                    assert!(open, "{}: end without a block", t.file);
                    open = false;
                } else if line.starts_with("<!-- os:") {
                    open = true;
                }
            }
            assert!(!open, "{}: block not closed", t.file);
            for os in [Os::Mac, Os::Windows, Os::Linux] {
                assert!(for_os(t.text, os).starts_with("# "), "{}", t.file);
            }
        }
    }

    /// Every mode has a section headed with its name and quoting what the
    /// mouse-function indicator says for each button, so a new mode cannot
    /// ship undocumented and the manual cannot drift from the indicator.
    #[test]
    fn every_mode_is_documented_with_its_indicator_words() {
        use rustfig_engine::{Engine, ToolKind};
        for t in ToolKind::ALL {
            let heading = format!("## {}\n", t.label());
            let Some(topic) = TOPICS.iter().find(|x| x.text.contains(&heading)) else {
                panic!("no manual topic has a section \"{}\" (mode {})", t.label(), t.id());
            };
            let section = topic.text.split(&heading).nth(1).unwrap_or("");
            let section = section.split("\n## ").next().unwrap_or("");
            let mut e = Engine::new();
            e.set_tool(t);
            let h = e.mouse_hints();
            for (button, word) in [("left", h.left), ("middle", h.middle), ("right", h.right)] {
                if !word.is_empty() {
                    assert!(
                        section.contains(&format!("\u{201c}{word}\u{201d}")),
                        "{}: the {button} button's indicator text \u{201c}{word}\u{201d} is not quoted in its section",
                        t.label()
                    );
                }
            }
            let key = t.shortcut();
            if !key.is_empty() {
                assert!(
                    section.contains(&format!("`{key}`")),
                    "{}: key `{key}` is not named in its section",
                    t.label()
                );
            }
        }
    }
}
