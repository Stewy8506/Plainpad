// Plainpad — Rich Inline Elements Engine
// Handles Antilink domain pills, rounded code blocks with Copy buttons, and relational extraction.

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const win = typeof window !== 'undefined' ? (window as any) : {};

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
  return null as unknown as T;
}

export interface AntilinkItem {
  fullUrl: string;
  shortenedUrl: string;
  startIndex: number;
  endIndex: number;
  line: number;
  lineStartIndex: number;
}

export interface CodeBlockItem {
  startLine: number;
  endLine: number;
  language: string;
  code: string;
  startIndex: number;
  endIndex: number;
}

export interface ExtractedChecklistItem {
  content: string;
  checked: boolean;
  index: number;
  line: number;
  responsibilityRangeLocation: number;
  responsibilityRangeLength: number;
}

const URL_REGEX = /https?:\/\/[^\s<>"'{}|\\^`]+/g;

/**
 * Shortens a URL to a compact domain pill label (e.g. "github.com/...").
 */
export function shortenUrl(url: string, disableShortening = false): string {
  if (disableShortening) return url;
  try {
    const parsed = new URL(url);
    let host = parsed.hostname.toLowerCase();
    if (host.startsWith('www.')) {
      host = host.substring(4);
    }
    const hasPathOrQuery = (parsed.pathname && parsed.pathname !== '/') || parsed.search;
    return hasPathOrQuery ? `${host}/...` : host;
  } catch {
    // Regex fallback if URL constructor fails on unconventional protocols
    const match = url.match(/https?:\/\/(?:www\.)?([^/\s]+)(\/.*)?/);
    if (match) {
      const host = match[1];
      const rest = match[2];
      return rest && rest !== '/' ? `${host}/...` : host;
    }
    return url;
  }
}

/**
 * Opens an external URL in the system's default browser.
 */
export function openExternalUrl(url: string): void {
  try {
    invoke('open_external_url', { url }).catch(() => {
      window.open(url, '_blank', 'noopener,noreferrer');
    });
  } catch {
    window.open(url, '_blank', 'noopener,noreferrer');
  }
}

/**
 * Finds all URLs across lines and returns Antilink descriptors.
 */
export function findAntilinks(
  text: string,
  disableShortening = false
): AntilinkItem[] {
  const items: AntilinkItem[] = [];
  const lines = text.split('\n');
  let currentOffset = 0;

  for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
    const line = lines[lineIdx];
    URL_REGEX.lastIndex = 0;
    let match: RegExpExecArray | null;

    while ((match = URL_REGEX.exec(line)) !== null) {
      let fullUrl = match[0];
      // Strip trailing punctuation often typed at end of sentence
      const trailingPunct = /[.,;:!?)\]]+$/;
      if (trailingPunct.test(fullUrl)) {
        fullUrl = fullUrl.replace(trailingPunct, '');
      }

      const matchStart = match.index;
      const shortenedUrl = shortenUrl(fullUrl, disableShortening);

      items.push({
        fullUrl,
        shortenedUrl,
        startIndex: currentOffset + matchStart,
        endIndex: currentOffset + matchStart + fullUrl.length,
        line: lineIdx,
        lineStartIndex: matchStart,
      });
    }

    currentOffset += line.length + 1;
  }

  return items;
}

/**
 * Scans text for fenced code blocks delimited by triple backticks.
 */
export function findCodeBlocks(text: string): CodeBlockItem[] {
  const blocks: CodeBlockItem[] = [];
  const lines = text.split('\n');

  let inBlock = false;
  let startLine = -1;
  let language = '';
  let blockLines: string[] = [];
  let startIndex = 0;
  let currentOffset = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();

    if (!inBlock && trimmed.startsWith('```')) {
      inBlock = true;
      startLine = i;
      language = trimmed.substring(3).trim();
      blockLines = [];
      startIndex = currentOffset;
    } else if (inBlock && trimmed === '```') {
      blocks.push({
        startLine,
        endLine: i,
        language,
        code: blockLines.join('\n'),
        startIndex,
        endIndex: currentOffset + line.length,
      });
      inBlock = false;
      startLine = -1;
      language = '';
      blockLines = [];
    } else if (inBlock) {
      blockLines.push(line);
    }

    currentOffset += line.length + 1;
  }

  return blocks;
}

