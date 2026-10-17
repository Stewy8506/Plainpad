// Plainpad — UI entry point
// No framework: textarea + mirror + overlay editor with IPC to Tauri core.
import { SlottedNotesBottomBar, DEITY_SLOTS } from './slotted.js';
import { initTimerUI, clearTimer } from './timer.js';
import {
  toggleCheckboxLine,
  parseCheckboxLine,
  handleListEnterKey,
} from './list_manager.js';
import {
  findAntilinks,
  findCodeBlocks,
  createAntilinkElement,
  createCodeBlockHeaderElement,
} from './rich_elements.js';

if (typeof navigator !== 'undefined' && (/Linux/i.test(navigator.userAgent) || /Linux/i.test(navigator.platform))) {
  document.documentElement.classList.add('platform-linux');
}
import { VimEngine } from './vim.js';
import {
  cycleHeadingLevel,
  applyInlineFormat,
  CaretNudgeController,
} from './typography.js';
import {
  extensionRuntime,
  findCommandInLine,
  executeCommand,
  type DetectedCommand,
} from './extensions.js';
import { voidShader } from './void_shaders.js';
import { FindAndReplaceManager } from './search.js';
import { ExportManager } from './export.js';

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
  last_modified: string;
  db_index: number;
  is_slotted: boolean;
  slot_index: number;
  is_locked: boolean;
  is_archived: boolean;
  is_private: boolean;
  is_tutorial: boolean;
  soft_deleted: boolean;
  deleted_at: string | null;
  pinned?: boolean;
  plain?: boolean;
}

