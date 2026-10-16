---
phase: 8
plan: 4
wave: 4
gap_closure: true
depends_on:
  - "8.3"
files_modified:
  - ui/index.html
autonomous: true
must_haves:
  truths:
    - "Export modal (#export-modal) is a sibling of #void-confirm-dialog in the DOM body and renders cleanly when summoned via Ctrl+Shift+E or #btn-export"
  artifacts:
    - ui/index.html
  key_links:
    - "ui/index.html properly closes #void-confirm-dialog before #export-modal"
---

# Plan 8.4 (Gap Closure): Fix Void Confirm Dialog HTML Tag Closure & Unnest Export Modal

<objective>
Fix an unclosed `<div>` in `ui/index.html` where `#void-confirm-dialog` was missing its closing tag, causing `#export-modal` to be parsed as a child of `#void-confirm-dialog`. Because `.void-confirm-dialog.hidden` has `display: none !important`, the export modal cannot render when summoned by the user.

Purpose: Restore proper DOM hierarchy so `#export-modal` can render independently and cleanly on screen.
Output: Fixed `ui/index.html` and rebuilt `ui/dist/index.html`.
</objective>

<context>
Load these files for context:
- ui/index.html
- ui/styles.css
- ui/src/export.ts
- ui/src/main.ts
</context>

<tasks>

<task type="auto">
  <name>Close #void-confirm-dialog div tag in ui/index.html</name>
  <files>
    ui/index.html
  </files>
  <action>
    In `ui/index.html`, close the `#void-confirm-dialog` element by adding `</div>` after the closing `</div>` of `.void-dialog-card` (around line 181), immediately prior to `<!-- Export Modal (#export-modal) -->`.

    Verify that:
    1. `<div id="void-confirm-dialog">` has balanced opening and closing tags.
    2. `<div id="export-modal">` is a direct child of `<body>`, not nested inside `#void-confirm-dialog`.
    3. Run `npm run build:ui` to copy updated HTML to `ui/dist`.
  </action>
  <verify>
    node -e '
      const fs = require("fs");
      const html = fs.readFileSync("ui/index.html", "utf8");
      const voidIdx = html.indexOf("id=\"void-confirm-dialog\"");
      const exportIdx = html.indexOf("id=\"export-modal\"");
      const snippet = html.substring(voidIdx, exportIdx);
      const opens = (snippet.match(/<div/g) || []).length;
      const closes = (snippet.match(/<\/div>/g) || []).length;
      if (opens !== closes) throw new Error(`Tag imbalance between void and export: opens=${opens}, closes=${closes}`);
      console.log("DOM hierarchy verified: opens and closes balanced");
    '
  </verify>
  <done>
    `ui/index.html` has balanced opening/closing tags, `#export-modal` is a sibling of `#void-confirm-dialog`, and `npm run build:ui` bundles cleanly.
  </done>
</task>

</tasks>

<verification>
After completion, verify:
- [ ] `node` check confirms opens == closes between `#void-confirm-dialog` and `#export-modal`
- [ ] `npm run build:ui` succeeds without error
- [ ] `npm run typecheck` passes
</verification>

<success_criteria>
- [ ] Gap resolved: `#export-modal` is unnested from `#void-confirm-dialog`
- [ ] All Phase 8 must-haves 100% verified
</success_criteria>
