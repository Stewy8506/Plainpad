// Plainpad — Modal Vim Mode Engine
// Implements NORMAL, INSERT, VISUAL, and VISUAL_LINE modal editing
// with debounced motion updates, action operators, registers, and status line.

export enum VimMode {
  NORMAL = 'NORMAL',
  INSERT = 'INSERT',
  VISUAL = 'VISUAL',
  VISUAL_LINE = 'VISUAL_LINE',
}

export interface VimEngineOptions {
  textarea: HTMLTextAreaElement;
  container: HTMLElement;
  statusIndicator?: HTMLElement | null;
  onTextChange?: () => void;
  onModeChange?: (mode: VimMode) => void;
}

interface UndoEntry {
  text: string;
  cursor: number;
}

export class VimEngine {
  public mode: VimMode = VimMode.NORMAL;
  public enabled: boolean = false;
  public commandBuffer: string = '';
  public yankRegister: string = '';
  public isLineYank: boolean = false;
  public anchorPos: number = 0;
  public cursorPos: number = 0;

  private textarea: HTMLTextAreaElement;
  private container: HTMLElement;
  private statusIndicator: HTMLElement | null = null;
  private caretEl: HTMLElement | null = null;
  private mirrorEl: HTMLDivElement | null = null;

  private debounceTimerForVimMotions: number | null = null;
  private undoStack: UndoEntry[] = [];
  private readonly maxUndoEntries: number = 100;

  private onTextChange?: () => void;
  private onModeChange?: (mode: VimMode) => void;

  // Pending operator state: 'd' | 'y' | 'c' | 'r' | 'g' | ''
  private pendingOperator: string = '';
  private pendingCount: number = 0;

  constructor(options: VimEngineOptions) {
    this.textarea = options.textarea;
    this.container = options.container;
    this.statusIndicator = options.statusIndicator || null;
    this.onTextChange = options.onTextChange;
    this.onModeChange = options.onModeChange;

    this.initCaretElement();
  }

  private initCaretElement(): void {
    let caret = this.container.querySelector('.vim-block-caret') as HTMLElement | null;
    if (!caret) {
      caret = document.createElement('div');
      caret.className = 'vim-block-caret hidden';
      caret.id = 'vim-block-caret';
      this.container.appendChild(caret);
    }
    this.caretEl = caret;
  }

  public setEnabled(enabled: boolean): void {
    this.enabled = enabled;
    if (enabled) {
      this.mode = VimMode.NORMAL;
      this.commandBuffer = '';
      this.pendingOperator = '';
      this.pendingCount = 0;
      this.textarea.classList.add('vim-normal-mode');
      this.cursorPos = Math.min(this.textarea.selectionStart, this.textarea.value.length);
      this.clampCursorForNormal();
      this.syncTextareaCursor();
      if (this.statusIndicator) {
        this.statusIndicator.classList.remove('hidden');
      }
      this.renderCaret();
      this.updateStatusLine();
      if (this.onModeChange) this.onModeChange(this.mode);
    } else {
      this.mode = VimMode.INSERT;
      this.commandBuffer = '';
      this.pendingOperator = '';
      this.pendingCount = 0;
      this.textarea.classList.remove('vim-normal-mode');
      if (this.caretEl) {
        this.caretEl.classList.add('hidden');
      }
      if (this.statusIndicator) {
        this.statusIndicator.classList.add('hidden');
      }
      if (this.onModeChange) this.onModeChange(this.mode);
    }
  }

  public setStatusIndicator(el: HTMLElement | null): void {
    this.statusIndicator = el;
    this.updateStatusLine();
  }

  public getMode(): VimMode {
    return this.mode;
  }

  public onScroll(): void {
    if (!this.enabled || this.mode === VimMode.INSERT) return;
    this.scheduleDebouncedMotionRender();
  }

  public enterVisualLineMode(): void {
    this.setMode(VimMode.VISUAL_LINE);
  }

  public onTextLoaded(): void {
    if (!this.enabled) return;
    this.cursorPos = Math.min(this.textarea.selectionStart, this.textarea.value.length);
    if (this.mode === VimMode.NORMAL) {
      this.clampCursorForNormal();
      this.syncTextareaCursor();
    }
    this.renderCaret();
    this.updateStatusLine();
  }

