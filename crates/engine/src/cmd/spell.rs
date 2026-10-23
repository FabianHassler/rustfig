//! Spell check of the text objects (plan F5; xfig `w_srchrepl.c`, spell check
//! part): the texts go to a temporary file, the preference
//! `spellcheckcommand` (`%f` is the file) runs through the shell, and the
//! words it prints are the misspelt ones. Replacing a word is
//! `edit.search_replace`, as in xfig. Native only.

use super::{always, has_objects, ids_value, CmdResult, Command};
use crate::{Engine, EngineError};
use rustfig_model::{Object, ObjectId, ObjectKind};
use serde_json::{json, Value};

/// xfig's `MAX_MISSPELLED_WORDS`.
const MAX_WORDS: usize = 200;
/// The checker's output is read up to this many bytes.
#[cfg(not(target_arch = "wasm32"))]
const MAX_OUTPUT: u64 = 256 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const TIMEOUT_SECS: u64 = 30;
/// The default of `spellcheckcommand`; when its program is missing the
/// next one is tried.
const DEFAULT_COMMAND: &str = "aspell list < %f";
const FALLBACK_COMMAND: &str = "hunspell -l %f";

pub(super) fn commands() -> Vec<Command> {
    vec![Command {
        id: "text.spell_check",
        label: "Spell Check",
        menu: "",
        shortcut: "",
        params: "{}  — runs the spell checker (preference spellcheckcommand, %f is a file with every text) and returns {\"words\": [{\"word\", \"objects\": [id], \"texts\": [{x,y,text}]}], \"checker\", \"message\"}; fix a word with edit.search_replace. Native only",
        mutates: false,
        enabled: |e| always(e) && has_objects(e),
        run: spell_check,
    }]
}

fn spell_check(e: &mut Engine, _: &Value) -> CmdResult {
    let mut texts: Vec<(ObjectId, String, f64, f64)> = Vec::new();
    for o in e.doc().objects.iter() {
        collect(o, o.id, &mut texts);
    }
    let configured = e
        .prefs
        .str("spellcheckcommand")
        .unwrap_or_else(|| DEFAULT_COMMAND.to_string());
    let configured = if configured.trim().is_empty() {
        DEFAULT_COMMAND.to_string()
    } else {
        configured
    };
    let mut body = String::new();
    for (_, t, _, _) in &texts {
        body.push_str(t);
        body.push('\n');
    }
    let mut tried = vec![configured.clone()];
    let mut out = run_checker(&configured, &body);
    if configured == DEFAULT_COMMAND && matches!(out, Err(Failure::Missing)) {
        tried.push(FALLBACK_COMMAND.to_string());
        out = run_checker(FALLBACK_COMMAND, &body);
    }
    let (checker, output) = match out {
        Ok(o) => (tried.last().cloned().unwrap_or_default(), o),
        Err(Failure::Missing) => {
            return Err(EngineError::not_found(format!(
                "No spell checker found (tried {}). Install aspell or hunspell, or set the spell checker in Global settings.",
                tried.join(", ")
            )))
        }
        Err(Failure::Other(m)) => return Err(EngineError::io(format!("Spell check: {m}"))),
    };
    let mut words: Vec<String> = Vec::new();
    for line in output.lines() {
        let w = line.trim();
        if !w.is_empty() && !words.iter().any(|x| x == w) {
            words.push(w.to_string());
            if words.len() >= MAX_WORDS {
                break;
            }
        }
    }
    let list: Vec<Value> = words
        .iter()
        .map(|w| {
            let mut ids: Vec<ObjectId> = Vec::new();
            let mut at = Vec::new();
            for (id, t, x, y) in &texts {
                if t.contains(w.as_str()) {
                    if !ids.contains(id) {
                        ids.push(*id);
                    }
                    at.push(json!({"x": x, "y": y, "text": t}));
                }
            }
            json!({"word": w, "objects": ids_value(&ids), "texts": at})
        })
        .collect();
    let message = match words.len() {
        0 => "No misspelled words found".to_string(),
        n if n >= MAX_WORDS => format!("{n} (limit) misspelled words found. There may be more."),
        n => format!("{n} misspelled words found"),
    };
    Ok(json!({"words": list, "checker": checker, "message": message}))
}

/// Every text (inside compounds too) with the id of its top-level object.
fn collect(o: &Object, top: ObjectId, out: &mut Vec<(ObjectId, String, f64, f64)>) {
    match &o.kind {
        ObjectKind::Text(t) => out.push((top, t.text.clone(), t.origin.x, t.origin.y)),
        ObjectKind::Compound(c) => c.objects.iter().for_each(|x| collect(x, top, out)),
        _ => {}
    }
}

enum Failure {
    /// The program is not installed.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))] // only the native runner reports it
    Missing,
    Other(String),
}

#[cfg(target_arch = "wasm32")]
fn run_checker(_command: &str, _body: &str) -> Result<String, Failure> {
    Err(Failure::Other("the spell checker is not available on the web".into()))
}

#[cfg(not(target_arch = "wasm32"))]
fn run_checker(command: &str, body: &str) -> Result<String, Failure> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("rustfig-spell-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| Failure::Other(format!("cannot make a work folder: {e}")))?;
    let result = (|| {
        let input = dir.join("texts.txt");
        let output = dir.join("out.txt");
        std::fs::write(&input, body).map_err(|e| Failure::Other(format!("cannot write the temporary file: {e}")))?;
        let out_file =
            std::fs::File::create(&output).map_err(|e| Failure::Other(format!("cannot write the output file: {e}")))?;
        let path = input.to_string_lossy().to_string();
        let full = build_command(command, &path);
        #[cfg(windows)]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(&full);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.arg("-c").arg(&full);
            c
        };
        let mut child = cmd
            .stdin(Stdio::null())
            .stdout(out_file)
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Failure::Other(format!("cannot start the shell: {e}")))?;
        let start = std::time::Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if start.elapsed().as_secs() >= TIMEOUT_SECS => {
                    let _ = child.kill(); // already failing; the wait below reaps it
                    let _ = child.wait(); // same
                    return Err(Failure::Other(format!(
                        "\"{command}\" took longer than {TIMEOUT_SECS} s"
                    )));
                }
                Ok(None) => std::thread::sleep(std::time::Duration::from_millis(10)),
                Err(e) => return Err(Failure::Other(format!("\"{command}\" did not finish: {e}"))),
            }
        };
        let mut buf = Vec::new();
        if let Ok(f) = std::fs::File::open(&output) {
            // A read error leaves what was read: the list is only shorter.
            let _ = f.take(MAX_OUTPUT).read_to_end(&mut buf);
        }
        let text = String::from_utf8_lossy(&buf).to_string();
        if status.success() {
            Ok(text)
        } else if status.code() == Some(127) || (cfg!(windows) && status.code() == Some(1) && text.is_empty()) {
            Err(Failure::Missing)
        } else {
            Err(Failure::Other(format!("\"{command}\" failed ({status})")))
        }
    })();
    // Best effort: a stray temp folder is harmless.
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[cfg(not(target_arch = "wasm32"))]
/// xfig's `build_command`: `%f` becomes the file name (quoted for the shell).
fn build_command(command: &str, file: &str) -> String {
    #[cfg(windows)]
    let quoted = format!("\"{file}\"");
    #[cfg(not(windows))]
    let quoted = format!("'{}'", file.replace('\'', "'\\''"));
    if command.contains("%f") {
        command.replace("%f", &quoted)
    } else {
        format!("{command} {quoted}")
    }
}
