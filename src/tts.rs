/// Piper TTS engine — text-to-speech via Python `piper-tts` package.
///
/// Sets up a Python venv with piper-tts, downloads a voice model,
/// then pipes text through it to produce 16-bit PCM audio.
use std::path::{Path, PathBuf};

/// Piper TTS wrapper.
pub struct PiperTts {
    piper_bin: PathBuf,
    model_path: PathBuf,
    sample_rate: u32,
}

impl PiperTts {
    /// Load Piper from `piper_dir/` with a specific voice model.
    /// Expects `venv/bin/piper` and `{voice_id}.onnx` + `{voice_id}.onnx.json`.
    pub fn new(piper_dir: &Path, voice_id: &str) -> Result<Self, String> {
        dbg_log!(
            "[TTS] PiperTts::new from {} voice={}",
            piper_dir.display(),
            voice_id
        );

        let piper_bin = piper_dir.join("venv/bin/piper");
        let model_path = piper_dir.join(format!("{voice_id}.onnx"));
        let config_path = piper_dir.join(format!("{voice_id}.onnx.json"));

        if !piper_bin.exists() {
            return Err(format!("missing piper binary: {}", piper_bin.display()));
        }
        if !model_path.exists() {
            return Err(format!("missing voice model: {}", model_path.display()));
        }
        if !config_path.exists() {
            return Err(format!("missing voice config: {}", config_path.display()));
        }

        // Extract sample_rate from config
        let config_str =
            std::fs::read_to_string(&config_path).map_err(|e| format!("read config: {e}"))?;
        let sample_rate = extract_sample_rate(&config_str).unwrap_or(22050);
        dbg_log!("[TTS] sample_rate={sample_rate}");

        dbg_log!("[TTS] Piper ready");
        Ok(Self {
            piper_bin,
            model_path,
            sample_rate,
        })
    }

    /// The output sample rate from the voice config.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Synthesize speech from text. Returns i16 PCM samples.
    pub fn synthesize(&self, text: &str) -> Result<Vec<i16>, String> {
        let text = clean_for_speech(text);
        if text.is_empty() {
            return Err("nothing to speak after cleaning".into());
        }
        dbg_log!("[TTS] synthesize, cleaned len={}", text.len());
        let t0 = std::time::Instant::now();

        let output = std::process::Command::new(&self.piper_bin)
            .arg("--model")
            .arg(&self.model_path)
            .arg("--output_raw")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn piper: {e}"))?;

        use std::io::Write;
        let mut child = output;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| format!("write to piper: {e}"))?;
        }

        let result = child
            .wait_with_output()
            .map_err(|e| format!("piper wait: {e}"))?;

        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            return Err(format!("piper failed: {stderr}"));
        }

        let raw = &result.stdout;
        if raw.len() % 2 != 0 {
            return Err("piper output has odd byte count".into());
        }

        let samples: Vec<i16> = raw
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        dbg_log!(
            "[TTS] synthesized {} samples ({:.1}s audio) in {:.1}s",
            samples.len(),
            samples.len() as f32 / self.sample_rate as f32,
            t0.elapsed().as_secs_f32()
        );

        Ok(samples)
    }
}

/// Clean text for speech synthesis — strip terminal/markdown noise.
fn clean_for_speech(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim();
        // Skip empty lines, lines that are just symbols/decoration
        if trimmed.is_empty()
            || trimmed
                .chars()
                .all(|c| "─━═⏺•◆▸▹►▪■□●○-=_*#>|+".contains(c))
        {
            continue;
        }
        // Strip leading bullet/decoration chars
        let cleaned = trimmed
            .trim_start_matches(|c: char| "⏺•◆▸▹►▪■□●○-*>#".contains(c))
            .trim();
        if cleaned.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(cleaned);
    }
    // Collapse multiple spaces
    let mut result = String::with_capacity(out.len());
    let mut prev_space = false;
    for c in out.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

/// Extract sample_rate from piper config JSON.
fn extract_sample_rate(config: &str) -> Option<u32> {
    let needle = "\"sample_rate\"";
    let pos = config.find(needle)?;
    let rest = &config[pos + needle.len()..];
    let colon = rest.find(':')?;
    let after_colon = rest[colon + 1..].trim_start();
    let num_end = after_colon.find(|c: char| !c.is_ascii_digit())?;
    after_colon[..num_end].parse().ok()
}
