//! Handy-keys based keyboard shortcut implementation
//!
//! This module provides an alternative to Tauri's global-shortcut plugin
//! using the handy-keys library for more control over keyboard events.
//!
//! ## Architecture
//!
//! The implementation uses a dedicated manager thread that owns the `HotkeyManager`:
//!
//! ```text
//! ┌─────────────────┐     commands      ┌──────────────────────┐
//! │   Main Thread   │ ───────────────▶ │   Manager Thread     │
//! │                 │   (via channel)   │                      │
//! │ - register()    │                   │ - owns HotkeyManager │
//! │ - unregister()  │                   │ - polls for events   │
//! └─────────────────┘                   │ - dispatches actions │
//!                                       └──────────────────────┘
//! ```
//!
//! This design ensures thread-safety since `HotkeyManager` is only accessed
//! from a single thread. Commands (register/unregister) are sent via an mpsc
//! channel and responses are synchronously awaited.
//!
//! ## Recording Mode
//!
//! For UI key capture, a separate `KeyboardListener` is created on-demand and
//! polled from a dedicated recording thread. Events are emitted to the frontend
//! via Tauri's event system.

use handy_keys::{Hotkey, HotkeyId, HotkeyManager, HotkeyState, KeyboardListener};
use log::{debug, error, info, warn};
use serde::Serialize;
use specta::Type;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::{self, get_settings, ShortcutBinding};

use super::handler::handle_shortcut_event;

/// Generation counter for manager threads. `reinstall_hook` bumps this and
/// abandons the old thread WITHOUT joining it (a stuck thread would hang the
/// caller forever). The old thread compares its generation before dispatching
/// any event, so events it drained before stalling can never replay actions
/// after a reinstall.
static MANAGER_GENERATION: AtomicU64 = AtomicU64::new(0);

/// Commands that can be sent to the hotkey manager thread
enum ManagerCommand {
    Register {
        binding_id: String,
        hotkey_string: String,
        response: Sender<Result<(), String>>,
    },
    Unregister {
        binding_id: String,
        response: Sender<Result<(), String>>,
    },
    Shutdown,
}

/// A cancel reconciliation owns a shared lock while waiting for this reply.
/// Recovery abandons the old worker, which may keep its response sender alive
/// forever. Poll only to notice an actual sender replacement; elapsed time by
/// itself must never fail a command that could still register a hotkey later.
fn wait_for_cancel_response(
    response: Receiver<Result<(), String>>,
    sent_generation: u64,
    current_generation: &AtomicU64,
) -> Result<(), String> {
    loop {
        if current_generation.load(Ordering::SeqCst) != sent_generation {
            return Err("Cancel shortcut command superseded by hook recovery".into());
        }
        match response.recv_timeout(std::time::Duration::from_millis(25)) {
            Ok(result) => return result,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Failed to receive cancel shortcut response".into());
            }
        }
    }
}

/// State for the handy-keys shortcut manager
pub struct HandyKeysState {
    /// Channel to send commands to the manager thread (wrapped in Mutex for Sync)
    command_sender: Mutex<Sender<ManagerCommand>>,
    /// Handle to the manager thread (wrapped in Mutex for Sync, allows proper join on drop)
    thread_handle: Mutex<Option<JoinHandle<()>>>,
    /// Recording listener for UI key capture (only active during recording)
    recording_listener: Mutex<Option<KeyboardListener>>,
    /// Flag indicating if we're in recording mode
    is_recording: AtomicBool,
    /// The binding ID being recorded (if any)
    recording_binding_id: Mutex<Option<String>>,
    /// Flag to stop recording loop
    recording_running: Arc<AtomicBool>,
}

/// Key event sent to frontend during recording mode
#[derive(Debug, Clone, Serialize, Type)]
pub struct FrontendKeyEvent {
    /// Currently pressed modifier keys
    pub modifiers: Vec<String>,
    /// The key that was pressed (if any)
    pub key: Option<String>,
    /// Whether this is a key down event
    pub is_key_down: bool,
    /// The full hotkey string (e.g., "option+space")
    pub hotkey_string: String,
}

impl HandyKeysState {
    /// Create a new HandyKeysState
    pub fn new(app: AppHandle) -> Result<Self, String> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<ManagerCommand>();

