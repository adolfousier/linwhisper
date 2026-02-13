# LinWhisper

Floating voice-to-text tool for Linux. Click to record, click to transcribe and paste — powered by Groq's Whisper API.

## Features

- Always-on-top floating microphone button
- One-click voice recording with visual feedback
- Transcription via Groq API (whisper-large-v3-turbo)
- Auto-pastes transcribed text into focused input field
- SQLite history with right-click access

## Dependencies

### System packages (Debian/Ubuntu)

```bash
sudo apt install libgtk-4-dev libgraphene-1.0-dev libvulkan-dev libasound2-dev xclip xdotool
```

### Runtime

- X11 session (Wayland not currently supported for paste/positioning)
- Working microphone

## Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/adolfousier/linwhisper.git
   cd linwhisper
   ```

2. Create a `.env` file with your Groq API key:
   ```
   GROQ_API_KEY=your_key_here
   GROQ_STT_MODEL=whisper-large-v3-turbo
   ```

3. Build and run:
   ```bash
   cargo build --release
   cargo run --release
   ```

## Usage

- **Left-click** the floating button to start recording (turns red with pulse)
- **Left-click** again to stop — audio is sent to Groq, transcription is pasted into your focused input
- **Right-click** to view transcription history or quit

> **Note:** Auto-paste uses `xclip` and `xdotool` to simulate Ctrl+V. Some applications or Wayland sessions may not support this. If text doesn't paste automatically, it will still be copied to your clipboard — just paste manually with Ctrl+V.

## Stack

| Component | Crate/Tool |
|-----------|-----------|
| GUI | gtk4-rs (GTK 4) |
| Audio | cpal + hound |
| API | reqwest (multipart) |
| Database | rusqlite (bundled SQLite) |
| Paste | xclip + xdotool |
| Config | dotenvy |

## License

MIT — see [LICENSE](LICENSE)
