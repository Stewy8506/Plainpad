// Plainpad — Pomodoro Break Screen & Fullscreen Takeover (BreakTimeView)
// Frosted glass atmospheric takeover, offline trivia/mindfulness fact catalog,
// "Sneak a peek" temporary desktop exposure, and SQLite No-Peek Streak engine.

export interface BreakFact {
  category: string;
  text: string;
}

export const DEFAULT_BREAK_FACTS: BreakFact[] = [
  { category: 'WILDLIFE FACT-FILE', text: 'Ostriches can run faster than horses, reaching speeds up to 43 miles per hour (70 km/h).' },
  { category: 'WILDLIFE FACT-FILE', text: "Sea otters hold hands while sleeping so they don't drift away in ocean currents." },
  { category: 'WILDLIFE FACT-FILE', text: 'Octopuses have three hearts, nine brains, and blue blood based on copper instead of iron.' },
  { category: 'SPACE TRIVIA', text: 'A day on Venus lasts longer than an entire Venusian year.' },
  { category: 'MINDFUL MOMENT', text: 'Take a deep breath. Soften your jaw, drop your shoulders away from your ears, and release your brow.' },
  { category: 'MINDFUL MOMENT', text: 'Gently look away from screens into the distance for 20 seconds to relieve eye strain.' },
  { category: 'SPACE TRIVIA', text: 'Neutron stars can spin at a rate of 600 rotations per second.' },
  { category: 'WILDLIFE FACT-FILE', text: 'Crows can recognize individual human faces and remember them for years.' },
];

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
  return null as unknown as T;
}

// Internal state
let hasPeekedThisBreak = false;
let breakFacts: BreakFact[] = [];
let isInitialized = false;

// DOM references
let breakOverlayEl: HTMLElement | null = null;
let breakCategoryBadgeEl: HTMLElement | null = null;
let breakFactTextEl: HTMLElement | null = null;
let breakCountdownDigitsEl: HTMLElement | null = null;
let breakPauseBtnEl: HTMLButtonElement | null = null;
let breakSneakBtnEl: HTMLButtonElement | null = null;
let breakEndBtnEl: HTMLButtonElement | null = null;
let breakStreakCurrentEl: HTMLElement | null = null;
let breakStreakBestEl: HTMLElement | null = null;
let peekBannerEl: HTMLElement | null = null;
let peekReturnBtnEl: HTMLButtonElement | null = null;

// Callbacks
let onCancelTimerCb: (() => void) | null = null;
let onTogglePauseCb: (() => Promise<void> | void) | null = null;

export function getHasPeeked(): boolean {
  return hasPeekedThisBreak;
}

export function resetPeekStatus(): void {
  hasPeekedThisBreak = false;
}

export function setHasPeeked(val: boolean): void {
  hasPeekedThisBreak = val;
}

