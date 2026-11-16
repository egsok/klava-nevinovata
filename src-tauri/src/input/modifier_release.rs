use ::windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VIRTUAL_KEY, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_RCONTROL,
    VK_RMENU, VK_RSHIFT, VK_RWIN,
};
use std::time::{Duration, Instant};

const RELEASE_WAIT_MAX: Duration = Duration::from_secs(1);
const RELEASE_POLL: Duration = Duration::from_millis(5);
const MODIFIERS: [VIRTUAL_KEY; 8] = [
    VK_LCONTROL,
    VK_RCONTROL,
    VK_LSHIFT,
    VK_RSHIFT,
    VK_LMENU,
    VK_RMENU,
    VK_LWIN,
    VK_RWIN,
];

/// Wait on the paste worker, before dispatching the chord to the main thread.
/// An already released shortcut adds no sleep. This observes OS key state,
/// not modifier-only shortcuts whose events a keyboard hook fully swallows.
pub(crate) struct ModifierReleaseWait {
    started: Instant,
    deadline: Option<Duration>,
}

impl ModifierReleaseWait {
    pub(crate) fn new() -> Self {
        Self {
            started: Instant::now(),
            deadline: None,
        }
    }

    pub(crate) fn wait(&mut self) {
        let result = wait_while_held(
            &mut self.deadline,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || self.started.elapsed(),
            || {
                modifiers_down(|key| {
                    // SAFETY: GetAsyncKeyState accepts a virtual-key code, no pointers.
                    unsafe { GetAsyncKeyState(i32::from(key.0)) }
                })
            },
            std::thread::sleep,
        );
        if result.timed_out {
            log::warn!(
                "Modifier release deadline reached; waited {} ms in this path; sending paste anyway",
                result.waited.as_millis()
            );
        } else if !result.waited.is_zero() {
            log::debug!(
                "Waited {} ms for modifier release before sending the paste chord",
                result.waited.as_millis()
            );
        }
    }
}

fn modifiers_down(mut key_state: impl FnMut(VIRTUAL_KEY) -> i16) -> bool {
    // The high bit means down now. The low bit only means pressed since a
    // previous query, and must not delay a paste after the key was released.
    MODIFIERS.into_iter().any(|key| key_state(key) < 0)
}

#[derive(Debug, PartialEq, Eq)]
struct ReleaseWait {
    waited: Duration,
    timed_out: bool,
}

