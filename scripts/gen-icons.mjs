// Generates Plainpad icons offline: icon.ico (multi-size) + PNGs. No dependencies.
// Design: coral rounded square, white "P" — matches the app's accent palette.
import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "src-tauri/icons");
mkdirSync(outDir, { recursive: true });

// ---------- tiny PNG encoder ----------
const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}
function encodePng(width, height, rgba) {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; ihdr[9] = 6; // 8-bit RGBA
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) {
    raw[y * (width * 4 + 1)] = 0; // filter: none
    rgba.copy ? rgba.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4)
              : raw.set(rgba.subarray(y * width * 4, (y + 1) * width * 4), y * (width * 4 + 1) + 1);
  }
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", deflateSync(raw)), chunk("IEND", Buffer.alloc(0))]);
}

// ---------- drawing ----------
function makeCanvas(size) { return { size, px: new Uint8Array(size * size * 4) }; }
function put(c, x, y, r, g, b, a) {
  if (x < 0 || y < 0 || x >= c.size || y >= c.size) return;
  const i = (y * c.size + x) * 4;
  const na = a / 255;
  c.px[i] = Math.round(r * na + c.px[i] * (1 - na));
  c.px[i + 1] = Math.round(g * na + c.px[i + 1] * (1 - na));
  c.px[i + 2] = Math.round(b * na + c.px[i + 2] * (1 - na));
  c.px[i + 3] = Math.max(c.px[i + 3], a);
}
function roundedRect(c, x0, y0, x1, y1, rad, col) {
  for (let y = Math.floor(y0); y < y1; y++)
    for (let x = Math.floor(x0); x < x1; x++) {
      const dx = Math.max(x0 + rad - x, 0, x - (x1 - rad - 1));
      const dy = Math.max(y0 + rad - y, 0, y - (y1 - rad - 1));
      const d = Math.sqrt(dx * dx + dy * dy);
      if (d <= rad) put(c, x, y, col[0], col[1], col[2], 255);
    }
}
function rect(c, x0, y0, x1, y1, col) { roundedRect(c, x0, y0, x1, y1, 0, col); }

// Draw the icon at `size`, scaled from a 128-unit design grid.
const CORAL = [255, 107, 94];
const DARK = [23, 25, 29];
const WHITE = [250, 250, 250];
function drawIcon(size) {
  const s = size / 128;
  const c = makeCanvas(size);
  roundedRect(c, 6 * s, 6 * s, 122 * s, 122 * s, 26 * s, DARK);
  roundedRect(c, 14 * s, 14 * s, 114 * s, 114 * s, 20 * s, CORAL);
  // "P": stem + bowl
  rect(c, 34 * s, 24 * s, 52 * s, 104 * s, WHITE);        // stem
  rect(c, 52 * s, 24 * s, 94 * s, 42 * s, WHITE);         // bowl top
  rect(c, 76 * s, 42 * s, 94 * s, 60 * s, WHITE);         // bowl right
  rect(c, 52 * s, 60 * s, 94 * s, 78 * s, WHITE);         // bowl bottom
  return c;
}

function png(size) {
  const c = drawIcon(size);
  return encodePng(size, size, Buffer.from(c.px));
}

// ---------- ICO packer (PNG-compressed entries, valid for Vista+) ----------
function icoFromPngs(sizes, pngBuffers) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2); // type: icon
  header.writeUInt16LE(sizes.length, 4);
  const entries = [];
  let offset = 6 + 16 * sizes.length;
  for (let i = 0; i < sizes.length; i++) {
    const e = Buffer.alloc(16);
    e[0] = sizes[i] === 256 ? 0 : sizes[i];
    e[1] = sizes[i] === 256 ? 0 : sizes[i];
    e[2] = 0; e[3] = 0;
    e.writeUInt16LE(1, 4);  // planes
    e.writeUInt16LE(32, 6); // bpp
    e.writeUInt32LE(pngBuffers[i].length, 8);
    e.writeUInt32LE(offset, 12);
    offset += pngBuffers[i].length;
    entries.push(e);
  }
  return Buffer.concat([header, ...entries, ...pngBuffers]);
}

const p16 = png(16), p32 = png(32), p48 = png(48), p64 = png(64), p128 = png(128), p256 = png(256);
writeFileSync(join(outDir, "icon.ico"), icoFromPngs([16, 32, 48, 64, 128, 256], [p16, p32, p48, p64, p128, p256]));
writeFileSync(join(outDir, "icon.png"), p128);
writeFileSync(join(outDir, "32x32.png"), p32);
console.log("icons written to", outDir);