export function formatTimeDigits(ms: number): string {
  const safeMs = Math.max(0, ms);
  const totalSeconds = Math.ceil(safeMs / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  }
  return `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
}

export function updateStreakDisplay(current: number, best: number): void {
  if (breakStreakCurrentEl) breakStreakCurrentEl.textContent = String(current);
  if (breakStreakBestEl) breakStreakBestEl.textContent = String(best);
}

export async function refreshStreak(): Promise<void> {
  try {
    const res = await invoke<[number, number]>('get_no_peek_streak');
    if (Array.isArray(res) && res.length >= 2) {
      updateStreakDisplay(res[0], res[1]);
    }
  } catch (err) {
    console.debug('[BreakView] Could not refresh streak:', err);
  }
}

export async function recordPeekPenalty(): Promise<void> {
  hasPeekedThisBreak = true;
  try {
    const res = await invoke<[number, number]>('record_peek_penalty');
    if (Array.isArray(res) && res.length >= 2) {
      updateStreakDisplay(res[0], res[1]);
    }
  } catch (err) {
    console.error('[BreakView] Failed to record peek penalty:', err);
  }
}

export async function completeBreakStreak(): Promise<void> {
  try {
    const res = await invoke<[number, number]>('complete_break_streak');
    if (Array.isArray(res) && res.length >= 2) {
      updateStreakDisplay(res[0], res[1]);
    }
  } catch (err) {
    console.error('[BreakView] Failed to increment break streak:', err);
  }
}

export function updateBreakCountdown(remainingMs: number, isPaused: boolean): void {
  if (breakCountdownDigitsEl) {
    breakCountdownDigitsEl.textContent = formatTimeDigits(remainingMs);
  }
  if (breakPauseBtnEl) {
    breakPauseBtnEl.textContent = isPaused ? '▶' : '⏸';
    breakPauseBtnEl.title = isPaused ? 'Resume break' : 'Pause break';
  }
}

export async function showBreakTimeView(remainingMs: number, isPaused: boolean = false): Promise<void> {
  if (!breakOverlayEl) {
    breakOverlayEl = document.getElementById('break-time-view') || document.getElementById('break-time-overlay');
  }
  if (!breakOverlayEl) return;

  hasPeekedThisBreak = false;
  breakOverlayEl.classList.remove('is-peeking');
  if (peekBannerEl) peekBannerEl.classList.add('hidden');

  const catalog = breakFacts.length > 0 ? breakFacts : DEFAULT_BREAK_FACTS;
  const randomFact = catalog[Math.floor(Math.random() * catalog.length)];
  if (breakCategoryBadgeEl) breakCategoryBadgeEl.textContent = randomFact.category;
  if (breakFactTextEl) breakFactTextEl.textContent = randomFact.text;

  updateBreakCountdown(remainingMs, isPaused);
  await refreshStreak();

  breakOverlayEl.classList.remove('hidden');
}

export function hideBreakTimeView(): void {
  if (breakOverlayEl) {
    breakOverlayEl.classList.add('hidden');
    breakOverlayEl.classList.remove('is-peeking');
  }
  if (peekBannerEl) {
    peekBannerEl.classList.add('hidden');
  }
}

export function enterPeekMode(): void {
  hasPeekedThisBreak = true;
  recordPeekPenalty().catch(console.error);
  if (breakOverlayEl) {
    breakOverlayEl.classList.add('is-peeking');
  }
  if (peekBannerEl) {
    peekBannerEl.classList.remove('hidden');
  }
}

export function exitPeekMode(): void {
  if (breakOverlayEl) {
    breakOverlayEl.classList.remove('is-peeking');
  }
  if (peekBannerEl) {
    peekBannerEl.classList.add('hidden');
  }
}

export async function endPomodoroSession(): Promise<void> {
  try {
    await invoke('cancel_timer');
  } catch (err) {
    console.error('[BreakView] Failed to cancel timer:', err);
  }
  try {
    await invoke('exit_break_fullscreen');
  } catch (err) {
    console.error('[BreakView] Failed to exit break fullscreen:', err);
  }
  hideBreakTimeView();
  if (onCancelTimerCb) {
    onCancelTimerCb();
  }
}

export function initBreakView(options?: {
  onCancelTimer?: () => void;
  onTogglePause?: () => Promise<void> | void;
}): void {
  if (options?.onCancelTimer) onCancelTimerCb = options.onCancelTimer;
  if (options?.onTogglePause) onTogglePauseCb = options.onTogglePause;

  breakOverlayEl = document.getElementById('break-time-view') || document.getElementById('break-time-overlay');
  if (!breakOverlayEl) return;

  breakCategoryBadgeEl = document.getElementById('break-category-badge');
  breakFactTextEl = document.getElementById('break-fact-text');
  breakCountdownDigitsEl = document.getElementById('break-countdown-digits');
  breakPauseBtnEl = document.getElementById('break-pause-btn') as HTMLButtonElement | null;
  breakSneakBtnEl = document.getElementById('break-sneak-btn') as HTMLButtonElement | null;
  breakEndBtnEl = document.getElementById('break-end-btn') as HTMLButtonElement | null;
  breakStreakCurrentEl = document.getElementById('break-streak-current');
  breakStreakBestEl = document.getElementById('break-streak-best');
  peekBannerEl = document.getElementById('peek-banner');
  peekReturnBtnEl = document.getElementById('peek-return-btn') as HTMLButtonElement | null;

  if (isInitialized) return;
  isInitialized = true;

  // 1. "Sneak a peek" button handler
  breakSneakBtnEl?.addEventListener('click', (e) => {
    e.stopPropagation();
    enterPeekMode();
  });

  // 2. Return from peek button
  peekReturnBtnEl?.addEventListener('click', (e) => {
    e.stopPropagation();
    exitPeekMode();
  });

  // 3. "End Pomodoro" button handler
  breakEndBtnEl?.addEventListener('click', async (e) => {
    e.stopPropagation();
    await endPomodoroSession();
  });

  // 4. Pause / Resume break countdown button
  breakPauseBtnEl?.addEventListener('click', async (e) => {
    e.stopPropagation();
    if (onTogglePauseCb) {
      await onTogglePauseCb();
    } else {
      try {
        await invoke('pause_resume_timer');
      } catch (err) {
        console.error('[BreakView] Failed to toggle pause:', err);
      }
    }
  });

  // 5. Global Escape key to return from peek
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && breakOverlayEl && breakOverlayEl.classList.contains('is-peeking')) {
      e.preventDefault();
      exitPeekMode();
    }
  });

  // 6. Load facts catalog
  invoke<BreakFact[]>('get_break_facts')
    .then((facts) => {
      if (facts && facts.length > 0) {
        breakFacts = facts;
      }
    })
    .catch((err) => {
      console.debug('[BreakView] Could not load facts catalog:', err);
    });

  // 7. Load initial streak
  refreshStreak().catch(() => {});
}
