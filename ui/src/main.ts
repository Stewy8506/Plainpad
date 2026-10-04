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
  const t = win.__TAURI__;
  if (t) {
    if (t.core && typeof t.core.invoke === 'function') {
      return t.core.invoke(cmd, args) as Promise<T>;
    }
    if (typeof t.invoke === 'function') {
      return t.invoke(cmd, args) as Promise<T>;
    }
  }
  const internals = win.__TAURI_INTERNALS__;
  if (internals && typeof internals.invoke === 'function') {
    return internals.invoke(cmd, args) as Promise<T>;
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
let hideOnBlur = false;
let swipeNav = false;
let swipeSensitivity = 60;
let autoBrackets = true;
let tabWidth = '2';
let saveTimeout: ReturnType<typeof setTimeout> | null = null;
let analyzeTimeout: ReturnType<typeof setTimeout> | null = null;
let activeTimers: Timer[] = [];

// ---------- DOM refs ----------

const editor = document.getElementById('editor') as HTMLTextAreaElement;
const backdrop = document.getElementById('editor-backdrop') as HTMLDivElement;
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
const btnAlwaysOnTop = document.getElementById('btn-always-on-top') as HTMLButtonElement;
const btnMinimize = document.getElementById('btn-minimize') as HTMLButtonElement;
const btnMaximize = document.getElementById('btn-maximize') as HTMLButtonElement;
const btnClose = document.getElementById('btn-close') as HTMLButtonElement;
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
    if (typeof settings.theme === 'string') applyTheme(settings.theme);
    if (typeof settings.font_family === 'string') applyFontFamily(settings.font_family);
    if (typeof settings.font_size === 'number') applyFontSize(settings.font_size);
    if (typeof settings.line_height === 'number') applyLineHeight(settings.line_height);
    if (typeof settings.dimming === 'number') applyDimming(settings.dimming);
    if (typeof settings.window_opacity === 'number') applyWindowOpacity(settings.window_opacity);
    if (typeof settings.global_plain === 'boolean') globalPlain = settings.global_plain;
    if (typeof settings.hide_on_blur === 'boolean') hideOnBlur = settings.hide_on_blur;
    if (typeof settings.swipe_nav === 'boolean') swipeNav = settings.swipe_nav;
    if (typeof settings.swipe_sensitivity === 'number') swipeSensitivity = settings.swipe_sensitivity;
    if (typeof settings.auto_brackets === 'boolean') autoBrackets = settings.auto_brackets;
    if (typeof settings.tab_width === 'string') tabWidth = settings.tab_width;
  }

  // Real-time listener for settings changed in Preferences window
  await listen('setting-changed', (payload: unknown) => {
    const p = payload as { key: string; value: unknown };
    if (!p || !p.key) return;
    if (p.key === 'theme') applyTheme(String(p.value));
    else if (p.key === 'font_family') applyFontFamily(String(p.value));
    else if (p.key === 'font_size') applyFontSize(Number(p.value));
    else if (p.key === 'line_height') applyLineHeight(Number(p.value));
    else if (p.key === 'dimming') applyDimming(Number(p.value));
    else if (p.key === 'window_opacity') applyWindowOpacity(Number(p.value));
    else if (p.key === 'global_plain') {
      globalPlain = !!p.value;
      scheduleAnalyze();
    } else if (p.key === 'hide_on_blur') {
      hideOnBlur = !!p.value;
    } else if (p.key === 'swipe_nav') {
      swipeNav = !!p.value;
    } else if (p.key === 'swipe_sensitivity') {
      swipeSensitivity = Number(p.value);
    } else if (p.key === 'auto_brackets') {
      autoBrackets = !!p.value;
    } else if (p.key === 'tab_width') {
      tabWidth = String(p.value);
    }
  });

  const isPinned = await invoke<boolean>('get_always_on_top');
  if (btnAlwaysOnTop) btnAlwaysOnTop.classList.toggle('active', !!isPinned);

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

  // Listen for live rate updates
  try {
    await listen('rates-updated', () => {
      scheduleAnalyze();
    });
  } catch (err) {
    console.warn('Failed to listen for rates-updated:', err);
  }
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
  updateBackdrop();

  activeAnnoCache.clear();
  overlay.innerHTML = '';
  skipKineticAnimation = true;
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
  updateBackdrop();
  await loadNotes();

  // New note goes to front
  notes.unshift({ id, title: '', mtime_ms: Date.now(), pinned: false, plain: false });
  currentIndex = 0;
  currentPlain = false;
  activeAnnoCache.clear();
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
  analyzeTimeout = setTimeout(() => runAnalysis(), 35);
}