interface Note {
  id: string;
  content: string;
  created: string;
  last_modified: string;
  db_index: number;
  is_slotted: boolean;
  slot_index: number;
  is_locked: boolean;
  is_archived: boolean;
  is_private: boolean;
  is_tutorial: boolean;
  soft_deleted: boolean;
  deleted_at: string | null;
  skipped_urls?: string[];
  ck_change_tag?: string | null;
  ck_system_fields?: number[] | null;
  text?: string;
  meta?: NoteMeta;
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


// ---------- State ----------

let notes: NoteMeta[] = [];
let currentIndex = 0;
let currentId = '';
let currentText = '';
let currentPlain = false;
let currentIsSlotted = false;
let currentSlotIndex = -1;
let slottedBar: SlottedNotesBottomBar | null = null;
let globalPlain = false;
let hideOnBlur = false;
let swipeNav = false;
let swipeSensitivity = 60;
let autoBrackets = true;
let tabWidth = '2';
let inlineChecklistCheckboxesEnabled = true;
let inlineChecklistBulletsEnabled = true;
let inlineNumberedListsEnabled = true;
let disableLinkShortening = false;
let disableLinkHighlighting = false;
let saveTimeout: ReturnType<typeof setTimeout> | null = null;
let analyzeTimeout: ReturnType<typeof setTimeout> | null = null;
let statusTimeout: ReturnType<typeof setTimeout> | null = null;
let isAutoPasteActive = false;
let autoPastePollTimer: number | null = null;
let triggerConfig: Record<string, unknown> | null = null;
const AUTOPASTE_PROPOSED_MSG = "Enter to start AutoPaste. Everything you copy will be added here. Esc stops autopaste.";
const AUTOPASTE_ACTIVE_MSG = "AutoPaste active. Everything you copy will be added here. Esc stops autopaste.";

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
const syncStatus = document.getElementById('sync-status') as HTMLSpanElement | null;
const autopasteIndicator = document.getElementById('autopaste-indicator') as HTMLDivElement | null;
const autopasteStatusText = document.getElementById('autopaste-status-text') as HTMLSpanElement | null;
const autopasteBtnStop = document.getElementById('autopaste-btn-stop') as HTMLButtonElement | null;
const vimStatusIndicator = document.getElementById('vim-status-indicator') as HTMLSpanElement | null;
const readonlyBadge = document.getElementById('readonly-badge') as HTMLElement | null;
const editorContainer = document.getElementById('editor-container') as HTMLElement;

// The Void & Shaders DOM elements
const btnVoid = document.getElementById('btn-void') as HTMLButtonElement | null;
const voidModal = document.getElementById('void-modal') as HTMLDivElement | null;
const voidModalBackdrop = document.getElementById('void-modal-backdrop') as HTMLDivElement | null;
const btnCloseVoid = document.getElementById('btn-close-void') as HTMLButtonElement | null;
const btnEmptyVoid = document.getElementById('btn-empty-void') as HTMLButtonElement | null;
const voidNoteList = document.getElementById('void-note-list') as HTMLDivElement | null;
const voidConfirmDialog = document.getElementById('void-confirm-dialog') as HTMLDivElement | null;
const voidConfirmBackdrop = document.getElementById('void-confirm-backdrop') as HTMLDivElement | null;
const voidConfirmMsg = document.getElementById('void-confirm-message') as HTMLParagraphElement | null;
const btnCancelEmptyVoid = document.getElementById('btn-cancel-empty-void') as HTMLButtonElement | null;
const btnConfirmEmptyVoid = document.getElementById('btn-confirm-empty-void') as HTMLButtonElement | null;

let activateVimMode = false;
let forceRTL = false;
let isReadOnly = false;
let caretNudgeRaw = 0;
let caretNudgeForListsRaw = 0;

const vimEngine = new VimEngine({
  textarea: editor,
  container: document.getElementById('editor-container') as HTMLElement,
  statusIndicator: vimStatusIndicator,
  onTextChange: () => {
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
  },
});

const btnSearch = document.getElementById('btn-search') as HTMLButtonElement | null;
const findReplaceContainer = document.getElementById('find-replace-container') as HTMLElement;
const searchInput = document.getElementById('search-input') as HTMLInputElement;
const replaceInput = document.getElementById('replace-input') as HTMLInputElement;
const searchMatchType = document.getElementById('search-match-type') as HTMLSelectElement;
const btnSearchCase = document.getElementById('btn-search-case') as HTMLButtonElement;
const btnSearchRegex = document.getElementById('btn-search-regex') as HTMLButtonElement;
const btnSearchScopeNote = document.getElementById('btn-search-scope-note') as HTMLButtonElement;
const btnSearchScopeAll = document.getElementById('btn-search-scope-all') as HTMLButtonElement;
const btnSearchPrev = document.getElementById('btn-search-prev') as HTMLButtonElement;
const btnSearchNext = document.getElementById('btn-search-next') as HTMLButtonElement;
const btnSearchClose = document.getElementById('btn-search-close') as HTMLButtonElement;
const btnReplaceOne = document.getElementById('btn-replace-one') as HTMLButtonElement;
const btnReplaceAll = document.getElementById('btn-replace-all') as HTMLButtonElement;
const searchMatchCount = document.getElementById('search-match-count') as HTMLElement;
const searchResultsList = document.getElementById('search-results-list') as HTMLElement;
const findReplaceRow = document.getElementById('find-replace-row') as HTMLElement;

const findAndReplaceManager = new FindAndReplaceManager({
  editor,
  container: findReplaceContainer,
  searchInput,
  replaceInput,
  matchTypeSelect: searchMatchType,
  btnCase: btnSearchCase,
  btnRegex: btnSearchRegex,
  btnScopeNote: btnSearchScopeNote,
  btnScopeAll: btnSearchScopeAll,
  btnPrev: btnSearchPrev,
  btnNext: btnSearchNext,
  btnClose: btnSearchClose,
  btnReplaceOne,
  btnReplaceAll,
  matchCountLabel: searchMatchCount,
  resultsList: searchResultsList,
  replaceRow: findReplaceRow,
});

findAndReplaceManager.onHighlightUpdate = () => {
  updateBackdrop();
};
findAndReplaceManager.onTextChange = (newText: string) => {
  currentText = newText;
  editor.value = newText;
  updateBackdrop();
  scheduleSave();
  scheduleAnalyze();
};
findAndReplaceManager.onStatusMessage = (msg: string) => {
  showStatusMessage(msg);
};
findAndReplaceManager.fetchAllNotes = async () => {
  try {
    const all = await invoke<any[]>('get_all_notes');
    if (Array.isArray(all) && all.length > 0) return all;
  } catch {
    // fallback if get_all_notes not yet compiled
  }
  const list = (await invoke<NoteMeta[]>('list_notes')) || [];
  const fullNotes: any[] = [];
  for (const meta of list) {
    try {
      const full = await invoke<any>('load_note', { id: meta.id });
      if (full) fullNotes.push(full);
    } catch {
      // ignore
    }
  }
  return fullNotes;
};
findAndReplaceManager.onSelectNote = async (noteId, isSlotted, slotIndex, offset) => {
  if (isSlotted && slotIndex >= 0) {
    await switchToSlot(slotIndex);
  } else {
    const idx = notes.findIndex((n) => n.id === noteId);
    if (idx >= 0) {
      await loadNote(idx);
    }
  }
  if (offset !== undefined && offset >= 0) {
    setTimeout(() => {
      findAndReplaceManager.focusMatch(0, true);
    }, 50);
  }
};

const exportModal = document.getElementById('export-modal') as HTMLElement;
const exportModalBackdrop = document.getElementById('export-modal-backdrop') as HTMLElement;
const btnCloseExport = document.getElementById('btn-close-export') as HTMLButtonElement;

const exportManager = new ExportManager({
  modal: exportModal,
  backdrop: exportModalBackdrop,
  closeBtn: btnCloseExport,
});

exportManager.onStatusMessage = (msg: string) => {
  showStatusMessage(msg);
};

const caretNudgeController = new CaretNudgeController(editorContainer, editor);

function setRTLEnabled(enabled: boolean): void {
  forceRTL = enabled;
  const dirValue = enabled ? 'rtl' : 'ltr';
  editor.setAttribute('dir', dirValue);
  backdrop.setAttribute('dir', dirValue);
  mirror.setAttribute('dir', dirValue);
  editorContainer.setAttribute('dir', dirValue);
  document.body.classList.toggle('rtl-mode', enabled);
  scheduleAnalyze();
}

function setGridOverlay(mode: string): void {
  if (!editorContainer) return;
  editorContainer.classList.remove('grid-none', 'grid-dots', 'grid-small-grid', 'grid-big-grid');
  if (mode === 'dots') {
    editorContainer.classList.add('grid-dots');
  } else if (mode === 'smallGrid' || mode === 'small-grid') {
    editorContainer.classList.add('grid-small-grid');
  } else if (mode === 'bigGrid' || mode === 'big-grid') {
    editorContainer.classList.add('grid-big-grid');
  } else {
    editorContainer.classList.add('grid-none');
  }
}

function setSolidBars(enabled: boolean): void {
  document.body.classList.toggle('solid-bars', !!enabled);
}

function setReadOnly(readOnly: boolean): void {
  isReadOnly = readOnly;
  editor.readOnly = readOnly;
  if (readonlyBadge) {
    readonlyBadge.classList.toggle('hidden', !readOnly);
  }
  document.body.classList.toggle('readonly-mode', readOnly);
  if (readOnly) {
    showStatusMessage('Read-Only mode active (Ctrl+Shift+R to unlock)');
  } else {
    showStatusMessage('Read-Only mode disabled');
  }
}

function showStatusMessage(msg: string, durationMs = 2500): void {
  if (!syncStatus) return;
  syncStatus.textContent = msg;
  if (statusTimeout) clearTimeout(statusTimeout);
  statusTimeout = setTimeout(() => {
    if (syncStatus.textContent === msg) {
      syncStatus.textContent = '';
    }
  }, durationMs);
}

let isExecutingExtensionCommand = false;

async function executeEditorCommand(
  detected: DetectedCommand,
  lineStart: number,
  lineEnd: number
): Promise<boolean> {
  if (isExecutingExtensionCommand) return false;
  isExecutingExtensionCommand = true;

  const statusIndicator = document.getElementById('extension-status-indicator');
  const statusText = document.getElementById('extension-status-text');
  if (statusIndicator) {
    if (statusText) statusText.textContent = `Running ::${detected.commandName}…`;
    statusIndicator.classList.remove('hidden');
  }

  try {
    const fullNote = editor.value;
    const currentLineText = fullNote.substring(lineStart, lineEnd);
    const cmd = extensionRuntime.findCommand(detected.commandName);
    const cmdType = cmd?.type || 'insert';

    const result = await executeCommand(
      detected.commandName,
      detected.rawParams,
      currentLineText,
      fullNote
    );

    if (result.status === 'success') {
      const payloadStr =
        result.payload !== undefined && result.payload !== null
          ? String(result.payload)
          : '';

      if (cmdType === 'replaceAll') {
        editor.value = payloadStr;
        editor.selectionStart = editor.selectionEnd = payloadStr.length;
      } else if (cmdType === 'replaceLine') {
        const before = fullNote.substring(0, lineStart);
        const after = fullNote.substring(lineEnd);
        editor.value = before + payloadStr + after;
        editor.selectionStart = editor.selectionEnd = lineStart + payloadStr.length;
      } else if (cmdType === 'openURL') {
        if (payloadStr) {
          invoke('open_external_url', { url: payloadStr }).catch(() => {
            window.open(payloadStr, '_blank');
          });
        }
      } else {
        // 'insert': replace the invocation expression itself with payload
        const absStart = lineStart + detected.startIndex;
        const absEnd = lineStart + detected.endIndex;
        const before = fullNote.substring(0, absStart);
        const after = fullNote.substring(absEnd);
        editor.value = before + payloadStr + after;
        editor.selectionStart = editor.selectionEnd = absStart + payloadStr.length;
      }

      updateBackdrop();
      scheduleSave();
      scheduleAnalyze();
      showStatusMessage(`::${detected.commandName} completed`);
      return true;
    } else {
      showStatusMessage(result.message || `::${detected.commandName} failed`, 4000);
      return false;
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err);
    showStatusMessage(`Error executing ::${detected.commandName}: ${msg}`, 4000);
    return false;
  } finally {
    isExecutingExtensionCommand = false;
    if (statusIndicator) {
      statusIndicator.classList.add('hidden');
    }
  }
}

// ---------- Init ----------

async function init(): Promise<void> {
  const voidCanvas = document.getElementById('void-canvas') as HTMLCanvasElement | null;
  if (voidCanvas) {
    voidShader.attach(voidCanvas);
  }

  await extensionRuntime.init().catch(console.error);

  const slottedBarContainer = document.getElementById('slotted-bar');
  if (slottedBarContainer) {
    slottedBar = new SlottedNotesBottomBar(slottedBarContainer, (slotIndex) => {
      switchToSlot(slotIndex).catch(console.error);
    });
  }

  await loadNotes();
  if (notes.length === 0) {
    const slotted = await invoke<NoteMeta[]>('list_slotted_notes');
    if (slotted && slotted.length > 0) {
      await switchToSlot(0);
    } else {
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
      await loadNote(0);
    }
  } else {
    await loadNote(0);
  }
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
    if (typeof settings.inlineChecklistCheckboxesEnabled === 'boolean') {
      inlineChecklistCheckboxesEnabled = settings.inlineChecklistCheckboxesEnabled;
    } else if (typeof settings.list_checkboxes === 'boolean') {
      inlineChecklistCheckboxesEnabled = settings.list_checkboxes;
    }
    if (typeof settings.inlineChecklistBulletsEnabled === 'boolean') {
      inlineChecklistBulletsEnabled = settings.inlineChecklistBulletsEnabled;
    }
    if (typeof settings.inlineNumberedListsEnabled === 'boolean') {
      inlineNumberedListsEnabled = settings.inlineNumberedListsEnabled;
    }
    if (typeof settings.disableLinkShortening === 'boolean') {
      disableLinkShortening = settings.disableLinkShortening;
    }
    if (typeof settings.disableLinkHighlighting === 'boolean') {
      disableLinkHighlighting = settings.disableLinkHighlighting;
    }
    if (typeof settings.activateVimMode === 'boolean') {
      activateVimMode = settings.activateVimMode;
      vimEngine.setEnabled(activateVimMode);
    }
    if (typeof settings.forceRTL === 'boolean') {
      setRTLEnabled(settings.forceRTL);
    } else if (typeof settings._forceRTLStorage === 'boolean') {
      setRTLEnabled(settings._forceRTLStorage);
    }
    if (typeof settings.read_only === 'boolean') {
      setReadOnly(settings.read_only);
    }
    if (typeof settings._caretVerticalNudgeRaw === 'number') {
      caretNudgeRaw = settings._caretVerticalNudgeRaw;
      caretNudgeController.setBaseNudge(caretNudgeRaw);
    } else if (typeof settings.caret_vertical_nudge === 'number') {
      caretNudgeRaw = settings.caret_vertical_nudge;
      caretNudgeController.setBaseNudge(caretNudgeRaw);
    }
    if (typeof settings._caretVerticalNudgeForListsRaw === 'number') {
      caretNudgeForListsRaw = settings._caretVerticalNudgeForListsRaw;
      caretNudgeController.setListNudge(caretNudgeForListsRaw);
    } else if (typeof settings.caret_vertical_nudge_list === 'number') {
      caretNudgeForListsRaw = settings.caret_vertical_nudge_list;
      caretNudgeController.setListNudge(caretNudgeForListsRaw);
    }
    if (typeof settings.gridOverlayMode === 'string') {
      setGridOverlay(settings.gridOverlayMode);
    } else if (typeof settings.grid_overlay === 'string') {
      setGridOverlay(settings.grid_overlay);
    }
    if (typeof settings.solidBarsInTransparencyMode === 'boolean') {
      setSolidBars(settings.solidBarsInTransparencyMode);
    } else if (typeof settings.solid_bars === 'boolean') {
      setSolidBars(settings.solid_bars);
    } else if (typeof settings._solidBarsInTransparencyMode === 'boolean') {
      setSolidBars(settings._solidBarsInTransparencyMode);
    }
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
    } else if (p.key === 'inlineChecklistCheckboxesEnabled' || p.key === 'list_checkboxes') {
      inlineChecklistCheckboxesEnabled = !!p.value;
      scheduleAnalyze();
    } else if (p.key === 'inlineChecklistBulletsEnabled') {
      inlineChecklistBulletsEnabled = !!p.value;
    } else if (p.key === 'inlineNumberedListsEnabled') {
      inlineNumberedListsEnabled = !!p.value;
    } else if (p.key === 'disableLinkShortening') {
      disableLinkShortening = !!p.value;
      scheduleAnalyze();
    } else if (p.key === 'disableLinkHighlighting') {
      disableLinkHighlighting = !!p.value;
      scheduleAnalyze();
    } else if (p.key === 'activateVimMode') {
      activateVimMode = !!p.value;
      vimEngine.setEnabled(activateVimMode);
    } else if (p.key === 'forceRTL' || p.key === '_forceRTLStorage') {
      setRTLEnabled(!!p.value);
    } else if (p.key === 'read_only') {
      setReadOnly(!!p.value);
    } else if (p.key === '_caretVerticalNudgeRaw' || p.key === 'caret_vertical_nudge') {
      caretNudgeRaw = Number(p.value) || 0;
      caretNudgeController.setBaseNudge(caretNudgeRaw);
    } else if (p.key === '_caretVerticalNudgeForListsRaw' || p.key === 'caret_vertical_nudge_list') {
      caretNudgeForListsRaw = Number(p.value) || 0;
      caretNudgeController.setListNudge(caretNudgeForListsRaw);
    } else if (p.key === 'gridOverlayMode' || p.key === 'grid_overlay') {
      setGridOverlay(String(p.value));
    } else if (p.key === 'solidBarsInTransparencyMode' || p.key === 'solid_bars' || p.key === '_solidBarsInTransparencyMode') {
      setSolidBars(!!p.value);
    }
  });

  ['keyup', 'click', 'input', 'select'].forEach((evt) => {
    editor.addEventListener(evt, () => {
      caretNudgeController.updateCursorLine(editor.selectionStart);
    });
  });
  document.addEventListener('selectionchange', () => {
    if (document.activeElement === editor) {
      caretNudgeController.updateCursorLine(editor.selectionStart);
    }
  });

  const isPinned = await invoke<boolean>('get_always_on_top');
  if (btnAlwaysOnTop) btnAlwaysOnTop.classList.toggle('active', !!isPinned);

  // Interactive Timer Pill UI
  await initTimerUI();

  // Listen for live rate updates
  try {
    await listen('rates-updated', () => {
      scheduleAnalyze();
    });
  } catch (err) {
    console.warn('Failed to listen for rates-updated:', err);
  }

  // Load trigger configuration (REQ-12, REQ-14)
  try {
    triggerConfig = await invoke<Record<string, unknown>>('get_trigger_config');
  } catch (err) {
    console.warn('Failed to get trigger config:', err);
  }

  // AutoPaste stop button handler
  if (autopasteBtnStop) {
    autopasteBtnStop.addEventListener('click', () => {
      stopAutoPaste().catch(console.error);
    });
  }

  // Listen for incoming AutoPaste clipboard events
  try {
    await listen('autopaste-clip', (payload: unknown) => {
      const p = payload as { text?: string };
      const text = p?.text;
      if (typeof text === 'string' && text.length > 0) {
        if (editor.value.length > 0 && !editor.value.endsWith('\n')) {
          editor.value += '\n';
        }
        editor.value += text;
        editor.selectionStart = editor.selectionEnd = editor.value.length;
        editor.scrollTop = editor.scrollHeight;
        updateBackdrop();
        scheduleSave();
        scheduleAnalyze();
      }
    });
  } catch (err) {
    console.warn('Failed to listen for autopaste-clip:', err);
  }

  // Listen for tray quick menu events (REQ-26, Plan 8.1)
  try {
    await listen('menu-new-note', () => {
      createNewNote().catch(console.error);
    });
    await listen('menu-select-slot', (payload: unknown) => {
      const slotIdx = typeof payload === 'number' ? payload : Number(payload);
      if (!isNaN(slotIdx)) {
        switchToSlot(slotIdx).catch(console.error);
      }
    });
  } catch (err) {
    console.warn('Failed to listen for tray menu events:', err);
  }
}

