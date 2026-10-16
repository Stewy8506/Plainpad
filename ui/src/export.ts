// Plainpad — Third-Party Export Suite (REQ-30)
// Integrations: Plain Text, Markdown, Obsidian, Bear, Apple Notes, Notion, Custom URL Template, Bulk ZIP Archive.

export interface ExportOptionsDTO {
  use_first_line_as_title: boolean;
  omit_keywords: boolean;
  convert_to_markdown: boolean;
  convert_to_rich_text: boolean;
}

export interface ExportSettings {
  useFirstLineAsTitle: boolean;
  omitKeywords: boolean;
  plainTextEnabled: boolean;
  markdownEnabled: boolean;
  obsidianEnabled: boolean;
  obsidianVault: string;
  obsidianConvertToMarkdown: boolean;
  bearEnabled: boolean;
  bearConvertToMarkdown: boolean;
  appleNotesEnabled: boolean;
  appleNotesRichText: boolean;
  notionEnabled: boolean;
  notionSaveDatabase: boolean;
  notionDatabaseName: string;
  customEnabled: boolean;
  customName: string;
  customURLTemplate: string;
}

const DEFAULT_SETTINGS: ExportSettings = {
  useFirstLineAsTitle: true,
  omitKeywords: false,
  plainTextEnabled: true,
  markdownEnabled: true,
  obsidianEnabled: true,
  obsidianVault: 'Notes',
  obsidianConvertToMarkdown: true,
  bearEnabled: true,
  bearConvertToMarkdown: true,
  appleNotesEnabled: true,
  appleNotesRichText: false,
  notionEnabled: true,
  notionSaveDatabase: false,
  notionDatabaseName: '',
  customEnabled: false,
  customName: 'Custom App',
  customURLTemplate: 'myapp://create?title={TITLE}&text={CONTENT}&date={DATE}',
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const win = window as any;

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

export class ExportManager {
  private modal: HTMLElement;
  private backdrop: HTMLElement;
  private closeBtn: HTMLButtonElement;
  private activeNoteId = '';
  private activeNoteContent = '';
  public isOpen = false;

  public onStatusMessage?: (msg: string) => void;

  constructor(elements: {
    modal: HTMLElement;
    backdrop: HTMLElement;
    closeBtn: HTMLButtonElement;
  }) {
    this.modal = elements.modal;
    this.backdrop = elements.backdrop;
    this.closeBtn = elements.closeBtn;

    this.bindEvents();
  }

  private bindEvents(): void {
    this.closeBtn.addEventListener('click', () => this.close());
    this.backdrop.addEventListener('click', () => this.close());

    // Bind item buttons inside modal
    const items = this.modal.querySelectorAll<HTMLElement>('.export-menu-item');
    items.forEach((item) => {
      item.addEventListener('click', () => {
        const target = item.dataset.target;
        if (target) {
          this.executeExport(target).catch(console.error);
        }
      });
    });

    window.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.key === 'Escape' && this.isOpen) {
        e.preventDefault();
        this.close();
      }
    });
  }

  public async loadSettings(): Promise<ExportSettings> {
    try {
      const s = (await invoke<Record<string, unknown>>('get_settings')) || {};
      return {
        useFirstLineAsTitle: (s['exportGeneralUseFirstLineAsTitle'] as boolean) ?? DEFAULT_SETTINGS.useFirstLineAsTitle,
        omitKeywords: (s['exportGeneralOmitKeyword'] as boolean) ?? DEFAULT_SETTINGS.omitKeywords,
        plainTextEnabled: (s['exportPlainTextEnabled'] as boolean) ?? DEFAULT_SETTINGS.plainTextEnabled,
        markdownEnabled: (s['exportMarkdownEnabled'] as boolean) ?? DEFAULT_SETTINGS.markdownEnabled,
        obsidianEnabled: (s['exportObsidianEnabled'] as boolean) ?? DEFAULT_SETTINGS.obsidianEnabled,
        obsidianVault: (s['exportObsidianVault'] as string) || DEFAULT_SETTINGS.obsidianVault,
        obsidianConvertToMarkdown: (s['exportObsidianConvertToMarkdown'] as boolean) ?? DEFAULT_SETTINGS.obsidianConvertToMarkdown,
        bearEnabled: (s['exportBearEnabled'] as boolean) ?? DEFAULT_SETTINGS.bearEnabled,
        bearConvertToMarkdown: (s['exportBearConvertToMarkdown'] as boolean) ?? DEFAULT_SETTINGS.bearConvertToMarkdown,
        appleNotesEnabled: (s['exportAppleNotesEnabled'] as boolean) ?? DEFAULT_SETTINGS.appleNotesEnabled,
        appleNotesRichText: (s['exportAppleNotesConvertToRichText'] as boolean) ?? DEFAULT_SETTINGS.appleNotesRichText,
        notionEnabled: (s['exportNotionEnabled'] as boolean) ?? DEFAULT_SETTINGS.notionEnabled,
        notionSaveDatabase: (s['exportNotionSaveInDatabase'] as boolean) ?? DEFAULT_SETTINGS.notionSaveDatabase,
        notionDatabaseName: (s['exportNotionDatabaseName'] as string) || DEFAULT_SETTINGS.notionDatabaseName,
        customEnabled: (s['exportCustomEnabled'] as boolean) ?? DEFAULT_SETTINGS.customEnabled,
        customName: (s['exportCustomName'] as string) || DEFAULT_SETTINGS.customName,
        customURLTemplate: (s['exportCustomURLTemplate'] as string) || DEFAULT_SETTINGS.customURLTemplate,
      };
    } catch {
      return DEFAULT_SETTINGS;
    }
  }

  public async open(noteId: string, noteContent: string, _noteTitle?: string): Promise<void> {
    this.activeNoteId = noteId;
    this.activeNoteContent = noteContent;
    this.isOpen = true;

    // Refresh dynamic labels based on saved settings
    const settings = await this.loadSettings();

    const obsidianLabel = this.modal.querySelector('#export-label-obsidian');
    if (obsidianLabel) {
      obsidianLabel.textContent = settings.obsidianVault ? `Obsidian (${settings.obsidianVault})` : 'Obsidian';
    }

    const customLabel = this.modal.querySelector('#export-label-custom');
    if (customLabel) {
      customLabel.textContent = settings.customName || 'Custom URL Scheme';
    }

    // Toggle disabled status visual indicators if providers are turned off
    this.toggleItemDisabled('obsidian', !settings.obsidianEnabled);
    this.toggleItemDisabled('bear', !settings.bearEnabled);
    this.toggleItemDisabled('apple_notes', !settings.appleNotesEnabled);
    this.toggleItemDisabled('notion', !settings.notionEnabled);
    this.toggleItemDisabled('custom', !settings.customEnabled);

    this.modal.classList.remove('hidden');
  }

  private toggleItemDisabled(target: string, disabled: boolean): void {
    const item = this.modal.querySelector(`.export-menu-item[data-target="${target}"]`);
    if (item) {
      item.classList.toggle('disabled', disabled);
    }
  }

  public close(): void {
    this.isOpen = false;
    this.modal.classList.add('hidden');
  }

  public toggle(noteId: string, noteContent: string, noteTitle?: string): void {
    if (this.isOpen) {
      this.close();
    } else {
      this.open(noteId, noteContent, noteTitle).catch(console.error);
    }
  }

  public async executeExport(target: string): Promise<void> {
    if (target === 'zip') {
      await this.exportBulkZip();
      this.close();
      return;
    }

    if (!this.activeNoteContent && !this.activeNoteId) {
      if (this.onStatusMessage) this.onStatusMessage('No note to export');
      return;
    }

    const settings = await this.loadSettings();

    const optionsDTO: ExportOptionsDTO = {
      use_first_line_as_title: settings.useFirstLineAsTitle,
      omit_keywords: settings.omitKeywords,
      convert_to_markdown: target === 'md'
        || (target === 'obsidian' && settings.obsidianConvertToMarkdown)
        || (target === 'bear' && settings.bearConvertToMarkdown),
      convert_to_rich_text: target === 'apple_notes' && settings.appleNotesRichText,
    };

    try {
      if (target === 'txt' || target === 'md') {
        const kind = target === 'md' ? 'markdown' : 'txt';
        const formatted = await invoke<string>('export_note_to_target', {
          id: this.activeNoteId,
          target,
          options: optionsDTO,
        });

        const filename = await invoke<string>('export_file_name', {
          text: this.activeNoteContent,
          kind,
        });

        // Trigger file download
        const blob = new Blob([formatted || this.activeNoteContent], { type: 'text/plain;charset=utf-8' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = filename || `note.${target}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        if (this.onStatusMessage) {
          this.onStatusMessage(`Exported to ${filename}`);
        }
      } else if (target === 'obsidian' || target === 'bear' || target === 'custom') {
        const uri = await invoke<string>('export_note_to_target', {
          id: this.activeNoteId,
          target,
          options: optionsDTO,
          vault: settings.obsidianVault,
          customTemplate: settings.customURLTemplate,
        });

        if (uri) {
          await invoke('open_external_url', { url: uri });
          const targetName = target === 'obsidian' ? 'Obsidian' : target === 'bear' ? 'Bear' : settings.customName;
          if (this.onStatusMessage) {
            this.onStatusMessage(`Dispatched to ${targetName}`);
          }
        }
      } else if (target === 'apple_notes') {
        const formatted = await invoke<string>('export_note_to_target', {
          id: this.activeNoteId,
          target,
          options: optionsDTO,
        });

        if (formatted) {
          await navigator.clipboard.writeText(formatted);
          if (this.onStatusMessage) {
            this.onStatusMessage('Copied to clipboard for Apple Notes');
          }
        }
      } else if (target === 'notion') {
        const formatted = await invoke<string>('export_note_to_target', {
          id: this.activeNoteId,
          target,
          options: optionsDTO,
          databaseName: settings.notionDatabaseName,
        });

        if (formatted) {
          await navigator.clipboard.writeText(formatted);
          if (this.onStatusMessage) {
            this.onStatusMessage('Copied to clipboard for Notion');
          }
        }
      }
    } catch (err) {
      console.error('Export failed:', err);
      if (this.onStatusMessage) {
        this.onStatusMessage('Export failed');
      }
    }

    this.close();
  }

  public async exportBulkZip(): Promise<void> {
    try {
      const settings = await this.loadSettings();
      const optionsDTO: ExportOptionsDTO = {
        use_first_line_as_title: settings.useFirstLineAsTitle,
        omit_keywords: settings.omitKeywords,
        convert_to_markdown: settings.markdownEnabled,
        convert_to_rich_text: false,
      };

      const zipBytes = await invoke<number[]>('export_all_notes_zip', { options: optionsDTO });
      if (!zipBytes || zipBytes.length === 0) {
        if (this.onStatusMessage) this.onStatusMessage('No notes available for ZIP archive');
        return;
      }

      const u8 = new Uint8Array(zipBytes);
      const blob = new Blob([u8], { type: 'application/zip' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      const now = new Date().toISOString().slice(0, 10);
      a.href = url;
      a.download = `plainpad-notes-export-${now}.zip`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);

      if (this.onStatusMessage) {
        this.onStatusMessage('Exported all notes to ZIP archive');
      }
    } catch (err) {
      console.error('Bulk ZIP export failed:', err);
      if (this.onStatusMessage) {
        this.onStatusMessage('Bulk ZIP export failed');
      }
    }
  }
}