async function runAnalysis(): Promise<void> {
  const text = editor.value;
  const plain = currentPlain || globalPlain;
  const analysis = await invoke<Analysis>('analyze_note', { text, plain });
  if (!analysis) {
    for (const child of Array.from(overlay.children) as HTMLElement[]) {
      triggerExit(child);
    }
    activeAnnoCache.clear();
    return;
  }

  renderOverlay(analysis, text);

  // Code mode: toggle body class
  document.body.classList.toggle('code-mode', analysis.code_mode);
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function updateBackdrop(): void {
  if (!backdrop) return;
  const text = editor.value;
  if (!text) {
    backdrop.innerHTML = '';
    return;
  }
  const lines = text.split('\n');
  const firstNonEmpty = lines.findIndex((l) => l.trim().length > 0);

  const htmlLines = lines.map((line, idx) => {
    const trimmed = line.trim();
    const lower = trimmed.toLowerCase();

    // Mode keyword on first non-empty line turns green/colored immediately
    if (idx === firstNonEmpty) {
      if (lower === 'math') {
        return `<span class="hl-mode hl-mode-math">${escapeHtml(line)}</span>`;
      } else if (lower === 'calcs') {
        return `<span class="hl-mode hl-mode-calcs">${escapeHtml(line)}</span>`;
      } else if (lower === 'sum') {
        return `<span class="hl-mode hl-mode-sum">${escapeHtml(line)}</span>`;
      } else if (lower === 'list') {
        return `<span class="hl-mode hl-mode-list">${escapeHtml(line)}</span>`;
      } else if (lower === 'paste') {
        return `<span class="hl-mode hl-mode-paste">${escapeHtml(line)}</span>`;
      } else if (lower === 'code' || lower.startsWith('code:')) {
        return `<span class="hl-mode hl-mode-code">${escapeHtml(line)}</span>`;
      }
    }

    // Comment lines (// ...)
    if (trimmed.startsWith('//')) {
      return `<span class="hl-comment">${escapeHtml(line)}</span>`;
    }

    // Variable / label lines: e.g. "trees: 23,405" or "rent = 12000"
    const colonIdx = line.indexOf(':');
    const eqIdx = line.indexOf('=');
    if (colonIdx > 0 && (eqIdx < 0 || colonIdx < eqIdx)) {
      const label = line.substring(0, colonIdx);
      if (label.trim().length > 0 && label.trim().split(/\s+/).length <= 4) {
        const rest = line.substring(colonIdx + 1);
        return `<span class="hl-var">${escapeHtml(label)}</span><span class="hl-punct">:</span>${escapeHtml(rest)}`;
      }
    }

    return escapeHtml(line);
  });

  let resultHtml = htmlLines.join('\n');
  if (text.endsWith('\n')) {
    resultHtml += '\n ';
  }
  backdrop.innerHTML = resultHtml;
}

// Cache of rendered annotation values to prevent re-animating already-settled lines
let activeAnnoCache = new Map<string, string>();
// Suppress kinetic animations during app startup, note loading, and note switching
let skipKineticAnimation = false;

function renderKineticContent(parent: HTMLElement, prefix: string, text: string, isNew: boolean): void {
  if (!isNew) {
    if (prefix) {
      const p = document.createElement('span');
      p.className = 'anno-prefix';
      p.textContent = prefix;
      parent.appendChild(p);
    }
    parent.appendChild(document.createTextNode(text));
    return;
  }

  const container = document.createElement('span');
  container.className = 'kinetic-container';
  let wordIndex = 0;

  if (prefix) {
    const p = document.createElement('span');
    p.className = 'kinetic-word anno-prefix';
    p.style.setProperty('--word-index', String(wordIndex++));
    p.textContent = prefix;
    container.appendChild(p);
  }

  // Tokenize by whitespace so each word receives fluid kinetic stagger
  const tokens = text.split(/(\s+)/);
  for (const token of tokens) {
    if (token.trim().length === 0) {
      container.appendChild(document.createTextNode(token));
    } else {
      const wSpan = document.createElement('span');
      wSpan.className = 'kinetic-word';
      wSpan.style.setProperty('--word-index', String(wordIndex++));
      wSpan.textContent = token;
      container.appendChild(wSpan);
    }
  }
  parent.appendChild(container);
}

function triggerExit(el: HTMLElement): void {
  if (el.classList.contains('kinetic-exiting')) return;

  const rawKey = el.dataset.annoKey || '';
  const baseKey = rawKey.split(':exiting')[0];
  const exitingPrefix = `${baseKey}:exiting`;

  // If there's an older exiting element on this same line, remove it immediately
  // so rapid backspacing doesn't stack multiple ghost annotations.
  if (baseKey && el.parentNode) {
    for (const child of Array.from(el.parentNode.children) as HTMLElement[]) {
      if (child !== el && child.dataset.annoKey?.startsWith(exitingPrefix)) {
        child.remove();
      }
    }
  }

  el.dataset.annoKey = `${baseKey}:exiting:${Date.now()}`;
  el.classList.add('kinetic-exiting');
  el.style.pointerEvents = 'none';

  // Stop child entrance animations so they don't fight the exit or fire rogue animationend events
  const animChildren = el.querySelectorAll<HTMLElement>('.kinetic-word, .kinetic-reveal-chip');
  for (const child of Array.from(animChildren)) {
    child.style.animation = 'none';
  }

  const cleanup = () => {
    if (el.parentNode) {
      el.remove();
    }
  };

  el.addEventListener('animationend', (e: AnimationEvent) => {
    if (e.target === el) {
      cleanup();
    }
  }, { once: true });

  setTimeout(cleanup, 260);
}

function renderOverlay(analysis: Analysis, text: string): void {
  const lines = text.split('\n');

  // Sync scroll position with textarea
  overlay.style.top = `-${editor.scrollTop}px`;

  // Compute line heights and text widths from the mirror
  mirror.textContent = '';
  const lineEls: HTMLDivElement[] = [];
  const textSpans: HTMLSpanElement[] = [];
  for (let i = 0; i < lines.length; i++) {
    const row = document.createElement('div');
    const span = document.createElement('span');
    span.textContent = lines[i] || ' ';
    span.style.display = 'inline-block';
    row.appendChild(span);
    mirror.appendChild(row);
    lineEls.push(row);
    textSpans.push(span);
  }

  // Map existing active elements currently in the DOM
  const existingEls = new Map<string, HTMLElement>();
  for (const child of Array.from(overlay.children) as HTMLElement[]) {
    const key = child.dataset.annoKey;
    if (key && !child.classList.contains('kinetic-exiting')) {
      existingEls.set(key, child);
    }
  }

  const nextAnnoCache = new Map<string, string>();

  for (const anno of analysis.annotations) {
    if (anno.line >= lineEls.length) continue;
    const lineEl = lineEls[anno.line];
    const textSpan = textSpans[anno.line];
    if (!lineEl || !textSpan) continue;
    const top = lineEl.offsetTop;
    const rawLine = lines[anno.line] || '';
    const lineText = rawLine.trim();

    // Inline left position: placed right after typed text with clean spacing
    const padX = 20; // var(--pad-x)
    const textWidth = lineText.length > 0 ? textSpan.offsetWidth : 0;
    const maxLeft = Math.max(padX, editor.clientWidth - 130);
    const left = Math.min(padX + textWidth + 10, maxLeft);

    const annoKey = `${anno.line}:${anno.kind}`;
    const annoValue = String(anno.value || anno.chip || '');
    const isNew = !skipKineticAnimation && (activeAnnoCache.get(annoKey) !== annoValue);
    nextAnnoCache.set(annoKey, annoValue);

    const prevEl = existingEls.get(annoKey);

    if (!isNew && prevEl) {
      // Unchanged: update layout position in case typing on previous lines shifted vertical line offsets
      prevEl.style.top = `${anno.kind === 'checklist' ? top + 2 : top}px`;
      if (anno.kind !== 'checklist') {
        prevEl.style.left = `${left}px`;
      }
      existingEls.delete(annoKey);
      continue;
    }

    // If an old element with this key was already in the DOM but its value changed, smoothly exit the old one
    if (prevEl) {
      triggerExit(prevEl);
      existingEls.delete(annoKey);
    }

    let el: HTMLElement | null = null;

    if (anno.kind === 'math' && anno.confidence >= 0.85 && anno.value) {
      el = document.createElement('div');
      const hasTrailingEquals = lineText.endsWith('=') || lineText.endsWith('?');
      el.className = `anno anno-math${hasTrailingEquals ? ' has-equals' : ''}`;
      el.style.top = `${top}px`;
      el.style.left = `${left}px`;
      const prefix = hasTrailingEquals ? '' : '= ';
      const formatted = formatNumber(anno.value);
      renderKineticContent(el, prefix, formatted, isNew);
      el.title = anno.why;
    } else if (anno.kind === 'conversion' && anno.confidence >= 0.85 && anno.value) {
      el = document.createElement('div');
      const hasTrailingEquals = lineText.endsWith('=') || lineText.endsWith('?');
      el.className = `anno anno-conversion${hasTrailingEquals ? ' has-equals' : ''}`;
      el.style.top = `${top}px`;
      el.style.left = `${left}px`;
      const prefix = hasTrailingEquals ? '' : '= ';
      renderKineticContent(el, prefix, anno.value, isNew);
      el.title = anno.why;
    } else if (anno.kind === 'timer') {
      el = document.createElement('div');
      el.className = `anno anno-timer-chip${isNew ? ' kinetic-reveal-chip' : ''}`;
      el.style.top = `${top}px`;
      el.style.left = `${left}px`;
      const isRunning = activeTimers.some((t) => !t.fired);
      const clockSvg = `<svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" style="vertical-align: -1px; margin-right: 5px;"><circle cx="8" cy="8" r="6"/><path d="M8 4.5v3.5l2 2"/></svg>`;
      if (isRunning) {
        el.innerHTML = `${clockSvg}Running (Click to stop)`;
        el.classList.add('running');
        el.title = 'Click to stop timer';
        el.addEventListener('click', () => stopActiveTimer());
      } else {
        const iconSpan = document.createElement('span');
        iconSpan.innerHTML = clockSvg;
        el.appendChild(iconSpan);
        const timerText = `Start ${anno.value} (Enter)`;
        el.appendChild(document.createTextNode(timerText));
        el.title = 'Click or press Enter to start timer';
        el.addEventListener('click', () => startTimerFromLine(anno.line));
      }
    } else if (anno.kind === 'timer_stop') {
      el = document.createElement('div');
      el.className = `anno anno-timer-chip stop${isNew ? ' kinetic-reveal-chip' : ''}`;
      el.style.top = `${top}px`;
      el.style.left = `${left}px`;
      const stopSvg = `<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="vertical-align: -1px; margin-right: 5px;"><rect x="3" y="3" width="10" height="10" rx="1.5"/></svg>`;
      const iconSpan = document.createElement('span');
      iconSpan.innerHTML = stopSvg;
      el.appendChild(iconSpan);
      el.appendChild(document.createTextNode('Stop timer (Enter)'));
      el.title = 'Click or press Enter to stop active timer';
      el.addEventListener('click', () => stopActiveTimer());
    } else if (anno.kind === 'checklist' && (lineText.startsWith('[ ] ') || lineText.startsWith('[x] ') || lineText.startsWith('[X] '))) {
      const isChecked = lineText.startsWith('[x] ') || lineText.startsWith('[X] ');
      el = document.createElement('div');
      el.className = `anno-checkbox${isChecked ? ' checked' : ''}`;
      el.style.top = `${top + 2}px`;
      el.dataset.line = String(anno.line);
      el.addEventListener('click', () => toggleCheckbox(anno.line));
    } else if (anno.chip) {
      el = document.createElement('div');
      el.className = `anno anno-chip${isNew ? ' kinetic-reveal-chip' : ''}`;
      el.style.top = `${top}px`;
      el.style.left = `${left}px`;
      el.dataset.line = String(anno.line);
      el.textContent = anno.chip;
      el.title = anno.why;
      el.addEventListener('click', () => acceptChip(anno.line, lines[anno.line] || ''));
    }

    if (el) {
      el.dataset.annoKey = annoKey;
      overlay.appendChild(el);
    }
  }

  // Any remaining elements in existingEls were invalidated (e.g. user backspaced)!
  // Gracefully animate them out with Apple-style kinetic exit blur
  for (const [, elToExit] of existingEls) {
    if (skipKineticAnimation) {
      elToExit.remove();
    } else {
      triggerExit(elToExit);
    }
  }

  activeAnnoCache = nextAnnoCache;
  skipKineticAnimation = false;
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

  // Timer line: start timer
  if (line.toLowerCase().startsWith('timer') || line.toLowerCase().startsWith('remind')) {
    startTimerFromLine(lineIndex);
    return;
  }

  // "Make checklist" → convert comma-list or "todo" to checkboxes
  if (text.toLowerCase().startsWith('todo ')) {
    lines[lineIndex] = '[ ] ' + line.substring(line.toLowerCase().indexOf('todo ') + 5);
  } else if (line.includes(',')) {
    // Comma list → multiple checkbox lines
    const items = line.split(',').map((s) => s.trim()).filter(Boolean);
    lines.splice(lineIndex, 1, ...items.map((item) => `[ ] ${item}`));
  }

  editor.value = lines.join('\n');
  updateBackdrop();
  scheduleSave();
  scheduleAnalyze();
}

// ---------- Timer actions ----------

async function startTimerFromLine(lineIndex: number): Promise<void> {
  const lines = editor.value.split('\n');
  if (lineIndex >= lines.length) return;
  const line = lines[lineIndex].trim();

  // Parse duration from the analysis or raw text
  const analysis = await invoke<Analysis>('analyze_note', { text: line, plain: false });
  const timerAnno = analysis?.annotations?.find((a) => a.kind === 'timer');

  const durationMs = timerAnno ? parseDurationFromValue(timerAnno.value) : (parseDurationFromValue(line) || 300_000);
  if (durationMs > 0) {
    let name = 'Timer';
    if (timerAnno?.chip) {
      name = timerAnno.chip
        .replace(/^Start timer \(/i, '')
        .replace(/^⏱ Start /i, '')
        .replace(/\)\s*\(Enter\)$/i, '')
        .replace(/\(Enter\)$/i, '')
        .replace(/\)$/i, '')
        .trim() || 'Timer';
    }
    const id = await invoke<number>('start_timer', { name, durationMs, duration_ms: durationMs });
    activeTimers.push({
      id: id || Date.now(),
      name,
      due_ms: Date.now() + durationMs,
      duration_ms: durationMs,
      fired: false,
    });
    updateTimerDisplay();
  }
}

