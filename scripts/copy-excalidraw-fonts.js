import { cpSync, mkdirSync, readdirSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const src = join(__dirname, '../node_modules/@excalidraw/excalidraw/dist/prod/fonts');
const dest = join(__dirname, '../static/excalidraw-assets/fonts');

// Skip Xiaolai (Chinese font, 12MB)
const exclude = ['Xiaolai'];

mkdirSync(dest, { recursive: true });

for (const font of readdirSync(src)) {
  if (!exclude.includes(font)) {
    cpSync(join(src, font), join(dest, font), { recursive: true });
  }
}

console.log('Copied Excalidraw fonts (excluding Xiaolai)');
