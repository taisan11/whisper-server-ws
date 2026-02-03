mod config;
mod vad;

use config::Config;
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use vad::SileroVadDetector;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

struct Task {
    audio_data: Vec<f32>,
    responder: tokio::sync::mpsc::Sender<String>,
}

#[derive(Clone, Debug)]
struct SegmentInfo {
    start: f64,
    end: f64,
    text: String,
}

#[tokio::main]
async fn main() {
    // Load configuration
    let config = Config::from_env();
    config.print_config();

    // Ctrl+C handler
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    ctrlc::set_handler(move || {
        println!("\n🛑 Shutting down gracefully...");
        shutdown_clone.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let bind_addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&bind_addr).await.unwrap();
    println!("🚀 WebSocket server running on {}", bind_addr);
    println!("Press Ctrl+C to stop");

    // Task queue
    let (tx, mut rx) = mpsc::channel::<Task>(100);
    let tx = Arc::new(tx);

    // Initialize Whisper model
    println!("📦 Loading Whisper model: {}", config.whisper_model_path);
    let ctx = WhisperContext::new_with_params(
        &config.whisper_model_path,
        WhisperContextParameters::default(),
    )
    .expect("Failed to load Whisper model");
    let ctx = Arc::new(ctx);
    println!("✅ Whisper model loaded");

    // Worker task for processing transcription
    let shutdown_worker = shutdown.clone();
    let ng_words = config.ng_words.clone();
    let whisper_language = config.whisper_language.clone();
    let whisper_threads = config.whisper_threads;

    tokio::spawn(async move {
        while let Some(task) = rx.recv().await {
            if shutdown_worker.load(Ordering::SeqCst) {
                println!("Worker shutting down...");
                break;
            }

            let duration = task.audio_data.len() as f64 / 16000.0;
            println!(
                "🎤 Transcribing audio data ({} samples, {:.2}s)...",
                task.audio_data.len(),
                duration
            );

            let ctx = ctx.clone();
            let ng_words = ng_words.clone();
            let language = whisper_language.clone();

            // Run Whisper inference in blocking task
            let result = tokio::task::spawn_blocking(move || {
                let mut state = ctx
                    .create_state()
                    .map_err(|e| format!("Failed to create state: {}", e))?;

                let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
                params.set_language(Some(&language));
                params.set_print_progress(false);
                params.set_print_special(false);
                params.set_print_realtime(false);
                params.set_n_threads(whisper_threads as i32);

                state
                    .full(params, &task.audio_data)
                    .map_err(|e| format!("Transcription failed: {}", e))?;

                let mut transcription = String::new();
                let mut segments = Vec::new();

                // Extract segments
                for segment in state.as_iter() {
                    let text = segment.to_string();
                    let trimmed_text = text.trim();

                    // Filter NG words
                    if ng_words.iter().any(|ng| ng == trimmed_text) {
                        continue;
                    }

                    transcription.push_str(trimmed_text);
                    transcription.push(' ');

                    // Get timing info (centiseconds / 100 = seconds)
                    let start = segment.start_timestamp() as f64 / 100.0;
                    let end = segment.end_timestamp() as f64 / 100.0;

                    segments.push(SegmentInfo {
                        start,
                        end,
                        text: trimmed_text.to_string(),
                    });
                }

                Ok::<(String, Vec<SegmentInfo>, f64), String>((
                    transcription.trim().to_string(),
                    segments,
                    duration,
                ))
            })
            .await;

            let result_text = match result {
                Ok(Ok((transcription, segments, duration))) => {
                    if transcription.is_empty() {
                        format!(
                            "{{\"transcription\": \"\", \"message\": \"No speech detected\", \"duration\": {:.2}}}",
                            duration
                        )
                    } else {
                        // Build JSON response with segments
                        let segments_json: Vec<String> = segments
                            .iter()
                            .map(|s| {
                                format!(
                                    "{{\"start\": {:.2}, \"end\": {:.2}, \"text\": \"{}\"}}",
                                    s.start,
                                    s.end,
                                    s.text.replace('\"', "\\\"").replace('\n', "\\n")
                                )
                            })
                            .collect();

                        format!(
                            "{{\"transcription\": \"{}\", \"segments\": [{}], \"duration\": {:.2}}}",
                            transcription.replace('\"', "\\\"").replace('\n', "\\n"),
                            segments_json.join(","),
                            duration
                        )
                    }
                }
                Ok(Err(e)) => format!("{{\"error\": \"{}\"}}", e.replace('\"', "\\\"")),
                Err(e) => format!("{{\"error\": \"Task join error: {}\"}}", e),
            };

            // Send result back
            let _ = task.responder.send(result_text).await;
        }
    });

    // Accept connections
    loop {
        if shutdown.load(Ordering::SeqCst) {
            println!("✅ Server stopped");
            break;
        }

        tokio::select! {
            result = listener.accept() => {
                let (stream, addr) = match result {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("❌ Accept error: {}", e);
                        continue;
                    }
                };

                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                let ws = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        eprintln!("❌ WebSocket handshake error: {}", e);
                        continue;
                    }
                };

                println!("🔗 Client connected: {}", addr);

                let (mut write, mut read) = ws.split();

                // Response channel for this connection
                let (resp_tx, mut resp_rx) = mpsc::channel::<String>(10);

                let tx = tx.clone();
                let resp_tx_clone = resp_tx.clone();
                let config = config.clone();

                // WebSocket receive and VAD processing
                tokio::spawn(async move {
                    // Initialize VAD
                    let mut vad = match SileroVadDetector::new(
                        &config.vad_model_path,
                        config.vad_threshold,
                        config.sample_rate,
                        config.vad_min_speech_duration_ms,
                        config.vad_max_speech_duration_seconds,
                        config.vad_min_silence_duration_ms,
                        config.vad_speech_pad_ms,
                    ) {
                        Ok(v) => {
                            println!("✅ VAD initialized");
                            v
                        }
                        Err(e) => {
                            eprintln!("❌ Failed to initialize VAD: {}", e);
                            let _ = resp_tx_clone
                                .send(format!("{{\"error\": \"VAD initialization failed: {}\"}}", e))
                                .await;
                            return;
                        }
                    };

                    let mut accumulated_audio: Vec<f32> = Vec::new();

                    while let Some(Ok(msg)) = read.next().await {
                        match msg {
                            Message::Binary(data) => {
                                // Convert binary data to f32 array
                                let audio_chunk: Vec<f32> = data
                                    .chunks_exact(4)
                                    .map(|chunk| {
                                        let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                                        f32::from_le_bytes(bytes)
                                    })
                                    .collect();

                                println!("📨 Received {} audio samples", audio_chunk.len());

                                // Accumulate audio
                                accumulated_audio.extend_from_slice(&audio_chunk);

                                // Process in blocks using VAD
                                let block_size = config.sample_rate as usize * config.whisper_block_seconds;

                                if accumulated_audio.len() >= block_size {
                                    // Extract block to process
                                    let block_to_process: Vec<f32> = accumulated_audio
                                        .drain(..block_size)
                                        .collect();

                                    println!(
                                        "🔍 Processing block: {} samples ({:.2}s)",
                                        block_to_process.len(),
                                        block_to_process.len() as f64 / config.sample_rate as f64
                                    );

                                    // Run VAD on the block
                                    match vad.get_speech_segments(&block_to_process) {
                                        Ok(segments) => {
                                            println!("🎯 VAD detected {} speech segments", segments.len());

                                            for segment in segments {
                                                let speech_audio = block_to_process
                                                    [segment.start_offset..segment.end_offset]
                                                    .to_vec();

                                                println!(
                                                    "  📢 Segment: {:.2}s - {:.2}s ({} samples)",
                                                    segment.start_second,
                                                    segment.end_second,
                                                    speech_audio.len()
                                                );

                                                // Only process if meets minimum length
                                                if speech_audio.len() >= config.min_speech_samples {
                                                    let (result_tx, mut result_rx) =
                                                        mpsc::channel::<String>(1);

                                                    if let Err(e) = tx
                                                        .send(Task {
                                                            audio_data: speech_audio,
                                                            responder: result_tx,
                                                        })
                                                        .await
                                                    {
                                                        eprintln!("❌ Worker dropped: {}", e);
                                                    } else {
                                                        if let Some(result) = result_rx.recv().await {
                                                            let _ = resp_tx_clone.send(result).await;
                                                        }
                                                    }
                                                } else {
                                                    println!(
                                                        "  ⚠️  Segment too short, skipping ({} < {})",
                                                        speech_audio.len(),
                                                        config.min_speech_samples
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("❌ VAD error: {}", e);
                                        }
                                    }
                                }
                            }
                            Message::Text(text) => {
                                if text == "flush" {
                                    // Process remaining audio
                                    if accumulated_audio.len() >= config.min_speech_samples {
                                        println!(
                                            "🔄 Flushing remaining {} samples",
                                            accumulated_audio.len()
                                        );

                                        match vad.get_speech_segments(&accumulated_audio) {
                                            Ok(segments) => {
                                                for segment in segments {
                                                    let speech_audio = accumulated_audio
                                                        [segment.start_offset..segment.end_offset]
                                                        .to_vec();

                                                    if speech_audio.len() >= config.min_speech_samples
                                                    {
                                                        let (result_tx, mut result_rx) =
                                                            mpsc::channel::<String>(1);

                                                        if let Ok(_) = tx
                                                            .send(Task {
                                                                audio_data: speech_audio,
                                                                responder: result_tx,
                                                            })
                                                            .await
                                                        {
                                                            if let Some(result) =
                                                                result_rx.recv().await
                                                            {
                                                                let _ =
                                                                    resp_tx_clone.send(result).await;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("❌ VAD error on flush: {}", e);
                                            }
                                        }

                                        accumulated_audio.clear();
                                    }
                                } else {
                                    let _ = resp_tx_clone
                                        .send(
                                            "{\"error\": \"Send binary audio data (f32 PCM) or 'flush' command\"}"
                                                .to_string(),
                                        )
                                        .await;
                                }
                            }
                            Message::Close(_) => {
                                println!("👋 Client disconnecting");
                                break;
                            }
                            _ => {}
                        }
                    }

                    // On disconnect, process any remaining audio
                    if accumulated_audio.len() >= config.min_speech_samples {
                        println!(
                            "🔄 Processing remaining audio on disconnect: {} samples",
                            accumulated_audio.len()
                        );

                        match vad.get_speech_segments(&accumulated_audio) {
                            Ok(segments) => {
                                for segment in segments {
                                    let speech_audio = accumulated_audio
                                        [segment.start_offset..segment.end_offset]
                                        .to_vec();

                                    if speech_audio.len() >= config.min_speech_samples {
                                        let (result_tx, mut result_rx) = mpsc::channel::<String>(1);

                                        if let Ok(_) = tx
                                            .send(Task {
                                                audio_data: speech_audio,
                                                responder: result_tx,
                                            })
                                            .await
                                        {
                                            if let Some(result) = result_rx.recv().await {
                                                let _ = resp_tx_clone.send(result).await;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ VAD error on disconnect: {}", e);
                            }
                        }
                    }

                    println!("🔌 Client disconnected");
                });

                // Response sender loop
                tokio::spawn(async move {
                    while let Some(res) = resp_rx.recv().await {
                        println!("📤 Sending response: {}", &res[..res.len().min(100)]);
                        if let Err(e) = write.send(Message::Text(res)).await {
                            eprintln!("❌ Failed to send response: {}", e);
                            break;
                        }
                    }
                });
            }
            _ = tokio::signal::ctrl_c() => {
                println!("✅ Server stopped");
                break;
            }
        }
    }
}
