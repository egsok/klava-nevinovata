//! Pipeline stall watchdog
//!
//! The hotkey-to-paste pipeline spans three threads, and a stall in any of
//! them makes the app "deaf" to the transcription hotkey while the key keeps
//! being swallowed system-wide (the WH_KEYBOARD_LL hook blocks registered
//! hotkeys independently of whether the pipeline consumes them):
//!
//! 1. **manager thread** (`shortcut::handy_keys::manager_thread`) — drains
//!    hook events and dispatches them; if it stalls, presses queue up in the
//!    handy-keys channel and replay in a burst once it recovers.
//! 2. **coordinator thread** (`TranscriptionCoordinator`) — runs the
//!    start/stop actions; mic open (WASAPI) and tray/overlay updates happen
//!    here; if it stalls, presses queue up in the coordinator channel.
//! 3. **main thread** (Tauri event loop) — tray `set_icon`, window getters
//!    and input injection. Ordinary clipboard snapshots/restores now run on
//!    paste workers; input injection still waits for the main thread.
//!
//! Each thread reports a heartbeat; a watchdog thread checks them every few
//! seconds, logs an ERROR naming the exact stalled component (this is the
//! key diagnostic for the intermittent hotkey-deafness bug), and — for the
//! manager thread only — force-reinstalls the keyboard hook after a
//! prolonged stall, since that path has a safe recovery (detached teardown).

use log::{error, info, warn};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

static MANAGER_BEAT: AtomicU64 = AtomicU64::new(0);
static COORD_BEAT: AtomicU64 = AtomicU64::new(0);
static MAIN_BEAT: AtomicU64 = AtomicU64::new(0);

/// Mirror of the coordinator `Stage` for diagnostics: 0=Idle, 1=Recording,
/// 2=Processing.
static COORD_STAGE: AtomicU8 = AtomicU8::new(0);

const CHECK_INTERVAL: Duration = Duration::from_secs(5);
/// A component is considered stalled when its heartbeat is older than this.
const STALL_THRESHOLD_MS: u64 = 10_000;
/// Manager-thread stalls longer than this trigger an automatic hook
/// reinstall (the equivalent of the "Re-register Hotkeys" tray action).
const MANAGER_REINIT_THRESHOLD_MS: u64 = 30_000;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn beat_manager() {
    MANAGER_BEAT.store(now_ms(), Ordering::Relaxed);
}

pub fn beat_coordinator() {
    COORD_BEAT.store(now_ms(), Ordering::Relaxed);
}

pub fn set_coordinator_stage(stage: u8) {
    COORD_STAGE.store(stage, Ordering::Relaxed);
}

fn stage_name() -> &'static str {
    match COORD_STAGE.load(Ordering::Relaxed) {
        0 => "Idle",
        1 => "Recording",
        2 => "Processing",
        _ => "?",
    }
}

/// Tracks one monitored component across watchdog checks so stall begin /
/// ongoing / recovery transitions each get logged exactly once per state.
struct Monitor {
    name: &'static str,
    beat: &'static AtomicU64,
    stalled_since: Option<u64>,
    last_report: u64,
}

impl Monitor {
    fn new(name: &'static str, beat: &'static AtomicU64) -> Self {
        Self {
            name,
            beat,
            stalled_since: None,
            last_report: 0,
        }
    }

    /// Returns the current stall age in ms (0 if healthy).
    fn check(&mut self, now: u64) -> u64 {
        let age = now.saturating_sub(self.beat.load(Ordering::Relaxed));
        if age > STALL_THRESHOLD_MS {
            if self.stalled_since.is_none() {
                self.stalled_since = Some(now.saturating_sub(age));
                self.last_report = now;
                error!(
                    "watchdog: {} thread unresponsive for {}ms (coordinator stage={}) — \
                     hotkey presses are queuing and will replay when it recovers",
                    self.name,
                    age,
                    stage_name()
                );
            } else if now.saturating_sub(self.last_report) >= 30_000 {
                self.last_report = now;
                error!(
                    "watchdog: {} thread STILL unresponsive ({}ms, coordinator stage={})",
                    self.name,
                    age,
                    stage_name()
                );
            }
            age
        } else {
            if let Some(since) = self.stalled_since.take() {
                warn!(
                    "watchdog: {} thread recovered after ~{}ms stall",
                    self.name,
                    now.saturating_sub(since)
                );
            }
            0
        }
    }
}

/// Spawn the watchdog thread. Call once after shortcuts and the coordinator
/// are initialized.
pub fn start(app: AppHandle) {
    let now = now_ms();
    MANAGER_BEAT.store(now, Ordering::Relaxed);
    COORD_BEAT.store(now, Ordering::Relaxed);
    MAIN_BEAT.store(now, Ordering::Relaxed);

    // The manager thread only exists for the HandyKeys backend. Read the
    // setting once here (not in the loop — a hung settings store would take
    // the watchdog down with it); switching backends needs a restart anyway
    // for this monitor to follow.
    let monitor_manager = crate::settings::get_settings(&app).keyboard_implementation
        == crate::settings::KeyboardImplementation::HandyKeys;

    std::thread::spawn(move || {
        info!("watchdog: started (manager monitoring: {monitor_manager})");
        let mut manager = Monitor::new("hotkey-manager", &MANAGER_BEAT);
        let mut coordinator = Monitor::new("coordinator", &COORD_BEAT);
        let mut main = Monitor::new("main(event-loop)", &MAIN_BEAT);
        let mut reinit_fired_for_stall = false;
        let mut last_resyncs = handy_keys::modifier_resync_count();

        loop {
            std::thread::sleep(CHECK_INTERVAL);

            // Probe the main thread: a non-blocking post that stamps the
            // heartbeat when (if) the event loop gets around to running it.
            let _ = app.run_on_main_thread(|| {
                MAIN_BEAT.store(now_ms(), Ordering::Relaxed);
            });

            // The hook callback can't log (no I/O allowed there), so drift
            // corrections are surfaced here. Every count is a modifier
            // transition the hook never saw — the root cause of "bare Space
            // toggles dictation" / "Ctrl+Space goes deaf" before the fork fix.
            let resyncs = handy_keys::modifier_resync_count();
            if resyncs != last_resyncs {
                warn!(
                    "watchdog: keyboard hook resynced modifier state {} time(s) \
                     (total {}) — missed modifier transitions were corrected",
                    resyncs - last_resyncs,
                    resyncs
                );
                last_resyncs = resyncs;
            }

            let now = now_ms();
            let manager_age = if monitor_manager {
                manager.check(now)
            } else {
                0
            };
            coordinator.check(now);
            main.check(now);

            // Auto-recovery is only safe for the manager thread: reinstall
            // spawns a fresh thread + OS hook and abandons the stuck one
            // (stale events are dropped via the generation guard). The
            // coordinator and main thread have no safe forced recovery.
            if manager_age > MANAGER_REINIT_THRESHOLD_MS {
                if !reinit_fired_for_stall {
                    reinit_fired_for_stall = true;
                    error!(
                        "watchdog: auto-reinstalling keyboard hook (manager thread stalled {}ms)",
                        manager_age
                    );
                    let app = app.clone();
                    std::thread::spawn(move || match crate::shortcut::force_reinit(&app) {
                        Ok(()) => info!("watchdog: keyboard hook reinstalled"),
                        Err(e) => error!("watchdog: hook reinstall failed: {}", e),
                    });
                }
            } else if manager_age == 0 {
                reinit_fired_for_stall = false;
            }
        }
    });
}