        // Start the manager thread
        let generation = MANAGER_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
        let app_clone = app.clone();
        let thread_handle = thread::spawn(move || {
            Self::manager_thread(cmd_rx, app_clone, generation);
        });

        Ok(Self {
            command_sender: Mutex::new(cmd_tx),
            thread_handle: Mutex::new(Some(thread_handle)),
            recording_listener: Mutex::new(None),
            is_recording: AtomicBool::new(false),
            recording_binding_id: Mutex::new(None),
            recording_running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// The main manager thread - owns the HotkeyManager and processes commands
    fn manager_thread(cmd_rx: Receiver<ManagerCommand>, app: AppHandle, generation: u64) {
        info!("handy-keys manager thread started (generation {generation})");

        // Time HotkeyManager creation — this is when the OS-level
        // WH_KEYBOARD_LL hook actually gets installed on Windows. Before this
        // returns, hotkey events from the OS are lost. See debug plan: hotkey
        // "не срабатывает сразу после рестарта" diagnosis.
        let hotkey_init_start = std::time::Instant::now();
        let manager = match HotkeyManager::new_with_blocking() {
            Ok(m) => {
                info!(
                    "handy-keys: HotkeyManager (WH_KEYBOARD_LL hook) ready in {}ms",
                    hotkey_init_start.elapsed().as_millis()
                );
                m
            }
            Err(e) => {
                error!("Failed to create HotkeyManager: {}", e);
                return;
            }
        };

        // One-shot flag: log the very first hook event with timing relative
        // to manager init, to detect the gap between "hook ready" and "events
        // actually flowing". Subsequent events use the per-event debug log.
        let mut first_event_logged = false;

        // Maps binding IDs to HotkeyIds and hotkey strings
        let mut binding_to_hotkey: HashMap<String, HotkeyId> = HashMap::new();
        let mut hotkey_to_binding: HashMap<HotkeyId, (String, String)> = HashMap::new(); // (binding_id, hotkey_string)

        loop {
            crate::watchdog::beat_manager();

            // A reinstall happened while we were (presumably) stuck — a fresh
            // manager thread owns the pipeline now. Exit without dispatching
            // anything else so queued stale events can't replay actions.
            if MANAGER_GENERATION.load(Ordering::SeqCst) != generation {
                warn!(
                    "handy-keys manager thread (generation {generation}) superseded; \
                     exiting and discarding queued events"
                );
                break;
            }

            // Check for hotkey events (non-blocking)
            while let Some(event) = manager.try_recv() {
                if MANAGER_GENERATION.load(Ordering::SeqCst) != generation {
                    warn!(
                        "handy-keys: dropping stale event for generation {generation} \
                         (superseded by reinstall)"
                    );
                    break;
                }
                if let Some((binding_id, hotkey_string)) = hotkey_to_binding.get(&event.id) {
                    if !first_event_logged {
                        info!(
                            "handy-keys: first hook event received {}ms after init start \
                             (binding={}, hotkey={})",
                            hotkey_init_start.elapsed().as_millis(),
                            binding_id,
                            hotkey_string
                        );
                        first_event_logged = true;
                    }
                    debug!(
                        "handy-keys event: binding={}, hotkey={}, state={:?}",
                        binding_id, hotkey_string, event.state
                    );
                    let is_pressed = event.state == HotkeyState::Pressed;
                    handle_shortcut_event(&app, binding_id, hotkey_string, is_pressed);
                }
            }

            // Check for commands (non-blocking with timeout)
            match cmd_rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(cmd) => match cmd {
                    ManagerCommand::Register {
                        binding_id,
                        hotkey_string,
                        response,
                    } => {
                        let result = Self::do_register(
                            &manager,
                            &mut binding_to_hotkey,
                            &mut hotkey_to_binding,
                            &binding_id,
                            &hotkey_string,
                        );
                        let _ = response.send(result);
                    }
                    ManagerCommand::Unregister {
                        binding_id,
                        response,
                    } => {
                        let result = Self::do_unregister(
                            &manager,
                            &mut binding_to_hotkey,
                            &mut hotkey_to_binding,
                            &binding_id,
                        );
                        let _ = response.send(result);
                    }
                    ManagerCommand::Shutdown => {
                        info!("handy-keys manager thread shutting down");
                        break;
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // No command, continue
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    info!("Command channel disconnected, shutting down");
                    break;
                }
            }
        }

        info!("handy-keys manager thread stopped");
    }

    /// Register a hotkey
    fn do_register(
        manager: &HotkeyManager,
        binding_to_hotkey: &mut HashMap<String, HotkeyId>,
        hotkey_to_binding: &mut HashMap<HotkeyId, (String, String)>,
        binding_id: &str,
        hotkey_string: &str,
    ) -> Result<(), String> {
        let hotkey: Hotkey = hotkey_string
            .parse()
            .map_err(|e| format!("Failed to parse hotkey '{}': {}", hotkey_string, e))?;

        let id = manager
            .register(hotkey)
            .map_err(|e| format!("Failed to register hotkey: {}", e))?;

        binding_to_hotkey.insert(binding_id.to_string(), id);
        hotkey_to_binding.insert(id, (binding_id.to_string(), hotkey_string.to_string()));

        debug!(
            "Registered handy-keys shortcut: {} -> {:?}",
            binding_id, hotkey
        );
        Ok(())
    }

    /// Unregister a hotkey
    fn do_unregister(
        manager: &HotkeyManager,
        binding_to_hotkey: &mut HashMap<String, HotkeyId>,
        hotkey_to_binding: &mut HashMap<HotkeyId, (String, String)>,
        binding_id: &str,
    ) -> Result<(), String> {
        if let Some(id) = binding_to_hotkey.remove(binding_id) {
            manager
                .unregister(id)
                .map_err(|e| format!("Failed to unregister hotkey: {}", e))?;
            hotkey_to_binding.remove(&id);
            debug!("Unregistered handy-keys shortcut: {}", binding_id);
        }
        Ok(())
    }

    /// Register a shortcut binding
    pub fn register(&self, binding: &ShortcutBinding) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        let sender = self
            .command_sender
            .lock()
            .map_err(|_| "Failed to lock command_sender")?;
        // Sender and generation must describe the same worker. Recovery updates
        // both under this lock, after it has installed the replacement sender.
        let generation = super::HANDY_KEYS_GENERATION.load(Ordering::SeqCst);
        sender
            .send(ManagerCommand::Register {
                binding_id: binding.id.clone(),
                hotkey_string: binding.current_binding.clone(),
                response: tx,
            })
            .map_err(|_| "Failed to send register command")?;
        drop(sender);

        if binding.id == "cancel" {
            return wait_for_cancel_response(rx, generation, &super::HANDY_KEYS_GENERATION);
        }

        rx.recv()
            .map_err(|_| "Failed to receive register response")?
    }

    /// Tear down the current manager thread (and its OS hook) and spawn a new
    /// one. This is the recovery path when the WH_KEYBOARD_LL hook gets killed
    /// by Windows (e.g. callback timeout exceeded LowLevelHooksTimeout) and
    /// no longer fires events. After this returns, all bindings need to be
    /// re-registered via `register()` since the new manager thread starts
    /// with empty maps.
    pub fn reinstall_hook(&self, app: AppHandle) -> Result<(), String> {
        info!("HandyKeysState: reinstalling hook (abandoning old manager thread)");

        // Bump the generation FIRST: from this moment the old thread refuses
        // to dispatch events, even ones already drained into its local queue.
        let generation = MANAGER_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

        // Ask the old thread to shut down, but do NOT join it — if it's stuck
        // (the very situation this recovery exists for), a join would hang
        // the caller forever. A healthy thread exits via Shutdown; a stuck
        // one exits via the generation check whenever it unstalls, dropping
        // its HotkeyManager (and OS hook) at that point.
        if let Ok(sender) = self.command_sender.lock() {
            let _ = sender.send(ManagerCommand::Shutdown);
        }
        if let Ok(mut handle_guard) = self.thread_handle.lock() {
            let _ = handle_guard.take(); // drop the JoinHandle, detaching the thread
        }

        // Spawn fresh manager thread (this re-installs the OS hook via
        // `HotkeyManager::new_with_blocking()` on the new thread). The newest
        // WH_KEYBOARD_LL hook runs first in the chain, so it wins blocking
        // decisions even while the old hook is still installed.
        let (new_tx, new_rx) = mpsc::channel::<ManagerCommand>();
        let app_clone = app.clone();
        let new_handle = thread::spawn(move || {
            Self::manager_thread(new_rx, app_clone, generation);
        });

        // Replace command_sender + thread_handle in place
        if let Ok(mut sender_guard) = self.command_sender.lock() {
            *sender_guard = new_tx;
            // Publish only when requests can actually reach the new worker.
            // MANAGER_GENERATION is bumped earlier to suppress stale events.
            super::handy_keys_recreated(&app);
        }
        if let Ok(mut handle_guard) = self.thread_handle.lock() {
            *handle_guard = Some(new_handle);
        }

        info!("HandyKeysState: hook reinstalled (new manager thread, generation {generation})");
        Ok(())
    }

    /// Unregister a shortcut binding
    pub fn unregister(&self, binding: &ShortcutBinding) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        let sender = self
            .command_sender
            .lock()
            .map_err(|_| "Failed to lock command_sender")?;
        let generation = super::HANDY_KEYS_GENERATION.load(Ordering::SeqCst);
        sender
            .send(ManagerCommand::Unregister {
                binding_id: binding.id.clone(),
                response: tx,
            })
            .map_err(|_| "Failed to send unregister command")?;
        drop(sender);

        if binding.id == "cancel" {
            return wait_for_cancel_response(rx, generation, &super::HANDY_KEYS_GENERATION);
        }

        rx.recv()
            .map_err(|_| "Failed to receive unregister response")?
    }

