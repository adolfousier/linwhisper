use reqwest::multipart;

const GROQ_URL: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

pub async fn transcribe(api_key: &str, model: &str, wav_data: Vec<u8>) -> Result<String, String> {
    let file_part = multipart::Part::bytes(wav_data)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| format!("Multipart error: {e}"))?;

    let form = multipart::Form::new()
        .text("model", model.to_string())
        .text("response_format", "json")
        .part("file", file_part);

    let client = reqwest::Client::new();
    let resp = client
        .post(GROQ_URL)
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
