//! The bundled manual (plans W10-28 and W10-2): Markdown topics from
//! `docs/manual/`, compiled into the program (no file access at run time, so
//! it works offline and on the web), a topic list, a search box and a small
//! Markdown renderer for the subset the topics use: headings, paragraphs,
//! bullet and numbered lists, **bold**, *italic*, `code`, code blocks, simple
//! tables and links between topics (`[text](topic.md)`).

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
    topic!("help-tools.md"),
];

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
/// recognised is a paragraph.
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
            Block::Code(_) => {}
        }
    }
    out
}

// ---- window --------------------------------------------------------------------

/// The manual window's state.
pub struct HelpState {
    pub open: bool,
    /// Index into [`TOPICS`].
    pub topic: usize,
    pub query: String,
    scroll_top: bool,
}

impl Default for HelpState {
    fn default() -> Self {
        HelpState {
            open: false,
            topic: 0,
            query: String::new(),
            scroll_top: true,
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

fn show_blocks(ui: &mut egui::Ui, tk: &Tokens, blocks: &[Block], nav: &mut Option<String>) {
    for b in blocks {
        match b {
            Block::Heading(level, spans) => {
                ui.add_space(if *level <= 2 { 10.0 } else { 6.0 });
                let size = match level {
                    1 => 24.0,
                    2 => 18.0,
                    3 => 15.0,
                    _ => 13.0,
                };
                inline(ui, tk, spans, Some(size), nav);
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
                        show_blocks(ui, &tk, &parse(&crate::platform::for_os(t.text, os)), &mut nav);
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
