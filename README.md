# LinWhisper

Floating voice-to-text tool for Linux. Click to record, click to transcribe and paste. Supports fully local transcription via whisper.cpp or cloud via Groq API.

![LinWhisper button states](src/screenshots/ui-buttons.png)

## Privacy

LinWhisper has no account, no telemetry, and no background processes. Your microphone is **never accessed** until you explicitly click the record button. Audio is captured in-memory, never written to disk. Only the transcribed text is stored locally in SQLite on your machine.

With **local mode** (`PRIMARY_TRANSCRIPTION_SERVICE=local`), everything stays on your machine - no network requests at all. With **Groq mode**, audio is sent to the Groq API and immediately discarded ([Groq's privacy policy](https://groq.com/privacy-policy/)).

## Features

- Always-on-top floating microphone button (draggable, position persists)
- One-click voice recording with visual feedback (red idle, green recording)
- Local transcription via whisper.cpp (no internet required)
- Cloud transcription via Groq API (whisper-large-v3-turbo)
- Auto-pastes transcribed text into focused input field
- SQLite history with right-click access
- No background mic access - recording only on explicit click
- Audio stays in-memory, never saved to disk

## Dependencies

### System packages

**Debian/Ubuntu:**
```bash
sudo apt install libgtk-4-dev libgraphene-1.0-dev libvulkan-dev libasound2-dev xclip xdotool wmctrl cmake libclang-dev
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 graphene vulkan-icd-loader alsa-lib xclip xdotool wmctrl cmake clang
```

### Build tools

- [just](https://github.com/casey/just) (optional, for convenient commands)

### Runtime requirements

- **X11 session required** - LinWhisper relies on `xdotool`, `xclip`, and `wmctrl` for window positioning, clipboard, and paste simulation. These are X11-only tools and **do not work on native Wayland**. If your distro runs Wayland by default (GNOME 41+, Fedora, etc.), you can either:
  - Log in to an X11/Xorg session from your display manager
  - Run under XWayland (may partially work, but not guaranteed)
- Working microphone

## Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/adolfousier/linwhisper.git
   cd linwhisper
   ```

2. Build and run:

   **Local mode** (downloads model automatically on first run):
   ```bash
   just run-local
   ```

   **With a different model:**
   ```bash
   just run-local ggml-small.en.bin
   ```

   **Groq API mode** (requires `GROQ_API_KEY` in `.env`):
   ```bash
   just run-groq
   ```

   **Without just** (manual setup):
   ```bash
   # Download a whisper model for local mode
   mkdir -p ~/.local/share/linwhisper/models
   curl -L -o ~/.local/share/linwhisper/models/ggml-base.en.bin \
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin

   # Set backend in .env
   # PRIMARY_TRANSCRIPTION_SERVICE=local  (or groq)

   cargo build --release
   cargo run --release
   ```

### Available whisper models

Models are downloaded from [HuggingFace (ggerganov/whisper.cpp)](https://huggingface.co/ggerganov/whisper.cpp). Run `just list-models` to see options.

| Model | Size | Speed | Notes |
|-------|------|-------|-------|
| `ggml-tiny.en.bin` | ~75MB | Fastest | English only |
| `ggml-base.en.bin` | ~142MB | Fast | English only (default) |
| `ggml-small.en.bin` | ~466MB | Medium | English only, better accuracy |
| `ggml-medium.en.bin` | ~1.5GB | Slow | English only, high accuracy |
| `ggml-large-v3.bin` | ~3.1GB | Slowest | Multilingual, best accuracy |

## Usage

| Action | What happens |
|---|---|
| **Left-click** | Start recording (button turns green with pulse, shows stop icon) |
| **Left-click again** | Stop recording, transcribe, auto-paste into focused input |
| **Right-click** | Popover menu with History and Quit |
| **Drag** | Move the button anywhere on screen - position saved across sessions |

> **Note:** Auto-paste uses `xclip` and `xdotool` to simulate Ctrl+V. If text doesn't paste automatically, it will still be copied to your clipboard - just paste manually with Ctrl+V.

## Stack

| Component | Crate/Tool |
|-----------|-----------|
| GUI | gtk4-rs (GTK 4) |
| Audio | cpal + hound |
| Local STT | whisper-rs (whisper.cpp) + rubato |
| Cloud STT | reqwest + Groq API |
| Database | rusqlite (bundled SQLite) |
| Paste | xclip + xdotool |
| Config | dotenvy |

## License

MIT - see [LICENSE](LICENSE)
