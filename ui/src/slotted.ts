// Plainpad — Slotted Notes (Greek Gods System)
// 9 permanent scratchpad slots mapped to Greek deities.

export interface DeitySlot {
  index: number;
  name: string;
  shortcut: string;
}

export const DEITY_SLOTS: readonly DeitySlot[] = [
  { index: 0, name: 'Hermes', shortcut: 'Ctrl+1' },
  { index: 1, name: 'Apollo', shortcut: 'Ctrl+2' },
  { index: 2, name: 'Ares', shortcut: 'Ctrl+3' },
  { index: 3, name: 'Cronus', shortcut: 'Ctrl+4' },
  { index: 4, name: 'Gaia', shortcut: 'Ctrl+5' },
  { index: 5, name: 'Hades', shortcut: 'Ctrl+6' },
  { index: 6, name: 'Ouranos', shortcut: 'Ctrl+7' },
  { index: 7, name: 'Poseidon', shortcut: 'Ctrl+8' },
  { index: 8, name: 'Zeus', shortcut: 'Ctrl+9' },
] as const;

export function createSlottedNoteIconView(
  slot: DeitySlot,
  onClick: (index: number) => void
): HTMLButtonElement {
  const pill = document.createElement('button');
  pill.type = 'button';
  pill.className = 'slotted-pill';
  pill.dataset.slot = String(slot.index);
  pill.title = `${slot.name} (${slot.shortcut})`;
  pill.setAttribute('aria-label', `${slot.name} (${slot.shortcut})`);

  const dot = document.createElement('span');
  dot.className = 'slotted-pill-dot';
  pill.appendChild(dot);

  pill.addEventListener('click', (e) => {
    e.preventDefault();
    onClick(slot.index);
  });

  return pill;
}

export class SlottedNotesBottomBar {
  private container: HTMLElement;
  private onSelectSlot: (index: number) => void;
  private activeSlot: number | null = null;
  private pills: HTMLButtonElement[] = [];

  constructor(container: HTMLElement, onSelectSlot: (index: number) => void) {
    this.container = container;
    this.onSelectSlot = onSelectSlot;
    this.render();
  }

  public render(): void {
    this.container.innerHTML = '';
    this.pills = [];

    for (const slot of DEITY_SLOTS) {
      const pill = createSlottedNoteIconView(slot, (index) => {
        this.onSelectSlot(index);
      });
      this.pills.push(pill);
      this.container.appendChild(pill);
    }

    this.applyActiveState();
  }

  public setActiveSlot(slotIndex: number | null): void {
    if (slotIndex !== null && (slotIndex < 0 || slotIndex >= DEITY_SLOTS.length)) {
      this.activeSlot = null;
    } else {
      this.activeSlot = slotIndex;
    }
    this.applyActiveState();
  }

  public getActiveSlot(): number | null {
    return this.activeSlot;
  }

  private applyActiveState(): void {
    for (const pill of this.pills) {
      const pillSlot = pill.dataset.slot !== undefined ? parseInt(pill.dataset.slot, 10) : -1;
      if (this.activeSlot !== null && pillSlot === this.activeSlot) {
        pill.classList.add('active');
        pill.setAttribute('aria-current', 'true');
      } else {
        pill.classList.remove('active');
        pill.removeAttribute('aria-current');
      }
    }
  }
}