fn wait_while_held(
    deadline: &mut Option<Duration>,
    max: Duration,
    poll: Duration,
    mut now: impl FnMut() -> Duration,
    mut held: impl FnMut() -> bool,
    mut sleep: impl FnMut(Duration),
) -> ReleaseWait {
    if !held() {
        return ReleaseWait {
            waited: Duration::ZERO,
            timed_out: false,
        };
    }
    let start = now();
    // Start the shared budget only when something is held. A fallback cannot
    // reset it, and time spent between the two paths also counts toward it.
    let deadline = *deadline.get_or_insert(start + max);
    loop {
        let current = now();
        if current >= deadline {
            return ReleaseWait {
                waited: current.saturating_sub(start),
                timed_out: true,
            };
        }
        sleep(poll.min(deadline - current));
        // Observe release even on the deadline, before classifying a timeout.
        if !held() {
            return ReleaseWait {
                waited: now().saturating_sub(start),
                timed_out: false,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[test]
    fn released_modifiers_add_no_sleep_or_clock_read() {
        let result = wait_while_held(
            &mut None,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || panic!("no clock needed for immediate paste"),
            || false,
            |_| panic!("released modifiers must not add latency"),
        );
        assert_eq!(
            result,
            ReleaseWait {
                waited: Duration::ZERO,
                timed_out: false
            }
        );
    }

    #[test]
    fn every_left_and_right_modifier_uses_only_the_current_down_bit() {
        assert_eq!(
            MODIFIERS.map(|key| key.0),
            [0xa2, 0xa3, 0xa0, 0xa1, 0xa4, 0xa5, 0x5b, 0x5c]
        );
        for modifier in MODIFIERS {
            for state in [i16::MIN, i16::MIN | 1] {
                assert!(modifiers_down(|key| if key == modifier {
                    state
                } else {
                    0
                }));
            }
            assert!(!modifiers_down(|key| if key == modifier { 1 } else { 0 }));
        }
    }

    #[test]
    fn held_modifier_delays_dispatch_only_until_release() {
        let now = Cell::new(Duration::ZERO);
        let trace = RefCell::new(Vec::new());
        let result = wait_while_held(
            &mut None,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || now.get(),
            || {
                trace.borrow_mut().push(("check", now.get()));
                now.get() < Duration::from_millis(15)
            },
            |duration| {
                trace.borrow_mut().push(("sleep", duration));
                now.set(now.get() + duration);
            },
        );
        trace.borrow_mut().push(("dispatch", now.get()));
        assert_eq!(
            result,
            ReleaseWait {
                waited: Duration::from_millis(15),
                timed_out: false
            }
        );
        let trace = trace.borrow();
        assert_eq!(trace[trace.len() - 2], ("check", Duration::from_millis(15)));
        assert_eq!(trace.last(), Some(&("dispatch", Duration::from_millis(15))));
        assert_eq!(
            trace.iter().filter(|(event, _)| *event == "sleep").count(),
            3
        );
    }

    #[test]
    fn sustained_hold_is_bounded_and_final_poll_is_capped() {
        let now = Cell::new(Duration::ZERO);
        let sleeps = RefCell::new(Vec::new());
        let max = Duration::from_millis(12);
        let result = wait_while_held(
            &mut None,
            max,
            RELEASE_POLL,
            || now.get(),
            || true,
            |duration| {
                sleeps.borrow_mut().push(duration);
                now.set(now.get() + duration);
            },
        );
        assert_eq!(
            result,
            ReleaseWait {
                waited: max,
                timed_out: true
            }
        );
        assert_eq!(*sleeps.borrow(), [5, 5, 2].map(Duration::from_millis));
    }

    #[test]
    fn production_timeout_does_not_wait_longer_than_one_second() {
        let now = Cell::new(Duration::ZERO);
        let result = wait_while_held(
            &mut None,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || now.get(),
            || true,
            |duration| now.set(now.get() + duration),
        );
        assert_eq!(
            result,
            ReleaseWait {
                waited: Duration::from_secs(1),
                timed_out: true,
            }
        );
    }

    #[test]
    fn legacy_fallback_does_not_restart_an_exhausted_reliable_wait() {
        let now = Cell::new(Duration::ZERO);
        let mut deadline = None;
        let reliable = wait_while_held(
            &mut deadline,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || now.get(),
            || true,
            |duration| now.set(now.get() + duration),
        );
        assert!(reliable.timed_out);
        now.set(now.get() + Duration::from_millis(60)); // Existing legacy delay.
        let fallback = wait_while_held(
            &mut deadline,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || now.get(),
            || true,
            |_| panic!("fallback must not add a second wait budget"),
        );
        assert_eq!(
            fallback,
            ReleaseWait {
                waited: Duration::ZERO,
                timed_out: true
            }
        );
        assert_eq!(deadline, Some(Duration::from_secs(1)));
    }

    #[test]
    fn fallback_uses_only_the_deadline_remaining_after_release_and_existing_delay() {
        let now = Cell::new(Duration::ZERO);
        let mut deadline = None;
        let reliable = wait_while_held(
            &mut deadline,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || now.get(),
            || now.get() < Duration::from_millis(400),
            |duration| now.set(now.get() + duration),
        );
        assert!(!reliable.timed_out);
        now.set(now.get() + Duration::from_millis(100)); // Work/delay before fallback.
        let fallback = wait_while_held(
            &mut deadline,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || now.get(),
            || true, // The user pressed a modifier again.
            |duration| now.set(now.get() + duration),
        );
        assert_eq!(reliable.waited, Duration::from_millis(400));
        assert_eq!(
            fallback,
            ReleaseWait {
                waited: Duration::from_millis(500),
                timed_out: true
            }
        );
        assert_eq!(now.get(), Duration::from_secs(1));
    }

    #[test]
    fn released_fallback_stays_immediate_even_with_an_expired_deadline() {
        let mut deadline = Some(Duration::ZERO);
        let result = wait_while_held(
            &mut deadline,
            RELEASE_WAIT_MAX,
            RELEASE_POLL,
            || panic!("no clock needed once keys are released"),
            || false,
            |_| panic!("no sleep once keys are released"),
        );
        assert_eq!(
            result,
            ReleaseWait {
                waited: Duration::ZERO,
                timed_out: false
            }
        );
    }

    #[test]
    fn release_at_deadline_is_not_reported_as_timeout() {
        let now = Cell::new(Duration::ZERO);
        let max = Duration::from_millis(10);
        let result = wait_while_held(
            &mut None,
            max,
            RELEASE_POLL,
            || now.get(),
            || now.get() < max,
            |duration| now.set(now.get() + duration),
        );
        assert_eq!(
            result,
            ReleaseWait {
                waited: max,
                timed_out: false
            }
        );
    }
}
