# LinWhisper

Floating voice-to-text tool for Linux. Click to record, click to transcribe, text copied to clipboard. Supports fully local transcription via whisper.cpp or any OpenAI-compatible API endpoint (Groq, Ollama, OpenRouter, LM Studio, LocalAI, etc.).

![LinWhisper button states](src/screenshots/ui-buttons.png)

## Privacy

LinWhisper has no account, no telemetry, and no background processes. Your microphone is **never accessed** until you explicitly click the record button. Audio is captured in-memory, never written to disk. Only the transcribed text is stored locally in SQLite on your machine.

With **local mode** (`PRIMARY_TRANSCRIPTION_SERVICE=local`), everything stays on your machine - no network requests at all. With **API mode** (`PRIMARY_TRANSCRIPTION_SERVICE=api`), audio is sent to your configured endpoint (Groq by default, but can point to a local Ollama/LM Studio instance too).

## Features

- Floating microphone button (draggable, position persists)
- One-click voice recording with visual feedback (red idle, green recording, orange transcribing)
- Global keyboard shortcuts via D-Bus (works on GNOME, KDE, Sway, etc.)
- Local transcription via whisper.cpp (no internet required)
- API transcription via any OpenAI-compatible endpoint (Groq, Ollama, OpenRouter, LM Studio, LocalAI)
- Transcribed text copied to clipboard automatically
- SQLite history with right-click access
- No background mic access - recording only on explicit click
- Audio stays in-memory, never saved to disk

## Dependencies

### System packages

**Debian/Ubuntu:**
```bash
sudo apt install libgtk-4-dev libgraphene-1.0-dev libvulkan-dev libasound2-dev xclip cmake libclang-dev
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 graphene vulkan-icd-loader alsa-lib xclip cmake clang
```

### Build tools

- [just](https://github.com/casey/just) (optional, for convenient commands)

### Runtime requirements

- Works on both **Wayland** and **X11**
- `xclip` for clipboard access
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

   **API mode** (requires `API_KEY` in `.env`):
   ```bash
   just run-api
   ```

   **Without just** (manual setup):
   ```bash
   # Download a whisper model for local mode
   mkdir -p ~/.local/share/linwhisper/models
   curl -L -o ~/.local/share/linwhisper/models/ggml-base.en.bin \
     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin

   # Set backend in .env
   # PRIMARY_TRANSCRIPTION_SERVICE=local  (or api)

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
| **Left-click** | Start recording (button turns green with pulse) |
| **Left-click again** | Stop recording, transcribe, copy to clipboard |
| **Esc** (when focused) | Stop recording |
| **Right-click** | Popover menu with History and Quit |
| **Drag** | Move the button anywhere on screen |

After transcription completes, the text is copied to your clipboard. Paste with **Ctrl+V** wherever you need it.

### Sound notification

Play an audio cue when transcription completes:

```env
SOUND_NOTIFICATION_ON_COMPLETION=true
```

This is especially useful with local models that may take a few seconds to transcribe. You can keep working in another window, hear the notification when it's done, and just Ctrl+V to paste.

## Keyboard Shortcuts

LinWhisper exposes D-Bus actions that you can bind to global keyboard shortcuts in your desktop environment. This works on **GNOME, KDE, Sway, Hyprland, i3**, and any DE that supports custom shortcuts.

**Start recording** (raises window and begins recording):
```
gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate record [] {}
```

**Stop recording** (stops recording and triggers transcription):
```
gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate stop [] {}
```

### GNOME

Settings > Keyboard > Custom Shortcuts:

| Name | Command | Suggested shortcut |
|------|---------|-------------------|
| LinWhisper Record | `gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate record [] {}` | Alt+Shift+R |
| LinWhisper Stop | `gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate stop [] {}` | Alt+Shift+S |

### KDE Plasma

System Settings > Shortcuts > Custom Shortcuts > Edit > New > Global Shortcut > Command/URL. Add the same `gdbus` commands above.

### Sway / Hyprland / i3

Add to your config:
```
# Sway / i3
bindsym Alt+Shift+r exec gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate record [] {}
bindsym Alt+Shift+s exec gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate stop [] {}

# Hyprland
bind = ALT SHIFT, R, exec, gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate record [] {}
bind = ALT SHIFT, S, exec, gdbus call --session --dest=dev.linwhisper.app --object-path=/dev/linwhisper/app --method=org.gtk.Actions.Activate stop [] {}
```

## Compatible API Backends

Any service exposing an OpenAI-compatible `/v1/audio/transcriptions` endpoint works. Set `API_BASE_URL`, `API_KEY`, and `API_MODEL` in your `.env`:

**Groq (default, no config needed):**
```env
PRIMARY_TRANSCRIPTION_SERVICE=api
API_KEY=gsk_...
```

**Ollama (local, no API key needed):**
```env
PRIMARY_TRANSCRIPTION_SERVICE=api
API_BASE_URL=http://localhost:11434/v1
API_KEY=unused
API_MODEL=whisper
```

**OpenRouter:**
```env
PRIMARY_TRANSCRIPTION_SERVICE=api
API_BASE_URL=https://openrouter.ai/api/v1
API_KEY=sk-or-...
API_MODEL=openai/whisper-1
```

**LM Studio:**
```env
PRIMARY_TRANSCRIPTION_SERVICE=api
API_BASE_URL=http://localhost:1234/v1
API_KEY=unused
API_MODEL=whisper-1
```

## Stack

| Component | Crate/Tool |
|-----------|-----------|
| GUI | gtk4-rs (GTK 4) |
| Audio | cpal + hound |
| Local STT | whisper-rs (whisper.cpp) + rubato |
| API STT | reqwest + OpenAI-compatible API |
| Database | rusqlite (bundled SQLite) |
| Clipboard | xclip |
| Config | dotenvy |

## License

MIT - see [LICENSE](LICENSE)