// ---------- Note CRUD ----------

async function loadNotes(): Promise<void> {
  notes = (await invoke<NoteMeta[]>('list_notes')) || [];
}

async function switchToSlot(slotIndex: number): Promise<void> {
  if (slotIndex < 0 || slotIndex >= DEITY_SLOTS.length) return;
  await saveCurrentNote();

  const note = await invoke<Note>('get_slotted_note', { slot: slotIndex });
  if (!note) {
    console.warn(`[slotted] Slot ${slotIndex} not found`);
    return;
  }

  currentId = note.id;
  currentText = note.content ?? note.text ?? '';
  editor.value = currentText;
  currentIsSlotted = true;
  currentSlotIndex = slotIndex;
  currentPlain = false;

  updateBackdrop();
  activeAnnoCache.clear();
  overlay.innerHTML = '';
  skipKineticAnimation = true;

  if (slottedBar) {
    slottedBar.setActiveSlot(slotIndex);
  }
  updateNoteUI();
  scheduleAnalyze();
  editor.focus();
  if (activateVimMode) {
    vimEngine.onTextLoaded();
  }
  if (findAndReplaceManager && findAndReplaceManager.isOpen) {
    findAndReplaceManager.executeSearch();
  }
}

async function loadNote(index: number): Promise<void> {
  if (notes.length === 0) return;
  currentIndex = Math.max(0, Math.min(index, notes.length - 1));
  const meta = notes[currentIndex];
  currentId = meta.id;
  currentIsSlotted = meta.is_slotted;
  currentSlotIndex = meta.is_slotted ? meta.slot_index : -1;
  currentPlain = meta.plain ?? false;

  const note = await invoke<Note>('load_note', { id: currentId });
  currentText = note?.content ?? note?.text ?? '';
  editor.value = currentText;
  updateBackdrop();

  activeAnnoCache.clear();
  overlay.innerHTML = '';
  skipKineticAnimation = true;
  if (slottedBar) {
    slottedBar.setActiveSlot(meta.is_slotted ? meta.slot_index : null);
  }
  updateNoteUI();
  scheduleAnalyze();
  if (activateVimMode) {
    vimEngine.onTextLoaded();
  }
  if (findAndReplaceManager && findAndReplaceManager.isOpen) {
    findAndReplaceManager.executeSearch();
  }
}

