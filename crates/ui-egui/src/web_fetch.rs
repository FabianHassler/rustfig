//! Fetching the bundled libraries in the browser (C2): the page serves
//! `libraries/` and its index `libraries.json` as static files; the library
//! panel asks for the index when it opens and for each figure the first time
//! it draws it. Answers go into the engine's cache
//! (`rustfig_engine::library::web`) and wake the UI. Browser only.

use rustfig_engine::library::web;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::Path;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// Figures asked for at once (a big library shows hundreds of icons).
const MAX_IN_FLIGHT: usize = 8;

thread_local! {
    static CTX: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
    static IN_FLIGHT: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static FAILED: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    static INDEX_ASKED: Cell<bool> = const { Cell::new(false) };
    static INDEX_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Remember the context, so an answer wakes the UI.
pub fn attach(ctx: &egui::Context) {
    CTX.with(|c| {
        if let Ok(mut c) = c.try_borrow_mut() {
            if c.is_none() {
                *c = Some(ctx.clone());
            }
        }
    });
}

fn wake() {
    CTX.with(|c| {
        if let Ok(c) = c.try_borrow() {
            if let Some(c) = c.as_ref() {
                c.request_repaint();
            }
        }
    });
}

fn js_err(e: wasm_bindgen::JsValue) -> String {
    format!("{e:?}")
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let resp = JsFuture::from(window.fetch_with_str(url)).await.map_err(js_err)?;
    let resp: web_sys::Response = resp.dyn_into().map_err(|_| "unexpected fetch result".to_string())?;
    if !resp.ok() {
        return Err(format!("{url}: HTTP {}", resp.status()));
    }
    let buf = JsFuture::from(resp.array_buffer().map_err(js_err)?)
        .await
        .map_err(js_err)?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

/// Ask for the index (once); it is in when `web::index_loaded()` says so.
pub fn request_index() {
    if INDEX_ASKED.with(Cell::get) {
        return;
    }
    INDEX_ASKED.with(|a| a.set(true));
    wasm_bindgen_futures::spawn_local(async {
        let r = match fetch_bytes(web::INDEX_FILE).await {
            Ok(b) => web::set_index(&String::from_utf8_lossy(&b)).map(|_| ()),
            Err(e) => Err(e),
        };
        if let Err(e) = r {
            INDEX_ERROR.with(|x| *x.borrow_mut() = Some(e));
            // Ask again the next time the panel opens.
            INDEX_ASKED.with(|a| a.set(false));
        }
        wake();
    });
}

/// Why the index could not be loaded, once.
pub fn take_index_error() -> Option<String> {
    INDEX_ERROR.with(|x| x.borrow_mut().take())
}

/// Whether the figure is in the cache; if not, start fetching it (a failed
/// figure is not asked again, so it counts as ready: reading it then fails
/// with the message the engine has).
pub fn figure_ready(dir: &Path, name: &str) -> bool {
    if web::cached(dir, name).is_some() {
        return true;
    }
    let key = format!("{}/{name}", dir.to_string_lossy());
    if FAILED.with(|f| f.borrow().contains(&key)) {
        return true;
    }
    let busy = IN_FLIGHT.with(|f| {
        let f = f.borrow();
        f.contains(&key) || f.len() >= MAX_IN_FLIGHT
    });
    if !busy {
        IN_FLIGHT.with(|f| f.borrow_mut().insert(key.clone()));
        let (dir, name, url) = (dir.to_path_buf(), name.to_string(), web::figure_url(dir, name));
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_bytes(&url).await {
                Ok(b) => web::put(&dir, &name, b),
                Err(_) => {
                    FAILED.with(|f| f.borrow_mut().insert(key.clone()));
                }
            }
            IN_FLIGHT.with(|f| f.borrow_mut().remove(&key));
            wake();
        });
    }
    false
}
