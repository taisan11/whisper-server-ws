#!/usr/bin/env python3
"""
Whisper Server WebSocket Client Example

Usage:
    python examples/client.py input.wav
"""

import asyncio
import json
import struct
import sys
import wave
from pathlib import Path

try:
    import websockets
    import numpy as np
except ImportError:
    print("Required packages not installed. Install with:")
    print("  pip install websockets numpy")
    sys.exit(1)


async def send_audio_file(websocket, audio_file):
    """Send audio file to the server"""
    print(f"📂 Loading audio file: {audio_file}")
    
    # Read WAV file
    with wave.open(str(audio_file), 'rb') as wav:
        sample_rate = wav.getframerate()
        n_channels = wav.getnchannels()
        sample_width = wav.getsampwidth()
        n_frames = wav.getnframes()
        
        print(f"  Sample Rate: {sample_rate} Hz")
        print(f"  Channels: {n_channels}")
        print(f"  Sample Width: {sample_width} bytes")
        print(f"  Duration: {n_frames / sample_rate:.2f}s")
        
        # Read audio data
        audio_data = wav.readframes(n_frames)
    
    # Convert to numpy array
    if sample_width == 2:  # 16-bit PCM
        audio_array = np.frombuffer(audio_data, dtype=np.int16)
    elif sample_width == 4:  # 32-bit PCM
        audio_array = np.frombuffer(audio_data, dtype=np.int32)
    else:
        print(f"❌ Unsupported sample width: {sample_width}")
        return
    
    # Convert to mono if stereo
    if n_channels == 2:
        audio_array = audio_array.reshape(-1, 2).mean(axis=1)
    
    # Normalize to float32 [-1, 1]
    if sample_width == 2:
        audio_float = audio_array.astype(np.float32) / 32768.0
    else:
        audio_float = audio_array.astype(np.float32) / 2147483648.0
    
    # Resample to 16kHz if needed
    if sample_rate != 16000:
        print(f"⚠️  Resampling from {sample_rate}Hz to 16000Hz...")
        from scipy import signal
        audio_float = signal.resample(audio_float, int(len(audio_float) * 16000 / sample_rate))
    
    print(f"📤 Sending {len(audio_float)} samples...")
    
    # Send in chunks (simulating streaming)
    chunk_size = 16000  # 1 second chunks
    for i in range(0, len(audio_float), chunk_size):
        chunk = audio_float[i:i+chunk_size]
        # Convert to bytes (f32 little-endian)
        chunk_bytes = chunk.tobytes()
        await websocket.send(chunk_bytes)
        print(f"  Sent chunk {i//chunk_size + 1}/{(len(audio_float) + chunk_size - 1)//chunk_size}")
        await asyncio.sleep(0.1)  # Small delay to simulate real-time
    
    # Send flush command to process remaining audio
    print("🔄 Flushing...")
    await websocket.send("flush")


async def receive_results(websocket):
    """Receive and display transcription results"""
    print("\n📥 Waiting for results...")
    
    async for message in websocket:
        try:
            result = json.loads(message)
            
            if "error" in result:
                print(f"❌ Error: {result['error']}")
            elif "transcription" in result:
                print("\n" + "="*60)
                print("✅ Transcription Result:")
                print("="*60)
                print(f"Text: {result['transcription']}")
                print(f"Duration: {result.get('duration', 'N/A')}s")
                
                if "segments" in result and result["segments"]:
                    print("\nSegments:")
                    for i, seg in enumerate(result["segments"], 1):
                        print(f"  {i}. [{seg['start']:.2f}s - {seg['end']:.2f}s] {seg['text']}")
                print("="*60)
            else:
                print(f"Received: {message}")
        except json.JSONDecodeError:
            print(f"Invalid JSON: {message}")


async def main():
    if len(sys.argv) < 2:
        print("Usage: python client.py <audio_file.wav>")
        print("\nExample:")
        print("  python client.py test.wav")
        sys.exit(1)
    
    audio_file = Path(sys.argv[1])
    if not audio_file.exists():
        print(f"❌ File not found: {audio_file}")
        sys.exit(1)
    
    uri = "ws://127.0.0.1:9000"
    print(f"🔗 Connecting to {uri}...")
    
    try:
        async with websockets.connect(uri) as websocket:
            print("✅ Connected!")
            
            # Create tasks for sending and receiving
            send_task = asyncio.create_task(send_audio_file(websocket, audio_file))
            receive_task = asyncio.create_task(receive_results(websocket))
            
            # Wait for sending to complete
            await send_task
            
            # Wait a bit for final results
            try:
                await asyncio.wait_for(receive_task, timeout=30.0)
            except asyncio.TimeoutError:
                print("\n⏱️  Timeout waiting for results")
            
    except ConnectionRefusedError:
        print("❌ Connection refused. Is the server running?")
        print("   Start the server with: cargo run --release")
    except Exception as e:
        print(f"❌ Error: {e}")


if __name__ == "__main__":
    asyncio.run(main())
