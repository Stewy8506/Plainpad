// Plainpad — In-Editor Typography Shortcuts, Baseline Caret Nudge, & RTL Engine
// REQ-28: Markdown formatting, heading cycles, sub-pixel cursor alignment, and RTL layout

export type InlineFormatKind = 'bold' | 'italic' | 'strike' | 'underline';

export interface InlineFormatTags {
  open: string;
  close: string;
}

const FORMAT_TAGS: Record<InlineFormatKind, InlineFormatTags> = {
  bold: { open: '**', close: '**' },
  italic: { open: '*', close: '*' },
  strike: { open: '~~', close: '~~' },
  underline: { open: '<u>', close: '</u>' },
};

/**
 * Cycle line prefix: Body -> # (H1) -> ## (H2) -> ### (H3) -> Body.
 * Preserves cursor position within the modified line text.
 */
export function cycleHeadingLevel(text: string, cursorPos: number): { newText: string; newCursorPos: number } {
  const lineStart = text.lastIndexOf('\n', cursorPos - 1) + 1;
  const rawLineEnd = text.indexOf('\n', cursorPos);
  const lineEnd = rawLineEnd === -1 ? text.length : rawLineEnd;
  const lineText = text.substring(lineStart, lineEnd);

  let newLineText = '';
  let delta = 0;

  if (lineText.startsWith('### ')) {
    newLineText = lineText.substring(4);
    delta = -4;
  } else if (lineText.startsWith('## ')) {
    newLineText = '### ' + lineText.substring(3);
    delta = 1;
  } else if (lineText.startsWith('# ')) {
    newLineText = '## ' + lineText.substring(2);
    delta = 1;
  } else {
    newLineText = '# ' + lineText;
    delta = 2;
  }

  const newCursorPos = Math.max(lineStart, Math.min(lineStart + newLineText.length, cursorPos + delta));
  const newText = text.substring(0, lineStart) + newLineText + text.substring(lineEnd);

  return { newText, newCursorPos };
}

/**
 * Wrap or unwrap selection with inline Markdown / HTML formatting tokens.
 * If selection is empty, inserts tag pair and positions cursor in between.
 */
export function applyInlineFormat(
  text: string,
  selStart: number,
  selEnd: number,
  kind: InlineFormatKind,
): { newText: string; newStart: number; newEnd: number } {
  const { open, close } = FORMAT_TAGS[kind];

  // Case 1: Empty selection -> insert token pair and place cursor between
  if (selStart === selEnd) {
    const newText = text.substring(0, selStart) + open + close + text.substring(selEnd);
    const newStart = selStart + open.length;
    return { newText, newStart, newEnd: newStart };
  }

  const selectedText = text.substring(selStart, selEnd);

  // Case 2: Selected text is internally wrapped with open and close tags -> unwrap
  if (
    selectedText.length >= open.length + close.length &&
    selectedText.startsWith(open) &&
    selectedText.endsWith(close)
  ) {
    const unwrapped = selectedText.substring(open.length, selectedText.length - close.length);
    const newText = text.substring(0, selStart) + unwrapped + text.substring(selEnd);
    return {
      newText,
      newStart: selStart,
      newEnd: selStart + unwrapped.length,
    };
  }

  // Case 3: Text immediately outside selection bounds is wrapped with open and close tags -> unwrap
  const hasOuterBefore = selStart >= open.length && text.substring(selStart - open.length, selStart) === open;
  const hasOuterAfter = selEnd + close.length <= text.length && text.substring(selEnd, selEnd + close.length) === close;
  if (hasOuterBefore && hasOuterAfter) {
    const newText = text.substring(0, selStart - open.length) + selectedText + text.substring(selEnd + close.length);
    const newStart = selStart - open.length;
    return {
      newText,
      newStart,
      newEnd: newStart + selectedText.length,
    };
  }

  // Case 4: Not wrapped -> wrap selection
  const wrapped = open + selectedText + close;
  const newText = text.substring(0, selStart) + wrapped + text.substring(selEnd);
  return {
    newText,
    newStart: selStart + open.length,
    newEnd: selStart + open.length + selectedText.length,
  };
}

/**
 * Check whether a line represents a checklist or bullet/numbered list item.
 */
export function isListLine(line: string): boolean {
  const trimmed = line.trimStart();
  if (
    trimmed.startsWith('- [ ]') ||
    trimmed.startsWith('- [x]') ||
    trimmed.startsWith('- [X]') ||
    trimmed.startsWith('* [ ]') ||
    trimmed.startsWith('* [x]') ||
    trimmed.startsWith('* [X]') ||
    trimmed.startsWith('[ ]') ||
    trimmed.startsWith('[x]') ||
    trimmed.startsWith('[X]')
  ) {
    return true;
  }
  if (trimmed.startsWith('- ') || trimmed.startsWith('* ') || trimmed.startsWith('+ ')) {
    return true;
  }
  if (/^\d+\.\s/.test(trimmed)) {
    return true;
  }
  return false;
}

/**
 * Caret Nudge Controller: manages base sub-pixel cursor alignment and dynamic list item baseline compensation.
 */
export class CaretNudgeController {
  private baseNudge = 0;
  private listNudge = 0;
  private isCurrentlyOnList = false;

  constructor(
    private container: HTMLElement,
    private editor: HTMLTextAreaElement,
  ) {
    this.updateCssVariables();
  }

  public setBaseNudge(nudge: number): void {
    this.baseNudge = nudge;
    this.updateCssVariables();
  }

  public setListNudge(nudge: number): void {
    this.listNudge = nudge;
    this.updateCssVariables();
  }

  public getBaseNudge(): number {
    return this.baseNudge;
  }

  public getListNudge(): number {
    return this.listNudge;
  }

  /**
   * Evaluates current cursor line and dynamically applies extra list nudge if on a list item.
   */
  public updateCursorLine(cursorPos: number): void {
    const text = this.editor.value;
    const lineStart = text.lastIndexOf('\n', cursorPos - 1) + 1;
    const rawLineEnd = text.indexOf('\n', cursorPos);
    const lineEnd = rawLineEnd === -1 ? text.length : rawLineEnd;
    const lineText = text.substring(lineStart, lineEnd);

    const onList = isListLine(lineText);
    if (onList !== this.isCurrentlyOnList) {
      this.isCurrentlyOnList = onList;
      this.updateCssVariables();
    }
  }

  private updateCssVariables(): void {
    const appliedListNudge = this.isCurrentlyOnList ? this.listNudge : 0;
    this.container.style.setProperty('--caret-vertical-nudge', `${this.baseNudge}px`);
    this.container.style.setProperty('--caret-vertical-nudge-list', `${this.listNudge}px`);
    this.container.style.setProperty('--caret-vertical-nudge-applied-list', `${appliedListNudge}px`);
  }
}
