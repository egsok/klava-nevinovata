use super::*;
use std::cell::RefCell;

fn keys(events: &[INPUT]) -> Vec<(u16, bool)> {
    events
        .iter()
        .map(|event| {
            assert_eq!(event.r#type, INPUT_KEYBOARD);
            // SAFETY: PasteChord only creates INPUT_KEYBOARD with the ki member.
            let key = unsafe { event.Anonymous.ki };
            (key.wVk.0, key.dwFlags == KEYEVENTF_KEYUP)
        })
        .collect()
}

fn chord(shift: bool) -> PasteChord {
    PasteChord::new(shift, 0x1234, |key| match key {
        VK_CONTROL => 0x1d,
        VK_SHIFT => 0x2a,
        VK_V => 0x2f,
        _ => panic!("unexpected key"),
    })
}

#[test]
fn paste_is_one_batch_then_hold_then_modifier_release() {
    for shift in [false, true] {
        let chord = chord(shift);
        let trace = RefCell::new(Vec::new());
        chord
            .send(
                Duration::from_millis(100),
                |events| {
                    trace.borrow_mut().push(format!("{:?}", keys(events)));
                    events.len() as u32
                },
                |duration| {
                    assert_eq!(duration, Duration::from_millis(100));
                    trace.borrow_mut().push("hold".into());
                },
            )
            .unwrap();
        let expected_press = if shift {
            vec![(0x11, false), (0x10, false), (0x56, false), (0x56, true)]
        } else {
            vec![(0x11, false), (0x56, false), (0x56, true)]
        };
        let expected_release = if shift {
            vec![(0x10, true), (0x11, true)]
        } else {
            vec![(0x11, true)]
        };
        assert_eq!(
            *trace.borrow(),
            vec![
                format!("{expected_press:?}"),
                "hold".into(),
                format!("{expected_release:?}")
            ]
        );
    }
}

#[test]
fn virtual_keys_keep_scan_codes_and_custom_enigo_marker() {
    let chord = chord(true);
    for event in chord
        .press
        .iter()
        .chain(&chord.release)
        .chain(&chord.cleanup)
    {
        assert_eq!(event.r#type, INPUT_KEYBOARD);
        // SAFETY: All events were constructed with the ki union member.
        let key = unsafe { event.Anonymous.ki };
        assert_eq!(key.dwExtraInfo, 0x1234);
        assert_eq!(key.time, 0);
        assert!(key.dwFlags == KEYBD_EVENT_FLAGS::default() || key.dwFlags == KEYEVENTF_KEYUP);
        assert_eq!(
            key.wScan,
            match key.wVk {
                VK_CONTROL => 0x1d,
                VK_SHIFT => 0x2a,
                VK_V => 0x2f,
                _ => panic!("unexpected key"),
            }
        );
    }
}

#[test]
fn rejected_chord_does_not_release_physical_keys_or_wait() {
    for shift in [false, true] {
        let chord = chord(shift);
        let mut calls = 0;
        let error = chord
            .send(
                Duration::ZERO,
                |_| {
                    calls += 1;
                    0
                },
                |_| panic!("must not wait"),
            )
            .unwrap_err();
        assert!(error.contains("inserted 0/"));
        assert_eq!(calls, 1);
    }
}

#[test]
fn every_partial_chord_gets_one_key_up_cleanup_without_replaying_paste() {
    for shift in [false, true] {
        let chord = chord(shift);
        for partial in 1..chord.press.len() as u32 {
            for cleanup_count in 0..=chord.cleanup.len() as u32 {
                let mut calls = 0;
                let error = chord
                    .send(
                        Duration::ZERO,
                        |events| {
                            calls += 1;
                            if calls == 1 {
                                return partial;
                            }
                            assert_eq!(calls, 2, "cleanup must be bounded");
                            assert_eq!(keys(events), keys(&chord.cleanup));
                            assert!(keys(events).iter().all(|(_, up)| *up));
                            cleanup_count
                        },
                        |_| panic!("must not hold a failed chord"),
                    )
                    .unwrap_err();
                assert_eq!(calls, 2);
                assert!(error.contains(&format!("inserted {partial}/")));
                assert_eq!(
                    error.contains("cleanup"),
                    cleanup_count < chord.cleanup.len() as u32
                );
            }
        }
    }
}

#[test]
fn every_failed_release_gets_one_modifier_cleanup_and_stays_an_error() {
    for shift in [false, true] {
        let chord = chord(shift);
        for partial in 0..chord.release.len() as u32 {
            for cleanup_count in 0..=chord.release.len() as u32 {
                let mut calls = 0;
                let held = RefCell::new(false);
                let error = chord
                    .send(
                        Duration::ZERO,
                        |events| {
                            calls += 1;
                            match calls {
                                1 => events.len() as u32,
                                2 => {
                                    assert!(*held.borrow());
                                    partial
                                }
                                3 => {
                                    assert_eq!(keys(events), keys(&chord.release));
                                    assert!(keys(events)
                                        .iter()
                                        .all(|(key, up)| *up && *key != VK_V.0));
                                    cleanup_count
                                }
                                _ => panic!("cleanup must be bounded"),
                            }
                        },
                        |_| *held.borrow_mut() = true,
                    )
                    .unwrap_err();
                assert_eq!(calls, 3);
                assert!(error.contains("Failed to release paste modifiers"));
                assert_eq!(
                    error.contains("cleanup"),
                    cleanup_count < chord.release.len() as u32
                );
            }
        }
    }
}

#[test]
fn physical_ctrl_release_between_calls_cannot_split_the_paste_chord() {
    // Model only the documented SendInput boundary: physical input may arrive
    // between calls, but not between events in one submitted array. This is
    // not a live Windows/UI acceptance test.
    fn receive_batch(events: &[INPUT], ctrl: &mut bool, bare_v: &mut bool, pasted: &mut bool) {
        for (key, up) in keys(events) {
            if key == VK_CONTROL.0 {
                *ctrl = !up;
            }
            if key == VK_V.0 && !up {
                *bare_v |= !*ctrl;
                *pasted |= *ctrl;
            }
        }
        *ctrl = false; // Physical Ctrl-up at each call boundary.
    }
    let chord = chord(false);
    let (mut ctrl, mut bare_v, mut pasted) = (true, false, false);
    receive_batch(&chord.press[..1], &mut ctrl, &mut bare_v, &mut pasted);
    receive_batch(&chord.press[1..], &mut ctrl, &mut bare_v, &mut pasted);
    assert!(bare_v && !pasted, "the previous split calls allow a bare V");

    (ctrl, bare_v, pasted) = (true, false, false);
    chord
        .send(
            Duration::ZERO,
            |events| {
                receive_batch(events, &mut ctrl, &mut bare_v, &mut pasted);
                events.len() as u32
            },
            |_| {},
        )
        .unwrap();
    assert!(pasted && !bare_v, "Ctrl must still be down at the V event");
}
