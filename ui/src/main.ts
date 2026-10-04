// Plainpad — UI entry point
// No framework: textarea + mirror + overlay editor with IPC to Tauri core.
export {};

interface TauriAPI {
  core: {
    invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
  };
  event: {
    listen: (event: string, cb: (ev: { payload: unknown }) => void) => Promise<() => void>;
  };
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const win = window as any;

// ---------- Tauri IPC bridge ----------

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const tauri: TauriAPI | undefined = win.__TAURI__;
  if (tauri) {
    return tauri.core.invoke(cmd, args) as Promise<T>;
  }
  console.warn(`[mock] invoke(${cmd})`, args);
  return null as unknown as T;
}

async function listen(event: string, cb: (payload: unknown) => void): Promise<void> {
  const tauri: TauriAPI | undefined = win.__TAURI__;
  if (tauri) {
    await tauri.event.listen(event, (ev: { payload: unknown }) => cb(ev.payload));
  }
}

// ---------- Types (mirrors Rust structs) ----------

interface NoteMeta {
  id: string;
  title: string;
  mtime_ms: number;
  pinned: boolean;
  plain: boolean;
}

interface Note {
  meta: NoteMeta;
  text: string;
}

interface Annotation {
  line: number;
  kind: string;
  confidence: number;
  value: string;
  chip: string;
  why: string;
}

interface Analysis {
  annotations: Annotation[];
  code_mode: boolean;
  code_lang: string;
  elapsed_us: number;
}

interface Timer {
  id: number;
  name: string;
  due_ms: number;
  duration_ms: number;
  fired: boolean;
}

// ---------- State ----------

let notes: NoteMeta[] = [];
let currentIndex = 0;
let currentId = '';
let currentText = '';
let currentPlain = false;
let globalPlain = false;
let saveTimeout: ReturnType<typeof setTimeout> | null = null;
let analyzeTimeout: ReturnType<typeof setTimeout> | null = null;
let activeTimers: Timer[] = [];

// ---------- DOM refs ----------

const editor = document.getElementById('editor') as HTMLTextAreaElement;
const mirror = document.getElementById('editor-mirror') as HTMLDivElement;
const overlay = document.getElementById('editor-overlay') as HTMLDivElement;
const noteIndex = document.getElementById('note-index') as HTMLSpanElement;
const btnPrev = document.getElementById('btn-prev') as HTMLButtonElement;
const btnNext = document.getElementById('btn-next') as HTMLButtonElement;
const btnNew = document.getElementById('btn-new') as HTMLButtonElement;
const btnDelete = document.getElementById('btn-delete') as HTMLButtonElement;
const btnExport = document.getElementById('btn-export') as HTMLButtonElement;
const btnPin = document.getElementById('btn-pin') as HTMLButtonElement;
const btnPlain = document.getElementById('btn-plain') as HTMLButtonElement;
const btnSettings = document.getElementById('btn-settings') as HTMLButtonElement;
const settingsPanel = document.getElementById('settings-panel') as HTMLDivElement;
const btnCloseSettings = document.getElementById('btn-close-settings') as HTMLButtonElement;
const settingTheme = document.getElementById('setting-theme') as HTMLSelectElement;
const settingGlobalPlain = document.getElementById('setting-global-plain') as HTMLInputElement;
const timerDisplay = document.getElementById('timer-display') as HTMLDivElement;

// ---------- Init ----------

