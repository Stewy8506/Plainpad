// Plainpad — List Management & Auto-Continuation Engine
// Handles interactive checkboxes, bullet lists, numbered lists, and smart Enter/termination.

export interface ListOptions {
  bulletListsEnabled?: boolean;
  numberedListsEnabled?: boolean;
  checkboxesEnabled?: boolean;
}

export interface CheckboxInfo {
  isCheckbox: boolean;
  isChecked: boolean;
  indent: string;
  bullet: string; // e.g. "- ", "* ", "+ ", or ""
  marker: string; // e.g. "[ ] ", "[x] ", "- [ ] ", etc.
  body: string;
  checkOffset: number; // character offset where the "[ ]" or "[x]" begins in the line
}

const CHECKBOX_BULLET_REGEX = /^(\s*)([-*+]\s+)(\[[ xX]\]\s*)(.*)$/;
const CHECKBOX_PLAIN_REGEX = /^(\s*)(\[[ xX]\]\s*)(.*)$/;
const BULLET_REGEX = /^(\s*)([-*+]\s+)(.*)$/;
const NUMBERED_REGEX = /^(\s*)(\d+)([.)]\s+)(.*)$/;

/**
 * Parses a line to check if it contains a supported markdown checkbox marker.
 */
export function parseCheckboxLine(line: string): CheckboxInfo | null {
  // Check bulleted checkbox: e.g. "  - [ ] Task" or "  * [x] Done"
  const bulletMatch = line.match(CHECKBOX_BULLET_REGEX);
  if (bulletMatch) {
    const indent = bulletMatch[1];
    const bullet = bulletMatch[2];
    const marker = bulletMatch[3];
    const body = bulletMatch[4];
    const isChecked = marker.includes('x') || marker.includes('X');
    return {
      isCheckbox: true,
      isChecked,
      indent,
      bullet,
      marker: bullet + marker,
      body,
      checkOffset: indent.length + bullet.length,
    };
  }

  // Check plain checkbox: e.g. "  [ ] Task" or "[x] Done"
  const plainMatch = line.match(CHECKBOX_PLAIN_REGEX);
  if (plainMatch) {
    const indent = plainMatch[1];
    const marker = plainMatch[2];
    const body = plainMatch[3];
    const isChecked = marker.includes('x') || marker.includes('X');
    return {
      isCheckbox: true,
      isChecked,
      indent,
      bullet: '',
      marker,
      body,
      checkOffset: indent.length,
    };
  }

  return null;
}

/**
 * Toggles the checkbox marker on the specified line index.
 * Preserves exact indentation, bullet styles, and all subsequent text.
 */
export function toggleCheckboxLine(
  text: string,
  lineIndex: number
): { nextText: string; isChecked: boolean } {
  const lines = text.split('\n');
  if (lineIndex < 0 || lineIndex >= lines.length) {
    return { nextText: text, isChecked: false };
  }

  const line = lines[lineIndex];
  const info = parseCheckboxLine(line);
  if (!info) {
    return { nextText: text, isChecked: false };
  }

  const nextChecked = !info.isChecked;
  const nextBracket = nextChecked ? '[x] ' : '[ ] ';

  if (info.bullet) {
    lines[lineIndex] = `${info.indent}${info.bullet}${nextBracket}${info.body}`;
  } else {
    lines[lineIndex] = `${info.indent}${nextBracket}${info.body}`;
  }

  return {
    nextText: lines.join('\n'),
    isChecked: nextChecked,
  };
}

/**
 * Computes list continuation or termination behavior for a line.
 */
export function getContinuationAction(
  fullLine: string,
  options: ListOptions = {}
): { action: 'terminate' | 'continue'; insertion: string } | null {
  const bulletsEnabled = options.bulletListsEnabled !== false;
  const numbersEnabled = options.numberedListsEnabled !== false;
  const checkboxesEnabled = options.checkboxesEnabled !== false;

  // 1. Checkbox item (bulleted or plain)
  const cbInfo = parseCheckboxLine(fullLine);
  if (cbInfo) {
    if (!checkboxesEnabled && !bulletsEnabled) return null;
    const isBlank = cbInfo.body.trim().length === 0;
    if (isBlank) {
      return { action: 'terminate', insertion: '' };
    }
    // Continue with unchecked checkbox
    const nextMarker = cbInfo.bullet ? `${cbInfo.bullet}[ ] ` : '[ ] ';
    return {
      action: 'continue',
      insertion: `\n${cbInfo.indent}${nextMarker}`,
    };
  }

  // 2. Standard bullet list (-, *, +)
  const bulletMatch = fullLine.match(BULLET_REGEX);
  if (bulletMatch && bulletsEnabled) {
    const indent = bulletMatch[1];
    const marker = bulletMatch[2];
    const body = bulletMatch[3];
    const isBlank = body.trim().length === 0;
    if (isBlank) {
      return { action: 'terminate', insertion: '' };
    }
    return {
      action: 'continue',
      insertion: `\n${indent}${marker}`,
    };
  }

  // 3. Numbered list (1., 2), 10.)
  const numMatch = fullLine.match(NUMBERED_REGEX);
  if (numMatch && numbersEnabled) {
    const indent = numMatch[1];
    const digits = numMatch[2];
    const delimiter = numMatch[3]; // e.g. ". " or ") "
    const body = numMatch[4];
    const isBlank = body.trim().length === 0;
    if (isBlank) {
      return { action: 'terminate', insertion: '' };
    }
    const currentNum = parseInt(digits, 10);
    const nextNum = isNaN(currentNum) ? 1 : currentNum + 1;
    return {
      action: 'continue',
      insertion: `\n${indent}${nextNum}${delimiter}`,
    };
  }

  return null;
}

/**
 * Handles Enter key press on an HTMLTextAreaElement.
 * Returns true if the key event was intercepted and handled (preventing default), false otherwise.
 */
export function handleListEnterKey(
  editor: HTMLTextAreaElement,
  options: ListOptions = {}
): boolean {
  const cursorPos = editor.selectionStart;
  if (cursorPos !== editor.selectionEnd) {
    return false; // User has selected a range; do not auto-continue
  }

  const value = editor.value;
  const lineStart = value.lastIndexOf('\n', cursorPos - 1) + 1;
  const rawLineEnd = value.indexOf('\n', cursorPos);
  const lineEnd = rawLineEnd === -1 ? value.length : rawLineEnd;
  const fullLine = value.substring(lineStart, lineEnd);

  const continuation = getContinuationAction(fullLine, options);
  if (!continuation) {
    return false;
  }

  if (continuation.action === 'terminate') {
    // Delete the empty list marker from the current line, terminating the list
    // Replace current line with empty string
    const before = value.substring(0, lineStart);
    const after = value.substring(lineEnd);
    editor.value = before + after;
    editor.selectionStart = editor.selectionEnd = lineStart;
    editor.dispatchEvent(new Event('input', { bubbles: true }));
    return true;
  }

  if (continuation.action === 'continue') {
    const insertion = continuation.insertion;
    let handled = false;
    try {
      handled = document.execCommand('insertText', false, insertion);
    } catch {
      handled = false;
    }

    if (!handled) {
      const before = value.substring(0, cursorPos);
      const after = value.substring(cursorPos);
      editor.value = before + insertion + after;
      const newPos = cursorPos + insertion.length;
      editor.selectionStart = editor.selectionEnd = newPos;
    }

    editor.dispatchEvent(new Event('input', { bubbles: true }));
    return true;
  }

  return false;
}
