// Copies static UI assets (index.html, styles.css) next to the tsc output.
import { copyFileSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(fileURLToPath(import.meta.url)) + "/..";
mkdirSync(join(root, "ui/dist"), { recursive: true });
for (const f of ["index.html", "styles.css"]) {
  copyFileSync(join(root, "ui", f), join(root, "ui/dist", f));
}
console.log("copied ui assets to ui/dist");
