import { copyFileSync, mkdirSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url)) + "/..";
mkdirSync(join(root, "ui/dist"), { recursive: true });

const files = readdirSync(join(root, "ui")).filter(f => /\.(html|css|svg)$/i.test(f));
for (const f of files) {
  copyFileSync(join(root, "ui", f), join(root, "ui/dist", f));
}
console.log(`copied ui assets (${files.join(", ")}) to ui/dist`);
