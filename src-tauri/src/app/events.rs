use tauri::{Emitter, EventTarget};

/// Emit an event to ALL windows (main + pill).
pub fn emit_all<S: serde::Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: S) {
    let _ = app.emit(event, payload.clone());
    let _ = app.emit_to(EventTarget::labeled("pill"), event, payload.clone());
    let _ = app.emit_to(EventTarget::labeled("main"), event, payload);
}
