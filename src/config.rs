use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
pub enum TranscriptionService {
    Api,
    Local,
}

pub struct Config {
    pub transcription_service: TranscriptionService,
    pub api_base_url: String,
    pub api_key: Option<String>,
    pub api_model: String,
    pub db_path: PathBuf,
    pub whisper_model_path: PathBuf,
    pub sound_notification: bool,
}

impl Config {
    pub fn load() -> Self {
        // Try loading .env from current dir, ignore if missing
        let _ = dotenvy::dotenv();

        let transcription_service = match std::env::var("PRIMARY_TRANSCRIPTION_SERVICE")
            .unwrap_or_else(|_| "api".into())
            .to_lowercase()
            .as_str()
        {
            "local" => TranscriptionService::Local,
            // Accept "api" and legacy "groq"
            _ => TranscriptionService::Api,
        };

        // API_BASE_URL with default pointing to Groq (backwards compatible)
        let api_base_url = std::env::var("API_BASE_URL")
            .unwrap_or_else(|_| "https://api.groq.com/openai/v1".into());

        // API_KEY with GROQ_API_KEY as legacy fallback
        let api_key = std::env::var("API_KEY")
            .or_else(|_| std::env::var("GROQ_API_KEY"))
            .ok();

        if transcription_service == TranscriptionService::Api && api_key.is_none() {
            panic!("API_KEY must be set when PRIMARY_TRANSCRIPTION_SERVICE=api (legacy: GROQ_API_KEY also accepted)");
        }

        // API_MODEL with GROQ_STT_MODEL as legacy fallback
        let api_model = std::env::var("API_MODEL")
            .or_else(|_| std::env::var("GROQ_STT_MODEL"))
            .unwrap_or_else(|_| "whisper-large-v3-turbo".into());

        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("linwhisper");
        std::fs::create_dir_all(&data_dir).ok();
        let db_path = data_dir.join("history.db");

        let models_dir = data_dir.join("models");
        std::fs::create_dir_all(&models_dir).ok();
        let model_name =
            std::env::var("WHISPER_MODEL").unwrap_or_else(|_| "ggml-base.en.bin".into());
        let whisper_model_path = models_dir.join(&model_name);

        if transcription_service == TranscriptionService::Local && !whisper_model_path.exists() {
            eprintln!("ERROR: Whisper model not found at {}", whisper_model_path.display());
            eprintln!("Download it with:");
            eprintln!(
                "  mkdir -p {} && curl -L -o {} https://huggingface.co/ggerganov/whisper.cpp/resolve/main/{}",
                models_dir.display(),
                whisper_model_path.display(),
                model_name,
            );
            std::process::exit(1);
        }

        let sound_notification = std::env::var("SOUND_NOTIFICATION_ON_COMPLETION")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        Self {
            transcription_service,
            api_base_url,
            api_key,
            api_model,
            db_path,
            whisper_model_path,
            sound_notification,
        }
    }
}