async function init(): Promise<void> {
  await loadNotes();
  if (notes.length === 0) {
    // First run: create a demo note
    const id = await invoke<string>('create_note');
    const demoText = `math

rent 12000
food 4500
transport 1800

rent + food + transport

5 km in miles

timer 25 min

todo call dentist

buy milk, eggs, bread`;
    await invoke('save_note', { id, text: demoText });
    await loadNotes();
  }
  await loadNote(0);
  editor.focus();

  // Settings
  const settings = await invoke<Record<string, unknown>>('get_settings');
  if (settings) {
    if (typeof settings.theme === 'string') {
      applyTheme(settings.theme as string);
      settingTheme.value = settings.theme as string;
    }
    if (typeof settings.global_plain === 'boolean') {
      globalPlain = settings.global_plain as boolean;
      settingGlobalPlain.checked = globalPlain;
    }
  }

  // Active timers
  activeTimers = (await invoke<Timer[]>('list_timers')) || [];
  updateTimerDisplay();

  // Listen for timer events
  await listen('timer-fired', (payload: unknown) => {
    const p = payload as { id: number; name: string; overdue: boolean };
    showTimerNotification(p.name, p.overdue);
    activeTimers = activeTimers.filter((t) => t.id !== p.id);
    updateTimerDisplay();
  });
}

// ---------- Note CRUD ----------

async function loadNotes(): Promise<void> {
  notes = (await invoke<NoteMeta[]>('list_notes')) || [];
}

async function loadNote(index: number): Promise<void> {
  if (notes.length === 0) return;
  currentIndex = Math.max(0, Math.min(index, notes.length - 1));
  const meta = notes[currentIndex];
  currentId = meta.id;
  currentPlain = meta.plain;

  const note = await invoke<Note>('load_note', { id: currentId });
  currentText = note?.text || '';
  editor.value = currentText;

  updateNoteUI();
  scheduleAnalyze();
}

async function saveCurrentNote(): Promise<void> {
  if (!currentId) return;
  const text = editor.value;
  if (text === currentText) return;
  currentText = text;

  // Discard empty notes
  if (text.trim() === '') {
    await invoke('delete_note', { id: currentId });
    await loadNotes();
    if (notes.length === 0) {
      await createNewNote();
    } else {
      await loadNote(Math.min(currentIndex, notes.length - 1));
    }
    return;
  }

  await invoke('save_note', { id: currentId, text });
  await loadNotes();
  updateNoteUI();
}

async function createNewNote(): Promise<void> {
  await saveCurrentNote();
  const id = await invoke<string>('create_note');
  currentId = id;
  currentText = '';
  editor.value = '';
  await loadNotes();

  // New note goes to front
  notes.unshift({ id, title: '', mtime_ms: Date.now(), pinned: false, plain: false });
  currentIndex = 0;
  currentPlain = false;
  overlay.innerHTML = '';
  updateNoteUI();
  editor.focus();
}

async function deleteCurrentNote(): Promise<void> {
  if (!currentId) return;
  await invoke('delete_note', { id: currentId });
  await loadNotes();
  if (notes.length === 0) {
    await createNewNote();
  } else {
    await loadNote(Math.min(currentIndex, notes.length - 1));
  }
}

// ---------- Navigation ----------

async function prevNote(): Promise<void> {
  if (currentIndex > 0) {
    await saveCurrentNote();
    await loadNote(currentIndex - 1);
  }
}

async function nextNote(): Promise<void> {
  if (currentIndex < notes.length - 1) {
    await saveCurrentNote();
    await loadNote(currentIndex + 1);
  } else {
    // Past the newest note → create new
    await createNewNote();
  }
}

function updateNoteUI(): void {
  const idx = notes.findIndex((n) => n.id === currentId);
  if (idx >= 0) currentIndex = idx;
  noteIndex.textContent = notes.length > 0 ? `${currentIndex + 1}/${notes.length}` : '0/0';

  const meta = notes[currentIndex];
  if (meta) {
    btnPin.classList.toggle('active', meta.pinned);
    btnPlain.classList.toggle('active', currentPlain || globalPlain);
  }
}

// ---------- Analysis & Overlay ----------

function scheduleAnalyze(): void {
  if (analyzeTimeout) clearTimeout(analyzeTimeout);
  analyzeTimeout = setTimeout(() => runAnalysis(), 50);
}

