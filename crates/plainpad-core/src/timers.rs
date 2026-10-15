//! Timers: Stopwatches, Countdowns, and Pomodoro cycles started from plain phrases,
//! persisted to disk so they survive window close AND app restart (SPEC §6, PRD R-2).
//! A watcher thread advances timers, handles phase transitions, and emits events.

use crate::now_ms;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimerType {
    Stopwatch,
    Countdown,
    Pomodoro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerControlAction {
    PauseResume,
    Restart,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimerPhrase {
    StartStopwatch,
    StartCountdown { duration_ms: u64, name: String },
    StartPomodoro { work_ms: u64, break_ms: u64, name: String },
    Control(TimerControlAction),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Timer {
    pub id: u64,
    pub name: String,
    pub timer_type: TimerType,
    pub duration_ms: u64,
    pub break_duration_ms: u64,
    pub elapsed_ms: u64,
    pub remaining_ms: u64,
    pub is_work_phase: bool,
    pub cycle_count: u32,
    pub is_paused: bool,
    pub started_at_ms: u64,
    pub due_ms: u64,
    pub last_tick_ms: u64,
    pub fired: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum TimerEvent {
    Tick(Timer),
    Fired { id: u64, name: String, sound: String, overdue: bool },
    PhaseTransition { id: u64, is_work_phase: bool, cycle_count: u32, sound: String },
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct FiredTimer {
    pub id: u64,
    pub name: String,
    pub overdue: bool,
}

struct Inner {
    timers: Vec<Timer>,
    next_id: u64,
}

/// Persistent timer engine.
pub struct TimerEngine {
    state: Arc<Mutex<Inner>>,
    path: PathBuf,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

impl TimerEngine {
    /// Load persisted timers; overdue timers fire immediately.
    pub fn load(path: &PathBuf) -> (Self, Vec<FiredTimer>) {
        let timers: Vec<Timer> = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let now = now_ms();
        let mut overdue = Vec::new();
        let mut timers: Vec<Timer> = timers
            .into_iter()
            .map(|mut t| {
                if !t.fired && t.timer_type != TimerType::Stopwatch && !t.is_paused && t.due_ms <= now {
                    t.fired = true;
                    t.remaining_ms = 0;
                    overdue.push(FiredTimer { id: t.id, name: t.name.clone(), overdue: true });
                }
                t.last_tick_ms = now;
                t
            })
            .collect();
        let next_id = timers.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let engine = Self {
            state: Arc::new(Mutex::new(Inner { timers: std::mem::take(&mut timers), next_id })),
            path: path.clone(),
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        engine.persist();
        (engine, overdue)
    }

    fn persist(&self) {
        let guard = self.state.lock().unwrap();
        if let Ok(bytes) = serde_json::to_vec_pretty(&guard.timers) {
            let tmp = self.path.with_extension(format!("tmp{}", std::process::id()));
            if std::fs::write(&tmp, &bytes).is_ok() {
                std::fs::rename(&tmp, &self.path).ok();
            }
        }
    }

    /// Start a stopwatch counting up from 00:00.
    pub fn start_stopwatch(&self, name: &str) -> Timer {
        let now = now_ms();
        let mut g = self.state.lock().unwrap();
        let id = g.next_id;
        g.next_id += 1;
        let timer = Timer {
            id,
            name: if name.trim().is_empty() { "Stopwatch".into() } else { name.trim().into() },
            timer_type: TimerType::Stopwatch,
            duration_ms: 0,
            break_duration_ms: 0,
            elapsed_ms: 0,
            remaining_ms: 0,
            is_work_phase: true,
            cycle_count: 0,
            is_paused: false,
            started_at_ms: now,
            due_ms: 0,
            last_tick_ms: now,
            fired: false,
        };
        g.timers.retain(|t| t.fired);
        g.timers.push(timer.clone());
        drop(g);
        self.persist();
        timer
    }

    /// Start a countdown timer.
    pub fn start_countdown(&self, name: &str, duration_ms: u64) -> Timer {
        let now = now_ms();
        let mut g = self.state.lock().unwrap();
        let id = g.next_id;
        g.next_id += 1;
        let timer = Timer {
            id,
            name: if name.trim().is_empty() { "Timer".into() } else { name.trim().into() },
            timer_type: TimerType::Countdown,
            duration_ms,
            break_duration_ms: 0,
            elapsed_ms: 0,
            remaining_ms: duration_ms,
            is_work_phase: true,
            cycle_count: 0,
            is_paused: false,
            started_at_ms: now,
            due_ms: now + duration_ms,
            last_tick_ms: now,
            fired: false,
        };
        g.timers.retain(|t| t.fired);
        g.timers.push(timer.clone());
        drop(g);
        self.persist();
        timer
    }

    /// Start a Pomodoro timer with work and break intervals.
    pub fn start_pomodoro(&self, name: &str, work_ms: u64, break_ms: u64) -> Timer {
        let now = now_ms();
        let mut g = self.state.lock().unwrap();
        let id = g.next_id;
        g.next_id += 1;
        let timer = Timer {
            id,
            name: if name.trim().is_empty() { "Pomodoro".into() } else { name.trim().into() },
            timer_type: TimerType::Pomodoro,
            duration_ms: work_ms,
            break_duration_ms: break_ms,
            elapsed_ms: 0,
            remaining_ms: work_ms,
            is_work_phase: true,
            cycle_count: 0,
            is_paused: false,
            started_at_ms: now,
            due_ms: now + work_ms,
            last_tick_ms: now,
            fired: false,
        };
        g.timers.retain(|t| t.fired);
        g.timers.push(timer.clone());
        drop(g);
        self.persist();
        timer
    }

    /// Start timer based on natural phrase parser.
    pub fn start_with_phrase(&self, phrase: &str) -> Option<Timer> {
        match parse_timer_phrase(phrase)? {
            TimerPhrase::StartStopwatch => Some(self.start_stopwatch("Stopwatch")),
            TimerPhrase::StartCountdown { duration_ms, name } => Some(self.start_countdown(&name, duration_ms)),
            TimerPhrase::StartPomodoro { work_ms, break_ms, name } => Some(self.start_pomodoro(&name, work_ms, break_ms)),
            TimerPhrase::Control(TimerControlAction::PauseResume) => self.pause_resume(None),
            TimerPhrase::Control(TimerControlAction::Restart) => self.restart(None),
            TimerPhrase::Control(TimerControlAction::Stop) => {
                let _ = self.stop(None);
                None
            }
        }
    }

    /// Backwards compatible simple start.
    pub fn start(&self, name: &str, duration_ms: u64) -> u64 {
        self.start_countdown(name, duration_ms).id
    }

    /// Toggle pause / resume on active timer or specific ID.
    pub fn pause_resume(&self, id: Option<u64>) -> Option<Timer> {
        let now = now_ms();
        let mut g = self.state.lock().unwrap();
        let timer = if let Some(id) = id {
            g.timers.iter_mut().find(|t| t.id == id && !t.fired)
        } else {
            g.timers.iter_mut().find(|t| !t.fired)
        }?;
        timer.is_paused = !timer.is_paused;
        timer.last_tick_ms = now;
        if !timer.is_paused && timer.timer_type != TimerType::Stopwatch {
            timer.due_ms = now + timer.remaining_ms;
        }
        let updated = timer.clone();
        drop(g);
        self.persist();
        Some(updated)
    }

    /// Pause the active timer if one is running and not paused.
    pub fn pause_active(&self) -> Option<Timer> {
        let now = now_ms();
        let mut g = self.state.lock().unwrap();
        let timer = g.timers.iter_mut().find(|t| !t.fired)?;
        if !timer.is_paused {
            timer.is_paused = true;
            timer.last_tick_ms = now;
            let updated = timer.clone();
            drop(g);
            self.persist();
            return Some(updated);
        }
        Some(timer.clone())
    }

    /// Resume the active timer if one is paused.
    pub fn resume_active(&self) -> Option<Timer> {
        let now = now_ms();
        let mut g = self.state.lock().unwrap();
        let timer = g.timers.iter_mut().find(|t| !t.fired)?;
        if timer.is_paused {
            timer.is_paused = false;
            timer.last_tick_ms = now;
            if timer.timer_type != TimerType::Stopwatch {
                timer.due_ms = now + timer.remaining_ms;
            }
            let updated = timer.clone();
            drop(g);
            self.persist();
            return Some(updated);
        }
        Some(timer.clone())
    }

    /// Restart timer from its starting parameters.
    pub fn restart(&self, id: Option<u64>) -> Option<Timer> {
        let now = now_ms();
        let mut g = self.state.lock().unwrap();
        let timer = if let Some(id) = id {
            g.timers.iter_mut().find(|t| t.id == id)
        } else {
            g.timers.iter_mut().find(|t| !t.fired)
        }?;
        timer.fired = false;
        timer.is_paused = false;
        timer.started_at_ms = now;
        timer.last_tick_ms = now;
        timer.elapsed_ms = 0;
        match timer.timer_type {
            TimerType::Stopwatch => {
                timer.remaining_ms = 0;
                timer.due_ms = 0;
            }
            TimerType::Countdown => {
                timer.remaining_ms = timer.duration_ms;
                timer.due_ms = now + timer.duration_ms;
            }
            TimerType::Pomodoro => {
                timer.is_work_phase = true;
                timer.remaining_ms = timer.duration_ms;
                timer.due_ms = now + timer.duration_ms;
            }
        }
        let updated = timer.clone();
        drop(g);
        self.persist();
        Some(updated)
    }

    /// Stop and remove timer.
    pub fn stop(&self, id: Option<u64>) -> bool {
        let mut g = self.state.lock().unwrap();
        let before = g.timers.len();
        if let Some(id) = id {
            g.timers.retain(|t| t.id != id);
        } else {
            if let Some(pos) = g.timers.iter().position(|t| !t.fired) {
                g.timers.remove(pos);
            }
        }
        let removed = before != g.timers.len();
        drop(g);
        if removed {
            self.persist();
        }
        removed
    }

    pub fn cancel(&self, id: u64) -> bool {
        self.stop(Some(id))
    }

    pub fn get_active(&self) -> Option<Timer> {
        self.state.lock().unwrap().timers.iter().find(|t| !t.fired).cloned()
    }

    pub fn list(&self) -> Vec<Timer> {
        self.state.lock().unwrap().timers.clone()
    }

    /// Advance active timers by delta and compute emitted events.
    pub fn tick(&self, now: u64) -> Vec<TimerEvent> {
        let mut events = Vec::new();
        let mut needs_persist = false;
        {
            let mut g = self.state.lock().unwrap();
            for t in g.timers.iter_mut() {
                if t.fired {
                    t.last_tick_ms = now;
                    continue;
                }
                if t.is_paused {
                    t.last_tick_ms = now;
                    // Do not advance elapsed/remaining time, but still emit periodic tick
                    // so clients stay synchronized on paused state and time values.
                    events.push(TimerEvent::Tick(t.clone()));
                    continue;
                }
                let delta = now.saturating_sub(t.last_tick_ms);
                t.last_tick_ms = now;
                if delta == 0 {
                    events.push(TimerEvent::Tick(t.clone()));
                    continue;
                }

                match t.timer_type {
                    TimerType::Stopwatch => {
                        t.elapsed_ms += delta;
                        events.push(TimerEvent::Tick(t.clone()));
                    }
                    TimerType::Countdown => {
                        t.elapsed_ms += delta;
                        if t.remaining_ms <= delta {
                            t.remaining_ms = 0;
                            t.fired = true;
                            needs_persist = true;
                            events.push(TimerEvent::Fired {
                                id: t.id,
                                name: t.name.clone(),
                                sound: "countdowndone.m4a".into(),
                                overdue: false,
                            });
                        } else {
                            t.remaining_ms -= delta;
                            events.push(TimerEvent::Tick(t.clone()));
                        }
                    }
                    TimerType::Pomodoro => {
                        t.elapsed_ms += delta;
                        if t.remaining_ms <= delta {
                            if t.is_work_phase {
                                t.is_work_phase = false;
                                t.remaining_ms = t.break_duration_ms;
                                t.due_ms = now + t.break_duration_ms;
                                needs_persist = true;
                                events.push(TimerEvent::PhaseTransition {
                                    id: t.id,
                                    is_work_phase: false,
                                    cycle_count: t.cycle_count,
                                    sound: "workdone.m4a".into(),
                                });
                            } else {
                                t.is_work_phase = true;
                                t.cycle_count += 1;
                                t.remaining_ms = t.duration_ms;
                                t.due_ms = now + t.duration_ms;
                                needs_persist = true;
                                events.push(TimerEvent::PhaseTransition {
                                    id: t.id,
                                    is_work_phase: true,
                                    cycle_count: t.cycle_count,
                                    sound: "backtowork.m4a".into(),
                                });
                            }
                        } else {
                            t.remaining_ms -= delta;
                            events.push(TimerEvent::Tick(t.clone()));
                        }
                    }
                }
            }
        }
        if needs_persist {
            self.persist();
        }
        events
    }

    /// Legacy poll_due returning fired countdowns.
    pub fn poll_due(&self) -> Vec<FiredTimer> {
        let now = now_ms();
        let events = self.tick(now);
        let mut fired = Vec::new();
        for ev in events {
            if let TimerEvent::Fired { id, name, overdue, .. } = ev {
                fired.push(FiredTimer { id, name, overdue });
            }
        }
        fired
    }

    /// Spawn watcher thread advancing timers and calling event callback every ~250ms.
    pub fn spawn_watcher(&self, on_event: impl Fn(TimerEvent) + Send + 'static) {
        let engine = Self {
            state: Arc::clone(&self.state),
            path: self.path.clone(),
            stop: Arc::clone(&self.stop),
        };
        std::thread::spawn(move || {
            while !engine.stop.load(std::sync::atomic::Ordering::Relaxed) {
                let now = now_ms();
                for ev in engine.tick(now) {
                    on_event(ev);
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        });
    }

    pub fn shutdown(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Comprehensive natural-language timer phrase parser.
pub fn parse_timer_phrase(line: &str) -> Option<TimerPhrase> {
    let t = line.trim();
    let lower = t.to_lowercase();

    // 1. Control keywords
    if lower == "timer p" || lower == "timer pause" || lower == "timer resume" {
        return Some(TimerPhrase::Control(TimerControlAction::PauseResume));
    }
    if lower == "timer r" || lower == "timer restart" || lower == "timer reset" {
        return Some(TimerPhrase::Control(TimerControlAction::Restart));
    }
    if lower == "timer s" || lower == "timer stop" || lower == "timer cancel"
        || lower == "stop timer" || lower == "cancel timer" {
        return Some(TimerPhrase::Control(TimerControlAction::Stop));
    }

    // 2. Bare "timer" starts a Stopwatch
    if lower == "timer" {
        return Some(TimerPhrase::StartStopwatch);
    }

    let rest = lower
        .strip_prefix("timer")
        .or_else(|| lower.strip_prefix("remind me in"))
        .or_else(|| lower.strip_prefix("remind in"))?;
    let trimmed_rest = rest.trim();
    if trimmed_rest.is_empty() {
        return Some(TimerPhrase::StartStopwatch);
    }

    // 3. Pomodoro keywords: timer pomo / timer pomodoro
    if trimmed_rest == "pomo" || trimmed_rest == "pomodoro" {
        return Some(TimerPhrase::StartPomodoro {
            work_ms: 25 * 60_000,
            break_ms: 5 * 60_000,
            name: "Pomodoro".to_string(),
        });
    }

    // Check custom pomodoro: e.g. "57 11" (work minutes, break minutes)
    let words: Vec<&str> = trimmed_rest.split_whitespace().collect();
    if words.len() == 2 {
        if let (Ok(work_m), Ok(break_m)) = (words[0].parse::<u64>(), words[1].parse::<u64>()) {
            if work_m > 0 && break_m > 0 {
                return Some(TimerPhrase::StartPomodoro {
                    work_ms: work_m * 60_000,
                    break_ms: break_m * 60_000,
                    name: "Pomodoro".to_string(),
                });
            }
        }
    }

    // Single integer minutes: timer 5 -> 5 mins
    if words.len() == 1 {
        if let Ok(m) = words[0].parse::<u64>() {
            return Some(TimerPhrase::StartCountdown {
                duration_ms: m * 60_000,
                name: "Timer".to_string(),
            });
        }
    }

    // 4. Token scan for duration tokens, colon syntax (3:30), and labels
    let mut ms_total: u64 = 0;
    let mut name_parts: Vec<String> = Vec::new();
    let mut pending_number: Option<u64> = None;

    for token in &words {
        // Support "3:30" (3 mins 30 secs) or "1:30:00"
        if let Some((m_str, s_str)) = token.split_once(':') {
            if let (Ok(m), Ok(s)) = (m_str.parse::<u64>(), s_str.parse::<u64>()) {
                ms_total += (m * 60 + s) * 1000;
                continue;
            }
        }

        if let Some(v) = parse_duration_token(token) {
            ms_total += v;
            pending_number = None;
        } else if token.chars().all(|c| c.is_ascii_digit()) && !token.is_empty() {
            if let Ok(n) = token.parse::<u64>() {
                if let Some(prev) = pending_number {
                    ms_total += prev * 60_000;
                }
                pending_number = Some(n);
            }
        } else if let Some(n) = pending_number.take() {
            match *token {
                "s" | "sec" | "secs" | "second" | "seconds" => ms_total += n * 1000,
                "m" | "min" | "mins" | "minute" | "minutes" => ms_total += n * 60_000,
                "h" | "hr" | "hrs" | "hour" | "hours" => ms_total += n * 3_600_000,
                _ => {
                    ms_total += n * 60_000;
                    name_parts.push(token.to_string());
                }
            }
        } else {
            name_parts.push(token.to_string());
        }
    }

    if let Some(n) = pending_number {
        ms_total += n * 60_000;
    }

    if ms_total == 0 {
        return None;
    }

    let name = if name_parts.is_empty() {
        "Timer".to_string()
    } else {
        name_parts.join(" ")
    };

    Some(TimerPhrase::StartCountdown { duration_ms: ms_total, name })
}

/// Helper parsing fused tokens like "25min", "1h", "90s", "10m".
fn parse_duration_token(token: &str) -> Option<u64> {
    let t = token.trim();
    let num_end = t.find(|c: char| !c.is_ascii_digit())?;
    if num_end == 0 {
        return None;
    }
    let (num_str, unit) = t.split_at(num_end);
    let n: u64 = num_str.parse().ok()?;
    let mult = match unit.trim() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1000,
        "m" | "min" | "mins" | "minute" | "minutes" => 60_000,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3_600_000,
        _ => return None,
    };
    Some(n * mult)
}

/// Backwards compatible helper checking if line is a timer stop request.
pub fn is_timer_stop_phrase(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    lower == "timer stop" || lower == "timer cancel" || lower == "timer s"
        || lower == "stop timer" || lower == "cancel timer"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phrase_parsing() {
        assert_eq!(parse_timer_phrase("timer"), Some(TimerPhrase::StartStopwatch));
        assert!(matches!(
            parse_timer_phrase("timer 5"),
            Some(TimerPhrase::StartCountdown { duration_ms: 300_000, .. })
        ));
        assert!(matches!(
            parse_timer_phrase("timer 3:30"),
            Some(TimerPhrase::StartCountdown { duration_ms: 210_000, .. })
        ));
        assert!(matches!(
            parse_timer_phrase("timer pomo"),
            Some(TimerPhrase::StartPomodoro { work_ms: 1_500_000, break_ms: 300_000, .. })
        ));
        assert!(matches!(
            parse_timer_phrase("timer 57 11"),
            Some(TimerPhrase::StartPomodoro { work_ms: 3_420_000, break_ms: 660_000, .. })
        ));
        assert_eq!(
            parse_timer_phrase("timer p"),
            Some(TimerPhrase::Control(TimerControlAction::PauseResume))
        );
        assert_eq!(
            parse_timer_phrase("timer r"),
            Some(TimerPhrase::Control(TimerControlAction::Restart))
        );
        assert_eq!(
            parse_timer_phrase("timer s"),
            Some(TimerPhrase::Control(TimerControlAction::Stop))
        );
    }

    #[test]
    fn start_poll_cancel_and_persistence() {
        let dir = std::env::temp_dir().join(format!("plainpad-timer-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("timers.json");
        let _ = std::fs::remove_file(&path);

        let (engine, overdue) = TimerEngine::load(&path);
        assert!(overdue.is_empty());
        let id = engine.start_countdown("Tea", 500).id;
        assert_eq!(engine.list().len(), 1);

        // Advance 600ms past expiration
        let fired = engine.tick(now_ms() + 600);
        assert!(fired.iter().any(|e| matches!(e, TimerEvent::Fired { .. })));

        assert!(engine.cancel(id));
        let _ = std::fs::remove_file(&path);
    }
}
