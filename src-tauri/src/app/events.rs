use tauri::Emitter;

/// Emit an event to ALL windows (main + pill).
/// `app.emit()` already broadcasts to every window, so no per-window calls needed.
pub fn emit_all<S: serde::Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: S) {
    let _ = app.emit(event, payload);
}
