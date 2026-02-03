# Configuration Guide

このドキュメントでは、whisper-server-wsの設定方法について説明します。

## 設定ファイル

`.env`ファイルをプロジェクトルートに作成することで、サーバーの動作をカスタマイズできます。

## .envファイルの例

```env
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
```

## 設定パラメータの詳細

### サーバー設定

#### HOST
- **デフォルト**: `127.0.0.1`
- **説明**: WebSocketサーバーがバインドするIPアドレス
- **例**:
  - ローカルのみ: `127.0.0.1`
  - すべてのインターフェース: `0.0.0.0`

#### PORT
- **デフォルト**: `9000`
- **説明**: WebSocketサーバーのポート番号
- **例**: `9000`, `8080`, `3000`

### Whisper設定

#### WHISPER_MODEL_PATH
- **デフォルト**: `./models/ggml-base.bin`
- **説明**: Whisperモデルファイル（ggml形式）のパス
- **例**:
  - `./models/ggml-tiny.bin` (最速、精度低)
  - `./models/ggml-base.bin` (バランス良)
  - `./models/ggml-small.bin` (高精度、遅)
  - `./models/ggml-medium.bin` (最高精度、最遅)

#### WHISPER_LANGUAGE
- **デフォルト**: `ja`
- **説明**: 認識する言語コード（ISO 639-1）
- **例**:
  - `ja` (日本語)
  - `en` (英語)
  - `zh` (中国語)
  - `ko` (韓国語)
  - `auto` (自動検出)

#### WHISPER_THREADS
- **デフォルト**: CPUコア数
- **説明**: Whisper処理に使用するスレッド数
- **推奨**: CPUコア数と同じか少し少ない値
- **例**: `4`, `8`, `16`

#### WHISPER_BLOCK_SECONDS
- **デフォルト**: `30`
- **説明**: 音声を処理するブロックの長さ（秒）
- **影響**: メモリ使用量とレスポンスタイム
- **推奨**:
  - リアルタイム性重視: `10`-`15`
  - メモリ節約: `30`-`60`
- **例**: `10`, `30`, `60`

### Silero VAD設定

#### VAD_MODEL_PATH
- **デフォルト**: `./models/silero_vad.onnx`
- **説明**: Silero VAD ONNXモデルのパス
- **ダウンロード**: `https://raw.githubusercontent.com/snakers4/silero-vad/master/files/silero_vad.onnx`

#### VAD_THRESHOLD
- **デフォルト**: `0.5`
- **範囲**: `0.0` - `1.0`
- **説明**: 音声と判定する確率の閾値
- **推奨値**:
  - 静かな環境: `0.3` - `0.4` (感度高)
  - 標準環境: `0.5` (バランス)
  - ノイズ多: `0.6` - `0.8` (感度低)

#### VAD_MIN_SPEECH_DURATION_MS
- **デフォルト**: `250`
- **単位**: ミリ秒
- **説明**: これより短い音声は無視される
- **推奨値**:
  - 雑音除去重視: `500` - `1000`
  - 短い発話も検出: `100` - `250`

#### VAD_MAX_SPEECH_DURATION_SECONDS
- **デフォルト**: `inf` (無制限)
- **単位**: 秒
- **説明**: 音声区間の最大長（これを超えると自動分割）
- **例**: `10`, `30`, `60`, `inf`

#### VAD_MIN_SILENCE_DURATION_MS
- **デフォルト**: `100`
- **単位**: ミリ秒
- **説明**: この長さの無音で音声区間を終了
- **推奨値**:
  - 素早く区切る: `50` - `100`
  - ゆっくり話す人: `200` - `500`

#### VAD_SPEECH_PAD_MS
- **デフォルト**: `30`
- **単位**: ミリ秒
- **説明**: 検出された音声区間の前後に追加するパディング
- **推奨値**: `30` - `100`

### 音声処理設定

#### SAMPLE_RATE
- **デフォルト**: `16000`
- **単位**: Hz
- **説明**: 入力音声のサンプリングレート
- **対応値**: `8000`, `16000` (推奨: `16000`)

#### MIN_SPEECH_SAMPLES
- **デフォルト**: `8000` (16kHzで0.5秒)
- **説明**: 処理する最小サンプル数
- **計算**: `秒数 × SAMPLE_RATE`
- **例**:
  - 0.25秒: `4000` (16kHzの場合)
  - 0.5秒: `8000`
  - 1.0秒: `16000`

#### MAX_SILENCE_SAMPLES
- **デフォルト**: `16000` (16kHzで1秒)
- **説明**: この長さの無音で発話を区切る
- **例**:
  - 0.5秒: `8000`
  - 1.0秒: `16000`
  - 2.0秒: `32000`

#### MAX_SPEECH_SAMPLES
- **デフォルト**: `48000` (16kHzで3秒)
- **説明**: 一度に処理する最大サンプル数
- **例**:
  - 2.0秒: `32000`
  - 3.0秒: `48000`
  - 5.0秒: `80000`

### NGワード設定