    /// Start recording mode for a specific binding
    pub fn start_recording(&self, app: &AppHandle, binding_id: String) -> Result<(), String> {
        if self.is_recording.load(Ordering::SeqCst) {
            return Err("Already recording".into());
        }

        // Create a new keyboard listener for recording
        let listener = KeyboardListener::new()
            .map_err(|e| format!("Failed to create keyboard listener: {}", e))?;

        {
            let mut recording = self
                .recording_listener
                .lock()
                .map_err(|_| "Failed to lock recording_listener")?;
            *recording = Some(listener);
        }
        {
            let mut binding = self
                .recording_binding_id
                .lock()
                .map_err(|_| "Failed to lock recording_binding_id")?;
            *binding = Some(binding_id);
        }

        self.is_recording.store(true, Ordering::SeqCst);
        self.recording_running.store(true, Ordering::SeqCst);

        // Start a thread to emit key events to the frontend
        let app_clone = app.clone();
        let recording_running = Arc::clone(&self.recording_running);
        thread::spawn(move || {
            Self::recording_loop(app_clone, recording_running);
        });

        debug!("Started handy-keys recording mode");
        Ok(())
    }

    /// Recording loop - emits key events to frontend during recording
    fn recording_loop(app: AppHandle, running: Arc<AtomicBool>) {
        while running.load(Ordering::SeqCst) {
            let event = {
                let state = match app.try_state::<HandyKeysState>() {
                    Some(s) => s,
                    None => break,
                };
                let listener = state.recording_listener.lock().ok();
                listener.as_ref().and_then(|l| l.as_ref()?.try_recv())
            };

            if let Some(key_event) = event {
                // Convert to frontend-friendly format
                let frontend_event = FrontendKeyEvent {
                    modifiers: modifiers_to_strings(key_event.modifiers),
                    key: key_event.key.map(|k| k.to_string().to_lowercase()),
                    is_key_down: key_event.is_key_down,
                    hotkey_string: key_event
                        .as_hotkey()
                        .map(|h| h.to_handy_string())
                        .unwrap_or_default(),
                };

                // Emit to frontend
                if let Err(e) = app.emit("handy-keys-event", &frontend_event) {
                    error!("Failed to emit key event: {}", e);
                }
            } else {
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        debug!("Recording loop ended");
    }

    /// Stop recording mode
    pub fn stop_recording(&self) -> Result<(), String> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.recording_running.store(false, Ordering::SeqCst);

        {
            let mut recording = self
                .recording_listener
                .lock()
                .map_err(|_| "Failed to lock recording_listener")?;
            *recording = None;
        }
        {
            let mut binding = self
                .recording_binding_id
                .lock()
                .map_err(|_| "Failed to lock recording_binding_id")?;
            *binding = None;
        }

        debug!("Stopped handy-keys recording mode");
        Ok(())
    }
}

impl Drop for HandyKeysState {
    fn drop(&mut self) {
        // Signal recording to stop
        self.recording_running.store(false, Ordering::SeqCst);
        self.is_recording.store(false, Ordering::SeqCst);

        // Send shutdown command
        if let Ok(sender) = self.command_sender.lock() {
            let _ = sender.send(ManagerCommand::Shutdown);
        }

        // Wait for the manager thread to finish
        if let Ok(mut handle) = self.thread_handle.lock() {
            if let Some(h) = handle.take() {
                let _ = h.join();
            }
        }
    }
}

/// Convert handy-keys Modifiers to a list of strings
fn modifiers_to_strings(modifiers: handy_keys::Modifiers) -> Vec<String> {
    let mut result = Vec::new();

    if modifiers.contains(handy_keys::Modifiers::CTRL) {
        result.push("ctrl".to_string());
    }
    if modifiers.contains(handy_keys::Modifiers::OPT) {
        #[cfg(target_os = "macos")]
        result.push("option".to_string());
        #[cfg(not(target_os = "macos"))]
        result.push("alt".to_string());
    }
    if modifiers.contains(handy_keys::Modifiers::SHIFT) {
        result.push("shift".to_string());
    }
    if modifiers.contains(handy_keys::Modifiers::CMD) {
        #[cfg(target_os = "macos")]
        result.push("command".to_string());
        #[cfg(not(target_os = "macos"))]
        result.push("super".to_string());
    }
    if modifiers.contains(handy_keys::Modifiers::FN) {
        result.push("fn".to_string());
    }

    result
}

/// Validate a shortcut string for the HandyKeys implementation.
/// HandyKeys is more permissive: allows modifier-only combos and the fn key.
pub fn validate_shortcut(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err("Shortcut cannot be empty".into());
    }
    // HandyKeys accepts modifier-only, key-only, and modifier+key combos
    // Just verify the string is parseable
    raw.parse::<Hotkey>()
        .map(|_| ())
        .map_err(|e| format!("Invalid shortcut for HandyKeys: {}", e))
}

