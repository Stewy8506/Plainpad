use plainpad_core::timers::{
    parse_timer_phrase, TimerControlAction, TimerEvent, TimerPhrase, TimerType, TimerEngine,
};
use std::path::PathBuf;

fn temp_timer_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "plainpad-timer-test-{tag}-{}-{}",
        std::process::id(),
        plainpad_core::now_ms()
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir.join("timers.json")
}

#[test]
fn test_parse_timer_phrase_stopwatch() {
    assert_eq!(parse_timer_phrase("timer"), Some(TimerPhrase::StartStopwatch));
    assert_eq!(parse_timer_phrase("  TIMER  "), Some(TimerPhrase::StartStopwatch));
}

#[test]
fn test_parse_timer_phrase_countdown() {
    // Single integer minutes: timer 5 -> 5 mins (300,000 ms)
    let p1 = parse_timer_phrase("timer 5").expect("failed to parse timer 5");
    match p1 {
        TimerPhrase::StartCountdown { duration_ms, .. } => assert_eq!(duration_ms, 300_000),
        _ => panic!("Expected StartCountdown for timer 5"),
    }

    // mm:ss syntax: timer 3:30 -> 210,000 ms
    let p2 = parse_timer_phrase("timer 3:30").expect("failed to parse timer 3:30");
    match p2 {
        TimerPhrase::StartCountdown { duration_ms, .. } => assert_eq!(duration_ms, 210_000),
        _ => panic!("Expected StartCountdown for timer 3:30"),
    }

    // Natural syntax: timer 1h 30m pasta
    let p3 = parse_timer_phrase("timer 1h 30m pasta").expect("failed to parse timer 1h 30m pasta");
    match p3 {
        TimerPhrase::StartCountdown { duration_ms, name } => {
            assert_eq!(duration_ms, 90 * 60_000);
            assert!(name.to_lowercase().contains("pasta"));
        }
        _ => panic!("Expected StartCountdown for timer 1h 30m pasta"),
    }
}

#[test]
fn test_parse_timer_phrase_pomodoro() {
    // Default pomodoro: timer pomo
    let p1 = parse_timer_phrase("timer pomo").expect("failed to parse timer pomo");
    match p1 {
        TimerPhrase::StartPomodoro { work_ms, break_ms, .. } => {
            assert_eq!(work_ms, 25 * 60_000);
            assert_eq!(break_ms, 5 * 60_000);
        }
        _ => panic!("Expected StartPomodoro for timer pomo"),
    }

    // Custom pomodoro: timer 57 11
    let p2 = parse_timer_phrase("timer 57 11").expect("failed to parse timer 57 11");
    match p2 {
        TimerPhrase::StartPomodoro { work_ms, break_ms, .. } => {
            assert_eq!(work_ms, 57 * 60_000);
            assert_eq!(break_ms, 11 * 60_000);
        }
        _ => panic!("Expected StartPomodoro for timer 57 11"),
    }
}

#[test]
fn test_parse_timer_phrase_controls() {
    // Pause / Resume
    assert_eq!(
        parse_timer_phrase("timer p"),
        Some(TimerPhrase::Control(TimerControlAction::PauseResume))
    );
    assert_eq!(
        parse_timer_phrase("timer pause"),
        Some(TimerPhrase::Control(TimerControlAction::PauseResume))
    );
    assert_eq!(
        parse_timer_phrase("timer resume"),
        Some(TimerPhrase::Control(TimerControlAction::PauseResume))
    );

    // Restart
    assert_eq!(
        parse_timer_phrase("timer r"),
        Some(TimerPhrase::Control(TimerControlAction::Restart))
    );
    assert_eq!(
        parse_timer_phrase("timer restart"),
        Some(TimerPhrase::Control(TimerControlAction::Restart))
    );
    assert_eq!(
        parse_timer_phrase("timer reset"),
        Some(TimerPhrase::Control(TimerControlAction::Restart))
    );

    // Stop
    assert_eq!(
        parse_timer_phrase("timer s"),
        Some(TimerPhrase::Control(TimerControlAction::Stop))
    );
    assert_eq!(
        parse_timer_phrase("timer stop"),
        Some(TimerPhrase::Control(TimerControlAction::Stop))
    );
    assert_eq!(
        parse_timer_phrase("stop timer"),
        Some(TimerPhrase::Control(TimerControlAction::Stop))
    );
    assert_eq!(
        parse_timer_phrase("cancel timer"),
        Some(TimerPhrase::Control(TimerControlAction::Stop))
    );
}

#[test]
fn test_stopwatch_lifecycle() {
    let path = temp_timer_path("stopwatch");
    let (engine, _) = TimerEngine::load(&path);

    let sw = engine.start_stopwatch("Work Session");
    assert_eq!(sw.timer_type, TimerType::Stopwatch);
    assert_eq!(sw.elapsed_ms, 0);
    assert!(!sw.is_paused);

    // Tick forward 1000ms
    let events = engine.tick(sw.started_at_ms + 1000);
    assert!(!events.is_empty());
    let active = engine.get_active().expect("active timer");
    assert_eq!(active.elapsed_ms, 1000);

    // Pause stopwatch
    let paused = engine.pause_resume(Some(active.id)).expect("paused");
    assert!(paused.is_paused);

    // Tick forward another 1000ms while paused -> elapsed should remain 1000
    engine.tick(sw.started_at_ms + 2000);
    let after_pause = engine.get_active().expect("active timer");
    assert_eq!(after_pause.elapsed_ms, 1000);

    // Restart stopwatch -> elapsed should reset to 0
    let restarted = engine.restart(Some(active.id)).expect("restarted");
    assert_eq!(restarted.elapsed_ms, 0);
    assert!(!restarted.is_paused);
}

