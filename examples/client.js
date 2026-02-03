#!/usr/bin/env node
/**
 * Whisper Server WebSocket Client Example (Node.js)
 * 
 * Usage:
 *   npm install ws
 *   node examples/client.js input.wav
 */

const WebSocket = require('ws');
const fs = require('fs');
const path = require('path');

function readWavFile(filename) {
    const buffer = fs.readFileSync(filename);
    
    // Parse WAV header (simplified)
    const riff = buffer.toString('ascii', 0, 4);
    const wave = buffer.toString('ascii', 8, 12);
    
    if (riff !== 'RIFF' || wave !== 'WAVE') {
        throw new Error('Not a valid WAV file');
    }
    
    // Find data chunk
    let offset = 12;
    while (offset < buffer.length) {
        const chunkId = buffer.toString('ascii', offset, offset + 4);
        const chunkSize = buffer.readUInt32LE(offset + 4);
        
        if (chunkId === 'data') {
            // Read audio data
            const audioData = buffer.slice(offset + 8, offset + 8 + chunkSize);
            
            // Convert to Float32Array (assuming 16-bit PCM)
            const samples = new Int16Array(audioData.buffer, audioData.byteOffset, audioData.length / 2);
            const float32 = new Float32Array(samples.length);
            
            for (let i = 0; i < samples.length; i++) {
                float32[i] = samples[i] / 32768.0;
            }
            
            return float32;
        }
        
        offset += 8 + chunkSize;
    }
    
    throw new Error('No data chunk found in WAV file');
}

async function sendAudio(ws, audioFile) {
    console.log(`📂 Loading audio file: ${audioFile}`);
    
    const audioData = readWavFile(audioFile);
    console.log(`📤 Sending ${audioData.length} samples...`);
    
    // Send in chunks
    const chunkSize = 16000; // 1 second at 16kHz
    for (let i = 0; i < audioData.length; i += chunkSize) {
        const chunk = audioData.slice(i, Math.min(i + chunkSize, audioData.length));
        ws.send(chunk.buffer);
        console.log(`  Sent chunk ${Math.floor(i / chunkSize) + 1}/${Math.ceil(audioData.length / chunkSize)}`);
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    // Flush
    console.log('🔄 Flushing...');
    ws.send('flush');
}

function main() {
    if (process.argv.length < 3) {
        console.log('Usage: node client.js <audio_file.wav>');
        console.log('\nExample:');
        console.log('  node client.js test.wav');
        process.exit(1);
    }
    
    const audioFile = process.argv[2];
    if (!fs.existsSync(audioFile)) {
        console.log(`❌ File not found: ${audioFile}`);
        process.exit(1);
    }
    
    const uri = 'ws://127.0.0.1:9000';
    console.log(`🔗 Connecting to ${uri}...`);
    
    const ws = new WebSocket(uri);
    
    ws.on('open', async () => {
        console.log('✅ Connected!');
        console.log('📥 Waiting for results...\n');
        
        await sendAudio(ws, audioFile);
    });
    
    ws.on('message', (data) => {
        try {
            const result = JSON.parse(data.toString());
            
            if (result.error) {
                console.log(`❌ Error: ${result.error}`);
            } else if (result.transcription !== undefined) {
                console.log('='.repeat(60));
                console.log('✅ Transcription Result:');
                console.log('='.repeat(60));
                console.log(`Text: ${result.transcription}`);
                console.log(`Duration: ${result.duration}s`);
                
                if (result.segments && result.segments.length > 0) {
                    console.log('\nSegments:');
                    result.segments.forEach((seg, i) => {
                        console.log(`  ${i + 1}. [${seg.start.toFixed(2)}s - ${seg.end.toFixed(2)}s] ${seg.text}`);
                    });
                }
                console.log('='.repeat(60));
            } else {
                console.log(`Received: ${data}`);
            }
        } catch (e) {
            console.log(`Invalid JSON: ${data}`);
        }
    });
    
    ws.on('error', (error) => {
        console.log(`❌ Error: ${error.message}`);
        if (error.code === 'ECONNREFUSED') {
            console.log('   Connection refused. Is the server running?');
            console.log('   Start the server with: cargo run --release');
        }
    });
    
    ws.on('close', () => {
        console.log('👋 Connection closed');
        process.exit(0);
    });
}

main();
