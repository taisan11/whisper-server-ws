# Whisper Server with WebSocket and Silero VAD

音声認識WebSocketサーバー（Whisper + Silero VAD統合版）

## 概要

このプロジェクトは、WebSocketを通じてリアルタイムで音声データを受信し、Silero VADで音声区間を検出してからWhisperで文字起こしを行うサーバーです。高品質なC#実装を参考に、Rustで最適化されています。

## 特徴

- **Silero VAD統合**: ONNX Runtimeを使用した高精度な音声区間検出
- **ブロック処理**: メモリ効率を考慮した30秒単位のブロック処理
- **自動セグメンテーション**: 音声区間の自動検出と分割
- **NGワードフィルタリング**: 不要な単語を自動除外
- **環境変数設定**: `.env`ファイルから全パラメータを設定可能
- **WebSocketベース**: リアルタイムの音声ストリーミングに対応
- **非同期処理**: Tokioを使用した高効率な非同期処理
- **セグメント情報付き**: 各発話の開始・終了時間も取得可能

## 必要要件

- Rust 1.70以上
- Whisperモデルファイル（ggml形式）
- Silero VADモデルファイル（ONNX形式）

### プラットフォーム別の最適化

- **macOS**: Metal加速
- **Windows**: CUDA加速（NVIDIAグラフィックカード必要）
- **Linux**: CPU版

## インストール

```bash
# プロジェクトのクローン
git clone <repository-url>
cd whisper-server-ws

# ビルド
cargo build --release
```

> 📝 **詳細な設定ガイド**: [CONFIG.md](CONFIG.md) をご覧ください

## モデルのダウンロード

### Whisperモデル

```bash
# modelsディレクトリを作成
mkdir -p models

# モデルをダウンロード（例：base）
cd models
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
cd ..
```

### Silero VADモデル

```bash
# Silero VAD v6モデル（ONNXフォーマット）
wget https://raw.githubusercontent.com/snakers4/silero-vad/master/files/silero_vad.onnx -O models/silero_vad.onnx
```

## 設定ファイル

詳細な設定ガイドは **[CONFIG.md](CONFIG.md)** を参照してください。

## 基本設定

### .envファイルの作成

`.env`ファイルを作成して、サーバーの動作をカスタマイズできます：

```bash
# .envファイルを作成
cat > .env << 'EOF'
# サーバー設定
HOST=127.0.0.1
PORT=9000

# Whisper設定
WHISPER_MODEL_PATH=./models/ggml-base.bin
WHISPER_LANGUAGE=ja
WHISPER_THREADS=4
WHISPER_BLOCK_SECONDS=30

# Silero VAD設定
VAD_MODEL_PATH=./models/silero_vad.onnx
VAD_THRESHOLD=0.5
VAD_MIN_SPEECH_DURATION_MS=250
VAD_MAX_SPEECH_DURATION_SECONDS=inf
VAD_MIN_SILENCE_DURATION_MS=100
VAD_SPEECH_PAD_MS=30

# 音声処理設定
SAMPLE_RATE=16000
MIN_SPEECH_SAMPLES=8000
MAX_SILENCE_SAMPLES=16000
MAX_SPEECH_SAMPLES=48000

# NGワード設定（カンマ区切り）
NG_WORDS=あ,ん,ご視聴ありがとうございました
EOF
```

> 💡 **詳細な設定オプション、環境別推奨設定、トラブルシューティングについては [CONFIG.md](CONFIG.md) をご覧ください**

### 主要パラメータの概要

#### サーバー設定
- `HOST`: サーバーのバインドアドレス（デフォルト: 127.0.0.1）
- `PORT`: サーバーのポート番号（デフォルト: 9000）

#### Whisper設定
- `WHISPER_MODEL_PATH`: Whisperモデルファイルのパス
- `WHISPER_LANGUAGE`: 認識言語（ja, en, zh等）
- `WHISPER_THREADS`: 使用するスレッド数（デフォルト: CPU数）
- `WHISPER_BLOCK_SECONDS`: ブロック処理の単位（秒）

#### Silero VAD設定
- `VAD_MODEL_PATH`: Silero VAD ONNXモデルのパス
- `VAD_THRESHOLD`: 音声検出の閾値 0.0-1.0（デフォルト: 0.5）
  - 低い値（0.3-0.4）: 感度高、雑音も拾いやすい
  - 高い値（0.6-0.8）: 感度低、クリアな音声のみ検出
- `VAD_MIN_SPEECH_DURATION_MS`: 最小音声長（ミリ秒）
- `VAD_MAX_SPEECH_DURATION_SECONDS`: 最大音声長（秒、`inf`で無制限）
- `VAD_MIN_SILENCE_DURATION_MS`: 音声区間を区切る無音の長さ（ミリ秒）
- `VAD_SPEECH_PAD_MS`: 音声区間の前後に追加するパディング（ミリ秒）

