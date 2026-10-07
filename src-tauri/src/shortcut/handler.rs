//! Shared shortcut event handling logic
//!
//! This module contains the common logic for handling shortcut events,
//! used by both the Tauri and handy-keys implementations.

use log::{debug, warn};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::actions::ACTION_MAP;
use crate::managers::audio::AudioRecordingManager;
use crate::transcription_coordinator::is_transcribe_binding;
use crate::TranscriptionCoordinator;

/// Handle a shortcut event from either implementation.
///
/// This function contains the shared logic for:
/// - Looking up the action in ACTION_MAP
/// - Handling the cancel binding (only fires when recording)
/// - Routing transcribe bindings to the coordinator, which applies the
///   configured activation mode (toggle / push-to-talk / hold-or-toggle)
///
/// # Arguments
/// * `app` - The Tauri app handle
/// * `binding_id` - The ID of the binding (e.g., "transcribe", "cancel")
/// * `hotkey_string` - The string representation of the hotkey
/// * `is_pressed` - Whether this is a key press (true) or release (false)
pub fn handle_shortcut_event(
    app: &AppHandle,
    binding_id: &str,
    hotkey_string: &str,
    is_pressed: bool,
) {
    debug!(
        "shortcut event: binding={}, hotkey={}, pressed={}",
        binding_id, hotkey_string, is_pressed
    );

    // NOTE: this function runs on the hotkey manager thread, which drains the
    // OS keyboard hook. Anything that can block (settings store, audio locks,
    // WASAPI calls) must be deferred to other threads, otherwise hotkey
    // events queue up and the app goes "deaf" until the block clears.

    // Transcribe bindings are handled by the coordinator.
    if is_transcribe_binding(binding_id) {
        if let Some(coordinator) = app.try_state::<TranscriptionCoordinator>() {
            // Settings are resolved on the coordinator thread, outside the hook path.
            coordinator.send_input(binding_id, hotkey_string, is_pressed);
        } else {
            warn!("TranscriptionCoordinator is not initialized");
        }
        return;
    }

    let Some(action) = ACTION_MAP.get(binding_id) else {
        warn!(
            "No action defined in ACTION_MAP for shortcut ID '{}'. Shortcut: '{}', Pressed: {}",
            binding_id, hotkey_string, is_pressed
        );
        return;
    };

    // Cancel binding: only fires when recording and key is pressed.
    // Cancellation stops the WASAPI stream and may unload the model — run it
    // on a worker thread so a slow audio driver can't stall the hook drain.
    if binding_id == "cancel" {
        if !is_pressed {
            return;
        }
        let app = app.clone();
        let action = Arc::clone(action);
        let binding_id = binding_id.to_string();
        let hotkey_string = hotkey_string.to_string();
        std::thread::spawn(move || {
            let audio_manager = app.state::<Arc<AudioRecordingManager>>();
            if audio_manager.is_recording() {
                action.start(&app, &binding_id, &hotkey_string);
            }
        });
        return;
    }

    // Remaining bindings (e.g. "test") use simple start/stop on press/release.
    if is_pressed {
        action.start(app, binding_id, hotkey_string);
    } else {
        action.stop(app, binding_id, hotkey_string);
    }
}
