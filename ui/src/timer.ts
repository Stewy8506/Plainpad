// Plainpad — Timer & Pomodoro Pill UI (TimerView)
// Circular SVG progress, work/break indicators, hover controls, sound alerts, and full-screen overlay.

import {
  initBreakView,
  showBreakTimeView as showBreakTimeViewModule,
  hideBreakTimeView as hideBreakTimeViewModule,
  updateBreakCountdown,
  updateStreakDisplay,
  refreshStreak,
  getHasPeeked,
  resetPeekStatus,
  completeBreakStreak,
  type BreakFact,
} from './break_view.js';

export {
  initBreakView,
  updateBreakCountdown,
  updateStreakDisplay,
  refreshStreak,
  type BreakFact,
};

export type TimerType = 'Stopwatch' | 'Countdown' | 'Pomodoro';

export interface Timer {
  id: number;
  name: string;
  timer_type: TimerType;
  duration_ms: number;
  break_duration_ms: number;
  elapsed_ms: number;
  remaining_ms: number;
  is_work_phase: boolean;
  cycle_count: number;
  is_paused: boolean;
  fired: boolean;
  started_at_ms?: number;
  due_ms?: number;
}

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

async function listen(event: string, cb: (payload: unknown) => void): Promise<void> {
  const tauri: TauriAPI | undefined = win.__TAURI__;
  if (tauri) {
    await tauri.event.listen(event, (ev: { payload: unknown }) => cb(ev.payload));
  }
}

// Circle geometry constants
// outer circleDiameter = 28px, strokeWidth = 2.5px
// radius = (28 - 2.5) / 2 = 12.75px
// circumference = 2 * Math.PI * 12.75 ≈ 80.11px
const CIRCLE_RADIUS = 12.75;
const CIRCUMFERENCE = 2 * Math.PI * CIRCLE_RADIUS; // ~80.1106

let activeTimer: Timer | null = null;
let soundEnabled = true;
let fullScreenAlertEnabled = false;

// DOM references
let timerDisplayEl: HTMLElement | null = null;
let timerPillEl: HTMLElement | null = null;
let ringProgressEl: SVGCircleElement | null = null;
let phaseBadgeEl: HTMLElement | null = null;
let digitsEl: HTMLElement | null = null;
let pauseBtnEl: HTMLButtonElement | null = null;
let restartBtnEl: HTMLButtonElement | null = null;
let stopBtnEl: HTMLButtonElement | null = null;

// Full-screen alert modal references
let alertModalEl: HTMLElement | null = null;
let alertIconEl: HTMLElement | null = null;
let alertHeadlineEl: HTMLElement | null = null;
let alertSummaryEl: HTMLElement | null = null;
let alertActionBtnEl: HTMLButtonElement | null = null;
let alertDismissBtnEl: HTMLButtonElement | null = null;

export async function showBreakTimeView(timer: Timer): Promise<void> {
  await showBreakTimeViewModule(timer.remaining_ms, timer.is_paused);
}

export function hideBreakTimeView(): void {
  hideBreakTimeViewModule();
}

