// Plainpad — Find & Replace Search Engine (FindAndReplaceManager)
// REQ-29: Multi-criteria search, live match counter, active note highlights,
// single/bulk replace, and all-notes excerpt search (SearchResultView).

export type MatchType = 'contains' | 'starts_with' | 'ends_with' | 'exact_match';
export type SearchScope = 'note' | 'all';

export interface MatchRange {
  start: number;
  end: number;
  lineIndex: number;
  text: string;
}

export interface SearchResultItem {
  noteId: string;
  title: string;
  isSlotted: boolean;
  slotIndex: number;
  matchCount: number;
  excerpts: string[];
  firstMatchStart: number;
}

export interface NoteItem {
  id: string;
  content?: string;
  text?: string;
  title?: string;
  is_slotted?: boolean;
  slot_index?: number;
}

export function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

export class FindAndReplaceManager {
  private editor: HTMLTextAreaElement;
  private container: HTMLElement;
  private searchInput: HTMLInputElement;
  private replaceInput: HTMLInputElement;
  private matchTypeSelect: HTMLSelectElement;
  private btnCase: HTMLButtonElement;
  private btnRegex: HTMLButtonElement;
  private btnScopeNote: HTMLButtonElement;
  private btnScopeAll: HTMLButtonElement;
  private btnPrev: HTMLButtonElement;
  private btnNext: HTMLButtonElement;
  private btnClose: HTMLButtonElement;
  private btnReplaceOne: HTMLButtonElement;
  private btnReplaceAll: HTMLButtonElement;
  private matchCountLabel: HTMLElement;
  private resultsList: HTMLElement;
  private replaceRow: HTMLElement;

  // Search State
  public query = '';
  public replacement = '';
  public matchType: MatchType = 'contains';
  public caseSensitive = false;
  public isRegex = false;
  public scope: SearchScope = 'note';

  public matches: MatchRange[] = [];
  public activeMatchIndex = -1;
  public hasRegexError = false;
  public isOpen = false;

  // External Callbacks
  public onHighlightUpdate?: () => void;
  public onTextChange?: (newText: string) => void;
  public onSelectNote?: (noteId: string, isSlotted: boolean, slotIndex: number, offset?: number) => void;
  public onStatusMessage?: (msg: string) => void;
  public fetchAllNotes?: () => Promise<NoteItem[]>;

  constructor(elements: {
    editor: HTMLTextAreaElement;
    container: HTMLElement;
    searchInput: HTMLInputElement;
    replaceInput: HTMLInputElement;
    matchTypeSelect: HTMLSelectElement;
    btnCase: HTMLButtonElement;
    btnRegex: HTMLButtonElement;
    btnScopeNote: HTMLButtonElement;
    btnScopeAll: HTMLButtonElement;
    btnPrev: HTMLButtonElement;
    btnNext: HTMLButtonElement;
    btnClose: HTMLButtonElement;
    btnReplaceOne: HTMLButtonElement;
    btnReplaceAll: HTMLButtonElement;
    matchCountLabel: HTMLElement;
    resultsList: HTMLElement;
    replaceRow: HTMLElement;
  }) {
    this.editor = elements.editor;
    this.container = elements.container;
    this.searchInput = elements.searchInput;
    this.replaceInput = elements.replaceInput;
    this.matchTypeSelect = elements.matchTypeSelect;
    this.btnCase = elements.btnCase;
    this.btnRegex = elements.btnRegex;
    this.btnScopeNote = elements.btnScopeNote;
    this.btnScopeAll = elements.btnScopeAll;
    this.btnPrev = elements.btnPrev;
    this.btnNext = elements.btnNext;
    this.btnClose = elements.btnClose;
    this.btnReplaceOne = elements.btnReplaceOne;
    this.btnReplaceAll = elements.btnReplaceAll;
    this.matchCountLabel = elements.matchCountLabel;
    this.resultsList = elements.resultsList;
    this.replaceRow = elements.replaceRow;

    this.bindEvents();
  }