#### NG_WORDS
- **デフォルト**: `あ,ん,ご視聴ありがとうございました`
- **説明**: フィルタリングする単語（カンマ区切り）
- **用途**: 誤認識や不要な単語を除外
- **例**: `あ,ん,えー,あのー,その`

## 環境別推奨設定

### 静かなスタジオ環境

```env
VAD_THRESHOLD=0.3
VAD_MIN_SPEECH_DURATION_MS=100
VAD_MIN_SILENCE_DURATION_MS=50
WHISPER_MODEL_PATH=./models/ggml-small.bin
```

### ノイズの多い環境

```env
VAD_THRESHOLD=0.7
VAD_MIN_SPEECH_DURATION_MS=500
VAD_MIN_SILENCE_DURATION_MS=200
VAD_SPEECH_PAD_MS=50
```

### リアルタイム配信・ストリーミング

```env
WHISPER_BLOCK_SECONDS=10
VAD_MIN_SILENCE_DURATION_MS=50
MAX_SPEECH_SAMPLES=32000
WHISPER_MODEL_PATH=./models/ggml-tiny.bin
```

### 会議・複数人の会話

```env
VAD_THRESHOLD=0.5
VAD_MIN_SPEECH_DURATION_MS=250
VAD_MIN_SILENCE_DURATION_MS=100
MAX_SILENCE_SAMPLES=8000
WHISPER_THREADS=8
```

### 高精度重視

```env
WHISPER_MODEL_PATH=./models/ggml-medium.bin
VAD_THRESHOLD=0.4
VAD_MIN_SPEECH_DURATION_MS=200
WHISPER_THREADS=8
WHISPER_BLOCK_SECONDS=60
```

### 低レイテンシ重視

```env
WHISPER_MODEL_PATH=./models/ggml-tiny.bin
WHISPER_BLOCK_SECONDS=5
VAD_MIN_SILENCE_DURATION_MS=30
MAX_SPEECH_SAMPLES=16000
```

## トラブルシューティング

### 音声が検出されない

**問題**: 音声を送っても何も返ってこない

**解決策**:
1. `VAD_THRESHOLD`を下げる（例: `0.3`）
2. `VAD_MIN_SPEECH_DURATION_MS`を下げる（例: `100`）
3. サンプルレートが正しいか確認
4. デバッグログを確認: `RUST_LOG=debug cargo run`

### 頻繁に音声が途切れる

**問題**: 一つの発話が複数に分かれる

**解決策**:
1. `VAD_MIN_SILENCE_DURATION_MS`を増やす（例: `300`）
2. `VAD_SPEECH_PAD_MS`を増やす（例: `100`）
3. `MAX_SILENCE_SAMPLES`を増やす

### 処理が遅い

**問題**: レスポンスが遅い

**解決策**:
1. より小さいモデルを使用（`ggml-tiny.bin`）
2. `WHISPER_THREADS`を増やす
3. `WHISPER_BLOCK_SECONDS`を短くする
4. ハードウェアアクセラレーション確認

### メモリ使用量が多い

**問題**: メモリを大量に使用する

**解決策**:
1. `WHISPER_BLOCK_SECONDS`を減らす（例: `15`）
2. `MAX_SPEECH_SAMPLES`を減らす
3. より小さいモデルを使用

### 雑音を拾いすぎる

**問題**: 無音部分で誤検出

**解決策**:
1. `VAD_THRESHOLD`を上げる（例: `0.7`）
2. `VAD_MIN_SPEECH_DURATION_MS`を増やす（例: `500`）

## モデル選択ガイド

### Whisperモデル比較

| モデル | サイズ | 速度 | 精度 | メモリ | 用途 |
|--------|--------|------|------|--------|------|
| tiny | 75MB | 最速 | 低 | 200MB | リアルタイム |
| base | 142MB | 速 | 中 | 300MB | バランス |
| small | 466MB | 中 | 高 | 600MB | 高精度 |
| medium | 1.5GB | 遅 | 最高 | 1.5GB | 最高精度 |
| large | 2.9GB | 最遅 | 最高 | 3GB | オフライン処理 |

### 推奨環境

- **CPU使用**: tiny または base
- **GPU使用**: small または medium
- **リアルタイム**: tiny
- **オフライン**: medium または large

## 設定の確認

サーバー起動時に現在の設定が表示されます：

```
📋 Configuration:
  Server: 127.0.0.1:9000
  Whisper Model: ./models/ggml-base.bin
  Whisper Language: ja
  Whisper Threads: 4
  Whisper Block: 30s
  VAD Model: ./models/silero_vad.onnx
  VAD Threshold: 0.5
  VAD Min Speech: 250ms
  VAD Max Speech: infs
  VAD Min Silence: 100ms
  VAD Speech Pad: 30ms
  Sample Rate: 16000Hz
  NG Words: ["あ", "ん", "ご視聴ありがとうございました"]
```

## 参考情報

- Whisperモデル: https://huggingface.co/ggerganov/whisper.cpp
- Silero VAD: https://github.com/snakers4/silero-vad
- サンプルレート換算: https://en.wikipedia.org/wiki/Sampling_(signal_processing)