function parseDurationFromValue(value: string): number {
  let ms = 0;
  const parts = value.match(/(\d+)\s*(h|hr|hour|hours|min|mins|minute|minutes|m|s|sec|secs|second|seconds)/gi);
  if (parts) {
    for (const part of parts) {
      const m = part.match(/(\d+)\s*(h|hr|hour|hours|min|mins|minute|minutes|m|s|sec|secs|second|seconds)/i);
      if (!m) continue;
      const n = parseInt(m[1], 10);
      const unit = m[2].toLowerCase();
      if (unit.startsWith('h')) ms += n * 3600000;
      else if (unit.startsWith('m') && !unit.startsWith('ms')) ms += n * 60000;
      else if (unit.startsWith('s')) ms += n * 1000;
    }
  }
  if (ms === 0) {
    const rawNum = parseInt(value.replace(/\D/g, ''), 10);
    if (!isNaN(rawNum) && rawNum > 0) {
      ms = rawNum * 60000; // default bare numbers to minutes (e.g. "5" -> 5 minutes)
    }
  }
  return ms;
}

async function stopActiveTimer(id?: number): Promise<void> {
  const timerToCancel = id !== undefined
    ? activeTimers.find((t) => t.id === id)
    : activeTimers.find((t) => !t.fired);
  if (!timerToCancel) return;

  await invoke('cancel_timer', { id: timerToCancel.id });
  activeTimers = activeTimers.filter((t) => t.id !== timerToCancel.id);
  updateTimerDisplay();
  scheduleAnalyze();
}

