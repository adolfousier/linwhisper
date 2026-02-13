use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq)]
pub enum TranscriptionService {
    Groq,
    Local,
}

pub struct Config {
    pub transcription_service: TranscriptionService,
    pub groq_api_key: Option<String>,
    pub groq_model: String,
    pub db_path: PathBuf,
    pub whisper_model_path: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        // Try loading .env from current dir, ignore if missing
        let _ = dotenvy::dotenv();

        let transcription_service = match std::env::var("PRIMARY_TRANSCRIPTION_SERVICE")
            .unwrap_or_else(|_| "groq".into())
            .to_lowercase()
            .as_str()
        {
            "local" => TranscriptionService::Local,
            _ => TranscriptionService::Groq,
        };

        let groq_api_key = std::env::var("GROQ_API_KEY").ok();

        if transcription_service == TranscriptionService::Groq && groq_api_key.is_none() {
            panic!("GROQ_API_KEY must be set when PRIMARY_TRANSCRIPTION_SERVICE=groq");
        }

        let groq_model =
            std::env::var("GROQ_STT_MODEL").unwrap_or_else(|_| "whisper-large-v3-turbo".into());

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

        Self {
            transcription_service,
            groq_api_key,
            groq_model,
            db_path,
            whisper_model_path,
        }
    }
}