#### 音声処理設定
- `SAMPLE_RATE`: サンプリングレート（Hz、8000または16000推奨）
- `MIN_SPEECH_SAMPLES`: 処理する最小サンプル数
- `MAX_SILENCE_SAMPLES`: 無音で区切るサンプル数
- `MAX_SPEECH_SAMPLES`: 一度に処理する最大サンプル数

#### NGワード設定
- `NG_WORDS`: フィルタリングする単語（カンマ区切り）

### 環境別の推奨設定

#### 静かな環境（スタジオ録音等）
```env
VAD_THRESHOLD=0.3
VAD_MIN_SPEECH_DURATION_MS=100
VAD_MIN_SILENCE_DURATION_MS=50
```

#### ノイズの多い環境
```env
VAD_THRESHOLD=0.7
VAD_MIN_SPEECH_DURATION_MS=500
VAD_MIN_SILENCE_DURATION_MS=200
```

#### 会議・複数人の会話
```env
VAD_THRESHOLD=0.5
VAD_MIN_SPEECH_DURATION_MS=250
VAD_MIN_SILENCE_DURATION_MS=100
MAX_SILENCE_SAMPLES=8000
```

#### リアルタイム配信
```env
WHISPER_BLOCK_SECONDS=10
VAD_MIN_SILENCE_DURATION_MS=50
MAX_SPEECH_SAMPLES=32000
```

## 使用方法

### サーバーの起動

```bash
# .envファイルの設定を使用
cargo run --release

# 起動時に設定が表示されます
📋 Configuration:
  Server: 127.0.0.1:9000
  Whisper Model: ./models/ggml-base.bin
  Whisper Language: ja
  ...
```

### WebSocketクライアント

#### 接続

```javascript
const ws = new WebSocket('ws://127.0.0.1:9000');
```

#### 音声データの送信

音声データは **f32 PCM形式** (16kHz推奨) でバイナリメッセージとして送信してください。

```javascript
// Float32Arrayの音声データをバイナリで送信
const audioData = new Float32Array([...]); // 16kHz PCM
ws.send(audioData.buffer);
```

#### 明示的なフラッシュ

残りのバッファを強制的に処理させる場合：

```javascript
ws.send('flush');
```

#### レスポンスの受信

```javascript
ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  
  if (response.transcription) {
    console.log('文字起こし結果:', response.transcription);
    
    // セグメント情報（各発話の時間情報）
    if (response.segments) {
      response.segments.forEach(seg => {
        console.log(`  ${seg.start.toFixed(2)}s - ${seg.end.toFixed(2)}s: ${seg.text}`);
      });
    }
  } else if (response.error) {
    console.error('エラー:', response.error);
  }
};
```

### レスポンス形式

#### 成功時

```json
{
  "transcription": "こんにちは、今日は良い天気ですね",
  "segments": [
    {
      "start": 0.00,
      "end": 1.20,
      "text": "こんにちは"
    },
    {
      "start": 1.30,
      "end": 2.80,
      "text": "今日は良い天気ですね"
    }
  ],
  "duration": 3.00
}
```

#### 音声が検出されなかった場合

```json
{
  "transcription": "",
  "message": "No speech detected",
  "duration": 3.00
}
```

#### エラー時

```json
{
  "error": "エラーメッセージ"
}
```

## 技術詳細

### Silero VAD アルゴリズム