#[test]
fn test_countdown_firing() {
    let path = temp_timer_path("countdown");
    let (engine, _) = TimerEngine::load(&path);

    let cd = engine.start_countdown("Tea", 5000);
    assert_eq!(cd.timer_type, TimerType::Countdown);
    assert_eq!(cd.remaining_ms, 5000);
    assert_eq!(cd.duration_ms, 5000);

    // Tick half way (2500ms)
    let ev1 = engine.tick(cd.started_at_ms + 2500);
    assert!(matches!(ev1.first(), Some(TimerEvent::Tick(_))));
    let mid = engine.get_active().expect("active");
    assert_eq!(mid.remaining_ms, 2500);

    // Tick past expiration (+5500ms total)
    let ev2 = engine.tick(cd.started_at_ms + 5500);
    let fired_ev = ev2
        .iter()
        .find(|e| matches!(e, TimerEvent::Fired { sound, .. } if sound == "countdowndone.m4a"));
    assert!(fired_ev.is_some(), "Countdown fired event with countdowndone.m4a expected");

    // Timer is now fired and not active
    assert!(engine.get_active().is_none());
}

#[test]
fn test_pomodoro_phase_transitions() {
    let path = temp_timer_path("pomodoro");
    let (engine, _) = TimerEngine::load(&path);

    // 2-second work, 1-second break
    let pomo = engine.start_pomodoro("Sprint", 2000, 1000);
    assert_eq!(pomo.timer_type, TimerType::Pomodoro);
    assert!(pomo.is_work_phase);
    assert_eq!(pomo.cycle_count, 0);
    assert_eq!(pomo.remaining_ms, 2000);

    // Advance past work phase (2100ms)
    let ev1 = engine.tick(pomo.started_at_ms + 2100);
    let trans1 = ev1.iter().find(|e| match e {
        TimerEvent::PhaseTransition { is_work_phase, sound, .. } => {
            !is_work_phase && sound == "workdone.m4a"
        }
        _ => false,
    });
    assert!(trans1.is_some(), "Should transition to break phase with workdone.m4a");

    let mid = engine.get_active().expect("active pomo in break");
    assert!(!mid.is_work_phase);
    assert_eq!(mid.cycle_count, 0);

    // Advance past break phase (+1200ms)
    let ev2 = engine.tick(pomo.started_at_ms + 2100 + 1200);
    let trans2 = ev2.iter().find(|e| match e {
        TimerEvent::PhaseTransition { is_work_phase, cycle_count, sound, .. } => {
            *is_work_phase && *cycle_count == 1 && sound == "backtowork.m4a"
        }
        _ => false,
    });
    assert!(
        trans2.is_some(),
        "Should transition back to work phase with cycle_count=1 and backtowork.m4a"
    );

    let next_work = engine.get_active().expect("active pomo in next work");
    assert!(next_work.is_work_phase);
    assert_eq!(next_work.cycle_count, 1);
}

#[test]
fn test_persistence_and_overdue_on_reload() {
    let path = temp_timer_path("persistence");

    let start_time = plainpad_core::now_ms();
    {
        let (engine, _) = TimerEngine::load(&path);
        // Start a 1-second countdown
        let _ = engine.start_countdown("Quick", 1000);
    }

    // Simulate reloading before expiration
    {
        let (engine, overdue) = TimerEngine::load(&path);
        assert!(overdue.is_empty());
        let active = engine.get_active().expect("active loaded");
        assert_eq!(active.name, "Quick");
        assert_eq!(active.duration_ms, 1000);
    }

    // Now manually modify JSON due_ms to simulate past expiration while app was closed
    {
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut timers: Vec<plainpad_core::timers::Timer> = serde_json::from_str(&raw).unwrap();
        for t in timers.iter_mut() {
            t.due_ms = start_time.saturating_sub(5000); // 5 seconds in the past
        }
        std::fs::write(&path, serde_json::to_string(&timers).unwrap()).unwrap();
    }

    // Reload -> should be detected as overdue
    {
        let (engine, overdue) = TimerEngine::load(&path);
        assert_eq!(overdue.len(), 1);
        assert_eq!(overdue[0].name, "Quick");
        assert!(overdue[0].overdue);
        assert!(engine.get_active().is_none());
    }
}

#[test]
fn test_pause_active_idempotent() {
    let path = temp_timer_path("pause_active");
    let (engine, _) = TimerEngine::load(&path);

    // No timer active -> returns None
    assert!(engine.pause_active().is_none());
    assert!(engine.resume_active().is_none());

    // Start countdown
    let timer = engine.start_countdown("Focus", 60_000);
    assert!(!timer.is_paused);

    // Pause active
    let p1 = engine.pause_active().expect("paused");
    assert!(p1.is_paused);

    // Call pause_active again -> idempotent, returns already paused timer
    let p2 = engine.pause_active().expect("still paused");
    assert!(p2.is_paused);

    // Resume active
    let r1 = engine.resume_active().expect("resumed");
    assert!(!r1.is_paused);

    // Call resume_active again -> idempotent, returns unpaused timer
    let r2 = engine.resume_active().expect("still unpaused");
    assert!(!r2.is_paused);
}