/// Force-reinitialize handy-keys shortcuts: tears down the worker thread,
/// reinstalls the OS hook, then re-registers all configured bindings.
/// Used by the "Re-register Hotkeys" tray recovery action when the hook
/// has gotten into a stuck state.
pub fn force_reinit(app: &AppHandle) -> Result<(), String> {
    let Some(state) = app.try_state::<HandyKeysState>() else {
        // No existing state — fall back to fresh init
        return init_shortcuts(app);
    };

    state.reinstall_hook(app.clone())?;

    // Re-register all bindings on the freshly-spawned manager thread
    let default_bindings = settings::get_default_settings().bindings;
    let user_settings = settings::load_or_create_app_settings(app);

    for (id, default_binding) in default_bindings {
        if id == "cancel" {
            continue;
        }
        if id == "transcribe_with_post_process" && !user_settings.post_process_enabled {
            continue;
        }

        let binding = user_settings
            .bindings
            .get(&id)
            .cloned()
            .unwrap_or(default_binding);

        if let Err(e) = state.register(&binding) {
            error!(
                "Failed to re-register handy-keys shortcut {} after reinit: {}",
                id, e
            );
        }
    }

    info!("handy-keys shortcuts re-initialized");
    Ok(())
}

/// Initialize handy-keys shortcuts
pub fn init_shortcuts(app: &AppHandle) -> Result<(), String> {
    let state = HandyKeysState::new(app.clone())?;

    let default_bindings = settings::get_default_settings().bindings;
    let user_settings = settings::load_or_create_app_settings(app);

    // Register all bindings except cancel (which is dynamic)
    for (id, default_binding) in default_bindings {
        if id == "cancel" {
            continue;
        }
        // Skip post-processing shortcut when the feature is disabled
        if id == "transcribe_with_post_process" && !user_settings.post_process_enabled {
            continue;
        }

        let binding = user_settings
            .bindings
            .get(&id)
            .cloned()
            .unwrap_or(default_binding);

        if let Err(e) = state.register(&binding) {
            error!(
                "Failed to register handy-keys shortcut {} during init: {}",
                id, e
            );
        }
    }

    app.manage(state);
    info!("handy-keys shortcuts initialized");
    Ok(())
}