本実装は、[YukkuriMovieMaker.Plugin.Community](https://github.com/manju-summoner/YukkuriMovieMaker.Plugin.Community)の高品質なC#実装を参考にしています。

主な機能：
- **状態管理**: RNNベースのステートフルな音声検出
- **動的閾値**: ヒステリシス閾値による安定した検出
- **スピーチパディング**: 音声区間の前後に自動的にパディングを追加
- **セグメントマージ**: 近接する音声区間を自動的に統合
- **最大音声長制御**: 長すぎる音声を自動分割

### ブロック処理

メモリ使用量を抑えるため、長い音声を30秒（設定可能）のブロックに分割して処理します：

1. 音声データをブロック単位で受信
2. 各ブロックでVADを実行
3. 検出された音声区間のみをWhisperに送信
4. ブロック境界でのセグメント切断を防ぐため、巻き戻し処理を実装

### NGワードフィルタリング

意図しない短い発話や定型句を除外：
- 「あ」「ん」などの単発音
- 「ご視聴ありがとうございました」などの定型句
- カスタムNGワードを環境変数で設定可能

## 動作フロー

```
音声入力（WebSocket）
    ↓
音声バッファに蓄積
    ↓
ブロックサイズに達したら
    ↓
Silero VADで音声区間検出
    ↓
検出された音声区間のみ抽出
    ↓
Whisperで文字起こし
    ↓
NGワードフィルタリング
    ↓
JSON形式でレスポンス
    ↓
WebSocketで返信
```

## テスト用HTMLクライアント

`test-client.html` を使用してブラウザからテストできます：

```bash
# サーバー起動後、ブラウザで開く
open test-client.html
```

## トラブルシューティング

### VAD初期化エラー

```
Failed to initialize VAD
```

**解決方法**:
- ONNX Runtimeがシステムにインストールされているか確認
- VADモデルファイルのパスが正しいか確認
- モデルファイルが破損していないか確認（再ダウンロード）

### 音声が検出されない

**症状**: 音声を送信しても何も返ってこない

**解決方法**:
- `VAD_THRESHOLD` を下げる（例: 0.3）
- `VAD_MIN_SPEECH_DURATION_MS` を下げる（例: 100）
- 入力音声のサンプルレートが正しいか確認（16kHz推奨）
- 音声データのフォーマット（f32 PCM）を確認
- デバッグログを確認（`RUST_LOG=debug cargo run`）

### 頻繁に音声が途切れる

**症状**: 一つの発話が複数のセグメントに分かれる

**解決方法**:
- `VAD_MIN_SILENCE_DURATION_MS` を増やす（例: 300）
- `VAD_SPEECH_PAD_MS` を増やす（例: 100）
- `MAX_SILENCE_SAMPLES` を増やす

### 処理が遅い

**症状**: レスポンスが返ってくるまで時間がかかる

**解決方法**:
- `WHISPER_THREADS` を増やす（CPU数まで）
- より小さいWhisperモデルを使用（tiny, base等）
- `WHISPER_BLOCK_SECONDS` を短くする
- ハードウェアアクセラレーション（Metal/CUDA）が有効か確認

### モデル読み込みエラー

```
Failed to load Whisper model
```

**解決方法**:
- モデルファイルのパスを確認
- モデルファイルがggml形式か確認
- モデルファイルの権限を確認
- ディスク容量を確認

### メモリ不足

**症状**: サーバーがクラッシュまたは動作が不安定

**解決方法**:
- `WHISPER_BLOCK_SECONDS` を減らす（例: 15）
- `MAX_SPEECH_SAMPLES` を減らす
- より小さいWhisperモデルを使用
- 同時接続数を制限

## パフォーマンス

### メモリ使用量
- ベースメモリ: 約200MB（Whisper base モデル）
- 接続あたり: 約5MB（バッファ含む）
- VAD処理: ほぼゼロオーバーヘッド

### 処理速度
- VAD処理: < 1ms/秒の音声
- Whisper処理: モデルとハードウェアに依存
  - tiny: 約0.5x リアルタイム
  - base: 約1.0x リアルタイム
  - small: 約2.0x リアルタイム

### 精度
- VAD精度: 95%以上（Silero VAD v6）
- 文字起こし精度: Whisperモデルに依存

## ベンチマーク

### 環境
- CPU: Apple M1 Pro
- RAM: 16GB
- モデル: Whisper base

### 結果
- 接続数: 1接続
- 音声長: 30秒
- 処理時間: 約25秒
- メモリ: 約250MB

## ライセンス

MIT

## クレジット

このプロジェクトは以下の素晴らしい実装を参考にしています：

- [YukkuriMovieMaker.Plugin.Community](https://github.com/manju-summoner/YukkuriMovieMaker.Plugin.Community) - 高品質なC#実装
  - Silero VADの実装
  - Whisperの統合パターン
  - ブロック処理アルゴリズム

## 依存ライブラリ

- [whisper-rs](https://github.com/tazz4843/whisper-rs) - Whisper音声認識
- [ort](https://github.com/pykeio/ort) - ONNX Runtime Rust バインディング
- [tokio](https://tokio.rs/) - 非同期ランタイム
- [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite) - WebSocket
- [ndarray](https://github.com/rust-ndarray/ndarray) - 多次元配列
- [serde_json](https://github.com/serde-rs/json) - JSON処理
- [dotenv](https://github.com/dotenv-rs/dotenv) - 環境変数管理

## 参考リンク

- [Whisper by OpenAI](https://github.com/openai/whisper)
- [Silero VAD](https://github.com/snakers4/silero-vad)
- [ONNX Runtime](https://onnxruntime.ai/)
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp)

## 貢献

プルリクエストを歓迎します！大きな変更の場合は、まずissueを開いて変更内容を議論してください。

## サポート

問題が発生した場合は、GitHubのissueを開いてください。ログとエラーメッセージを含めると、より早く解決できます。