  public setMode(newMode: VimMode): void {
    if (this.mode === newMode) return;
    const prevMode = this.mode;
    this.mode = newMode;
    this.commandBuffer = '';
    this.pendingOperator = '';
    this.pendingCount = 0;

    if (newMode === VimMode.NORMAL) {
      this.textarea.classList.add('vim-normal-mode');
      if (prevMode === VimMode.INSERT) {
        // Vim behavior: cursor moves 1 position left when exiting INSERT
        const lineStart = this.getLineStart(this.cursorPos);
        if (this.cursorPos > lineStart) {
          this.cursorPos--;
        }
      }
      this.clampCursorForNormal();
      this.syncTextareaCursor();
    } else if (newMode === VimMode.INSERT) {
      this.textarea.classList.remove('vim-normal-mode');
      if (this.caretEl) {
        this.caretEl.classList.add('hidden');
      }
    } else if (newMode === VimMode.VISUAL || newMode === VimMode.VISUAL_LINE) {
      this.textarea.classList.add('vim-normal-mode');
      this.anchorPos = this.cursorPos;
      this.updateVisualSelection();
    }

    this.renderCaret();
    this.updateStatusLine();
    if (this.onModeChange) this.onModeChange(this.mode);
  }

  public handleKeyDown(e: KeyboardEvent): boolean {
    if (!this.enabled) return false;

    // Esc or Ctrl+[ : always transition immediately to NORMAL mode
    if (e.key === 'Escape' || (e.ctrlKey && e.key === '[')) {
      e.preventDefault();
      if (this.pendingOperator || this.commandBuffer) {
        this.pendingOperator = '';
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;
      }
      this.setMode(VimMode.NORMAL);
      return true;
    }

    // In INSERT mode, allow all typing keys to pass through normally
    if (this.mode === VimMode.INSERT) {
      return false;
    }

    // Capture arrow keys in NORMAL or VISUAL modes
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      this.executeMotion('h', 1);
      return true;
    }
    if (e.key === 'ArrowRight') {
      e.preventDefault();
      this.executeMotion('l', 1);
      return true;
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      this.executeMotion('k', 1);
      return true;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      this.executeMotion('j', 1);
      return true;
    }

    // Allow standard browser window hotkeys (Ctrl+N, Ctrl+Q, Ctrl+W, etc.)
    if (e.ctrlKey && (e.key === 'n' || e.key === 'N' || e.key === 'Tab')) {
      return false;
    }

    // We intercept and handle this key in Vim modal mode
    e.preventDefault();

    // Check if waiting for replace character (r{char})
    if (this.pendingOperator === 'r') {
      if (e.key.length === 1) {
        this.executeReplaceChar(e.key);
      }
      this.pendingOperator = '';
      this.commandBuffer = '';
      this.updateStatusLine();
      return true;
    }

    // Count prefixes (e.g. 1-9 or 0 if count already > 0)
    if (/^[1-9]$/.test(e.key) || (e.key === '0' && this.pendingCount > 0)) {
      this.pendingCount = this.pendingCount * 10 + parseInt(e.key, 10);
      this.commandBuffer += e.key;
      this.updateStatusLine();
      return true;
    }

    const count = this.pendingCount > 0 ? this.pendingCount : 1;

    // Check second key of pending operators
    if (this.pendingOperator) {
      const op = this.pendingOperator;
      this.pendingOperator = '';
      this.pendingCount = 0;
      this.commandBuffer = '';

      // Double-key line operations: dd, yy, cc
      if (op === 'd' && e.key === 'd') {
        this.executeDeleteLines(count);
        return true;
      }
      if (op === 'y' && e.key === 'y') {
        this.executeYankLines(count);
        return true;
      }
      if (op === 'c' && e.key === 'c') {
        this.executeChangeLine();
        return true;
      }
      if (op === 'g' && e.key === 'g') {
        this.executeGoToLine(count === 1 ? 1 : count);
        return true;
      }

      // Operator + Motion: dw, de, db, d$, d0, d^, yw, cw, etc.
      this.executeOperatorWithMotion(op, e.key, count);
      return true;
    }