async function runAnalysis(): Promise<void> {
  const text = editor.value;
  const plain = currentPlain || globalPlain;
  const analysis = await invoke<Analysis>('analyze_note', { text, plain });
  if (!analysis) {
    overlay.innerHTML = '';
    return;
  }

  renderOverlay(analysis, text);

  // Code mode: toggle body class
  document.body.classList.toggle('code-mode', analysis.code_mode);
}

function renderOverlay(analysis: Analysis, text: string): void {
  const lines = text.split('\n');
  overlay.innerHTML = '';

  // Sync scroll position with textarea
  overlay.style.top = `-${editor.scrollTop}px`;

  // Compute line heights from the mirror
  mirror.textContent = '';
  const lineEls: HTMLSpanElement[] = [];
  for (let i = 0; i < lines.length; i++) {
    const span = document.createElement('span');
    span.textContent = lines[i] || ' ';
    span.style.display = 'block';
    mirror.appendChild(span);
    lineEls.push(span);
  }

  // Check for mode word on first non-empty line
  const firstNonEmpty = lines.findIndex((l) => l.trim().length > 0);
  if (firstNonEmpty >= 0) {
    const modeWord = lines[firstNonEmpty].trim().toLowerCase();
    const knownModes = ['math', 'list', 'code', 'sum', 'calcs', 'paste'];
    const isMode = knownModes.includes(modeWord) || modeWord.startsWith('code:');
    if (isMode) {
      const el = document.createElement('div');
      el.className = `anno anno-mode ${modeWord.split(':')[0]}`;
      const lineEl = lineEls[firstNonEmpty];
      if (lineEl) {
        el.style.top = `${lineEl.offsetTop}px`;
      }
      el.textContent = lines[firstNonEmpty].trim();
      // Hide the original text in the textarea for mode words?
      // No — "plain text is the source of truth" — we just style the overlay.
      overlay.appendChild(el);
    }
  }

  for (const anno of analysis.annotations) {
    if (anno.line >= lineEls.length) continue;
    const lineEl = lineEls[anno.line];
    if (!lineEl) continue;
    const top = lineEl.offsetTop;

    if (anno.kind === 'math' && anno.confidence >= 0.85 && anno.value) {
      const el = document.createElement('div');
      el.className = 'anno anno-math';
      el.style.top = `${top}px`;
      el.textContent = formatNumber(anno.value);
      el.title = anno.why;
      overlay.appendChild(el);
    } else if (anno.kind === 'conversion' && anno.confidence >= 0.85 && anno.value) {
      const el = document.createElement('div');
      el.className = 'anno anno-conversion';
      el.style.top = `${top}px`;
      el.textContent = anno.value;
      el.title = anno.why;
      overlay.appendChild(el);
    } else if (anno.kind === 'checklist' && anno.confidence >= 0.85) {
      // Render checkbox overlay
      const lineText = lines[anno.line]?.trim() || '';
      const isChecked = lineText.startsWith('[x] ') || lineText.startsWith('[X] ');
      const el = document.createElement('div');
      el.className = `anno-checkbox${isChecked ? ' checked' : ''}`;
      el.style.top = `${top + 2}px`;
      el.dataset.line = String(anno.line);
      el.addEventListener('click', () => toggleCheckbox(anno.line));
      overlay.appendChild(el);
    } else if (anno.chip && anno.confidence >= 0.5 && anno.confidence < 0.85) {
      const el = document.createElement('div');
      el.className = 'anno anno-chip';
      el.style.top = `${top}px`;
      el.textContent = anno.chip;
      el.title = anno.why;
      overlay.appendChild(el);
    } else if (anno.kind === 'timer' && anno.value) {
      const el = document.createElement('div');
      el.className = 'anno anno-timer';
      el.style.top = `${top}px`;
      el.textContent = `⏱ ${anno.value}`;
      el.title = anno.why;
      overlay.appendChild(el);
    }
  }
}