timerDisplay?.addEventListener('click', (e) => {
  e.stopPropagation();
  stopActiveTimer();
});

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
    <button class="timer-btn-stop" title="Stop timer"><svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg></button>
  `;

  const btnStop = timerDisplay.querySelector('.timer-btn-stop');
  btnStop?.addEventListener('click', (e) => {
    e.stopPropagation();
    stopActiveTimer(nearest.id);
  });
}

function showTimerNotification(name: string, overdue: boolean): void {
  const msg = overdue ? `${name} (missed while away)` : `${name} — time's up!`;
  // Show in-app notification (flash the timer display)
  timerDisplay.classList.remove('hidden');
  timerDisplay.innerHTML = `<span class="timer-value" style="color: var(--anno-math)">${msg}</span>`;
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
  if (theme === 'dark' || theme === 'oled' || theme === 'slate' || theme === 'light') {
    document.documentElement.setAttribute('data-theme', theme);
  } else {
    document.documentElement.removeAttribute('data-theme');
  }
}

function applyFontFamily(font?: string): void {
  if (!font) return;
  let family = "var(--font-mono)";
  if (font === 'jetbrains') family = "'JetBrains Mono', monospace";
  else if (font === 'cascadia') family = "'Cascadia Code', monospace";
  else if (font === 'fira') family = "'Fira Code', monospace";
  else if (font === 'inter') family = "Inter, -apple-system, sans-serif";
  else if (font === 'monospace') family = "monospace";
  editor.style.fontFamily = family;
  if (backdrop) backdrop.style.fontFamily = family;
  if (mirror) mirror.style.fontFamily = family;
}

