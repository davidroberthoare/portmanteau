//! Browser-only plumbing: keeping the project in the browser's own storage
//! (IndexedDB), and moving projects in and out as `.zip` files (a file picker
//! and a download). Compiled for the web build only.
//!
//! A project is stored as one zip (the same bytes File › Export project as zip
//! produces), under one key. Everything here is asynchronous in the browser, so
//! results come back through small queues that the app polls each frame.

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(inline_js = r#"
const DB = "portmanteau", STORE = "projects";
function open() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB, 1);
    request.onupgradeneeded = () => request.result.createObjectStore(STORE);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}
export async function idb_get(key) {
  const db = await open();
  return new Promise((resolve, reject) => {
    const query = db.transaction(STORE).objectStore(STORE).get(key);
    query.onsuccess = () => resolve(query.result ?? null);
    query.onerror = () => reject(query.error);
  });
}
export async function idb_put(key, bytes) {
  const copy = bytes.slice(); // the view into wasm memory is only valid right now
  const db = await open();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, "readwrite");
    tx.objectStore(STORE).put(copy, key);
    tx.oncomplete = () => resolve(true);
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error);
  });
}
export function pick_zip() {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".zip,application/zip";
    input.onchange = async () => {
      const file = input.files && input.files[0];
      if (!file) return resolve(null);
      resolve([file.name, new Uint8Array(await file.arrayBuffer())]);
    };
    input.addEventListener("cancel", () => resolve(null));
    input.click();
  });
}
export function download(name, bytes, mime) {
  const blob = new Blob([bytes.slice()], { type: mime });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  document.body.appendChild(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}
export async function storage_persist() {
  try { return !!(navigator.storage && await navigator.storage.persist()); } catch (e) { return false; }
}
"#)]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn idb_get(key: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn idb_put(key: &str, bytes: &[u8]) -> Result<JsValue, JsValue>;
    async fn pick_zip() -> JsValue;
    fn download(name: &str, bytes: &[u8], mime: &str);
    async fn storage_persist() -> JsValue;
}

const KEY: &str = "current-project";

thread_local! {
    static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
    static OPENED: RefCell<Option<(String, Vec<u8>)>> = const { RefCell::new(None) };
    static SAVE_RESULT: RefCell<Option<Result<(), String>>> = const { RefCell::new(None) };
}

fn wake() {
    CONTEXT.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

/// Remember the egui context so finished background work can wake the window.
pub fn set_context(ctx: egui::Context) {
    CONTEXT.with(|c| *c.borrow_mut() = Some(ctx));
}

/// The project saved in this browser last time, if any.
pub async fn load_saved() -> Option<Vec<u8>> {
    // Ask the browser not to throw this storage away when space runs short.
    let _ = storage_persist().await;
    let value = idb_get(KEY).await.ok()?;
    if value.is_null() || value.is_undefined() {
        return None;
    }
    Some(js_sys::Uint8Array::new(&value).to_vec())
}

/// Write the project's zip into the browser's storage, in the background.
pub fn save(bytes: Vec<u8>) {
    wasm_bindgen_futures::spawn_local(async move {
        let result = idb_put(KEY, &bytes).await.map(|_| ()).map_err(|e| format!("{e:?}"));
        SAVE_RESULT.with(|r| *r.borrow_mut() = Some(result));
        wake();
    });
}

/// How the last background save ended, once, if one has finished.
pub fn take_save_result() -> Option<Result<(), String>> {
    SAVE_RESULT.with(|r| r.borrow_mut().take())
}

/// Ask the writer for a project `.zip`. The bytes arrive via [`take_opened`].
pub fn request_open() {
    wasm_bindgen_futures::spawn_local(async move {
        let picked = pick_zip().await;
        if picked.is_null() || picked.is_undefined() {
            return;
        }
        let pair = js_sys::Array::from(&picked);
        let name = pair.get(0).as_string().unwrap_or_default();
        let bytes = js_sys::Uint8Array::new(&pair.get(1)).to_vec();
        OPENED.with(|o| *o.borrow_mut() = Some((name, bytes)));
        wake();
    });
}

/// A zip the writer picked: its file name and bytes, once.
pub fn take_opened() -> Option<(String, Vec<u8>)> {
    OPENED.with(|o| o.borrow_mut().take())
}

/// Hand the writer a file to save.
pub fn download_file(name: &str, bytes: &[u8], mime: &str) {
    download(name, bytes, mime);
}