function formatNumber(value: string): string {
  // Add thousands separators to plain numbers
  const num = parseFloat(value.replace(/,/g, ''));
  if (isNaN(num)) return value;
  if (Number.isInteger(num) && !value.includes('.')) {
    return num.toLocaleString('en-US');
  }
  // Keep decimal precision
  const parts = value.split('.');
  const intPart = parseInt(parts[0].replace(/,/g, ''), 10);
  if (isNaN(intPart)) return value;
  const formatted = intPart.toLocaleString('en-US');
  return parts.length > 1 ? `${formatted}.${parts[1]}` : formatted;
}

// ---------- Checkbox toggling ----------

function toggleCheckbox(lineIndex: number): void {
  const lines = editor.value.split('\n');
  if (lineIndex >= lines.length) return;
  const line = lines[lineIndex];
  const trimmed = line.trimStart();
  const indent = line.substring(0, line.length - trimmed.length);

  if (trimmed.startsWith('[ ] ')) {
    lines[lineIndex] = indent + '[x] ' + trimmed.substring(4);
  } else if (trimmed.startsWith('[x] ') || trimmed.startsWith('[X] ')) {
    lines[lineIndex] = indent + '[ ] ' + trimmed.substring(4);
  }

  editor.value = lines.join('\n');
  scheduleSave();
  scheduleAnalyze();
}

// ---------- Accept chip (Tab on suggestion) ----------

function acceptChip(lineIndex: number, text: string): void {
  const lines = editor.value.split('\n');
  if (lineIndex >= lines.length) return;
  const line = lines[lineIndex];

  // "Make checklist" → convert comma-list or "todo" to checkboxes
  if (text.toLowerCase().startsWith('todo ')) {
    lines[lineIndex] = '[ ] ' + line.substring(line.toLowerCase().indexOf('todo ') + 5);
  } else if (line.includes(',')) {
    // Comma list → multiple checkbox lines
    const items = line.split(',').map((s) => s.trim()).filter(Boolean);
    lines.splice(lineIndex, 1, ...items.map((item) => `[ ] ${item}`));
  }

  editor.value = lines.join('\n');
  scheduleSave();
  scheduleAnalyze();
}

// ---------- Timer actions ----------

async function startTimerFromLine(lineIndex: number): Promise<void> {
  // This is triggered when user presses Enter on a timer line
  const lines = editor.value.split('\n');
  if (lineIndex >= lines.length) return;
  const line = lines[lineIndex].trim();

  // Parse duration from the analysis
  const analysis = await invoke<Analysis>('analyze_note', { text: line, plain: false });
  const timerAnno = analysis?.annotations?.find((a) => a.kind === 'timer');
  if (!timerAnno) return;

  // Extract name and duration from the chip text
  // The Rust side already parsed it — we need the raw duration.
  // Call start_timer with parsed values
  const match = line.match(/timer\s+(.*)/i) || line.match(/remind\s+(?:me\s+)?in\s+(.*)/i);
  if (!match) return;

  // Re-parse on Rust side and start
  const durationMs = parseDurationFromValue(timerAnno.value);
  if (durationMs > 0) {
    const name = timerAnno.chip.replace(/^Start timer \(/, '').replace(/\)$/, '') || 'Timer';
    const id = await invoke<number>('start_timer', { name, durationMs });
    activeTimers.push({
      id,
      name,
      due_ms: Date.now() + durationMs,
      duration_ms: durationMs,
      fired: false,
    });
    updateTimerDisplay();
  }
}

function parseDurationFromValue(value: string): number {
  // Parse "25 min", "1 h 30 min", "90 s"
  let ms = 0;
  const parts = value.match(/(\d+)\s*(h|hr|hour|min|m|s|sec)/gi);
  if (!parts) return 0;
  for (const part of parts) {
    const m = part.match(/(\d+)\s*(h|hr|hour|min|m|s|sec)/i);
    if (!m) continue;
    const n = parseInt(m[1], 10);
    const unit = m[2].toLowerCase();
    if (unit === 'h' || unit === 'hr' || unit === 'hour') ms += n * 3600000;
    else if (unit === 'min' || unit === 'm') ms += n * 60000;
    else if (unit === 's' || unit === 'sec') ms += n * 1000;
  }
  return ms;
}