/**
 * Creates an Antilink pill DOM element for the overlay.
 */
export function createAntilinkElement(
  item: AntilinkItem,
  top: number,
  left: number
): HTMLElement {
  const pill = document.createElement('div');
  pill.className = 'antilink-pill';
  pill.style.top = `${top}px`;
  pill.style.left = `${left}px`;
  pill.title = item.fullUrl;

  const linkIconSvg = `<svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" style="flex-shrink: 0;"><path d="M6.5 9.5l3-3"/><path d="M9 5l1.5-1.5a3 3 0 0 1 4.24 4.24L13.24 9.24a3 3 0 0 1-4.24 0"/><path d="M7 11l-1.5 1.5a3 3 0 0 1-4.24-4.24L2.76 6.76a3 3 0 0 1 4.24 0"/></svg>`;
  const iconSpan = document.createElement('span');
  iconSpan.innerHTML = linkIconSvg;
  iconSpan.className = 'antilink-icon';
  pill.appendChild(iconSpan);

  const labelSpan = document.createElement('span');
  labelSpan.className = 'antilink-label';
  labelSpan.textContent = item.shortenedUrl;
  pill.appendChild(labelSpan);

  pill.addEventListener('click', (e) => {
    e.stopPropagation();
    openExternalUrl(item.fullUrl);
  });

  return pill;
}

/**
 * Creates a rounded code block header overlay element with language tag and Copy Code button.
 */
export function createCodeBlockHeaderElement(
  block: CodeBlockItem,
  top: number,
  right: number
): HTMLElement {
  const header = document.createElement('div');
  header.className = 'code-block-header';
  header.style.top = `${top}px`;
  header.style.right = `${right}px`;

  if (block.language) {
    const langBadge = document.createElement('span');
    langBadge.className = 'code-block-lang';
    langBadge.textContent = block.language.toLowerCase();
    header.appendChild(langBadge);
  }

  const copyBtn = document.createElement('button');
  copyBtn.type = 'button';
  copyBtn.className = 'code-copy-btn';
  copyBtn.textContent = 'Copy Code';

  let resetTimer: ReturnType<typeof setTimeout> | null = null;
  copyBtn.addEventListener('click', async (e) => {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(block.code);
      copyBtn.textContent = 'Copied!';
      copyBtn.classList.add('copied');
      if (resetTimer) clearTimeout(resetTimer);
      resetTimer = setTimeout(() => {
        copyBtn.textContent = 'Copy Code';
        copyBtn.classList.remove('copied');
      }, 1800);
    } catch (err) {
      console.error('Failed to copy code to clipboard', err);
    }
  });

  header.appendChild(copyBtn);
  return header;
}

/**
 * Extracts checklist items from note text for relational SQLite synchronization.
 */
export function extractChecklistItems(text: string): ExtractedChecklistItem[] {
  const items: ExtractedChecklistItem[] = [];
  const lines = text.split('\n');
  let currentOffset = 0;
  let itemIndex = 0;

  const cbRegex = /^\s*(?:[-*+]\s+)?\[([ xX])\]\s*(.*)$/;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const match = line.match(cbRegex);
    if (match) {
      const mark = match[1];
      const content = match[2];
      const checked = mark.toLowerCase() === 'x';
      items.push({
        content,
        checked,
        index: itemIndex++,
        line: i,
        responsibilityRangeLocation: currentOffset,
        responsibilityRangeLength: line.length,
      });
    }
    currentOffset += line.length + 1;
  }

  return items;
}
