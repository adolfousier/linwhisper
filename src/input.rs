use arboard::Clipboard;

/// Copy text to the system clipboard (cross-platform).
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Failed to open clipboard: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("Failed to copy to clipboard: {e}"))?;
    Ok(())
}
