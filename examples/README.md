# クライアント例

このディレクトリには、Whisper WebSocketサーバーに接続するクライアントの例が含まれています。

## Python クライアント

### 必要なパッケージ

```bash
pip install websockets numpy scipy
```

### 使用方法

```bash
python examples/client.py audio_file.wav
```

### 機能

- WAVファイルの読み込み
- 自動リサンプリング（16kHzへ）
- ステレオからモノラルへの変換
- チャンク単位での送信（ストリーミングシミュレート）
- セグメント情報付き結果の表示

## Node.js クライアント

### 必要なパッケージ

```bash
npm install ws
```

### 使用方法

```bash
node examples/client.js audio_file.wav
```

### 機能

- WAVファイルの読み込み（16-bit PCM）
- Float32への変換
- チャンク単位での送信
- JSON結果の表示

## ブラウザクライアント

プロジェクトルートの `test-client.html` をブラウザで開いてください。

### 機能

- マイクからの音声入力
- リアルタイム送信
- 結果の表示

## テスト音声の作成

### macOS (say コマンド)

```bash
say "こんにちは、これはテストです" -o test.aiff
ffmpeg -i test.aiff -ar 16000 -ac 1 test.wav
```

### Linux (espeak)

```bash
espeak "Hello, this is a test" -w test.wav
```

### FFmpeg で変換

```bash
# 既存の音声ファイルを16kHz モノラルに変換
ffmpeg -i input.mp3 -ar 16000 -ac 1 output.wav
```

## カスタマイズ

### サーバーのアドレスを変更

各クライアントファイルの `uri` または `websocket` の接続先を変更してください：

```python
# Python
uri = "ws://your-server:9000"
```

```javascript
// Node.js
const uri = 'ws://your-server:9000';
```

### チャンクサイズの調整

```python
# Python (client.py)
chunk_size = 16000  # サンプル数（1秒 @ 16kHz）
```

```javascript
// Node.js (client.js)
const chunkSize = 16000;  // サンプル数
```

## トラブルシューティング

### 接続エラー

```
Connection refused
```

**解決策**: サーバーが起動しているか確認

```bash
cargo run --release
```

### 音声が送信されない

- WAVファイルのフォーマットを確認（16-bit PCM推奨）
- ファイルパスが正しいか確認
- サンプルレートを確認（16kHz推奨）

### 結果が返ってこない

- サーバーのログを確認
- VAD設定を確認（`.env`ファイル）
- 音声ファイルに実際に音声が含まれているか確認

## 実装のヒント

### WebSocketでの音声送信

```javascript
// Float32Arrayを送信
const audioData = new Float32Array([...]);
websocket.send(audioData.buffer);
```

### 結果の受信

```javascript
websocket.onmessage = (event) => {
    const result = JSON.parse(event.data);
    console.log(result.transcription);
    result.segments.forEach(seg => {
        console.log(`${seg.start}s - ${seg.end}s: ${seg.text}`);
    });
};
```

### フラッシュコマンド

バッファに残っている音声を強制的に処理：

```javascript
websocket.send('flush');
```

## 参考リンク

- [WebSocket API (MDN)](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket)
- [websockets (Python)](https://websockets.readthedocs.io/)
- [ws (Node.js)](https://github.com/websockets/ws)