  private bindEvents(): void {
    this.searchInput.addEventListener('input', () => {
      this.query = this.searchInput.value;
      this.executeSearch();
    });

    this.replaceInput.addEventListener('input', () => {
      this.replacement = this.replaceInput.value;
    });

    this.matchTypeSelect.addEventListener('change', () => {
      this.matchType = this.matchTypeSelect.value as MatchType;
      this.executeSearch();
    });

    this.btnCase.addEventListener('click', () => {
      this.caseSensitive = !this.caseSensitive;
      this.btnCase.classList.toggle('active', this.caseSensitive);
      this.executeSearch();
    });

    this.btnRegex.addEventListener('click', () => {
      this.isRegex = !this.isRegex;
      this.btnRegex.classList.toggle('active', this.isRegex);
      this.executeSearch();
    });

    this.btnScopeNote.addEventListener('click', () => {
      this.setScope('note');
    });

    this.btnScopeAll.addEventListener('click', () => {
      this.setScope('all');
    });

    this.btnPrev.addEventListener('click', () => this.prevMatch());
    this.btnNext.addEventListener('click', () => this.nextMatch());
    this.btnClose.addEventListener('click', () => this.close());

    this.btnReplaceOne.addEventListener('click', () => this.replaceCurrent());
    this.btnReplaceAll.addEventListener('click', () => this.replaceAll());

    // Keyboard navigation within inputs
    this.searchInput.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (e.shiftKey) {
          this.prevMatch();
        } else {
          this.nextMatch();
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        this.close();
      }
    });

    this.replaceInput.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        this.replaceCurrent();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        this.close();
      }
    });
  }

  public setScope(scope: SearchScope): void {
    this.scope = scope;
    if (scope === 'note') {
      this.btnScopeNote.classList.add('active');
      this.btnScopeAll.classList.remove('active');
      this.resultsList.classList.add('hidden');
      this.replaceRow.classList.remove('hidden');
      this.executeSearch();
    } else {
      this.btnScopeAll.classList.add('active');
      this.btnScopeNote.classList.remove('active');
      this.resultsList.classList.remove('hidden');
      this.replaceRow.classList.add('hidden');
      this.executeAllNotesSearch().catch(console.error);
    }
  }

  public open(prefillText?: string): void {
    this.isOpen = true;
    this.container.classList.remove('hidden');

    if (prefillText !== undefined && prefillText.trim().length > 0) {
      this.searchInput.value = prefillText;
      this.query = prefillText;
    } else if (!this.searchInput.value && this.editor.selectionStart !== this.editor.selectionEnd) {
      const selected = this.editor.value.substring(this.editor.selectionStart, this.editor.selectionEnd);
      if (selected && !selected.includes('\n')) {
        this.searchInput.value = selected;
        this.query = selected;
      }
    }

    this.searchInput.focus();
    this.searchInput.select();

    if (this.scope === 'all') {
      this.executeAllNotesSearch().catch(console.error);
    } else {
      this.executeSearch();
    }
  }

  public close(): void {
    this.isOpen = false;
    this.container.classList.add('hidden');
    this.matches = [];
    this.activeMatchIndex = -1;
    this.hasRegexError = false;
    this.searchInput.classList.remove('regex-error');
    if (this.onHighlightUpdate) {
      this.onHighlightUpdate();
    }
    this.editor.focus();
  }

  public toggle(): void {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }

  public isSearching(): boolean {
    return this.isOpen && this.query.length > 0 && !this.hasRegexError && this.scope === 'note';
  }

  public compileRegex(): RegExp | null {
    if (!this.query) return null;

    let source = '';
    const safeQuery = this.isRegex ? this.query : escapeRegex(this.query);

    switch (this.matchType) {
      case 'contains':
        source = safeQuery;
        break;
      case 'starts_with':
        source = `(?:^|\\b)${safeQuery}`;
        break;
      case 'ends_with':
        source = `${safeQuery}(?:$|\\b)`;
        break;
      case 'exact_match':
        source = `\\b${safeQuery}\\b`;
        break;
    }

    let flags = 'g';
    if (!this.caseSensitive) flags += 'i';
    flags += 'm';

    try {
      const regex = new RegExp(source, flags);
      this.hasRegexError = false;
      this.searchInput.classList.remove('regex-error');
      return regex;
    } catch {
      this.hasRegexError = true;
      this.searchInput.classList.add('regex-error');
      return null;
    }
  }

  public executeSearch(): void {
    if (this.scope === 'all') {
      this.executeAllNotesSearch().catch(console.error);
      return;
    }

    const text = this.editor.value;
    const regex = this.compileRegex();

    if (!regex || !this.query) {
      this.matches = [];
      this.activeMatchIndex = -1;
      this.updateMatchCounter();
      if (this.onHighlightUpdate) {
        this.onHighlightUpdate();
      }
      return;
    }

    this.matches = [];
    let match: RegExpExecArray | null;

    while ((match = regex.exec(text)) !== null) {
      const start = match.index;
      const matchText = match[0];
      const end = start + matchText.length;

      // Calculate line index
      const lineIndex = text.substring(0, start).split('\n').length - 1;

      this.matches.push({
        start,
        end,
        lineIndex,
        text: matchText,
      });

      // Avoid infinite loop on zero-length matches
      if (matchText.length === 0) {
        regex.lastIndex++;
      }
    }

    if (this.matches.length > 0) {
      if (this.activeMatchIndex < 0 || this.activeMatchIndex >= this.matches.length) {
        // Find match closest to current selection, or first match
        const cursor = this.editor.selectionStart;
        const closestIdx = this.matches.findIndex((m) => m.start >= cursor);
        this.activeMatchIndex = closestIdx >= 0 ? closestIdx : 0;
      }
      this.focusMatch(this.activeMatchIndex, false);
    } else {
      this.activeMatchIndex = -1;
    }

    this.updateMatchCounter();
    if (this.onHighlightUpdate) {
      this.onHighlightUpdate();
    }
  }

  public updateMatchCounter(): void {
    if (this.hasRegexError) {
      this.matchCountLabel.textContent = 'Regex Error';
      return;
    }
    if (this.matches.length === 0) {
      this.matchCountLabel.textContent = this.query ? '0/0' : '0/0';
      return;
    }
    const current = this.activeMatchIndex >= 0 ? this.activeMatchIndex + 1 : 0;
    this.matchCountLabel.textContent = `${current}/${this.matches.length}`;
  }

  public focusMatch(index: number, scrollIntoView = true): void {
    if (index < 0 || index >= this.matches.length) return;
    this.activeMatchIndex = index;
    const match = this.matches[index];

    this.editor.setSelectionRange(match.start, match.end);

    if (scrollIntoView) {
      // Calculate scroll position to keep match vertically centered
      const textBefore = this.editor.value.substring(0, match.start);
      const lineCount = textBefore.split('\n').length - 1;
      const lineHeight = 24; // baseline height estimate
      const targetScroll = lineCount * lineHeight - this.editor.clientHeight / 2;

      this.editor.scrollTop = Math.max(0, targetScroll);
      this.editor.dispatchEvent(new Event('scroll'));
    }

    this.updateMatchCounter();
    if (this.onHighlightUpdate) {
      this.onHighlightUpdate();
    }
  }

  public nextMatch(): void {
    if (this.matches.length === 0) return;
    const nextIdx = (this.activeMatchIndex + 1) % this.matches.length;
    this.focusMatch(nextIdx, true);
  }

  public prevMatch(): void {
    if (this.matches.length === 0) return;
    const prevIdx = (this.activeMatchIndex - 1 + this.matches.length) % this.matches.length;
    this.focusMatch(prevIdx, true);
  }

  public replaceCurrent(): boolean {
    if (this.activeMatchIndex < 0 || this.matches.length === 0) return false;
    const m = this.matches[this.activeMatchIndex];
    const text = this.editor.value;

    const replacement = this.replaceInput.value;
    const newText = text.substring(0, m.start) + replacement + text.substring(m.end);

    if (this.onTextChange) {
      this.onTextChange(newText);
    } else {
      this.editor.value = newText;
    }

    // Move cursor after replacement
    const newCursor = m.start + replacement.length;
    this.editor.setSelectionRange(newCursor, newCursor);

    this.executeSearch();
    return true;
  }

  public replaceAll(): number {
    const regex = this.compileRegex();
    if (!regex || !this.query) return 0;

    const text = this.editor.value;
    const matches = text.match(regex);
    const count = matches ? matches.length : 0;
    if (count === 0) return 0;

    const replacement = this.replaceInput.value;
    const newText = text.replace(regex, replacement);

    if (this.onTextChange) {
      this.onTextChange(newText);
    } else {
      this.editor.value = newText;
    }

    this.executeSearch();

    const msg = `Replaced ${count} occurrence${count > 1 ? 's' : ''}`;
    if (this.onStatusMessage) {
      this.onStatusMessage(msg);
    }

    return count;
  }

  // Highlights search matches in text for #editor-backdrop
  public formatLineText(line: string, lineStartOffset: number): string {
    if (!this.isSearching() || this.matches.length === 0) {
      return escapeHtml(line);
    }

    const lineEndOffset = lineStartOffset + line.length;
    // Find all matches overlapping this line
    const overlapping = this.matches
      .map((m, idx) => ({ match: m, idx }))
      .filter(({ match }) => match.end > lineStartOffset && match.start < lineEndOffset);

    if (overlapping.length === 0) {
      return escapeHtml(line);
    }

    let resultHtml = '';
    let cursor = 0; // relative to line start

    for (const { match, idx } of overlapping) {
      const relStart = Math.max(0, match.start - lineStartOffset);
      const relEnd = Math.min(line.length, match.end - lineStartOffset);

      if (relStart > cursor) {
        resultHtml += escapeHtml(line.substring(cursor, relStart));
      }

      if (relEnd > relStart) {
        const matchSlice = line.substring(relStart, relEnd);
        const isActive = idx === this.activeMatchIndex;
        const cls = isActive ? 'search-match search-match-active' : 'search-match';
        resultHtml += `<span class="${cls}">${escapeHtml(matchSlice)}</span>`;
      }

      cursor = Math.max(cursor, relEnd);
    }

    if (cursor < line.length) {
      resultHtml += escapeHtml(line.substring(cursor));
    }

    return resultHtml;
  }

  // Multi-note Search across all notes
  public async executeAllNotesSearch(): Promise<void> {
    const regex = this.compileRegex();
    if (!regex || !this.query || !this.fetchAllNotes) {
      this.resultsList.innerHTML = '<div class="search-empty">Type to search across all notes...</div>';
      this.matchCountLabel.textContent = '0/0';
      return;
    }

    this.resultsList.innerHTML = '<div class="search-loading">Searching notes…</div>';

    try {
      const notes = await this.fetchAllNotes();
      const results: SearchResultItem[] = [];
      let totalMatches = 0;

      for (const note of notes) {
        const content = note.content ?? note.text ?? '';
        if (!content) continue;

        const noteRegex = this.compileRegex();
        if (!noteRegex) continue;

        const noteMatches: { start: number; end: number; text: string }[] = [];
        let m: RegExpExecArray | null;

        while ((m = noteRegex.exec(content)) !== null) {
          noteMatches.push({ start: m.index, end: m.index + m[0].length, text: m[0] });
          if (m[0].length === 0) noteRegex.lastIndex++;
        }

        if (noteMatches.length > 0) {
          totalMatches += noteMatches.length;

          // Extract up to 3 excerpts
          const excerpts: string[] = [];
          for (let i = 0; i < Math.min(3, noteMatches.length); i++) {
            const match = noteMatches[i];
            const start = Math.max(0, match.start - 40);
            const end = Math.min(content.length, match.end + 40);

            const before = escapeHtml(content.substring(start, match.start).replace(/\n/g, ' '));
            const matchText = escapeHtml(content.substring(match.start, match.end).replace(/\n/g, ' '));
            const after = escapeHtml(content.substring(match.end, end).replace(/\n/g, ' '));

            const prefix = start > 0 ? '…' : '';
            const suffix = end < content.length ? '…' : '';
            excerpts.push(`${prefix}${before}<mark class="search-match">${matchText}</mark>${after}${suffix}`);
          }

          results.push({
            noteId: note.id,
            title: note.title || (content.split('\n')[0].trim() || 'Untitled Note'),
            isSlotted: note.is_slotted ?? false,
            slotIndex: note.slot_index ?? -1,
            matchCount: noteMatches.length,
            excerpts,
            firstMatchStart: noteMatches[0].start,
          });
        }
      }

      this.matchCountLabel.textContent = `${totalMatches} match${totalMatches !== 1 ? 'es' : ''}`;

      if (results.length === 0) {
        this.resultsList.innerHTML = '<div class="search-empty">No matching notes found</div>';
        return;
      }

      this.resultsList.innerHTML = '';
      for (const res of results) {
        const item = document.createElement('div');
        item.className = 'search-result-item';
        item.dataset.noteId = res.noteId;

        const header = document.createElement('div');
        header.className = 'search-result-header';

        const titleSpan = document.createElement('span');
        titleSpan.className = 'search-result-title';
        titleSpan.textContent = res.title;

        const badgeSpan = document.createElement('span');
        badgeSpan.className = 'search-result-badge';
        badgeSpan.textContent = `${res.matchCount} match${res.matchCount > 1 ? 'es' : ''}`;

        header.appendChild(titleSpan);
        header.appendChild(badgeSpan);
        item.appendChild(header);

        const excerptsContainer = document.createElement('div');
        excerptsContainer.className = 'search-result-excerpts';
        for (const exc of res.excerpts) {
          const excDiv = document.createElement('div');
          excDiv.className = 'search-result-excerpt';
          excDiv.innerHTML = exc;
          excerptsContainer.appendChild(excDiv);
        }
        item.appendChild(excerptsContainer);

        item.addEventListener('click', () => {
          if (this.onSelectNote) {
            this.onSelectNote(res.noteId, res.isSlotted, res.slotIndex, res.firstMatchStart);
          }
          this.setScope('note');
        });

        this.resultsList.appendChild(item);
      }
    } catch (err) {
      console.error('All notes search failed:', err);
      this.resultsList.innerHTML = '<div class="search-empty">Search error</div>';
    }
  }
}
