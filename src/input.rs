use std::process::Command;

/// Copy text to clipboard via xclip.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn xclip: {e}"))?;

    if let Some(ref mut stdin) = child.stdin {
        use std::io::Write;
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to xclip: {e}"))?;
    }
    child
        .wait()
        .map_err(|e| format!("xclip failed: {e}"))?;

    Ok(())
}