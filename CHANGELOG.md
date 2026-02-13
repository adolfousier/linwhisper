# Changelog

## v0.1.12

- Added fully local transcription via whisper.cpp (whisper-rs + rubato)
- New `PRIMARY_TRANSCRIPTION_SERVICE` env var: `local` or `groq`
- `GROQ_API_KEY` only required when using Groq backend
- New `WHISPER_MODEL` env var for selecting whisper model (default: ggml-base.en.bin)
- Models stored in `~/.local/share/linwhisper/models/`
- Clear error message with download instructions if model file is missing

## v0.1.11

- Replaced Unicode emoji icons with GTK4 symbolic SVG icons (properly centered)
- Removed all box shadows for a clean flat look on any background
- Removed white border artifact
- Fixed accidental recording via Enter key (button no longer focusable)
- Bumped icon and status label font sizes
- Added button states screenshot to README
- Fixed misleading "runs entirely on your machine" wording
- Removed em-dashes from README
- Updated Cargo edition to 2024

## v0.1.1

- Improved UI: square button with soft border radius, red idle / green recording
- Drag-to-move with position persistence across sessions
- Right-click popover menu (History, Quit)
- Hover effect styling fixes
- History dialog no longer resets button position

## v0.1.0

- Initial release
- Floating always-on-top mic button (GTK4)
- One-click voice recording via cpal
- Transcription via Groq API (whisper-large-v3-turbo)
- Auto-paste into focused input via xclip + xdotool
- SQLite transcription history
- Privacy-first: audio in-memory only, never written to disk
