use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    // Server settings
    pub host: String,
    pub port: u16,

    // Whisper settings
    pub whisper_model_path: String,
    pub whisper_language: String,
    pub whisper_threads: usize,
    pub whisper_block_seconds: usize,

    // VAD settings
    pub vad_model_path: String,
    pub vad_threshold: f32,
    pub vad_min_speech_duration_ms: i32,
    pub vad_max_speech_duration_seconds: f32,
    pub vad_min_silence_duration_ms: i32,
    pub vad_speech_pad_ms: i32,

    // Processing settings
    pub sample_rate: i32,
    pub min_speech_samples: usize,
    pub max_silence_samples: usize,
    pub max_speech_samples: usize,

    // NG words (words to filter out)
    pub ng_words: Vec<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "9000".to_string())
            .parse()
            .unwrap_or(9000);

        let whisper_model_path =
            env::var("WHISPER_MODEL_PATH").unwrap_or_else(|_| "./models/ggml-base.bin".to_string());
        let whisper_language = env::var("WHISPER_LANGUAGE").unwrap_or_else(|_| "ja".to_string());
        let whisper_threads = env::var("WHISPER_THREADS")
            .unwrap_or_else(|_| num_cpus::get().to_string())
            .parse()
            .unwrap_or_else(|_| num_cpus::get());
        let whisper_block_seconds = env::var("WHISPER_BLOCK_SECONDS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        let vad_model_path =
            env::var("VAD_MODEL_PATH").unwrap_or_else(|_| "./models/silero_vad.onnx".to_string());
        let vad_threshold = env::var("VAD_THRESHOLD")
            .unwrap_or_else(|_| "0.5".to_string())
            .parse()
            .unwrap_or(0.5);
        let vad_min_speech_duration_ms = env::var("VAD_MIN_SPEECH_DURATION_MS")
            .unwrap_or_else(|_| "250".to_string())
            .parse()
            .unwrap_or(250);
        let vad_max_speech_duration_seconds = env::var("VAD_MAX_SPEECH_DURATION_SECONDS")
            .unwrap_or_else(|_| "inf".to_string())
            .parse()
            .unwrap_or(f32::INFINITY);
        let vad_min_silence_duration_ms = env::var("VAD_MIN_SILENCE_DURATION_MS")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or(100);
        let vad_speech_pad_ms = env::var("VAD_SPEECH_PAD_MS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);

        let sample_rate = env::var("SAMPLE_RATE")
            .unwrap_or_else(|_| "16000".to_string())
            .parse()
            .unwrap_or(16000);

        let min_speech_samples = env::var("MIN_SPEECH_SAMPLES")
            .unwrap_or_else(|_| "8000".to_string()) // 0.5 seconds at 16kHz
            .parse()
            .unwrap_or(8000);
        let max_silence_samples = env::var("MAX_SILENCE_SAMPLES")
            .unwrap_or_else(|_| "16000".to_string()) // 1 second at 16kHz
            .parse()
            .unwrap_or(16000);
        let max_speech_samples = env::var("MAX_SPEECH_SAMPLES")
            .unwrap_or_else(|_| "48000".to_string()) // 3 seconds at 16kHz
            .parse()
            .unwrap_or(48000);

        let ng_words_str = env::var("NG_WORDS")
            .unwrap_or_else(|_| "あ,ん,ご視聴ありがとうございました".to_string());
        let ng_words = ng_words_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self {
            host,
            port,
            whisper_model_path,
            whisper_language,
            whisper_threads,
            whisper_block_seconds,
            vad_model_path,
            vad_threshold,
            vad_min_speech_duration_ms,
            vad_max_speech_duration_seconds,
            vad_min_silence_duration_ms,
            vad_speech_pad_ms,
            sample_rate,
            min_speech_samples,
            max_silence_samples,
            max_speech_samples,
            ng_words,
        }
    }

    pub fn print_config(&self) {
        println!("📋 Configuration:");
        println!("  Server: {}:{}", self.host, self.port);
        println!("  Whisper Model: {}", self.whisper_model_path);
        println!("  Whisper Language: {}", self.whisper_language);
        println!("  Whisper Threads: {}", self.whisper_threads);
        println!("  Whisper Block: {}s", self.whisper_block_seconds);
        println!("  VAD Model: {}", self.vad_model_path);
        println!("  VAD Threshold: {}", self.vad_threshold);
        println!("  VAD Min Speech: {}ms", self.vad_min_speech_duration_ms);
        println!(
            "  VAD Max Speech: {}s",
            self.vad_max_speech_duration_seconds
        );
        println!("  VAD Min Silence: {}ms", self.vad_min_silence_duration_ms);
        println!("  VAD Speech Pad: {}ms", self.vad_speech_pad_ms);
        println!("  Sample Rate: {}Hz", self.sample_rate);
        println!("  NG Words: {:?}", self.ng_words);
    }
}