function updateTimerDisplay(): void {
  const running = activeTimers.filter((t) => !t.fired);
  if (running.length === 0) {
    timerDisplay.classList.add('hidden');
    return;
  }
  timerDisplay.classList.remove('hidden');

  const nearest = running.reduce((a, b) => (a.due_ms < b.due_ms ? a : b));
  const remaining = Math.max(0, nearest.due_ms - Date.now());
  const minutes = Math.floor(remaining / 60000);
  const seconds = Math.floor((remaining % 60000) / 1000);

  timerDisplay.innerHTML = `
    <span class="timer-label">${nearest.name}</span>
    <span class="timer-value">${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}</span>
  `;
}

function showTimerNotification(name: string, overdue: boolean): void {
  const msg = overdue ? `⏰ ${name} (missed while away)` : `⏰ ${name} — time's up!`;
  // Show in-app notification (flash the timer display)
  timerDisplay.classList.remove('hidden');
  timerDisplay.innerHTML = `<span class="timer-value" style="color: var(--accent-coral)">${msg}</span>`;
  setTimeout(() => updateTimerDisplay(), 5000);
}

// ---------- Export ----------

async function exportNote(): Promise<void> {
  const text = editor.value;
  if (!text.trim()) return;

  const md = await invoke<string>('export_note', { text, kind: 'markdown' });
  if (md) {
    await navigator.clipboard.writeText(md);
    // Brief visual feedback
    btnExport.textContent = '✓';
    setTimeout(() => {
      btnExport.textContent = '📤';
    }, 1500);
  }
}

// ---------- Settings ----------

function applyTheme(theme: string): void {
  if (theme === 'dark') {
    document.documentElement.setAttribute('data-theme', 'dark');
  } else if (theme === 'light') {
    document.documentElement.setAttribute('data-theme', 'light');
  } else {
    document.documentElement.removeAttribute('data-theme');
  }
}

// ---------- Autosave ----------

function scheduleSave(): void {
  if (saveTimeout) clearTimeout(saveTimeout);
  saveTimeout = setTimeout(() => saveCurrentNote(), 400);
}

// ---------- Paste handling ----------

editor.addEventListener('paste', (e: ClipboardEvent) => {
  // Strip formatting: always paste as plain text (PRD H-3)
  e.preventDefault();
  const text = e.clipboardData?.getData('text/plain') || '';
  const start = editor.selectionStart;
  const end = editor.selectionEnd;
  const before = editor.value.substring(0, start);
  const after = editor.value.substring(end);
  editor.value = before + text + after;
  editor.selectionStart = editor.selectionEnd = start + text.length;
  scheduleSave();
  scheduleAnalyze();
});

// ---------- Event wiring ----------

editor.addEventListener('input', () => {
  scheduleSave();
  scheduleAnalyze();
});

editor.addEventListener('scroll', () => {
  overlay.style.top = `-${editor.scrollTop}px`;
});

