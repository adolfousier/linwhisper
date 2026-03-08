use reqwest::multipart;
use std::time::Duration;

/// Send WAV audio to an OpenAI-compatible transcription endpoint and return the text.
pub async fn transcribe(
    base_url: &str,
    api_key: &str,
    model: &str,
    wav_data: Vec<u8>,
) -> Result<String, String> {
    // Validate URL scheme — reject file://, ftp://, etc.
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err("Invalid API URL: only http:// and https:// are allowed".into());
    }

    // Warn (in stderr) when using unencrypted HTTP for non-localhost
    if base_url.starts_with("http://")
        && !base_url.starts_with("http://localhost")
        && !base_url.starts_with("http://127.0.0.1")
        && !base_url.starts_with("http://[::1]")
    {
        eprintln!("WARNING: API endpoint uses unencrypted HTTP for a remote host");
    }

    let url = format!("{}/audio/transcriptions", base_url.trim_end_matches('/'));

    let file_part = multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Multipart error: {e}"))?;

    let form = multipart::Form::new()
        .text("model", model.to_string())
        .text("response_format", "json")
        .part("file", file_part);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("API error {status}: {body}"));
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {e}"))?;

    json["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No 'text' field in response: {json}"))
}
