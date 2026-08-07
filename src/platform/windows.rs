/// Apple Notes has no Windows equivalent to shell out to, so this exists
/// purely so the "Import Apple Notes" toggle in the welcome dialog has
/// something to call on Windows instead of the app failing to compile
/// there — it fails with a clear, honest message rather than silently
/// doing nothing or pretending to support something it can't.
pub fn fetch_apple_notes() -> Result<Vec<(String, String)>, String> {
    Err("Apple Notes import is only available on macOS".to_string())
}
