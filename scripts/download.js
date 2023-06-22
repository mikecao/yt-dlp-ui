const fs = require('fs');
const path = require('path');
const fetch = require('cross-fetch');

const dest = path.resolve(__dirname, '../src-tauri/binaries');
const url = 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe';
const filename = 'yt-dlp-x86_64-pc-windows-msvc.exe';

function toBuffer(arrayBuffer) {
  const buffer = Buffer.alloc(arrayBuffer.byteLength);
  const view = new Uint8Array(arrayBuffer);
  for (let i = 0; i < buffer.length; ++i) {
    buffer[i] = view[i];
  }
  return buffer;
}

(async () => {
  const response = await fetch(url);
  const file = path.resolve(dest, filename);

  if (response.ok) {
    fs.writeFileSync(file, toBuffer(await response.arrayBuffer()));

    console.log('Download completed');
  }
})();
