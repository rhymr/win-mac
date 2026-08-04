use std::process::Command;

pub fn fetch_apple_notes() -> Result<Vec<(String, String)>, String> {
    let script = r#"
        tell application "Notes"
            set output to ""
            repeat with aNote in notes
                set output to output & (name of aNote) & "|||" & (plaintext of aNote) & ":::END:::"
            end repeat
            return output
        end tell
    "#;

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| e.to_string())?;

    let raw_text = String::from_utf8_lossy(&output.stdout);
    let mut notes = Vec::new();

    for note_str in raw_text.split(":::END:::") {
        if let Some((title, body)) = note_str.split_once("|||") {
            let clean_title = title.trim().replace('/', "_");
            if !clean_title.is_empty() {
                notes.push((clean_title, body.trim().to_string()));
            }
        }
    }

    Ok(notes)
}