    // Single key handling
    switch (e.key) {
      // Entry to INSERT mode
      case 'i':
        this.pushUndo();
        this.setMode(VimMode.INSERT);
        return true;

      case 'a':
        this.pushUndo();
        if (this.cursorPos < this.textarea.value.length) {
          const lineEnd = this.getLineEnd(this.cursorPos);
          if (this.cursorPos < lineEnd) {
            this.cursorPos++;
            this.syncTextareaCursor();
          }
        }
        this.setMode(VimMode.INSERT);
        return true;

      case 'o':
        this.executeOpenBelow();
        return true;

      case 'O':
        this.executeOpenAbove();
        return true;

      // Visual selection entry
      case 'v':
        if (this.mode === VimMode.VISUAL) {
          this.setMode(VimMode.NORMAL);
        } else {
          this.setMode(VimMode.VISUAL);
        }
        return true;

      case 'V':
        if (this.mode === VimMode.VISUAL_LINE) {
          this.setMode(VimMode.NORMAL);
        } else {
          this.setMode(VimMode.VISUAL_LINE);
        }
        return true;

      // Basic Motions
      case 'h':
      case 'j':
      case 'k':
      case 'l':
      case 'w':
      case 'b':
      case 'e':
      case '0':
      case '^':
      case '$':
        this.executeMotion(e.key, count);
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      case 'g':
        this.pendingOperator = 'g';
        this.commandBuffer = `${this.pendingCount > 0 ? this.pendingCount : ''}g`;
        this.updateStatusLine();
        return true;

      case 'G':
        this.executeGoToBottom(this.pendingCount > 0 ? this.pendingCount : undefined);
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      // Manipulation Operators
      case 'x':
        this.executeDeleteChar(count);
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      case 'r':
        this.pendingOperator = 'r';
        this.commandBuffer = 'r';
        this.updateStatusLine();
        return true;

      case 'd':
        if (this.mode === VimMode.VISUAL || this.mode === VimMode.VISUAL_LINE) {
          this.executeVisualDelete();
        } else {
          this.pendingOperator = 'd';
          this.commandBuffer = `${this.pendingCount > 0 ? this.pendingCount : ''}d`;
          this.updateStatusLine();
        }
        return true;

      case 'D':
        this.executeDeleteToEndOfLine();
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      case 'y':
        if (this.mode === VimMode.VISUAL || this.mode === VimMode.VISUAL_LINE) {
          this.executeVisualYank();
        } else {
          this.pendingOperator = 'y';
          this.commandBuffer = `${this.pendingCount > 0 ? this.pendingCount : ''}y`;
          this.updateStatusLine();
        }
        return true;

      case 'Y':
        this.executeYankLines(count);
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      case 'c':
        if (this.mode === VimMode.VISUAL || this.mode === VimMode.VISUAL_LINE) {
          this.executeVisualChange();
        } else {
          this.pendingOperator = 'c';
          this.commandBuffer = `${this.pendingCount > 0 ? this.pendingCount : ''}c`;
          this.updateStatusLine();
        }
        return true;

      case 'C':
        this.executeChangeToEndOfLine();
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      case 'p':
        this.executePaste(true);
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      case 'P':
        this.executePaste(false);
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      case 'u':
        this.executeUndo();
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;

      default:
        // Ignore unrecognized keys in normal mode
        this.pendingCount = 0;
        this.commandBuffer = '';
        this.updateStatusLine();
        return true;
    }
  }

  // ---------- Motions & Navigation ----------

  private executeMotion(motion: string, count: number): void {
    const text = this.textarea.value;
    let target = this.cursorPos;

    for (let i = 0; i < count; i++) {
      switch (motion) {
        case 'h': {
          const lineStart = this.getLineStart(target);
          target = Math.max(lineStart, target - 1);
          break;
        }
        case 'l': {
          const lineEnd = this.getLineEnd(target);
          const maxTarget = this.mode === VimMode.NORMAL
            ? (lineEnd > this.getLineStart(target) ? lineEnd - 1 : lineEnd)
            : lineEnd;
          target = Math.min(maxTarget, target + 1);
          break;
        }
        case 'j': {
          target = this.getLineDown(target, 1);
          break;
        }
        case 'k': {
          target = this.getLineUp(target, 1);
          break;
        }
        case 'w': {
          target = this.getNextWordStart(text, target);
          break;
        }
        case 'b': {
          target = this.getPrevWordStart(text, target);
          break;
        }
        case 'e': {
          target = this.getWordEnd(text, target);
          break;
        }
        case '0': {
          target = this.getLineStart(target);
          break;
        }
        case '^': {
          target = this.getFirstNonWhitespace(text, target);
          break;
        }
        case '$': {
          const lineEnd = this.getLineEnd(target);
          const lineStart = this.getLineStart(target);
          target = this.mode === VimMode.NORMAL
            ? (lineEnd > lineStart ? lineEnd - 1 : lineStart)
            : lineEnd;
          break;
        }
      }
    }

    this.cursorPos = target;

    if (this.mode === VimMode.VISUAL || this.mode === VimMode.VISUAL_LINE) {
      this.updateVisualSelection();
    } else {
      this.clampCursorForNormal();
      this.syncTextareaCursor();
    }

    this.scheduleDebouncedMotionRender();
  }

  private executeGoToLine(lineNum: number): void {
    const text = this.textarea.value;
    const lines = text.split('\n');
    const targetLine = Math.max(0, Math.min(lineNum - 1, lines.length - 1));

    let pos = 0;
    for (let i = 0; i < targetLine; i++) {
      pos += lines[i].length + 1;
    }
    this.cursorPos = pos;
    this.clampCursorForNormal();
    this.syncTextareaCursor();
    this.scheduleDebouncedMotionRender();
  }

  private executeGoToBottom(lineNum?: number): void {
    if (lineNum !== undefined) {
      this.executeGoToLine(lineNum);
      return;
    }
    const text = this.textarea.value;
    const lastNewline = text.lastIndexOf('\n');
    this.cursorPos = lastNewline >= 0 ? lastNewline + 1 : 0;
    this.clampCursorForNormal();
    this.syncTextareaCursor();
    this.scheduleDebouncedMotionRender();
  }

  // ---------- Debounced Motion Timer ----------

  private scheduleDebouncedMotionRender(): void {
    if (this.debounceTimerForVimMotions !== null) {
      cancelAnimationFrame(this.debounceTimerForVimMotions);
    }
    this.debounceTimerForVimMotions = requestAnimationFrame(() => {
      this.debounceTimerForVimMotions = null;
      this.renderCaret();
      this.updateStatusLine();
    });
  }

  // ---------- Action Operators & Manipulation ----------

  private executeDeleteChar(count: number): void {
    const text = this.textarea.value;
    if (text.length === 0 || this.cursorPos >= text.length) return;

    this.pushUndo();
    const lineEnd = this.getLineEnd(this.cursorPos);
    const charsToDelete = Math.min(count, Math.max(1, lineEnd - this.cursorPos));
    const deleted = text.substring(this.cursorPos, this.cursorPos + charsToDelete);

    this.yankRegister = deleted;
    this.isLineYank = false;

    const nextText = text.substring(0, this.cursorPos) + text.substring(this.cursorPos + charsToDelete);
    this.textarea.value = nextText;
    this.clampCursorForNormal();
    this.syncTextareaCursor();
    this.renderCaret();
    this.updateStatusLine();
    if (this.onTextChange) this.onTextChange();
  }

  private executeReplaceChar(char: string): void {
    const text = this.textarea.value;
    if (this.cursorPos >= text.length) return;

    this.pushUndo();
    const nextText = text.substring(0, this.cursorPos) + char + text.substring(this.cursorPos + 1);
    this.textarea.value = nextText;
    this.syncTextareaCursor();
    this.renderCaret();
    this.updateStatusLine();
    if (this.onTextChange) this.onTextChange();
  }

  private executeDeleteLines(count: number): void {
    const text = this.textarea.value;
    if (text.length === 0) return;

    this.pushUndo();
    let start = this.getLineStart(this.cursorPos);
    let end = start;

    for (let i = 0; i < count; i++) {
      const nextNewline = text.indexOf('\n', end);
      if (nextNewline === -1) {
        end = text.length;
        break;
      } else {
        end = nextNewline + 1;
      }
    }

    let deleted = text.substring(start, end);
    if (!deleted.endsWith('\n') && start > 0 && text[start - 1] === '\n') {
      // If deleting the last line which has no trailing newline, consume preceding newline
      start--;
      deleted += '\n';
    }

    this.yankRegister = deleted;
    this.isLineYank = true;

    this.textarea.value = text.substring(0, start) + text.substring(end);
    this.cursorPos = Math.min(start, Math.max(0, this.textarea.value.length - 1));
    this.clampCursorForNormal();
    this.syncTextareaCursor();
    this.renderCaret();
    this.updateStatusLine();
    if (this.onTextChange) this.onTextChange();
  }

  private executeYankLines(count: number): void {
    const text = this.textarea.value;
    if (text.length === 0) return;

    let start = this.getLineStart(this.cursorPos);
    let end = start;

    for (let i = 0; i < count; i++) {
      const nextNewline = text.indexOf('\n', end);
      if (nextNewline === -1) {
        end = text.length;
        break;
      } else {
        end = nextNewline + 1;
      }
    }

    let yanked = text.substring(start, end);
    if (!yanked.endsWith('\n')) {
      yanked += '\n';
    }
    this.yankRegister = yanked;
    this.isLineYank = true;
    this.updateStatusLine();
  }

  private executeChangeLine(): void {
    this.pushUndo();
    const text = this.textarea.value;
    const lineStart = this.getLineStart(this.cursorPos);
    const lineEnd = this.getLineEnd(this.cursorPos);

    this.yankRegister = text.substring(lineStart, lineEnd) + '\n';
    this.isLineYank = true;

    this.textarea.value = text.substring(0, lineStart) + text.substring(lineEnd);
    this.cursorPos = lineStart;
    this.syncTextareaCursor();
    this.setMode(VimMode.INSERT);
    if (this.onTextChange) this.onTextChange();
  }

  private executeDeleteToEndOfLine(): void {
    const text = this.textarea.value;
    const lineEnd = this.getLineEnd(this.cursorPos);
    if (this.cursorPos >= lineEnd) return;

    this.pushUndo();
    this.yankRegister = text.substring(this.cursorPos, lineEnd);
    this.isLineYank = false;

    this.textarea.value = text.substring(0, this.cursorPos) + text.substring(lineEnd);
    this.clampCursorForNormal();
    this.syncTextareaCursor();
    this.renderCaret();
    this.updateStatusLine();
    if (this.onTextChange) this.onTextChange();
  }

  private executeChangeToEndOfLine(): void {
    this.pushUndo();
    const text = this.textarea.value;
    const lineEnd = this.getLineEnd(this.cursorPos);

    this.yankRegister = text.substring(this.cursorPos, lineEnd);
    this.isLineYank = false;

    this.textarea.value = text.substring(0, this.cursorPos) + text.substring(lineEnd);
    this.syncTextareaCursor();
    this.setMode(VimMode.INSERT);
    if (this.onTextChange) this.onTextChange();
  }

  private executeOperatorWithMotion(operator: string, motion: string, count: number): void {
    const text = this.textarea.value;
    const startPos = this.cursorPos;

    // Calculate motion end pos
    let endPos = startPos;
    switch (motion) {
      case 'w':
        for (let i = 0; i < count; i++) endPos = this.getNextWordStart(text, endPos);
        break;
      case 'b':
        for (let i = 0; i < count; i++) endPos = this.getPrevWordStart(text, endPos);
        break;
      case 'e':
        for (let i = 0; i < count; i++) endPos = this.getWordEnd(text, endPos);
        if (endPos < text.length) endPos++; // include the word end character
        break;
      case '$':
        endPos = this.getLineEnd(endPos);
        break;
      case '0':
        endPos = this.getLineStart(endPos);
        break;
      case '^':
        endPos = this.getFirstNonWhitespace(text, endPos);
        break;
      default:
        return;
    }

    const rangeStart = Math.min(startPos, endPos);
    const rangeEnd = Math.max(startPos, endPos);
    if (rangeStart === rangeEnd) return;

    if (operator === 'd' || operator === 'c') {
      this.pushUndo();
      this.yankRegister = text.substring(rangeStart, rangeEnd);
      this.isLineYank = false;

      this.textarea.value = text.substring(0, rangeStart) + text.substring(rangeEnd);
      this.cursorPos = rangeStart;

      if (operator === 'c') {
        this.syncTextareaCursor();
        this.setMode(VimMode.INSERT);
      } else {
        this.clampCursorForNormal();
        this.syncTextareaCursor();
        this.renderCaret();
      }
      if (this.onTextChange) this.onTextChange();
    } else if (operator === 'y') {
      this.yankRegister = text.substring(rangeStart, rangeEnd);
      this.isLineYank = false;
      this.updateStatusLine();
    }
  }

  private executePaste(after: boolean): void {
    if (!this.yankRegister) return;
    const text = this.textarea.value;
    this.pushUndo();

    if (this.isLineYank) {
      if (after) {
        // Paste below current line
        const lineEnd = this.getLineEnd(this.cursorPos);
        if (lineEnd === text.length) {
          const insertText = (text.endsWith('\n') || text.length === 0 ? '' : '\n') + this.yankRegister;
          this.textarea.value = text + insertText;
          this.cursorPos = lineEnd + (text.endsWith('\n') || text.length === 0 ? 0 : 1);
        } else {
          this.textarea.value = text.substring(0, lineEnd + 1) + this.yankRegister + text.substring(lineEnd + 1);
          this.cursorPos = lineEnd + 1;
        }
      } else {
        // Paste above current line
        const lineStart = this.getLineStart(this.cursorPos);
        this.textarea.value = text.substring(0, lineStart) + this.yankRegister + text.substring(lineStart);
        this.cursorPos = lineStart;
      }
    } else {
      let insertIndex = after ? Math.min(text.length, this.cursorPos + 1) : this.cursorPos;
      if (text.length === 0) insertIndex = 0;
      this.textarea.value = text.substring(0, insertIndex) + this.yankRegister + text.substring(insertIndex);
      this.cursorPos = insertIndex + this.yankRegister.length - 1;
    }

    this.clampCursorForNormal();
    this.syncTextareaCursor();
    this.renderCaret();
    this.updateStatusLine();
    if (this.onTextChange) this.onTextChange();
  }

  private executeOpenBelow(): void {
    this.pushUndo();
    const text = this.textarea.value;
    const lineEnd = this.getLineEnd(this.cursorPos);

    if (lineEnd === text.length) {
      this.textarea.value = text + '\n';
      this.cursorPos = this.textarea.value.length;
    } else {
      this.textarea.value = text.substring(0, lineEnd) + '\n' + text.substring(lineEnd);
      this.cursorPos = lineEnd + 1;
    }

    this.syncTextareaCursor();
    this.setMode(VimMode.INSERT);
    if (this.onTextChange) this.onTextChange();
  }

  private executeOpenAbove(): void {
    this.pushUndo();
    const text = this.textarea.value;
    const lineStart = this.getLineStart(this.cursorPos);

    this.textarea.value = text.substring(0, lineStart) + '\n' + text.substring(lineStart);
    this.cursorPos = lineStart;

    this.syncTextareaCursor();
    this.setMode(VimMode.INSERT);
    if (this.onTextChange) this.onTextChange();
  }

  // ---------- Visual Selection Operations ----------

  private updateVisualSelection(): void {
    const text = this.textarea.value;
    if (this.mode === VimMode.VISUAL) {
      const start = Math.min(this.anchorPos, this.cursorPos);
      const end = Math.min(text.length, Math.max(this.anchorPos, this.cursorPos) + 1);
      this.textarea.setSelectionRange(start, end);
    } else if (this.mode === VimMode.VISUAL_LINE) {
      const minPos = Math.min(this.anchorPos, this.cursorPos);
      const maxPos = Math.max(this.anchorPos, this.cursorPos);
      const lineStart = this.getLineStart(minPos);
      const lineEnd = this.getLineEnd(maxPos);
      const end = lineEnd < text.length ? lineEnd + 1 : lineEnd;
      this.textarea.setSelectionRange(lineStart, end);
    }
  }

  private executeVisualDelete(): void {
    const start = this.textarea.selectionStart;
    const end = this.textarea.selectionEnd;
    if (start === end) return;

    this.pushUndo();
    const text = this.textarea.value;
    this.yankRegister = text.substring(start, end);
    this.isLineYank = this.mode === VimMode.VISUAL_LINE;

    this.textarea.value = text.substring(0, start) + text.substring(end);
    this.cursorPos = Math.min(start, Math.max(0, this.textarea.value.length - 1));
    this.setMode(VimMode.NORMAL);
    if (this.onTextChange) this.onTextChange();
  }

  private executeVisualYank(): void {
    const start = this.textarea.selectionStart;
    const end = this.textarea.selectionEnd;
    if (start === end) return;

    const text = this.textarea.value;
    this.yankRegister = text.substring(start, end);
    this.isLineYank = this.mode === VimMode.VISUAL_LINE;

    this.cursorPos = start;
    this.setMode(VimMode.NORMAL);
  }

  private executeVisualChange(): void {
    const start = this.textarea.selectionStart;
    const end = this.textarea.selectionEnd;

    this.pushUndo();
    const text = this.textarea.value;
    this.yankRegister = text.substring(start, end);
    this.isLineYank = this.mode === VimMode.VISUAL_LINE;

    this.textarea.value = text.substring(0, start) + text.substring(end);
    this.cursorPos = start;
    this.syncTextareaCursor();
    this.setMode(VimMode.INSERT);
    if (this.onTextChange) this.onTextChange();
  }

  // ---------- Undo Stack ----------

  private pushUndo(): void {
    this.undoStack.push({
      text: this.textarea.value,
      cursor: this.cursorPos,
    });
    if (this.undoStack.length > this.maxUndoEntries) {
      this.undoStack.shift();
    }
  }

  private executeUndo(): void {
    if (this.undoStack.length === 0) return;
    const entry = this.undoStack.pop();
    if (!entry) return;

    this.textarea.value = entry.text;
    this.cursorPos = Math.min(entry.cursor, entry.text.length);
    this.clampCursorForNormal();
    this.syncTextareaCursor();
    this.renderCaret();
    this.updateStatusLine();
    if (this.onTextChange) this.onTextChange();
  }

  // ---------- Caret Rendering & Coordinate Measurement ----------

  private renderCaret(): void {
    if (!this.caretEl) return;
    if (!this.enabled || this.mode === VimMode.INSERT) {
      this.caretEl.classList.add('hidden');
      return;
    }

    const coords = this.measureCaretCoordinates(this.cursorPos);
    this.caretEl.style.top = `${coords.top}px`;
    this.caretEl.style.left = `${coords.left}px`;
    this.caretEl.style.width = `${coords.width}px`;
    this.caretEl.style.height = `${coords.height}px`;

    const charUnderCursor = this.textarea.value[this.cursorPos] || ' ';
    this.caretEl.textContent = charUnderCursor === '\n' ? ' ' : charUnderCursor;
    this.caretEl.classList.remove('hidden');
  }

  private measureCaretCoordinates(pos: number): { top: number; left: number; width: number; height: number } {
    if (!this.mirrorEl) {
      this.mirrorEl = document.createElement('div');
      this.mirrorEl.className = 'vim-caret-mirror';
      this.mirrorEl.setAttribute('aria-hidden', 'true');
      this.mirrorEl.style.position = 'absolute';
      this.mirrorEl.style.visibility = 'hidden';
      this.mirrorEl.style.pointerEvents = 'none';
      this.mirrorEl.style.whiteSpace = 'pre-wrap';
      this.mirrorEl.style.wordWrap = 'break-word';
      this.mirrorEl.style.boxSizing = 'border-box';
      this.mirrorEl.style.overflow = 'hidden';
      this.container.appendChild(this.mirrorEl);
    }

    const cs = window.getComputedStyle(this.textarea);
    this.mirrorEl.style.fontFamily = cs.fontFamily;
    this.mirrorEl.style.fontSize = cs.fontSize;
    this.mirrorEl.style.lineHeight = cs.lineHeight;
    this.mirrorEl.style.letterSpacing = cs.letterSpacing;
    this.mirrorEl.style.paddingTop = cs.paddingTop;
    this.mirrorEl.style.paddingLeft = cs.paddingLeft;
    this.mirrorEl.style.paddingRight = cs.paddingRight;
    this.mirrorEl.style.paddingBottom = cs.paddingBottom;
    this.mirrorEl.style.width = `${this.textarea.clientWidth}px`;

    const text = this.textarea.value;
    const before = text.substring(0, pos);
    const targetChar = text[pos] || ' ';

    this.mirrorEl.textContent = '';
    this.mirrorEl.appendChild(document.createTextNode(before));

    const marker = document.createElement('span');
    marker.textContent = targetChar === '\n' ? ' ' : targetChar;
    marker.style.display = 'inline-block';
    this.mirrorEl.appendChild(marker);

    const top = marker.offsetTop - this.textarea.scrollTop;
    const left = marker.offsetLeft - this.textarea.scrollLeft;
    const width = Math.max(8, marker.offsetWidth);
    const height = Math.max(16, marker.offsetHeight);

    return { top, left, width, height };
  }

  // ---------- Status Line Indicator ----------

  public updateStatusLine(): void {
    if (!this.statusIndicator || !this.enabled) return;

    let modeText = '-- NORMAL --';
    let modeClass = 'mode-normal';
    if (this.mode === VimMode.INSERT) {
      modeText = '-- INSERT --';
      modeClass = 'mode-insert';
    } else if (this.mode === VimMode.VISUAL) {
      modeText = '-- VISUAL --';
      modeClass = 'mode-visual';
    } else if (this.mode === VimMode.VISUAL_LINE) {
      modeText = '-- VISUAL LINE --';
      modeClass = 'mode-visual-line';
    }

    const { line, col } = this.getLineAndCol(this.cursorPos);
    const bufferDisplay = this.commandBuffer ? `<span class="vim-status-buffer">[${this.commandBuffer}]</span>` : '';

    this.statusIndicator.innerHTML = `
      <span class="vim-status-mode ${modeClass}">${modeText}</span>
      ${bufferDisplay}
      <span class="vim-status-coords">Ln ${line}, Col ${col}</span>
    `.trim();
  }

  // ---------- Position & Line Boundary Math ----------

  private clampCursorForNormal(): void {
    const text = this.textarea.value;
    if (text.length === 0) {
      this.cursorPos = 0;
      return;
    }
    const lineStart = this.getLineStart(this.cursorPos);
    const lineEnd = this.getLineEnd(this.cursorPos);
    if (lineEnd > lineStart) {
      this.cursorPos = Math.min(this.cursorPos, lineEnd - 1);
    } else {
      this.cursorPos = lineStart;
    }
  }

  private syncTextareaCursor(): void {
    this.textarea.selectionStart = this.cursorPos;
    this.textarea.selectionEnd = this.cursorPos;
  }

  public getLineStart(pos: number): number {
    const text = this.textarea.value;
    const lastNewline = text.lastIndexOf('\n', Math.max(0, pos - 1));
    return lastNewline === -1 ? 0 : lastNewline + 1;
  }

  public getLineEnd(pos: number): number {
    const text = this.textarea.value;
    const nextNewline = text.indexOf('\n', pos);
    return nextNewline === -1 ? text.length : nextNewline;
  }

  public getLineAndCol(pos: number): { line: number; col: number } {
    const text = this.textarea.value;
    const clampedPos = Math.max(0, Math.min(pos, text.length));
    const before = text.substring(0, clampedPos);
    const lines = before.split('\n');
    const line = lines.length;
    const col = lines[lines.length - 1].length + 1;
    return { line, col };
  }

  private getFirstNonWhitespace(text: string, pos: number): number {
    const lineStart = this.getLineStart(pos);
    const lineEnd = this.getLineEnd(pos);
    for (let i = lineStart; i < lineEnd; i++) {
      if (!/\s/.test(text[i])) {
        return i;
      }
    }
    return lineStart;
  }

  private getNextWordStart(text: string, pos: number): number {
    let i = pos;
    if (i >= text.length) return text.length;

    const isWord = (c: string) => /[a-zA-Z0-9_]/.test(c);
    const isPunct = (c: string) => !isWord(c) && !/\s/.test(c);

    // If starting on word char, skip through word chars
    if (isWord(text[i])) {
      while (i < text.length && isWord(text[i])) i++;
    } else if (isPunct(text[i])) {
      // If starting on punct, skip through punct chars
      while (i < text.length && isPunct(text[i])) i++;
    }

    // Skip trailing whitespace
    while (i < text.length && /\s/.test(text[i])) i++;
    return i;
  }

  private getPrevWordStart(text: string, pos: number): number {
    let i = pos - 1;
    if (i <= 0) return 0;

    const isWord = (c: string) => /[a-zA-Z0-9_]/.test(c);
    const isPunct = (c: string) => !isWord(c) && !/\s/.test(c);

    // Skip whitespace backwards
    while (i > 0 && /\s/.test(text[i])) i--;

    if (isWord(text[i])) {
      while (i > 0 && isWord(text[i - 1])) i--;
    } else if (isPunct(text[i])) {
      while (i > 0 && isPunct(text[i - 1])) i--;
    }
    return Math.max(0, i);
  }

  private getWordEnd(text: string, pos: number): number {
    let i = pos + 1;
    if (i >= text.length) return text.length - 1;

    const isWord = (c: string) => /[a-zA-Z0-9_]/.test(c);
    const isPunct = (c: string) => !isWord(c) && !/\s/.test(c);

    // Skip whitespace forwards
    while (i < text.length && /\s/.test(text[i])) i++;

    if (i < text.length && isWord(text[i])) {
      while (i + 1 < text.length && isWord(text[i + 1])) i++;
    } else if (i < text.length && isPunct(text[i])) {
      while (i + 1 < text.length && isPunct(text[i + 1])) i++;
    }
    return Math.min(i, text.length - 1);
  }

  private getLineDown(pos: number, count: number): number {
    const text = this.textarea.value;
    const lines = text.split('\n');
    const { line: curLine, col: curCol } = this.getLineAndCol(pos);
    const targetLineIdx = Math.min(lines.length - 1, curLine - 1 + count);

    let charOffset = 0;
    for (let i = 0; i < targetLineIdx; i++) {
      charOffset += lines[i].length + 1;
    }

    const targetLineLen = lines[targetLineIdx].length;
    const maxCol = this.mode === VimMode.NORMAL ? Math.max(0, targetLineLen - 1) : targetLineLen;
    const targetCol = Math.min(curCol - 1, maxCol);
    return charOffset + targetCol;
  }

  private getLineUp(pos: number, count: number): number {
    const text = this.textarea.value;
    const lines = text.split('\n');
    const { line: curLine, col: curCol } = this.getLineAndCol(pos);
    const targetLineIdx = Math.max(0, curLine - 1 - count);

    let charOffset = 0;
    for (let i = 0; i < targetLineIdx; i++) {
      charOffset += lines[i].length + 1;
    }

    const targetLineLen = lines[targetLineIdx].length;
    const maxCol = this.mode === VimMode.NORMAL ? Math.max(0, targetLineLen - 1) : targetLineLen;
    const targetCol = Math.min(curCol - 1, maxCol);
    return charOffset + targetCol;
  }
}