export function playTimerChime(soundFile: string): void {
  if (!soundEnabled) return;
  // Play sound natively through backend — completely avoids WebKitGTK
  // GStreamer crashes in MediaPlayerPrivateGStreamer::createAudioSink()
  invoke('play_sound', { sound: soundFile }).catch(() => {});
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

export function showFullScreenAlert(
  title: string,
  summary: string,
  actionLabel: string,
  icon: string = '⏱'
): void {
  if (!alertModalEl) return;
  if (alertHeadlineEl) alertHeadlineEl.textContent = title;
  if (alertSummaryEl) alertSummaryEl.textContent = summary;
  if (alertIconEl) alertIconEl.textContent = icon;
  if (alertActionBtnEl) alertActionBtnEl.textContent = actionLabel;
  alertModalEl.classList.remove('hidden');
}

export function hideFullScreenAlert(): void {
  if (alertModalEl) {
    alertModalEl.classList.add('hidden');
  }
}

export function clearTimer(): void {
  invoke('exit_break_fullscreen').catch(() => {});
  hideBreakTimeView();
  renderTimer(null);
}

export function renderTimer(timer: Timer | null): void {
  activeTimer = timer;
  if (!timerDisplayEl || !timerPillEl || !ringProgressEl || !phaseBadgeEl || !digitsEl || !pauseBtnEl) {
    return;
  }

  if (!timer || timer.fired) {
    timerDisplayEl.classList.add('hidden');
    timerPillEl.classList.remove('is-paused', 'work-phase', 'break-phase', 'stopwatch-phase');
    return;
  }

  timerDisplayEl.classList.remove('hidden');

  // Reset phase modifier classes
  timerPillEl.classList.remove('is-paused', 'work-phase', 'break-phase', 'stopwatch-phase');

  if (timer.is_paused) {
    timerPillEl.classList.add('is-paused');
    pauseBtnEl.textContent = '▶';
    pauseBtnEl.title = 'Resume timer';
  } else {
    pauseBtnEl.textContent = '⏸';
    pauseBtnEl.title = 'Pause timer';
  }

  // Phase badge & coloring
  if (timer.timer_type === 'Pomodoro') {
    if (timer.is_work_phase) {
      timerPillEl.classList.add('work-phase');
      phaseBadgeEl.textContent = `Work (Cycle ${timer.cycle_count || 1})`;
    } else {
      timerPillEl.classList.add('break-phase');
      phaseBadgeEl.textContent = `Break (Cycle ${timer.cycle_count || 1})`;
    }
  } else if (timer.timer_type === 'Stopwatch') {
    timerPillEl.classList.add('stopwatch-phase');
    phaseBadgeEl.textContent = timer.name && timer.name !== 'Stopwatch' ? timer.name : 'Stopwatch';
  } else {
    timerPillEl.classList.add('work-phase');
    phaseBadgeEl.textContent = timer.name || 'Timer';
  }

  // Time digits
  const msToDisplay = timer.timer_type === 'Stopwatch' ? timer.elapsed_ms : timer.remaining_ms;
  digitsEl.textContent = formatTimeDigits(msToDisplay);

  // Circular progress SVG
  ringProgressEl.style.strokeDasharray = `${CIRCUMFERENCE}`;

  if (timer.timer_type === 'Stopwatch') {
    ringProgressEl.style.strokeDashoffset = '0';
  } else if (timer.timer_type === 'Pomodoro') {
    const totalPhaseMs = timer.is_work_phase
      ? (timer.duration_ms > 0 ? timer.duration_ms : 1)
      : (timer.break_duration_ms > 0 ? timer.break_duration_ms : 1);
    const progress = Math.min(1, Math.max(0, timer.remaining_ms / totalPhaseMs));
    const offset = CIRCUMFERENCE * (1 - progress);
    ringProgressEl.style.strokeDashoffset = `${offset.toFixed(2)}`;
  } else {
    // Countdown
    const totalMs = timer.duration_ms > 0 ? timer.duration_ms : 1;
    const progress = Math.min(1, Math.max(0, timer.remaining_ms / totalMs));
    const offset = CIRCUMFERENCE * (1 - progress);
    ringProgressEl.style.strokeDashoffset = `${offset.toFixed(2)}`;
  }

  // Synchronize Break Time View countdown if Pomodoro break is active
  if (timer.timer_type === 'Pomodoro' && !timer.is_work_phase && !timer.fired) {
    updateBreakCountdown(timer.remaining_ms, timer.is_paused);
  }
}

export async function initTimerUI(): Promise<void> {
  timerDisplayEl = document.getElementById('timer-display');
  if (!timerDisplayEl) return;

  // Render Timer Pill structure inside #timer-display
  timerDisplayEl.innerHTML = `
    <div id="timer-pill" class="timer-pill">
      <div class="timer-progress-ring">
        <svg width="28" height="28" viewBox="0 0 28 28">
          <circle class="ring-bg" cx="14" cy="14" r="${CIRCLE_RADIUS}" />
          <circle class="ring-progress" cx="14" cy="14" r="${CIRCLE_RADIUS}" />
        </svg>
      </div>
      <div class="timer-meta">
        <span class="timer-phase-badge"></span>
        <span class="timer-digits">00:00</span>
      </div>
      <div class="timer-controls">
        <button class="timer-btn timer-btn-pause" type="button" title="Pause / Resume">⏸</button>
        <button class="timer-btn timer-btn-restart" type="button" title="Restart">↺</button>
        <button class="timer-btn timer-btn-stop" type="button" title="Stop">✕</button>
      </div>
    </div>
  `;

  timerPillEl = document.getElementById('timer-pill');
  ringProgressEl = timerPillEl?.querySelector('.ring-progress') as SVGCircleElement | null;
  phaseBadgeEl = timerPillEl?.querySelector('.timer-phase-badge') as HTMLElement | null;
  digitsEl = timerPillEl?.querySelector('.timer-digits') as HTMLElement | null;
  pauseBtnEl = timerPillEl?.querySelector('.timer-btn-pause') as HTMLButtonElement | null;
  restartBtnEl = timerPillEl?.querySelector('.timer-btn-restart') as HTMLButtonElement | null;
  stopBtnEl = timerPillEl?.querySelector('.timer-btn-stop') as HTMLButtonElement | null;

  // Full-screen alert modal elements
  alertModalEl = document.getElementById('timer-fullscreen-alert');
  if (alertModalEl) {
    alertIconEl = document.getElementById('timer-alert-icon');
    alertHeadlineEl = document.getElementById('timer-alert-headline');
    alertSummaryEl = document.getElementById('timer-alert-summary');
    alertActionBtnEl = document.getElementById('timer-alert-action-btn') as HTMLButtonElement | null;
    alertDismissBtnEl = document.getElementById('timer-alert-dismiss-btn') as HTMLButtonElement | null;

    alertActionBtnEl?.addEventListener('click', () => {
      hideFullScreenAlert();
    });
    alertDismissBtnEl?.addEventListener('click', () => {
      hideFullScreenAlert();
    });
    alertModalEl.addEventListener('click', (e) => {
      if (e.target === alertModalEl) {
        hideFullScreenAlert();
      }
    });
  }

  // Handle global escape/enter to close fullscreen alert modal
  window.addEventListener('keydown', (e) => {
    if (alertModalEl && !alertModalEl.classList.contains('hidden')) {
      if (e.key === 'Escape' || e.key === 'Enter') {
        e.preventDefault();
        hideFullScreenAlert();
      }
    }
  });

  // Initialize High-Fidelity Break Time View
  initBreakView({
    onCancelTimer: () => {
      renderTimer(null);
    },
    onTogglePause: async () => {
      try {
        const updated = await invoke<Timer | null>('pause_resume_timer');
        if (updated) {
          renderTimer(updated);
          updateBreakCountdown(updated.remaining_ms, updated.is_paused);
        }
      } catch (err) {
        console.error('[Timer] Failed to toggle break pause/resume:', err);
      }
    },
  });

  // Clicking the pill itself toggles pause/resume
  timerPillEl?.addEventListener('click', async (e) => {
    if ((e.target as HTMLElement).closest('.timer-controls')) return;
    try {
      const updated = await invoke<Timer | null>('pause_resume_timer');
      if (updated) renderTimer(updated);
    } catch (err) {
      console.error(err);
    }
  });

  pauseBtnEl?.addEventListener('click', async (e) => {
    e.stopPropagation();
    try {
      const updated = await invoke<Timer | null>('pause_resume_timer');
      if (updated) renderTimer(updated);
    } catch (err) {
      console.error(err);
    }
  });

  restartBtnEl?.addEventListener('click', async (e) => {
    e.stopPropagation();
    try {
      const updated = await invoke<Timer | null>('restart_timer');
      if (updated) renderTimer(updated);
    } catch (err) {
      console.error(err);
    }
  });

  stopBtnEl?.addEventListener('click', async (e) => {
    e.stopPropagation();
    try {
      await invoke('exit_break_fullscreen');
    } catch (_) {}
    try {
      await invoke<boolean>('cancel_timer');
      hideBreakTimeView();
      renderTimer(null);
    } catch (err) {
      console.error(err);
    }
  });

  // Fetch initial preferences
  try {
    const settings = await invoke<Record<string, unknown>>('get_settings');
    if (settings) {
      if (typeof settings.timerCountdownPlaySound === 'boolean') {
        soundEnabled = settings.timerCountdownPlaySound;
      } else if (typeof settings.timer_sound === 'boolean') {
        soundEnabled = settings.timer_sound;
      }
      if (typeof settings.timerPomoShowFullScreenAlert === 'boolean') {
        fullScreenAlertEnabled = settings.timerPomoShowFullScreenAlert;
      }
    }
  } catch (err) {
    console.debug('[Timer] Could not load initial timer settings:', err);
  }

  // Listen for real-time preferences updates
  await listen('setting-changed', (payload: unknown) => {
    const p = payload as { key: string; value: unknown };
    if (!p || !p.key) return;
    if (p.key === 'timerCountdownPlaySound' || p.key === 'timer_sound') {
      soundEnabled = !!p.value;
    } else if (p.key === 'timerPomoShowFullScreenAlert') {
      fullScreenAlertEnabled = !!p.value;
    }
  });

  // Listen for backend timer events
  await listen('timer-tick', (payload: unknown) => {
    const t = payload as Timer;
    renderTimer(t);
  });

  await listen('timer-stopped', async () => {
    try {
      await invoke('exit_break_fullscreen');
    } catch (_) {}
    hideBreakTimeView();
    renderTimer(null);
  });

  await listen('timer-fired', (payload: unknown) => {
    const p = payload as { id: number; name: string; sound: string; overdue: boolean };

    // 1. Immediately update visual digits to 00:00 and ring to completed
    if (digitsEl) {
      digitsEl.textContent = '00:00';
    }
    if (ringProgressEl) {
      ringProgressEl.style.strokeDashoffset = '0';
    }

    // 2. Pulse animation celebration
    if (timerPillEl) {
      timerPillEl.classList.add('timer-celebrate');
    }

    // 3. Play chime through native OS player
    playTimerChime(p.sound || 'countdowndone.wav');

    // 4. Alert modal if enabled
    if (fullScreenAlertEnabled) {
      showFullScreenAlert(
        "Time's Up!",
        p.overdue ? `${p.name || 'Timer'} (missed while away)` : `${p.name || 'Timer'} finished`,
        'Dismiss',
        '⏱'
      );
    }

    // 5. Cleanly hide the pill after celebration (3 seconds)
    setTimeout(() => {
      timerPillEl?.classList.remove('timer-celebrate');
      if (activeTimer?.id === p.id) {
        renderTimer(null);
      }
    }, 3000);
  });

  await listen('timer-phase-transition', async (payload: unknown) => {
    const p = payload as { id: number; is_work_phase: boolean; cycle_count: number; sound: string };
    if (p.is_work_phase) {
      if (!getHasPeeked()) {
        await completeBreakStreak();
      }
      try {
        await invoke('exit_break_fullscreen');
      } catch (err) {
        console.error('[Timer] Failed to exit break fullscreen:', err);
      }
      hideBreakTimeView();
      playTimerChime(p.sound || 'backtowork.m4a');
      if (fullScreenAlertEnabled) {
        showFullScreenAlert(
          'Break Finished!',
          `Starting Work Cycle ${p.cycle_count}`,
          'Resume Work',
          '⚡'
        );
      }
    } else {
      resetPeekStatus();
      playTimerChime(p.sound || 'workdone.m4a');
      try {
        await invoke('enter_break_fullscreen');
      } catch (err) {
        console.error('[Timer] Failed to enter break fullscreen:', err);
      }
      try {
        const active = await invoke<Timer | null>('get_active_timer');
        if (active) {
          await showBreakTimeView(active);
        }
      } catch (err) {
        console.error('[Timer] Failed to show break view on phase transition:', err);
      }
    }
  });

  // Restore active timer state on startup
  try {
    const active = await invoke<Timer | null>('get_active_timer');
    if (active) {
      renderTimer(active);
      if (active.timer_type === 'Pomodoro' && !active.is_work_phase && !active.fired) {
        await invoke('enter_break_fullscreen').catch(() => {});
        await showBreakTimeView(active);
      }
    } else {
      renderTimer(null);
    }
  } catch (err) {
    console.debug('[Timer] Could not get active timer on startup:', err);
  }
}
