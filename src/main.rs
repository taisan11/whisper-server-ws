use tokio::sync::mpsc;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};

struct Task {
    audio_data: Vec<f32>,
    responder: tokio::sync::mpsc::Sender<String>,
}

#[tokio::main]
async fn main() {
    // Ctrl+Cハンドラーの設定
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    
    ctrlc::set_handler(move || {
        println!("\n🛑 Shutting down gracefully...");
        shutdown_clone.store(true, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    println!("WebSocket server running on port 9000");
    println!("Press Ctrl+C to stop");

    // queue (channel)
    let (tx, mut rx) = mpsc::channel::<Task>(100);
    let tx = Arc::new(tx);

    // Whisperモデルの初期化
    let model_path = std::env::var("WHISPER_MODEL_PATH")
        .unwrap_or_else(|_| "./models/ggml-base.bin".to_string());

    let ctx = WhisperContext::new_with_params(
        &model_path,
        WhisperContextParameters::default()
    ).expect("Failed to load Whisper model");

    let ctx = Arc::new(ctx);

    // single worker (順次処理・spawnしない)
    let shutdown_worker = shutdown.clone();
    tokio::spawn(async move {
        while let Some(task) = rx.recv().await {
            if shutdown_worker.load(Ordering::SeqCst) {
                println!("Worker shutting down...");
                break;
            }
            println!("Transcribing audio data ({} samples)...", task.audio_data.len());

            let ctx = ctx.clone();

            // Whisper推論をブロッキングタスクで実行
            let result = tokio::task::spawn_blocking(move || {
                let mut state = ctx.create_state()
                    .map_err(|e| format!("Failed to create state: {}", e))?;

                let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
                params.set_language(Some("ja"));

                state.full(params, &task.audio_data)
                    .map_err(|e| format!("Transcription failed: {}", e))?;

                let n_segments = state.full_n_segments();

                let mut transcription = String::new();
                for i in 0..n_segments {
                    if let Some(segment) = state.get_segment(i) {
                        if let Ok(text) = segment.to_str() {
                            transcription.push_str(text);
                        }
                    }
                }

                Ok::<String, String>(transcription)
            }).await;

            let result_text = match result {
                Ok(Ok(text)) => format!("{{\"transcription\": \"{}\"}}", text),
                Ok(Err(e)) => format!("{{\"error\": \"{}\"}}", e),
                Err(e) => format!("{{\"error\": \"Task join error: {}\"}}", e),
            };

            // 結果をWebSocket接続側に送信（順番保証）
            let _ = task.responder.send(result_text).await;
        }
    });

    loop {
        if shutdown.load(Ordering::SeqCst) {
            println!("✅ Server stopped");
            break;
        }

        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = match result {
                    Ok(conn) => conn,
                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                        continue;
                    }
                };
                
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                
                let ws = accept_async(stream).await.unwrap();
                println!("Client connected");

                let (mut write, mut read) = ws.split();

                // この接続専用の返信チャネル（これがVecDequeの代わりのキュー）
                let (resp_tx, mut resp_rx) = mpsc::channel::<String>(10);

                let tx = tx.clone();
                let resp_tx_clone = resp_tx.clone();

                // WebSocket 受信を queue に投げるだけ（タスクspawnしない）
                tokio::spawn(async move {
                    while let Some(Ok(msg)) = read.next().await {
                        match msg {
                            Message::Binary(data) => {
                                // バイナリデータを f32 配列に変換（16kHz想定）
                                let audio_data: Vec<f32> = data.chunks_exact(4)
                                    .map(|chunk| {
                                        let bytes = [chunk[0], chunk[1], chunk[2], chunk[3]];
                                        f32::from_le_bytes(bytes)
                                    })
                                    .collect();

                                // queue に push
                                if let Err(_) = tx.send(Task {
                                    audio_data,
                                    responder: resp_tx_clone.clone(),
                                }).await {
                                    println!("Worker dropped");
                                }

                                // 受信確認だけは即返信
                                let _ = resp_tx_clone.send("{\"status\": \"queued\"}".to_string()).await;
                            }
                            Message::Text(_) => {
                                // テキストメッセージの場合はエラー応答
                                let _ = resp_tx_clone.send(
                                    "{\"error\": \"Send binary audio data (f32 PCM)\"}".to_string()
                                ).await;
                            }
                            _ => {}
                        }
                    }
                });

                // 結果返信ループ（ping/pong や closeもここで順番に返せる）
                // Combine the two previous consumers into a single task so `resp_rx` is not moved twice.
                tokio::spawn(async move {
                    while let Some(res) = resp_rx.recv().await {
                        println!("Sending back: {}", res);
                        // send as Text message to the client
                        let _ = write.send(Message::Text(res.into())).await;
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
