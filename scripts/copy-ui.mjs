import { copyFileSync, mkdirSync, readdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url)) + "/..";
mkdirSync(join(root, "ui/dist"), { recursive: true });

const files = readdirSync(join(root, "ui")).filter(f => /\.(html|css|svg)$/i.test(f));
for (const f of files) {
  copyFileSync(join(root, "ui", f), join(root, "ui/dist", f));
}
console.log(`copied ui assets (${files.join(", ")}) to ui/dist`);

const soundsDir = join(root, "ui/sounds");
if (existsSync(soundsDir)) {
  const distSoundsDir = join(root, "ui/dist/sounds");
  mkdirSync(distSoundsDir, { recursive: true });
  const soundFiles = readdirSync(soundsDir).filter(f => /\.(m4a|mp3|wav|ogg)$/i.test(f));
  for (const s of soundFiles) {
    copyFileSync(join(soundsDir, s), join(distSoundsDir, s));
  }
  console.log(`copied audio assets (${soundFiles.join(", ")}) to ui/dist/sounds`);
}