/// Register a shortcut
pub fn register_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    let state = app
        .try_state::<HandyKeysState>()
        .ok_or("HandyKeysState not initialized")?;
    state.register(&binding)
}

/// Unregister a shortcut
pub fn unregister_shortcut(app: &AppHandle, binding: ShortcutBinding) -> Result<(), String> {
    let state = app
        .try_state::<HandyKeysState>()
        .ok_or("HandyKeysState not initialized")?;
    state.unregister(&binding)
}

/// Start key recording mode
#[tauri::command]
#[specta::specta]
pub fn start_handy_keys_recording(app: AppHandle, binding_id: String) -> Result<(), String> {
    let settings = get_settings(&app);
    if settings.keyboard_implementation != settings::KeyboardImplementation::HandyKeys {
        return Err("handy-keys is not the active keyboard implementation".into());
    }

    // While Secure Input is active the tap receives no KeyDown/KeyUp, so the
    // recorder would silently capture just the modifier and overwrite the
    // binding with it (issue #1578). Refuse instead; the frontend maps this
    // marker to a localized explanation, and the noted impact makes the
    // warning banner appear with the full story.
    if crate::secure_input::is_enabled_now() {
        crate::secure_input::note_recorder_blocked(&app);
        return Err("secure-input-active".into());
    }

    let state = app
        .try_state::<HandyKeysState>()
        .ok_or("HandyKeysState not initialized")?;

    // Suspend every registered shortcut so a combo that overlaps an existing
    // binding can't fire it (or have its keys swallowed) mid-capture.
    super::suspend_all_shortcuts(&app);

    let result = state.start_recording(&app, binding_id);
    if result.is_err() {
        super::resume_all_shortcuts(&app);
    }
    result
}