function applyFontSize(size?: number): void {
  if (!size || size < 10) return;
  const s = `${size}px`;
  editor.style.fontSize = s;
  if (backdrop) backdrop.style.fontSize = s;
  if (mirror) mirror.style.fontSize = s;
}

function applyLineHeight(lh?: number): void {
  if (!lh || lh < 1) return;
  const s = `${lh}`;
  editor.style.lineHeight = s;
  if (backdrop) backdrop.style.lineHeight = s;
  if (mirror) mirror.style.lineHeight = s;
}

function applyDimming(dim?: number): void {
  if (dim === undefined || dim === null) return;
  const alpha = Math.min(1.0, Math.max(0.3, dim / 100));
  document.documentElement.style.setProperty('--anno-opacity', `${alpha}`);
}

function applyWindowOpacity(op?: number): void {
  if (op === undefined || op === null) return;
  const alpha = Math.min(1.0, Math.max(0.7, op / 100));
  document.documentElement.style.setProperty('--window-opacity', `${alpha}`);
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
  updateBackdrop();
  scheduleSave();
  scheduleAnalyze();
});

// ---------- Event wiring ----------

editor.addEventListener('input', () => {
  updateBackdrop();
  scheduleSave();
  scheduleAnalyze();
});

editor.addEventListener('scroll', () => {
  overlay.style.top = `-${editor.scrollTop}px`;
  if (backdrop) {
    backdrop.scrollTop = editor.scrollTop;
    backdrop.scrollLeft = editor.scrollLeft;
  }
});

