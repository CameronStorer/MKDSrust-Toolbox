#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Mutex},
};
use toolbox_core::service::{BinaryRequest, Request, Service, Snapshot};

#[derive(Clone)]
struct Backend {
    service: Arc<Mutex<Service>>,
    blobs: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    next: Arc<AtomicU64>,
}
impl Default for Backend {
    fn default() -> Self {
        Self {
            service: Arc::new(Mutex::new(Service::default())),
            blobs: Arc::default(),
            next: Arc::default(),
        }
    }
}
#[tauri::command]
#[specta::specta]
async fn dispatch(request: Request, state: tauri::State<'_, Backend>) -> Result<Snapshot, String> {
    let backend = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        backend
            .service
            .lock()
            .map_err(|e| e.to_string())?
            .dispatch(request)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
/// Returns an opaque URL to bounded binary IPC; no image or file bytes pass through JSON.
#[tauri::command]
#[specta::specta]
async fn binary(
    request: BinaryRequest,
    state: tauri::State<'_, Backend>,
) -> Result<String, String> {
    let backend = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = backend
            .service
            .lock()
            .map_err(|e| e.to_string())?
            .binary(request)
            .map_err(|e| e.to_string())?;
        let id = backend.next.fetch_add(1, Ordering::Relaxed).to_string();
        let mut blobs = backend.blobs.lock().map_err(|e| e.to_string())?;
        if blobs.len() >= 16 {
            blobs.clear();
        }
        blobs.insert(id.clone(), bytes);
        Ok(format!(
            "{}{id}",
            if cfg!(target_os = "windows") {
                "http://toolbox.localhost/"
            } else {
                "toolbox://localhost/"
            }
        ))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
#[specta::specta]
async fn inspect(
    path: String,
    state: tauri::State<'_, Backend>,
) -> Result<toolbox_core::preview::Preview, String> {
    let backend = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let s = backend.service.lock().map_err(|e| e.to_string())?;
        let session = s.session.as_ref().ok_or("open a ROM first")?;
        let bytes = match &s.project {
            Some(p) => p.bytes(session, &path),
            None => session.file(&path),
        }
        .map_err(|e| e.to_string())?;
        toolbox_core::preview::preview(bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn bindings() -> tauri_specta::Builder {
    tauri_specta::Builder::new().commands(tauri_specta::collect_commands![
        dispatch, binary, inspect, scene
    ])
}
#[tauri::command]
#[specta::specta]
async fn scene(
    path: String,
    state: tauri::State<'_, Backend>,
) -> Result<toolbox_core::scene::SceneInfo, String> {
    let backend = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let s = backend.service.lock().map_err(|e| e.to_string())?;
        let session = s.session.as_ref().ok_or("open a ROM first")?;
        toolbox_core::scene::CourseScene::load(s.project.as_ref(), session, &path)
            .map(|s| s.info)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
fn main() {
    let bindings = bindings();
    if std::env::args().any(|a| a == "--export-bindings") {
        bindings
            .export(
                specta_typescript::Typescript::default().header("// @ts-nocheck\n"),
                concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts"),
            )
            .expect("export bindings");
        return;
    }
    let state = Backend::default();
    let blobs = state.blobs.clone();
    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("toolbox", move |_, request| {
            let id = request.uri().path().trim_start_matches('/');
            let bytes = blobs.lock().ok().and_then(|mut b| b.remove(id));
            tauri::http::Response::builder()
                .status(if bytes.is_some() { 200 } else { 404 })
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-Type", "application/octet-stream")
                .header("Cache-Control", "no-store")
                .body(bytes.unwrap_or_default())
                .expect("binary response")
        })
        .invoke_handler(bindings.invoke_handler())
        .run(tauri::generate_context!())
        .expect("run Toolbox");
}
#[cfg(test)]
mod tests {
    #[test]
    fn generated_bindings_are_current() {
        let p =
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts"));
        let temp = std::env::temp_dir().join(format!("mkdstb-bindings-{}.ts", std::process::id()));
        super::bindings()
            .export(
                specta_typescript::Typescript::default().header("// @ts-nocheck\n"),
                &temp,
            )
            .unwrap();
        let generated = std::fs::read_to_string(&temp).unwrap();
        let _ = std::fs::remove_file(temp);
        assert_eq!(
            std::fs::read_to_string(p).unwrap().replace("\r\n", "\n"),
            generated.replace("\r\n", "\n"),
            "Run npm run bindings in app/"
        );
    }
}