async function saveCurrentNote(): Promise<void> {
  if (!currentId) return;
  const text = editor.value;
  if (text === currentText) return;
  currentText = text;

  // Discard empty notes ONLY IF NOT SLOTTED
  if (text.trim() === '') {
    if (currentIsSlotted) {
      // Slotted notes are permanent scratchpads and remain active even when empty!
      await invoke('save_note', { id: currentId, text });
      return;
    }

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
  if (!currentIsSlotted) {
    await loadNotes();
  }
  updateNoteUI();
}

async function createNewNote(): Promise<void> {
  await saveCurrentNote();
  const id = await invoke<string>('create_note');
  currentId = id;
  currentText = '';
  currentIsSlotted = false;
  currentSlotIndex = -1;
  editor.value = '';
  updateBackdrop();
  await loadNotes();

  // New note goes to front
  const now = new Date().toISOString();
  notes.unshift({
    id,
    title: '',
    last_modified: now,
    db_index: 0,
    is_slotted: false,
    slot_index: -1,
    is_locked: false,
    is_archived: false,
    is_private: false,
    is_tutorial: false,
    soft_deleted: false,
    deleted_at: null,
    pinned: false,
    plain: false,
  });
  currentIndex = 0;
  currentPlain = false;
  activeAnnoCache.clear();
  overlay.innerHTML = '';
  if (slottedBar) {
    slottedBar.setActiveSlot(null);
  }
  updateNoteUI();
  editor.focus();
  if (activateVimMode) {
    vimEngine.onTextLoaded();
  }
}

async function deleteCurrentNote(originX?: number, originY?: number): Promise<void> {
  if (!currentId) return;
  if (currentIsSlotted) {
    showStatusMessage('Slotted notes cannot be deleted');
    return;
  }
  // Wave distortion propagating outward from note drop/delete coordinates
  voidShader.triggerRipple(originX, originY);

  await invoke('delete_note', { id: currentId });
  await loadNotes();
  if (notes.length === 0) {
    await createNewNote();
  } else {
    await loadNote(Math.min(currentIndex, notes.length - 1));
  }
}

// ---------- The Void & Trash Lifecycle (REQ-27) ----------

interface TrashEntry {
  id: string;
  title: string;
  deleted_at_ms: number;
}

function formatRelativeTime(ms: number): string {
  if (!ms) return 'Unknown date';
  const now = Date.now();
  const diffSec = Math.max(0, Math.floor((now - ms) / 1000));
  if (diffSec < 60) return 'Deleted just now';
  const diffMin = Math.floor(diffSec / 60);
  if (diffMin < 60) return `Deleted ${diffMin}m ago`;
  const diffHours = Math.floor(diffMin / 60);
  if (diffHours < 24) return `Deleted ${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 30) return `Deleted ${diffDays}d ago`;
  return new Date(ms).toLocaleDateString();
}

async function refreshVoidList(): Promise<void> {
  if (!voidNoteList) return;
  voidNoteList.innerHTML = '';

  let entries: TrashEntry[] = [];
  try {
    entries = await invoke<TrashEntry[]>('list_trash');
  } catch (err) {
    console.error('Failed to list trash:', err);
  }

  if (entries.length === 0) {
    const emptyEl = document.createElement('div');
    emptyEl.className = 'void-empty-state';
    emptyEl.innerHTML = `
      <svg width="32" height="32" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="8" cy="8" r="6.5"/>
        <circle cx="8" cy="8" r="3.2"/>
        <circle cx="8" cy="8" r="1.2" fill="currentColor"/>
      </svg>
      <div class="void-empty-title">The Void is tranquil and empty</div>
      <div class="void-empty-desc">Deleted notes will appear here in gravitational stasis.</div>
    `;
    voidNoteList.appendChild(emptyEl);
    return;
  }

  for (const entry of entries) {
    const item = document.createElement('div');
    item.className = 'void-note-item';

    const meta = document.createElement('div');
    meta.className = 'void-note-meta';

    const titleEl = document.createElement('div');
    titleEl.className = 'void-note-title';
    titleEl.textContent = entry.title || 'Untitled Note';

    const dateEl = document.createElement('div');
    dateEl.className = 'void-note-date';
    dateEl.textContent = formatRelativeTime(entry.deleted_at_ms);

    meta.appendChild(titleEl);
    meta.appendChild(dateEl);

    const actions = document.createElement('div');
    actions.className = 'void-note-actions';

    const btnRestore = document.createElement('button');
    btnRestore.className = 'btn-restore-note';
    btnRestore.textContent = 'Restore';
    btnRestore.title = 'Restore note back to active notes';
    btnRestore.addEventListener('click', async (e) => {
      e.stopPropagation();
      try {
        await invoke('restore_note', { id: entry.id });
        showStatusMessage('Note restored from The Void');
        await loadNotes();
        await refreshVoidList();
      } catch (err) {
        showStatusMessage(`Failed to restore note: ${err}`);
      }
    });

    const btnPurge = document.createElement('button');
    btnPurge.className = 'btn-purge-note';
    btnPurge.textContent = 'Purge';
    btnPurge.title = 'Permanently delete from The Void';
    btnPurge.addEventListener('click', async (e) => {
      e.stopPropagation();
      const confirmed = window.confirm(
        `Permanently delete "${entry.title || 'Untitled Note'}" from The Void? This cannot be undone.`
      );
      if (confirmed) {
        try {
          await invoke('permanently_delete_note', { id: entry.id });
          showStatusMessage('Note permanently purged');
          await refreshVoidList();
        } catch (err) {
          showStatusMessage(`Failed to purge note: ${err}`);
        }
      }
    });

    actions.appendChild(btnRestore);
    actions.appendChild(btnPurge);

    item.appendChild(meta);
    item.appendChild(actions);
    voidNoteList.appendChild(item);
  }
}

function openVoidModal(): void {
  if (!voidModal) return;
  voidModal.classList.remove('hidden');
  voidShader.startAmbient();
  refreshVoidList().catch(console.error);
}

function closeVoidModal(): void {
  if (!voidModal) return;
  voidModal.classList.add('hidden');
  if (voidConfirmDialog) {
    voidConfirmDialog.classList.add('hidden');
  }
  voidShader.stopAmbient();
}

async function handleEmptyVoidClick(): Promise<void> {
  let entries: TrashEntry[] = [];
  try {
    entries = await invoke<TrashEntry[]>('list_trash');
  } catch (err) {
    console.error('Failed to list trash:', err);
  }

  if (entries.length === 0) {
    showStatusMessage('The Void is already empty');
    return;
  }

  if (voidConfirmMsg) {
    voidConfirmMsg.textContent = `This will permanently delete ${entries.length} notes. There are no turtles beyond The Void.`;
  }
  if (voidConfirmDialog) {
    voidConfirmDialog.classList.remove('hidden');
  }
}

async function confirmEmptyVoid(): Promise<void> {
  if (voidConfirmDialog) {
    voidConfirmDialog.classList.add('hidden');
  }
  // High-energy outward shockwave followed by collapse on Empty The Void
  voidShader.triggerBurst();

  try {
    const count = await invoke<number>('purge_void');
    showStatusMessage(`The Void purged (${count} notes obliterated)`);
  } catch (err) {
    showStatusMessage(`Purge error: ${err}`);
  }

  await refreshVoidList();
  await loadNotes();
}

// ---------- Navigation ----------

async function prevNote(): Promise<void> {
  if (currentIsSlotted) {
    if (notes.length > 0) {
      await saveCurrentNote();
      await loadNote(currentIndex);
    }
    return;
  }
  if (currentIndex > 0) {
    await saveCurrentNote();
    await loadNote(currentIndex - 1);
  }
}

async function nextNote(): Promise<void> {
  if (currentIsSlotted) {
    if (notes.length > 0) {
      await saveCurrentNote();
      await loadNote(currentIndex);
    } else {
      await createNewNote();
    }
    return;
  }
  if (currentIndex < notes.length - 1) {
    await saveCurrentNote();
    await loadNote(currentIndex + 1);
  } else {
    // Past the newest note → create new
    await createNewNote();
  }
}

function updateNoteUI(): void {
  if (currentIsSlotted && currentSlotIndex >= 0 && currentSlotIndex < DEITY_SLOTS.length) {
    noteIndex.textContent = `${DEITY_SLOTS[currentSlotIndex].name} (${currentSlotIndex + 1}/9)`;
    btnPin.classList.remove('active');
    btnPin.classList.add('disabled');
    btnPin.style.opacity = '0.3';
    btnPin.style.pointerEvents = 'none';
    btnPin.title = 'Slotted notes have a fixed position';
    btnPlain.classList.toggle('active', currentPlain || globalPlain);
  } else {
    btnPin.classList.remove('disabled');
    btnPin.style.opacity = '';
    btnPin.style.pointerEvents = '';
    btnPin.title = 'Pin note to top of list';

    const idx = notes.findIndex((n) => n.id === currentId);
    if (idx >= 0) currentIndex = idx;
    noteIndex.textContent = notes.length > 0 ? `${currentIndex + 1}/${notes.length}` : '0/0';

    const meta = notes[currentIndex];
    if (meta) {
      btnPin.classList.toggle('active', !!meta.pinned);
      btnPlain.classList.toggle('active', currentPlain || globalPlain);
    }
  }
}

// ---------- Analysis & Overlay ----------

function scheduleAnalyze(): void {
  if (analyzeTimeout) clearTimeout(analyzeTimeout);
  analyzeTimeout = setTimeout(() => runAnalysis(), 35);
}

let lastAnalysis: Analysis | null = null;

async function runAnalysis(): Promise<void> {
  const text = editor.value;
  const plain = currentPlain || globalPlain;
  const analysis = await invoke<Analysis>('analyze_note', { text, plain });
  lastAnalysis = analysis || null;
  if (!analysis) {
    for (const child of Array.from(overlay.children) as HTMLElement[]) {
      triggerExit(child);
    }
    activeAnnoCache.clear();
    if (!isAutoPasteActive && autopasteIndicator) {
      autopasteIndicator.classList.add('autopaste-hidden');
    }
    return;
  }

  renderOverlay(analysis, text);

  // AutoPaste indicator proposed banner handling
  const hasPasteChip = analysis.annotations?.some((a) => a.kind === 'paste');
  if (hasPasteChip) {
    if (!isAutoPasteActive && autopasteIndicator) {
      autopasteIndicator.classList.remove('autopaste-hidden');
      autopasteIndicator.classList.remove('active');
      if (autopasteStatusText) {
        autopasteStatusText.textContent = AUTOPASTE_PROPOSED_MSG;
      }
    }
  } else {
    if (!isAutoPasteActive && autopasteIndicator) {
      autopasteIndicator.classList.add('autopaste-hidden');
    }
  }

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

  let inFencedCode = false;
  let lineOffset = 0;

  const htmlLines = lines.map((line, idx) => {
    const startOffset = lineOffset;
    lineOffset += line.length + 1; // +1 for '\n'

    const formatText = (t: string, off: number) => {
      if (typeof findAndReplaceManager !== 'undefined' && findAndReplaceManager?.isSearching()) {
        return findAndReplaceManager.formatLineText(t, off);
      }
      return escapeHtml(t);
    };

    const trimmed = line.trim();
    const lower = trimmed.toLowerCase();

    // Fenced code blocks
    if (trimmed.startsWith('```')) {
      if (!inFencedCode) {
        inFencedCode = true;
        const lang = trimmed.substring(3).trim();
        return `<span class="hl-code-fence">\`\`\`</span><span class="hl-code-lang">${escapeHtml(lang)}</span>`;
      } else {
        inFencedCode = false;
        return `<span class="hl-code-fence">\`\`\`</span>`;
      }
    }

    if (inFencedCode) {
      return `<span class="hl-code-line">${formatText(line, startOffset)}</span>`;
    }

    // Checked checklist item dimming
    const cbInfo = parseCheckboxLine(line);
    if (cbInfo && cbInfo.isChecked) {
      return `<span class="checklist-item-checked">${formatText(line, startOffset)}</span>`;
    }

    // Mode keyword on first non-empty line turns green/colored immediately
    if (idx === firstNonEmpty) {
      if (lower === 'math') {
        return `<span class="hl-mode hl-mode-math">${formatText(line, startOffset)}</span>`;
      } else if (lower === 'calcs') {
        return `<span class="hl-mode hl-mode-calcs">${formatText(line, startOffset)}</span>`;
      } else if (lower === 'sum') {
        return `<span class="hl-mode hl-mode-sum">${formatText(line, startOffset)}</span>`;
      } else if (lower === 'list') {
        return `<span class="hl-mode hl-mode-list">${formatText(line, startOffset)}</span>`;
      } else if (lower === 'paste') {
        return `<span class="hl-mode hl-mode-paste">${formatText(line, startOffset)}</span>`;
      } else if (lower === 'code' || lower.startsWith('code:')) {
        return `<span class="hl-mode hl-mode-code">${formatText(line, startOffset)}</span>`;
      }
    }

    // Comment lines (// ...)
    if (trimmed.startsWith('//')) {
      return `<span class="hl-comment">${formatText(line, startOffset)}</span>`;
    }

    // Natural-language trigger keywords anywhere in note
    const triggerKws = [
      'sum', 'total', 'add', 'avg', 'average', 'math', 'calcs',
      'convert', 'count', 'list', 'check', 'todo', 'code', 'paste', 'timer',
    ];
    const triggerMatch = line.match(/^(\s*)([a-zA-Z]+)(.*)$/);
    if (triggerMatch) {
      const leading = triggerMatch[1];
      const word = triggerMatch[2];
      const rest = triggerMatch[3];
      const wordLower = word.toLowerCase();
      if (triggerKws.includes(wordLower)) {
        const wordOffset = startOffset + leading.length;
        const restOffset = wordOffset + word.length;
        return `${escapeHtml(leading)}<span class="hl-mode hl-mode-trigger hl-mode-${wordLower}">${formatText(word, wordOffset)}</span>${formatText(rest, restOffset)}`;
      }
    }

    // Variable / label lines: e.g. "trees: 23,405" or "rent = 12000"
    const colonIdx = line.indexOf(':');
    const eqIdx = line.indexOf('=');
    if (colonIdx > 0 && (eqIdx < 0 || colonIdx < eqIdx)) {
      const label = line.substring(0, colonIdx);
      if (label.trim().length > 0 && label.trim().split(/\s+/).length <= 4) {
        const rest = line.substring(colonIdx + 1);
        return `<span class="hl-var">${formatText(label, startOffset)}</span><span class="hl-punct">:</span>${formatText(rest, startOffset + colonIdx + 1)}`;
      }
    }

    return formatText(line, startOffset);
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

function getEditorPadX(): number {
  if (!editor) return 24;
  const pl = parseFloat(window.getComputedStyle(editor).paddingLeft);
  return Number.isFinite(pl) && pl > 0 ? pl : 24;
}

function getCalculationGhost(rawLine: string): { prefix: string; hasTrailingEquals: boolean } {
  // 1. User typed '=' or '?' followed by one or more spaces (e.g. "200 inr to cad = " or "200 inr to cad= ")
  if (/\s*[=?]\s+$/.test(rawLine)) {
    return { prefix: '', hasTrailingEquals: true };
  }
  // 2. User typed '=' or '?' without trailing spaces (e.g. "200 inr to cad =" or "200 inr to cad=")
  if (/\s*[=?]$/.test(rawLine)) {
    return { prefix: ' ', hasTrailingEquals: true };
  }
  // 3. User typed one or more trailing spaces after expression (e.g. "200 inr to cad ")
  if (/\s+$/.test(rawLine)) {
    return { prefix: '= ', hasTrailingEquals: false };
  }
  // 4. Default: raw expression without trailing space or equals (e.g. "200 inr to cad")
  return { prefix: ' = ', hasTrailingEquals: false };
}

function syncActiveCalculationGhost(): void {
  if (!lastAnalysis?.annotations) return;
  const cursor = editor.selectionStart;
  const text = editor.value;
  const cursorLine = text.substring(0, cursor).split('\n').length - 1;
  const anno = lastAnalysis.annotations.find(
    (a) => a.line === cursorLine && (a.kind === 'math' || a.kind === 'conversion') && a.value
  );
  if (!anno) return;

  const annoKey = `${cursorLine}:${anno.kind}`;
  const el = overlay.querySelector(`[data-anno-key="${annoKey}"]`) as HTMLElement | null;
  if (!el || el.classList.contains('kinetic-exiting')) return;

  const lines = text.split('\n');
  const rawLine = lines[cursorLine] || '';
  const lineText = rawLine.trim();
  if (lineText.length === 0) return;

  const calcGhost = getCalculationGhost(rawLine);
  el.classList.toggle('has-equals', calcGhost.hasTrailingEquals);

  const prefixEl = el.querySelector('.anno-prefix') as HTMLElement | null;
  if (prefixEl) {
    prefixEl.textContent = calcGhost.prefix;
    prefixEl.style.display = calcGhost.prefix ? 'inline-block' : 'none';
  }

  const padX = getEditorPadX();
  const tempSpan = document.createElement('span');
  tempSpan.textContent = rawLine || ' ';
  tempSpan.style.display = 'inline-block';
  tempSpan.style.whiteSpace = 'pre';
  mirror.appendChild(tempSpan);
  const textWidth = tempSpan.offsetWidth;
  tempSpan.remove();

  const maxOffset = Math.max(padX, editor.clientWidth - 130);
  const offset = Math.min(padX + textWidth, maxOffset);

  if (forceRTL) {
    el.style.right = `${offset}px`;
    el.style.left = '';
  } else {
    el.style.left = `${offset}px`;
    el.style.right = '';
  }
}

function renderKineticContent(parent: HTMLElement, prefix: string, text: string, isNew: boolean): void {
  if (!isNew) {
    const p = document.createElement('span');
    p.className = 'anno-prefix';
    p.textContent = prefix;
    if (!prefix) {
      p.style.display = 'none';
    }
    parent.appendChild(p);
    parent.appendChild(document.createTextNode(text));
    return;
  }

  const container = document.createElement('span');
  container.className = 'kinetic-container';
  let wordIndex = 0;

  const p = document.createElement('span');
  p.className = 'kinetic-word anno-prefix';
  p.style.setProperty('--word-index', String(wordIndex++));
  p.textContent = prefix;
  if (!prefix) {
    p.style.display = 'none';
  }
  container.appendChild(p);

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
  const padX = getEditorPadX();

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
    span.style.whiteSpace = 'pre';
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

    // Inline horizontal position: placed right after typed text with clean spacing
    const isCalculation = anno.kind === 'math' || anno.kind === 'conversion';
    const textWidth = lineText.length > 0 ? textSpan.offsetWidth : 0;
    const maxOffset = Math.max(padX, editor.clientWidth - 130);
    const gap = isCalculation ? 0 : 10;
    const offset = Math.min(padX + textWidth + gap, maxOffset);

    const applyPos = (targetEl: HTMLElement) => {
      if (forceRTL) {
        targetEl.style.right = `${offset}px`;
        targetEl.style.left = '';
      } else {
        targetEl.style.left = `${offset}px`;
        targetEl.style.right = '';
      }
    };

    const annoKey = `${anno.line}:${anno.kind}`;
    const annoValue = String(anno.value || anno.chip || '');
    const isNew = !skipKineticAnimation && (activeAnnoCache.get(annoKey) !== annoValue);
    nextAnnoCache.set(annoKey, annoValue);

    const prevEl = existingEls.get(annoKey);

    if (!isNew && prevEl) {
      // Unchanged: update layout position in case typing on previous lines shifted vertical line offsets
      prevEl.style.top = `${anno.kind === 'checklist' ? top + 2 : top}px`;
      if (anno.kind !== 'checklist') {
        applyPos(prevEl);
      } else {
        const cbInfo = parseCheckboxLine(rawLine);
        if (cbInfo) {
          const prefix = rawLine.substring(0, cbInfo.checkOffset);
          let checkOffset = padX;
          if (prefix.length > 0) {
            const tempSpan = document.createElement('span');
            tempSpan.textContent = prefix;
            tempSpan.style.display = 'inline-block';
            mirror.appendChild(tempSpan);
            checkOffset = padX + tempSpan.offsetWidth;
            tempSpan.remove();
          }
          if (forceRTL) {
            prevEl.style.right = `${checkOffset}px`;
            prevEl.style.left = '';
          } else {
            prevEl.style.left = `${checkOffset}px`;
            prevEl.style.right = '';
          }
        }
      }

      if (isCalculation) {
        const calcGhost = getCalculationGhost(rawLine);
        prevEl.classList.toggle('has-equals', calcGhost.hasTrailingEquals);
        const prefixEl = prevEl.querySelector('.anno-prefix') as HTMLElement | null;
        if (prefixEl) {
          prefixEl.textContent = calcGhost.prefix;
          prefixEl.style.display = calcGhost.prefix ? 'inline-block' : 'none';
        }
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
      const calcGhost = getCalculationGhost(rawLine);
      el.className = `anno anno-math${calcGhost.hasTrailingEquals ? ' has-equals' : ''}`;
      el.style.top = `${top}px`;
      applyPos(el);
      const formatted = formatNumber(anno.value);
      renderKineticContent(el, calcGhost.prefix, formatted, isNew);
      el.title = `${anno.why} (Click to copy answer)`;
      el.addEventListener('click', (e) => {
        e.stopPropagation();
        navigator.clipboard.writeText(formatted).catch(() => {});
      });
    } else if (anno.kind === 'conversion' && anno.confidence >= 0.85 && anno.value) {
      el = document.createElement('div');
      const calcGhost = getCalculationGhost(rawLine);
      el.className = `anno anno-conversion${calcGhost.hasTrailingEquals ? ' has-equals' : ''}`;
      el.style.top = `${top}px`;
      applyPos(el);
      renderKineticContent(el, calcGhost.prefix, anno.value, isNew);
      el.title = `${anno.why} (Click to copy answer)`;
      el.addEventListener('click', (e) => {
        e.stopPropagation();
        navigator.clipboard.writeText(anno.value).catch(() => {});
      });
    }
 else if (anno.kind === 'timer') {
      el = document.createElement('div');
      el.className = `anno anno-timer-chip${isNew ? ' kinetic-reveal-chip' : ''}`;
      el.style.top = `${top}px`;
      applyPos(el);
      const clockSvg = `<svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" style="vertical-align: -1px; margin-right: 5px;"><circle cx="8" cy="8" r="6"/><path d="M8 4.5v3.5l2 2"/></svg>`;
      const iconSpan = document.createElement('span');
      iconSpan.innerHTML = clockSvg;
      el.appendChild(iconSpan);
      const labelText = anno.chip || `Start ${anno.value} (Enter)`;
      el.appendChild(document.createTextNode(labelText));
      el.title = `Click or press Enter: ${labelText}`;
      el.addEventListener('click', () => {
        invoke('start_timer_phrase', { phrase: lineText }).catch(console.error);
      });
    } else if (anno.kind === 'timer_control') {
      el = document.createElement('div');
      el.className = `anno anno-timer-chip${isNew ? ' kinetic-reveal-chip' : ''}`;
      el.style.top = `${top}px`;
      applyPos(el);
      const isPause = anno.value === 'pause';
      const iconSvg = isPause
        ? `<svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" style="vertical-align: -1px; margin-right: 5px;"><path d="M4 3.5h3v9H4zm5 0h3v9H9z"/></svg>`
        : `<svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" style="vertical-align: -1px; margin-right: 5px;"><path d="M2.5 8a5.5 5.5 0 1 1 1.6 3.9"/><path d="M2.5 12V8h4"/></svg>`;
      const iconSpan = document.createElement('span');
      iconSpan.innerHTML = iconSvg;
      el.appendChild(iconSpan);
      const labelText = anno.chip || (isPause ? 'Pause / Resume (Enter)' : 'Restart timer (Enter)');
      el.appendChild(document.createTextNode(labelText));
      el.title = `Click or press Enter: ${labelText}`;
      el.addEventListener('click', () => {
        if (isPause) {
          invoke('pause_timer').catch(console.error);
        } else {
          invoke('restart_timer').catch(console.error);
        }
      });
    } else if (anno.kind === 'timer_stop') {
      el = document.createElement('div');
      el.className = `anno anno-timer-chip stop${isNew ? ' kinetic-reveal-chip' : ''}`;
      el.style.top = `${top}px`;
      applyPos(el);
      const stopSvg = `<svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" style="vertical-align: -1px; margin-right: 5px;"><rect x="3" y="3" width="10" height="10" rx="1.5"/></svg>`;
      const iconSpan = document.createElement('span');
      iconSpan.innerHTML = stopSvg;
      el.appendChild(iconSpan);
      const labelText = anno.chip || 'Stop timer (Enter)';
      el.appendChild(document.createTextNode(labelText));
      el.title = `Click or press Enter: ${labelText}`;
      el.addEventListener('click', () => {
        clearTimer();
        invoke('cancel_timer').catch(console.error);
      });
    } else if (anno.kind === 'checklist') {
      const cbInfo = parseCheckboxLine(rawLine);
      if (inlineChecklistCheckboxesEnabled && cbInfo) {
        el = document.createElement('div');
        el.className = `anno-checkbox${cbInfo.isChecked ? ' checked' : ''}`;
        el.style.top = `${top + 3}px`;
        const prefix = rawLine.substring(0, cbInfo.checkOffset);
        let checkOffset = padX;
        if (prefix.length > 0) {
          const tempSpan = document.createElement('span');
          tempSpan.textContent = prefix;
          tempSpan.style.display = 'inline-block';
          mirror.appendChild(tempSpan);
          checkOffset = padX + tempSpan.offsetWidth;
          tempSpan.remove();
        }
        if (forceRTL) {
          el.style.right = `${checkOffset}px`;
          el.style.left = '';
        } else {
          el.style.left = `${checkOffset}px`;
          el.style.right = '';
        }
        if (cbInfo.isChecked) {
          el.innerHTML = `<svg width="9" height="9" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 6.5l2.5 2.5 5-5"/></svg>`;
        }
        el.dataset.line = String(anno.line);
        el.addEventListener('click', (e) => {
          e.stopPropagation();
          toggleCheckbox(anno.line);
        });
      } else if (anno.chip) {
        el = document.createElement('div');
        el.className = `anno anno-chip${isNew ? ' kinetic-reveal-chip' : ''}`;
        el.style.top = `${top}px`;
        applyPos(el);
        el.dataset.line = String(anno.line);
        el.textContent = anno.chip;
        el.title = anno.why;
        el.addEventListener('click', () => acceptChip(anno.line, lines[anno.line] || ''));
      }
    }

    if (el) {
      el.dataset.annoKey = annoKey;
      overlay.appendChild(el);
    }
  }

  // Smart Antilink pills
  if (!disableLinkHighlighting) {
    const antilinks = findAntilinks(text, disableLinkShortening);
    for (let alIdx = 0; alIdx < antilinks.length; alIdx++) {
      const al = antilinks[alIdx];
      if (al.line >= lineEls.length) continue;
      const lineEl = lineEls[al.line];
      if (!lineEl) continue;
      const top = lineEl.offsetTop;
      const lineStr = lines[al.line] || '';
      const prefixStr = lineStr.substring(0, al.lineStartIndex);

      let alLeft = padX;
      if (prefixStr.length > 0) {
        const tempSpan = document.createElement('span');
        tempSpan.textContent = prefixStr;
        tempSpan.style.display = 'inline-block';
        mirror.appendChild(tempSpan);
        alLeft = padX + tempSpan.offsetWidth;
        tempSpan.remove();
      }

      const annoKey = `antilink:${al.line}:${alIdx}:${al.fullUrl}`;
      const prevEl = existingEls.get(annoKey);
      if (prevEl) {
        prevEl.style.top = `${top + 1}px`;
        if (forceRTL) {
          prevEl.style.right = `${alLeft}px`;
          prevEl.style.left = '';
        } else {
          prevEl.style.left = `${alLeft}px`;
          prevEl.style.right = '';
        }
        existingEls.delete(annoKey);
        continue;
      }

      const pill = createAntilinkElement(al, top + 1, alLeft);
      if (forceRTL) {
        pill.style.right = `${alLeft}px`;
        pill.style.left = '';
      }
      pill.dataset.annoKey = annoKey;
      overlay.appendChild(pill);
    }
  }

  // Rounded code blocks with Copy button
  const codeBlocks = findCodeBlocks(text);
  for (let cbIdx = 0; cbIdx < codeBlocks.length; cbIdx++) {
    const cb = codeBlocks[cbIdx];
    if (cb.startLine >= lineEls.length) continue;
    const startLineEl = lineEls[cb.startLine];
    const endLineIdx = Math.min(cb.endLine, lineEls.length - 1);
    const endLineEl = lineEls[endLineIdx];
    if (!startLineEl || !endLineEl) continue;

    const top = startLineEl.offsetTop;
    const height = (endLineEl.offsetTop + endLineEl.offsetHeight) - top;

    // Header element with Language badge and Copy Code button
    const headerKey = `codeblock-header:${cb.startLine}:${cbIdx}`;
    const prevHeader = existingEls.get(headerKey);
    if (prevHeader) {
      prevHeader.style.top = `${top}px`;
      existingEls.delete(headerKey);
    } else {
      const headerEl = createCodeBlockHeaderElement(cb, top, 0);
      headerEl.dataset.annoKey = headerKey;
      overlay.appendChild(headerEl);
    }

    // Rounded background container
    const bgKey = `codeblock-bg:${cb.startLine}:${cbIdx}`;
    const prevBg = existingEls.get(bgKey);
    if (prevBg) {
      prevBg.style.top = `${top}px`;
      prevBg.style.height = `${height}px`;
      existingEls.delete(bgKey);
    } else {
      const bgEl = document.createElement('div');
      bgEl.className = 'rounded-code-block';
      bgEl.style.top = `${top}px`;
      bgEl.style.height = `${height}px`;
      bgEl.dataset.annoKey = bgKey;
      overlay.appendChild(bgEl);
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
  syncOverlaySelection();
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
  const selStart = editor.selectionStart;
  const selEnd = editor.selectionEnd;
  const res = toggleCheckboxLine(editor.value, lineIndex);
  editor.value = res.nextText;
  editor.selectionStart = selStart;
  editor.selectionEnd = selEnd;
  updateBackdrop();
  scheduleSave();
  scheduleAnalyze();
}

// ---------- Accept chip (Tab on suggestion) ----------

async function startAutoPaste(): Promise<void> {
  if (isAutoPasteActive) return;
  isAutoPasteActive = true;
  if (autopasteIndicator) {
    autopasteIndicator.classList.remove('autopaste-hidden');
    autopasteIndicator.classList.add('active');
  }
  if (autopasteStatusText) {
    autopasteStatusText.textContent = AUTOPASTE_ACTIVE_MSG;
  }

  // Clear trigger keyword line if cursor is on it
  const cursorLine = editor.value.substring(0, editor.selectionStart).split('\n').length - 1;
  const lines = editor.value.split('\n');
  if (cursorLine >= 0 && cursorLine < lines.length) {
    const curLineTrimmed = lines[cursorLine].trim().toLowerCase();
    const pasteAliases = (triggerConfig?.paste_triggers as string[])?.map((t: string) => t.toLowerCase()) || ['paste'];
    if (pasteAliases.includes(curLineTrimmed)) {
      lines.splice(cursorLine, 1);
      editor.value = lines.join('\n');
      updateBackdrop();
      scheduleSave();
      scheduleAnalyze();
    }
  }

  try {
    await invoke('start_autopaste');
  } catch (err) {
    console.warn('[AutoPaste] Backend start_autopaste error:', err);
  }

  // Fallback poller in webview via navigator.clipboard
  if (autoPastePollTimer !== null) {
    clearInterval(autoPastePollTimer);
  }
  autoPastePollTimer = window.setInterval(async () => {
    if (!isAutoPasteActive) return;
    try {
      if (navigator.clipboard && typeof navigator.clipboard.readText === 'function') {
        const text = await navigator.clipboard.readText();
        if (text && text.trim().length > 0) {
          await invoke('submit_clipboard_clip', { text });
        }
      }
    } catch {
      // Ignore clipboard read permission or empty errors
    }
  }, 400);
}

async function stopAutoPaste(): Promise<void> {
  isAutoPasteActive = false;
  if (autoPastePollTimer !== null) {
    clearInterval(autoPastePollTimer);
    autoPastePollTimer = null;
  }
  if (autopasteIndicator) {
    autopasteIndicator.classList.remove('active');
    autopasteIndicator.classList.add('autopaste-hidden');
  }
  if (autopasteStatusText) {
    autopasteStatusText.textContent = AUTOPASTE_PROPOSED_MSG;
  }
  try {
    await invoke('stop_autopaste');
  } catch (err) {
    console.warn('[AutoPaste] Backend stop_autopaste error:', err);
  }
}

function acceptChip(lineIndex: number, text: string): void {
  const lines = editor.value.split('\n');
  if (lineIndex >= lines.length) return;
  const line = lines[lineIndex];
  const trimmed = line.trim();
  const lower = trimmed.toLowerCase();

  const anno = lastAnalysis?.annotations?.find((a) => a.line === lineIndex);
  const kind = anno?.kind || '';

  // 1. Timer triggers
  if (kind === 'timer' || lower.startsWith('timer') || lower.startsWith('remind')) {
    invoke('start_timer_phrase', { phrase: trimmed }).catch(console.error);
    return;
  }
  if (kind === 'timer_control') {
    if (anno?.value === 'pause' || lower.includes('pause') || lower.includes('resume')) {
      invoke('pause_timer').catch(console.error);
    } else if (anno?.value === 'restart' || lower.includes('restart')) {
      invoke('restart_timer').catch(console.error);
    } else {
      invoke('start_timer_phrase', { phrase: trimmed }).catch(console.error);
    }
    return;
  }
  if (kind === 'timer_stop' || lower === 'stop timer' || lower === 'timer s' || lower === 'timer stop' || lower === 'timer cancel') {
    clearTimer();
    invoke('cancel_timer').catch(console.error);
    return;
  }

  // 2. Paste / AutoPaste trigger
  if (kind === 'paste' || lower === 'paste') {
    startAutoPaste().catch(console.error);
    return;
  }

  // Helper to extract preceding non-empty paragraph block lines
  const getPrecedingBlockRange = (): { start: number; end: number } => {
    let start = lineIndex;
    for (let j = lineIndex - 1; j >= 0; j--) {
      const l = lines[j].trim();
      if (!l || l.startsWith('#')) break;
      start = j;
    }
    return { start, end: lineIndex - 1 };
  };

  // 3. List chip: convert preceding lines of the paragraph block into bullet list items (- <item>)
  if (kind === 'list' || lower === 'list') {
    const { start, end } = getPrecedingBlockRange();
    if (end >= start) {
      for (let j = start; j <= end; j++) {
        const item = lines[j].trimStart();
        if (item.startsWith('- ')) continue;
        if (item.startsWith('* ')) {
          lines[j] = '- ' + item.substring(2);
        } else {
          lines[j] = '- ' + item;
        }
      }
      lines.splice(lineIndex, 1);
    } else {
      lines[lineIndex] = '- ';
    }
    editor.value = lines.join('\n');
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // 4. Checklist chip: convert preceding lines of the paragraph block into checkboxes (- [ ] <item>)
  if (kind === 'checklist' || lower === 'check' || lower === 'todo') {
    const { start, end } = getPrecedingBlockRange();
    if (end >= start) {
      for (let j = start; j <= end; j++) {
        const item = lines[j].trimStart();
        if (item.startsWith('- [ ] ') || item.startsWith('[ ] ') || item.startsWith('- [x] ') || item.startsWith('[x] ')) {
          continue;
        }
        lines[j] = '- [ ] ' + item.replace(/^[-*]\s+/, '');
      }
      lines.splice(lineIndex, 1);
    } else if (text.toLowerCase().startsWith('todo ')) {
      lines[lineIndex] = '- [ ] ' + line.substring(line.toLowerCase().indexOf('todo ') + 5);
    } else if (line.includes(',')) {
      const items = line.split(',').map((s) => s.trim()).filter(Boolean);
      lines.splice(lineIndex, 1, ...items.map((item) => `- [ ] ${item}`));
    } else {
      lines[lineIndex] = '- [ ] ';
    }
    editor.value = lines.join('\n');
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // 5. Code chip: format target lines into fenced code block ```<lang>\n...\n```
  if (kind === 'code' || lower.startsWith('code')) {
    const lang = anno?.value?.trim() || lower.replace(/^code:?/, '').trim();
    const { start, end } = getPrecedingBlockRange();
    if (end >= start) {
      const blockCode = lines.slice(start, end + 1).join('\n');
      const fenced = `\`\`\`${lang}\n${blockCode}\n\`\`\``;
      lines.splice(start, end - start + 2, fenced);
    } else {
      lines[lineIndex] = `\`\`\`${lang}\n\n\`\`\``;
    }
    editor.value = lines.join('\n');
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // 6. Math / sum / avg / count evaluation chip: append evaluated result onto line
  if (anno && anno.value) {
    if (line.trim().endsWith('=')) {
      lines[lineIndex] = `${line} ${anno.value}`;
    } else if (line.trim().endsWith(':')) {
      lines[lineIndex] = `${line} ${anno.value}`;
    } else if (lower === 'sum' || lower === 'total' || lower === 'add' || lower === 'avg' || lower === 'average' || lower === 'count') {
      lines[lineIndex] = `${line}: ${anno.value}`;
    } else {
      lines[lineIndex] = `${line} = ${anno.value}`;
    }
    editor.value = lines.join('\n');
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // Fallback for comma list or todo
  if (text.toLowerCase().startsWith('todo ')) {
    lines[lineIndex] = '- [ ] ' + line.substring(line.toLowerCase().indexOf('todo ') + 5);
  } else if (line.includes(',')) {
    const items = line.split(',').map((s) => s.trim()).filter(Boolean);
    lines.splice(lineIndex, 1, ...items.map((item) => `- [ ] ${item}`));
  }

  editor.value = lines.join('\n');
  updateBackdrop();
  scheduleSave();
  scheduleAnalyze();
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

// ---------- Rich Copy and Selection Synchronization ----------

function syncOverlaySelection(): void {
  const selStart = editor.selectionStart;
  const selEnd = editor.selectionEnd;
  const isSelected = selStart !== selEnd;

  for (const el of Array.from(overlay.querySelectorAll('.anno.is-selected'))) {
    el.classList.remove('is-selected');
  }

  if (!isSelected || !lastAnalysis?.annotations) {
    return;
  }

  const text = editor.value;
  const lines = text.split('\n');
  let charCount = 0;

  for (let i = 0; i < lines.length; i++) {
    const lineStart = charCount;
    const lineLen = lines[i].length;
    const lineEnd = lineStart + lineLen;
    charCount = lineEnd + 1; // +1 for newline

    // If selection includes the end of this line's typed content
    if (selEnd >= lineEnd && selStart <= lineEnd && lineLen > 0) {
      const anno = lastAnalysis.annotations.find(
        (a) => a.line === i && (a.kind === 'math' || a.kind === 'conversion') && a.value
      );
      if (anno) {
        const annoEl = overlay.querySelector(`[data-anno-key^="${i}:${anno.kind}"]`) as HTMLElement | null;
        if (annoEl) {
          annoEl.classList.add('is-selected');
        }
      }
    }
  }
}

editor.addEventListener('copy', (e: ClipboardEvent) => {
  const selStart = editor.selectionStart;
  const selEnd = editor.selectionEnd;
  if (selStart === selEnd || !e.clipboardData) return;

  if (!lastAnalysis?.annotations || lastAnalysis.annotations.length === 0) {
    return;
  }

  const text = editor.value;
  const lines = text.split('\n');
  let charCount = 0;
  let hasEnhancedAnyLine = false;
  const enhancedLines: string[] = [];

  for (let i = 0; i < lines.length; i++) {
    const lineStart = charCount;
    const lineLen = lines[i].length;
    const lineEnd = lineStart + lineLen;
    charCount = lineEnd + 1; // +1 for newline

    // Skip lines outside selection
    if (lineEnd < selStart || lineStart > selEnd) {
      continue;
    }

    const lineSliceStart = Math.max(0, selStart - lineStart);
    const lineSliceEnd = Math.min(lineLen, selEnd - lineStart);
    let lineSlice = lines[i].substring(lineSliceStart, lineSliceEnd);

    // If selection spans to the end of the line's typed content and has an active calculation
    if (selEnd >= lineEnd && lineLen > 0) {
      const anno = lastAnalysis.annotations.find(
        (a) => a.line === i && (a.kind === 'math' || a.kind === 'conversion') && a.value
      );
      if (anno) {
        hasEnhancedAnyLine = true;
        const trimmedSlice = lineSlice.trimEnd();
        if (trimmedSlice.endsWith('=') || trimmedSlice.endsWith('?')) {
          lineSlice = `${trimmedSlice} ${anno.value}`;
        } else {
          lineSlice = `${trimmedSlice} = ${anno.value}`;
        }
      }
    }
    enhancedLines.push(lineSlice);
  }

  if (hasEnhancedAnyLine) {
    const enhancedText = enhancedLines.join('\n');
    e.clipboardData.setData('text/plain', enhancedText);
    e.preventDefault();
  }
});

editor.addEventListener('select', syncOverlaySelection);
editor.addEventListener('mouseup', syncOverlaySelection);
editor.addEventListener('keyup', syncOverlaySelection);
document.addEventListener('selectionchange', () => {
  if (document.activeElement === editor) {
    syncOverlaySelection();
  }
});

// ---------- Event wiring ----------

editor.addEventListener('input', () => {
  updateBackdrop();
  syncActiveCalculationGhost();
  syncOverlaySelection();
  scheduleSave();
  scheduleAnalyze();
});

editor.addEventListener('scroll', () => {
  overlay.style.top = `-${editor.scrollTop}px`;
  if (backdrop) {
    backdrop.scrollTop = editor.scrollTop;
    backdrop.scrollLeft = editor.scrollLeft;
  }
  vimEngine.onScroll();
});

editor.addEventListener('keydown', (e: KeyboardEvent) => {
  const isCtrlOrMeta = e.ctrlKey || e.metaKey;

  // Toggle Read-Only Mode: Ctrl+Shift+R / Cmd+Shift+R
  if (isCtrlOrMeta && e.shiftKey && (e.key === 'R' || e.key === 'r')) {
    e.preventDefault();
    setReadOnly(!isReadOnly);
    invoke('set_setting', { key: 'read_only', value: isReadOnly }).catch(console.error);
    return;
  }

  // Toggle RTL Mode: Ctrl+Shift+L / Cmd+Shift+L
  if (isCtrlOrMeta && e.shiftKey && (e.key === 'L' || e.key === 'l')) {
    e.preventDefault();
    setRTLEnabled(!forceRTL);
    invoke('set_setting', { key: 'forceRTL', value: forceRTL }).catch(console.error);
    invoke('set_setting', { key: '_forceRTLStorage', value: forceRTL }).catch(console.error);
    return;
  }

  // If in Read-Only mode, block text modifying keys
  if (isReadOnly) {
    const isNavigation = [
      'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight',
      'Home', 'End', 'PageUp', 'PageDown', 'Escape'
    ].includes(e.key);
    const isAllowedShortcut = isCtrlOrMeta && (
      e.key === 'c' || e.key === 'C' ||
      e.key === 'a' || e.key === 'A' ||
      (e.key >= '1' && e.key <= '9') ||
      e.key === 'Tab' ||
      e.key === 'ArrowLeft' || e.key === 'ArrowRight' ||
      (e.shiftKey && (e.key === 'E' || e.key === 'e'))
    );
    if (!isNavigation && !isAllowedShortcut) {
      e.preventDefault();
      showStatusMessage('Note is Read-Only (Ctrl+Shift+R to unlock)');
      return;
    }
  }

  if (activateVimMode) {
    const handled = vimEngine.handleKeyDown(e);
    if (handled) return;
  }

  // Heading cycle: Ctrl+H (Body -> # -> ## -> ### -> Body)
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && (e.key === 'h' || e.key === 'H')) {
    e.preventDefault();
    const { newText, newCursorPos } = cycleHeadingLevel(editor.value, editor.selectionStart);
    editor.value = newText;
    editor.selectionStart = editor.selectionEnd = newCursorPos;
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    caretNudgeController.updateCursorLine(editor.selectionStart);
    return;
  }

  // Bold: Ctrl+B
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && (e.key === 'b' || e.key === 'B')) {
    e.preventDefault();
    const { newText, newStart, newEnd } = applyInlineFormat(editor.value, editor.selectionStart, editor.selectionEnd, 'bold');
    editor.value = newText;
    editor.selectionStart = newStart;
    editor.selectionEnd = newEnd;
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // Italic: Ctrl+I
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && (e.key === 'i' || e.key === 'I')) {
    e.preventDefault();
    const { newText, newStart, newEnd } = applyInlineFormat(editor.value, editor.selectionStart, editor.selectionEnd, 'italic');
    editor.value = newText;
    editor.selectionStart = newStart;
    editor.selectionEnd = newEnd;
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // Strikethrough: Ctrl+Shift+X or Ctrl+Alt+S
  if (
    (isCtrlOrMeta && e.shiftKey && (e.key === 'x' || e.key === 'X')) ||
    (isCtrlOrMeta && e.altKey && (e.key === 's' || e.key === 'S'))
  ) {
    e.preventDefault();
    const { newText, newStart, newEnd } = applyInlineFormat(editor.value, editor.selectionStart, editor.selectionEnd, 'strike');
    editor.value = newText;
    editor.selectionStart = newStart;
    editor.selectionEnd = newEnd;
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // Underline: Ctrl+U
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && (e.key === 'u' || e.key === 'U')) {
    e.preventDefault();
    const { newText, newStart, newEnd } = applyInlineFormat(editor.value, editor.selectionStart, editor.selectionEnd, 'underline');
    editor.value = newText;
    editor.selectionStart = newStart;
    editor.selectionEnd = newEnd;
    updateBackdrop();
    scheduleSave();
    scheduleAnalyze();
    return;
  }

  // Esc → close void confirmation dialog or void modal first, then autopaste, then hide window
  if (e.key === 'Escape') {
    e.preventDefault();
    if (voidConfirmDialog && !voidConfirmDialog.classList.contains('hidden')) {
      voidConfirmDialog.classList.add('hidden');
      return;
    }
    if (voidModal && !voidModal.classList.contains('hidden')) {
      closeVoidModal();
      return;
    }
    if (isAutoPasteActive) {
      stopAutoPaste().catch(console.error);
      return;
    }
    invoke('hide_window');
    return;
  }

  // Execution shortcut: Ctrl+Enter / Cmd+Enter for ::command(...) or alias
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && e.key === 'Enter') {
    const cursor = editor.selectionStart;
    const lineStart = editor.value.lastIndexOf('\n', cursor - 1) + 1;
    const lineEndIdx = editor.value.indexOf('\n', cursor);
    const lineEnd = lineEndIdx === -1 ? editor.value.length : lineEndIdx;
    const lineText = editor.value.substring(lineStart, lineEnd);
    const detected = findCommandInLine(lineText);
    if (detected) {
      e.preventDefault();
      executeEditorCommand(detected, lineStart, lineEnd).catch(console.error);
      return;
    }
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

  // Tab → execute command, accept chip, trigger paste, or indent
  if (e.key === 'Tab' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
    const cursor = editor.selectionStart;
    const lineStart = editor.value.lastIndexOf('\n', cursor - 1) + 1;
    const lineEndIdx = editor.value.indexOf('\n', cursor);
    const lineEnd = lineEndIdx === -1 ? editor.value.length : lineEndIdx;
    const lineText = editor.value.substring(lineStart, lineEnd);
    const detected = findCommandInLine(lineText);
    if (detected) {
      e.preventDefault();
      executeEditorCommand(detected, lineStart, lineEnd).catch(console.error);
      return;
    }

    const cursorLine = editor.value.substring(0, editor.selectionStart).split('\n').length - 1;
    const curLine = editor.value.split('\n')[cursorLine]?.trim().toLowerCase() || '';
    const pasteAliases = (triggerConfig?.paste_triggers as string[])?.map((t: string) => t.toLowerCase()) || ['paste'];
    if (pasteAliases.includes(curLine)) {
      e.preventDefault();
      startAutoPaste().catch(console.error);
      return;
    }
    const chipEl = overlay.querySelector(`.anno-chip`) as HTMLElement | null;
    if (chipEl) {
      e.preventDefault();
      const chipLine = parseInt(chipEl.dataset.line || String(cursorLine), 10);
      acceptChip(chipLine, editor.value.split('\n')[chipLine] || '');
      return;
    }

    // Calculation answer absorption (Tab steps into equals or commits answer)
    const activeCalcAnno = lastAnalysis?.annotations?.find(
      (a) => a.line === cursorLine && (a.kind === 'math' || a.kind === 'conversion') && a.value
    );
    if (activeCalcAnno && cursor >= lineStart) {
      e.preventDefault();
      const rawCurLine = editor.value.substring(lineStart, lineEnd);
      if (!rawCurLine.trim().endsWith('=') && !rawCurLine.trim().endsWith('?')) {
        const separator = rawCurLine.endsWith(' ') ? '= ' : ' = ';
        editor.value = editor.value.substring(0, lineEnd) + separator + editor.value.substring(lineEnd);
        editor.selectionStart = editor.selectionEnd = lineEnd + separator.length;
      } else {
        const separator = rawCurLine.endsWith(' ') ? '' : ' ';
        const insertText = separator + activeCalcAnno.value;
        editor.value = editor.value.substring(0, lineEnd) + insertText + editor.value.substring(lineEnd);
        editor.selectionStart = editor.selectionEnd = lineEnd + insertText.length;
      }
      updateBackdrop();
      syncActiveCalculationGhost();
      scheduleSave();
      scheduleAnalyze();
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

  // Direct slot jump: Ctrl+1 through Ctrl+9
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && e.key >= '1' && e.key <= '9') {
    e.preventDefault();
    const slot = parseInt(e.key, 10) - 1;
    switchToSlot(slot).catch(console.error);
    return;
  }

  // Cycle navigation: Ctrl+Tab and Ctrl+Shift+Tab
  if (isCtrlOrMeta && !e.altKey && e.key === 'Tab') {
    e.preventDefault();
    if (currentIsSlotted && currentSlotIndex >= 0) {
      const targetSlot = e.shiftKey
        ? (currentSlotIndex + 8) % 9
        : (currentSlotIndex + 1) % 9;
      switchToSlot(targetSlot).catch(console.error);
    } else {
      switchToSlot(0).catch(console.error);
    }
    return;
  }

  // Enter on a command line, timer line or paste trigger line → execute command / start or stop timer / start AutoPaste
  if (e.key === 'Enter') {
    const cursor = editor.selectionStart;
    const lineStart = editor.value.lastIndexOf('\n', cursor - 1) + 1;
    const lineEndIdx = editor.value.indexOf('\n', cursor);
    const lineEnd = lineEndIdx === -1 ? editor.value.length : lineEndIdx;
    const lineText = editor.value.substring(lineStart, lineEnd);
    const detected = findCommandInLine(lineText);
    if (detected) {
      e.preventDefault();
      executeEditorCommand(detected, lineStart, lineEnd).catch(console.error);
      return;
    }

    const cursorLine = editor.value.substring(0, editor.selectionStart).split('\n').length - 1;
    const rawLine = editor.value.split('\n')[cursorLine]?.trim() || '';
    const line = rawLine.toLowerCase();
    const pasteAliases = (triggerConfig?.paste_triggers as string[])?.map((t: string) => t.toLowerCase()) || ['paste'];
    if (pasteAliases.includes(line)) {
      e.preventDefault();
      startAutoPaste().catch(console.error);
      return;
    }
    if (line === 'timer s' || line === 'timer stop' || line === 'timer cancel' || line === 'stop timer') {
      e.preventDefault();
      clearTimer();
      invoke('cancel_timer').catch(console.error);
      return;
    }
    if (line === 'timer p' || line === 'timer pause' || line === 'timer resume') {
      e.preventDefault();
      invoke('pause_timer').catch(console.error);
      return;
    }
    if (line === 'timer r' || line === 'timer restart') {
      e.preventDefault();
      invoke('restart_timer').catch(console.error);
      return;
    }
    if (line.startsWith('timer') || line.startsWith('remind')) {
      e.preventDefault();
      invoke('start_timer_phrase', { phrase: rawLine }).catch(console.error);
      return;
    }

    // List continuation and termination on plain Enter
    if (!e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey) {
      const handled = handleListEnterKey(editor, {
        bulletListsEnabled: inlineChecklistBulletsEnabled,
        numberedListsEnabled: inlineNumberedListsEnabled,
        checkboxesEnabled: inlineChecklistCheckboxesEnabled,
      });
      if (handled) {
        e.preventDefault();
        updateBackdrop();
        scheduleSave();
        scheduleAnalyze();
        return;
      }
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
  if (e.ctrlKey && e.shiftKey && (e.key === 'D' || e.key === 'd')) {
    e.preventDefault();
    deleteCurrentNote();
  }

  // Ctrl+Shift+E → export
  if (e.ctrlKey && e.shiftKey && (e.key === 'E' || e.key === 'e')) {
    e.preventDefault();
    exportNote();
  }

  // Ctrl+Shift+T → toggle The Void / Trash modal
  if (e.ctrlKey && e.shiftKey && (e.key === 'T' || e.key === 't')) {
    e.preventDefault();
    if (voidModal && !voidModal.classList.contains('hidden')) {
      closeVoidModal();
    } else {
      openVoidModal();
    }
  }

  // Ctrl+P → toggle plain mode
  if (e.ctrlKey && e.key === 'p') {
    e.preventDefault();
    togglePlainMode();
  }
});

// Global hotkeys (direct slot jumps Ctrl+1..9, cycle navigation Ctrl+Tab)
window.addEventListener('keydown', (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    if (exportManager && exportManager.isOpen) {
      e.preventDefault();
      exportManager.close();
      return;
    }
    if (findAndReplaceManager && findAndReplaceManager.isOpen) {
      e.preventDefault();
      findAndReplaceManager.close();
      return;
    }
    if (voidConfirmDialog && !voidConfirmDialog.classList.contains('hidden')) {
      e.preventDefault();
      voidConfirmDialog.classList.add('hidden');
      return;
    }
    if (voidModal && !voidModal.classList.contains('hidden')) {
      e.preventDefault();
      closeVoidModal();
      return;
    }
    if (isAutoPasteActive) {
      e.preventDefault();
      stopAutoPaste().catch(console.error);
      return;
    }
  }

  const isCtrlOrMeta = e.ctrlKey || e.metaKey;

  // Export Menu: Ctrl+Shift+E / Cmd+Shift+E
  if (isCtrlOrMeta && e.shiftKey && (e.key === 'E' || e.key === 'e')) {
    e.preventDefault();
    exportManager.toggle(currentId, editor.value);
    return;
  }

  // Find & Replace: Ctrl+F / Cmd+F
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && (e.key === 'f' || e.key === 'F')) {
    e.preventDefault();
    findAndReplaceManager.open();
    return;
  }

  // Global Ctrl+Shift+T → toggle The Void
  if (isCtrlOrMeta && e.shiftKey && (e.key === 'T' || e.key === 't')) {
    e.preventDefault();
    if (voidModal && !voidModal.classList.contains('hidden')) {
      closeVoidModal();
    } else {
      openVoidModal();
    }
    return;
  }

  // Direct slot jump: Ctrl+1 through Ctrl+9
  if (isCtrlOrMeta && !e.shiftKey && !e.altKey && e.key >= '1' && e.key <= '9') {
    e.preventDefault();
    const slot = parseInt(e.key, 10) - 1;
    switchToSlot(slot).catch(console.error);
    return;
  }

  // Cycle navigation: Ctrl+Tab and Ctrl+Shift+Tab
  if (isCtrlOrMeta && !e.altKey && e.key === 'Tab') {
    e.preventDefault();
    if (currentIsSlotted && currentSlotIndex >= 0) {
      const targetSlot = e.shiftKey
        ? (currentSlotIndex + 8) % 9
        : (currentSlotIndex + 1) % 9;
      switchToSlot(targetSlot).catch(console.error);
    } else {
      switchToSlot(0).catch(console.error);
    }
    return;
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
btnDelete.addEventListener('click', (e: MouseEvent) => deleteCurrentNote(e.clientX, e.clientY));
btnSearch?.addEventListener('click', () => findAndReplaceManager.toggle());
btnVoid?.addEventListener('click', () => {
  if (voidModal && !voidModal.classList.contains('hidden')) {
    closeVoidModal();
  } else {
    openVoidModal();
  }
});
btnCloseVoid?.addEventListener('click', () => closeVoidModal());
voidModalBackdrop?.addEventListener('click', () => closeVoidModal());
btnEmptyVoid?.addEventListener('click', () => handleEmptyVoidClick().catch(console.error));
btnCancelEmptyVoid?.addEventListener('click', () => {
  if (voidConfirmDialog) voidConfirmDialog.classList.add('hidden');
});
voidConfirmBackdrop?.addEventListener('click', () => {
  if (voidConfirmDialog) voidConfirmDialog.classList.add('hidden');
});
btnConfirmEmptyVoid?.addEventListener('click', () => confirmEmptyVoid().catch(console.error));
btnExport.addEventListener('click', () => exportManager.toggle(currentId, editor.value));
btnSettings.addEventListener('click', () => {
  invoke('open_settings_window').catch(console.error);
});

btnPin.addEventListener('click', async () => {
  if (currentIsSlotted) {
    showStatusMessage('Slotted notes cannot be pinned');
    return;
  }
  if (!currentId || !notes[currentIndex]) return;
  const newVal = !(notes[currentIndex].pinned ?? false);
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

// Top bar window dragging and maximize toggle
const topBar = document.getElementById('top-bar');
if (topBar) {
  topBar.addEventListener('mousedown', (e) => {
    // Only primary (left) button and don't drag if clicking buttons, inputs, badges or note index
    if (e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (target && target.closest('button, input, select, textarea, .chrome-btn, .readonly-badge, .note-index')) {
      return;
    }
    invoke('start_dragging').catch(() => {
      try {
        win.__TAURI__?.window?.getCurrentWindow()?.startDragging();
      } catch (_) {}
    });
  });

  topBar.addEventListener('dblclick', (e) => {
    const target = e.target as HTMLElement | null;
    if (target && target.closest('button, input, select, textarea, .chrome-btn')) {
      return;
    }
    invoke('toggle_maximize').catch(console.error);
  });
}

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

// ---------- Boot ----------
init().catch(console.error);
