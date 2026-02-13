use std::path::PathBuf;

pub struct Config {
    pub groq_api_key: String,
    pub groq_model: String,
    pub db_path: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        // Try loading .env from current dir, ignore if missing
        let _ = dotenvy::dotenv();

        let groq_api_key =
            std::env::var("GROQ_API_KEY").expect("GROQ_API_KEY must be set in .env or environment");

        let groq_model =
            std::env::var("GROQ_STT_MODEL").unwrap_or_else(|_| "whisper-large-v3-turbo".into());

        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("linwhisper");
        std::fs::create_dir_all(&data_dir).ok();
        let db_path = data_dir.join("history.db");

        Self {
            groq_api_key,
            groq_model,
            db_path,
        }
    }
}