editor.addEventListener('keydown', (e: KeyboardEvent) => {
  // Esc → hide window
  if (e.key === 'Escape') {
    e.preventDefault();
    if (!settingsPanel.classList.contains('hidden')) {
      settingsPanel.classList.add('hidden');
      return;
    }
    // Hide the Tauri window
    invoke('plugin:window|hide', { label: 'main' }).catch(() => {});
    return;
  }

  // Tab → accept chip or indent
  if (e.key === 'Tab' && !e.shiftKey) {
    const cursorLine = editor.value.substring(0, editor.selectionStart).split('\n').length - 1;
    const chipEl = overlay.querySelector(`.anno-chip`) as HTMLElement | null;
    if (chipEl) {
      e.preventDefault();
      const chipLine = parseInt(chipEl.dataset.line || String(cursorLine), 10);
      acceptChip(chipLine, editor.value.split('\n')[chipLine] || '');
      return;
    }
  }

  // Enter on a timer line → start timer
  if (e.key === 'Enter') {
    const cursorLine = editor.value.substring(0, editor.selectionStart).split('\n').length - 1;
    const line = editor.value.split('\n')[cursorLine]?.trim().toLowerCase() || '';
    if (line.startsWith('timer') || line.startsWith('remind')) {
      startTimerFromLine(cursorLine);
    }
  }

  // Ctrl+N → new note
  if (e.ctrlKey && e.key === 'n') {
    e.preventDefault();
    createNewNote();
  }

  // Ctrl+ArrowLeft → prev note
  if (e.ctrlKey && e.key === 'ArrowLeft') {
    e.preventDefault();
    prevNote();
  }

  // Ctrl+ArrowRight → next note
  if (e.ctrlKey && e.key === 'ArrowRight') {
    e.preventDefault();
    nextNote();
  }

  // Ctrl+Shift+D → delete note
  if (e.ctrlKey && e.shiftKey && e.key === 'D') {
    e.preventDefault();
    deleteCurrentNote();
  }

  // Ctrl+Shift+E → export
  if (e.ctrlKey && e.shiftKey && e.key === 'E') {
    e.preventDefault();
    exportNote();
  }

  // Ctrl+P → toggle plain mode
  if (e.ctrlKey && e.key === 'p') {
    e.preventDefault();
    togglePlainMode();
  }
});

// Ctrl held → show chrome bars
document.addEventListener('keydown', (e) => {
  if (e.key === 'Control') document.body.classList.add('ctrl-held');
});
document.addEventListener('keyup', (e) => {
  if (e.key === 'Control') document.body.classList.remove('ctrl-held');
});

// Button clicks
btnPrev.addEventListener('click', () => prevNote());
btnNext.addEventListener('click', () => nextNote());
btnNew.addEventListener('click', () => createNewNote());
btnDelete.addEventListener('click', () => deleteCurrentNote());
btnExport.addEventListener('click', () => exportNote());
btnSettings.addEventListener('click', () => settingsPanel.classList.toggle('hidden'));
btnCloseSettings.addEventListener('click', () => settingsPanel.classList.add('hidden'));

btnPin.addEventListener('click', async () => {
  if (!currentId || !notes[currentIndex]) return;
  const newVal = !notes[currentIndex].pinned;
  await invoke('set_pinned', { id: currentId, pinned: newVal });
  await loadNotes();
  updateNoteUI();
});

btnPlain.addEventListener('click', () => togglePlainMode());

async function togglePlainMode(): Promise<void> {
  currentPlain = !currentPlain;
  await invoke('set_plain', { id: currentId, plain: currentPlain });
  updateNoteUI();
  scheduleAnalyze();
}

// Settings changes
settingTheme.addEventListener('change', async () => {
  const theme = settingTheme.value;
  applyTheme(theme);
  await invoke('set_setting', { key: 'theme', value: theme });
});

settingGlobalPlain.addEventListener('change', async () => {
  globalPlain = settingGlobalPlain.checked;
  await invoke('set_setting', { key: 'global_plain', value: globalPlain });
  scheduleAnalyze();
});

// Window geometry persistence
let geoTimeout: ReturnType<typeof setTimeout> | null = null;
function saveGeometry(): void {
  if (geoTimeout) clearTimeout(geoTimeout);
  geoTimeout = setTimeout(async () => {
    // Tauri will handle this via the window's actual position
    const w = window.innerWidth;
    const h = window.innerHeight;
    await invoke('set_geometry', {
      geo: { width: w, height: h, x: window.screenX, y: window.screenY },
    });
  }, 500);
}
window.addEventListener('resize', saveGeometry);

// Timer display update loop
setInterval(updateTimerDisplay, 1000);

// ---------- Boot ----------
init().catch(console.error);