editor.addEventListener('keydown', (e: KeyboardEvent) => {
  // Esc → hide window
  if (e.key === 'Escape') {
    e.preventDefault();
    invoke('hide_window');
    return;
  }

  // Auto-close brackets and quotes
  const pairs: Record<string, string> = { '(': ')', '[': ']', '{': '}', '"': '"', "'": "'" };
  if (autoBrackets && pairs[e.key]) {
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    if (start === end) {
      e.preventDefault();
      const close = pairs[e.key];
      editor.value = editor.value.substring(0, start) + e.key + close + editor.value.substring(end);
      editor.selectionStart = editor.selectionEnd = start + 1;
      updateBackdrop();
      scheduleSave();
      scheduleAnalyze();
      return;
    }
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
    e.preventDefault();
    const indent = tabWidth === 'tab' ? '\t' : (tabWidth === '4' ? '    ' : '  ');
    const start = editor.selectionStart;
    const end = editor.selectionEnd;
    editor.value = editor.value.substring(0, start) + indent + editor.value.substring(end);
    editor.selectionStart = editor.selectionEnd = start + indent.length;
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // Enter on a timer line → start or stop timer
  if (e.key === 'Enter') {
    const cursorLine = editor.value.substring(0, editor.selectionStart).split('\n').length - 1;
    const line = editor.value.split('\n')[cursorLine]?.trim().toLowerCase() || '';
    if (line === 'timer s' || line === 'timer stop' || line === 'timer cancel' || line === 'stop timer') {
      e.preventDefault();
      stopActiveTimer();
      return;
    }
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
btnSettings.addEventListener('click', () => {
  invoke('open_settings_window').catch(console.error);
});

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
  skipKineticAnimation = true;
  scheduleAnalyze();
}

btnAlwaysOnTop?.addEventListener('click', async () => {
  const newVal = await invoke<boolean>('toggle_always_on_top');
  btnAlwaysOnTop.classList.toggle('active', !!newVal);
});

// Auto-hide when focus is lost (Antinote behavior: applies only while unpinned)
window.addEventListener('blur', async () => {
  if (hideOnBlur) {
    const isPinned = await invoke<boolean>('get_always_on_top').catch(() => false);
    if (!isPinned) {
      invoke('hide_window');
    }
  }
});

// Trackpad swipe between notes
let accumulatedDeltaX = 0;
let swipeCooldown = false;
window.addEventListener('wheel', (e: WheelEvent) => {
  if (!swipeNav || swipeCooldown) return;
  if (Math.abs(e.deltaX) > Math.abs(e.deltaY) && Math.abs(e.deltaX) > 10) {
    accumulatedDeltaX += e.deltaX;
    if (Math.abs(accumulatedDeltaX) >= swipeSensitivity) {
      swipeCooldown = true;
      if (accumulatedDeltaX > 0) {
        nextNote();
      } else {
        prevNote();
      }
      accumulatedDeltaX = 0;
      setTimeout(() => { swipeCooldown = false; }, 350);
    }
  } else {
    accumulatedDeltaX = 0;
  }
}, { passive: true });

btnMinimize?.addEventListener('click', () => {
  invoke('minimize_window');
});

btnMaximize?.addEventListener('click', () => {
  invoke('toggle_maximize');
});

btnClose?.addEventListener('click', async () => {
  await saveCurrentNote();
  invoke('hide_window');
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