/// Stop key recording mode
#[tauri::command]
#[specta::specta]
pub fn stop_handy_keys_recording(app: AppHandle) -> Result<(), String> {
    let settings = get_settings(&app);
    if settings.keyboard_implementation != settings::KeyboardImplementation::HandyKeys {
        return Err("handy-keys is not the active keyboard implementation".into());
    }

    let state = app
        .try_state::<HandyKeysState>()
        .ok_or("HandyKeysState not initialized")?;

    // Restore shortcuts from settings regardless of how recording ended.
    // A commit has already registered the new binding via change_binding;
    // re-registering it here fails cleanly and is ignored.
    let result = state.stop_recording();
    super::resume_all_shortcuts(&app);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn abandoned_cancel_worker_releases_reconciliation_lock_after_sender_replacement() {
        let generation = Arc::new(AtomicU64::new(1));
        let reconciliation_lock = Arc::new(Mutex::new(()));
        // Keep this sender alive: a disconnected-channel error would mask the bug.
        let (_old_response_sender, old_response) = mpsc::channel();
        let (started_tx, started_rx) = mpsc::channel();
        let (finished_tx, finished_rx) = mpsc::channel();
        let worker_generation = Arc::clone(&generation);
        let worker_lock = Arc::clone(&reconciliation_lock);
        let worker = thread::spawn(move || {
            let _guard = worker_lock.lock().unwrap();
            started_tx.send(()).unwrap();
            let result = wait_for_cancel_response(old_response, 1, &worker_generation);
            finished_tx.send(result).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        // A slow but still-current worker is not timed out into a late registration.
        assert!(finished_rx.recv_timeout(Duration::from_millis(80)).is_err());
        generation.store(2, Ordering::SeqCst);
        let error = finished_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap_err();
        assert!(error.contains("superseded by hook recovery"));
        worker.join().unwrap();

        // Another cancel reconciliation can now acquire the same lock and finish.
        let _guard = reconciliation_lock
            .try_lock()
            .expect("old waiter retained cancel lock");
        let (new_response_sender, new_response) = mpsc::channel();
        new_response_sender.send(Ok(())).unwrap();
        assert!(wait_for_cancel_response(new_response, 2, &generation).is_ok());
    }

    #[test]
    fn cancel_reply_from_current_worker_preserves_backend_error() {
        let generation = AtomicU64::new(5);
        let (sender, response) = mpsc::channel();
        sender
            .send(Err("key is already registered".into()))
            .unwrap();
        assert_eq!(
            wait_for_cancel_response(response, 5, &generation),
            Err("key is already registered".into())
        );
    }
}
