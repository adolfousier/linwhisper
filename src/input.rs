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

/// Simulate paste into the currently focused window.
/// Detects terminals (which need Ctrl+Shift+V) vs regular apps (Ctrl+V).
pub fn simulate_paste() -> Result<(), String> {
    // Small delay to let the target window fully activate
    std::thread::sleep(std::time::Duration::from_millis(150));

    // Detect if active window is a terminal emulator
    let is_terminal = Command::new("xdotool")
        .args(["getactivewindow", "getwindowclassname"])
        .output()
        .map(|out| {
            let class = String::from_utf8_lossy(&out.stdout).to_lowercase();
            class.contains("terminal")
                || class.contains("xterm")
                || class.contains("kitty")
                || class.contains("alacritty")
                || class.contains("konsole")
                || class.contains("tilix")
                || class.contains("terminator")
                || class.contains("wezterm")
                || class.contains("foot")
                || class.contains("st-")
                || class.contains("urxvt")
        })
        .unwrap_or(false);

    let key = if is_terminal { "ctrl+shift+v" } else { "ctrl+v" };
    eprintln!("Pasting with {key} (terminal={is_terminal})");

    Command::new("xdotool")
        .args(["key", "--clearmodifiers", key])
        .status()
        .map_err(|e| format!("xdotool failed: {e}"))?;

    Ok(())
}
