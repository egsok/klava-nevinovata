use ::windows::Win32::UI::{
    Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyExW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC_EX, VIRTUAL_KEY, VK_CONTROL, VK_SHIFT,
        VK_V,
    },
    WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId},
};
use std::{mem::size_of, time::Duration};

pub(super) fn send_paste(shift: bool, hold_ms: u64, marker: usize) -> Result<(), String> {
    // Match Enigo's virtual-key mapping, using one foreground layout for the chord.
    // SAFETY: These queries take no borrowed pointers and return OS-owned handles.
    let layout =
        unsafe { GetKeyboardLayout(GetWindowThreadProcessId(GetForegroundWindow(), None)) };
    let chord = PasteChord::new(shift, marker, |key| {
        // SAFETY: The layout came from GetKeyboardLayout; the input is a virtual key.
        unsafe { MapVirtualKeyExW(u32::from(key.0), MAPVK_VK_TO_VSC_EX, Some(layout)) as u16 }
    });
    chord.send(
        Duration::from_millis(hold_ms),
        |events| {
            // SAFETY: Every INPUT contains an initialized keyboard union member.
            // The slice remains valid for the call and cbSize matches INPUT.
            unsafe { SendInput(events, size_of::<INPUT>() as i32) }
        },
        std::thread::sleep,
    )
}

struct PasteChord {
    press: Vec<INPUT>,
    release: Vec<INPUT>,
    cleanup: Vec<INPUT>,
}

impl PasteChord {
    fn new(shift: bool, marker: usize, mut scan: impl FnMut(VIRTUAL_KEY) -> u16) -> Self {
        let mut modifiers = vec![VK_CONTROL];
        if shift {
            modifiers.push(VK_SHIFT);
        }
        let mut press = Vec::new();
        let mut release = Vec::new();
        for key in modifiers {
            let scan = scan(key);
            press.push(key_event(key, scan, KEYBD_EVENT_FLAGS::default(), marker));
            release.push(key_event(key, scan, KEYEVENTF_KEYUP, marker));
        }
        release.reverse();
        let scan = scan(VK_V);
        press.push(key_event(VK_V, scan, KEYBD_EVENT_FLAGS::default(), marker));
        let v_up = key_event(VK_V, scan, KEYEVENTF_KEYUP, marker);
        press.push(v_up);
        let mut cleanup = vec![v_up];
        cleanup.extend_from_slice(&release);
        Self {
            press,
            release,
            cleanup,
        }
    }

    fn send(
        &self,
        hold: Duration,
        mut send: impl FnMut(&[INPUT]) -> u32,
        wait: impl FnOnce(Duration),
    ) -> Result<(), String> {
        // SendInput does not interleave user input within one array. Separate
        // Enigo calls let a physical Ctrl release land before the V click.
        let inserted = send(&self.press);
        if inserted != self.press.len() as u32 {
            let error = format!(
                "Failed to send paste chord: inserted {inserted}/{} events",
                self.press.len()
            );
            // A zero result inserted nothing: don't release the user's keys.
            return Err(if inserted == 0 {
                error
            } else {
                cleanup_after_error(error, &self.cleanup, &mut send)
            });
        }
        wait(hold);
        let inserted = send(&self.release);
        if inserted != self.release.len() as u32 {
            return Err(cleanup_after_error(
                format!(
                    "Failed to release paste modifiers: inserted {inserted}/{} events",
                    self.release.len()
                ),
                &self.release,
                &mut send,
            ));
        }
        Ok(())
    }
}

fn cleanup_after_error(
    error: String,
    releases: &[INPUT],
    send: &mut impl FnMut(&[INPUT]) -> u32,
) -> String {
    // Partial insertion does not tell us which keys remain held. Make one
    // best-effort key-up-only cleanup; never replay V and risk a second paste.
    let inserted = send(releases);
    if inserted == releases.len() as u32 {
        error
    } else {
        format!(
            "{error}; key-up cleanup inserted {inserted}/{} events",
            releases.len()
        )
    }
}

fn key_event(key: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS, marker: usize) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: marker,
            },
        },
    }
}

#[cfg(test)]
mod tests;
