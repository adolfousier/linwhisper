# Changelog

## v0.1.14 — 2026-02-13

- Global keyboard shortcuts via D-Bus actions (`record`, `stop`) — works on GNOME, KDE, Sway, Hyprland, i3
- Sound notification on transcription completion (`SOUND_NOTIFICATION_ON_COMPLETION=true`)
- Removed auto-paste — text is now copied to clipboard, user pastes manually with Ctrl+V
- Removed X11 dependency for core features (xdotool, wmctrl no longer required)
- Fixed button shape changing between states (single Image widget with locked pixel size)
- Esc key stops recording when window is focused
- Wayland-native: works on GNOME Wayland without X11 tools
- Updated README with keyboard shortcut setup for all major DEs
- Added dates to all changelog entries
- Renamed project from LinWhisper to **WhisperCrabs**

## v0.1.13 — 2026-02-13

- Custom OpenAI-compatible API base URL via `API_BASE_URL` env var
- Works with any OpenAI-compatible backend: Groq (default), Ollama, OpenRouter, LM Studio, LocalAI, etc.
- Renamed env vars: `API_KEY`, `API_MODEL`, `API_BASE_URL` (old `GROQ_API_KEY`/`GROQ_STT_MODEL` still work as fallback)
- `TranscriptionService::Groq` renamed to `Api`
- `PRIMARY_TRANSCRIPTION_SERVICE` now accepts `api` (and `groq` as legacy alias)

## v0.1.12 — 2026-02-13

- Added fully local transcription via whisper.cpp (whisper-rs + rubato)
- New `PRIMARY_TRANSCRIPTION_SERVICE` env var: `local` or `groq`
- `GROQ_API_KEY` only required when using Groq backend
- New `WHISPER_MODEL` env var for selecting whisper model (default: ggml-base.en.bin)
- Models stored in `~/.local/share/whispercrabs/models/`
- Clear error message with download instructions if model file is missing

## v0.1.11 — 2026-02-13

- Replaced Unicode emoji icons with GTK4 symbolic SVG icons (properly centered)
- Removed all box shadows for a clean flat look on any background
- Removed white border artifact
- Fixed accidental recording via Enter key (button no longer focusable)
- Bumped icon and status label font sizes
- Added button states screenshot to README
- Fixed misleading "runs entirely on your machine" wording
- Removed em-dashes from README
- Updated Cargo edition to 2024

## v0.1.1 — 2026-02-13

- Improved UI: square button with soft border radius, red idle / green recording
- Drag-to-move with position persistence across sessions
- Right-click popover menu (History, Quit)
- Hover effect styling fixes
- History dialog no longer resets button position

## v0.1.0 — 2026-02-13

- Initial release
- Floating always-on-top mic button (GTK4)
- One-click voice recording via cpal
- Transcription via Groq API (whisper-large-v3-turbo)
- Auto-paste into focused input via xclip + xdotool
- SQLite transcription history
- Privacy-first: audio in-memory only, never written to disk
