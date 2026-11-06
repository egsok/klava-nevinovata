//! Background transcription. Imported files use chunks; Whisper dictations
//! retain a whole-clip run so decoder context survives a history retry.

use anyhow::Result;
use log::{debug, info, warn};
use std::time::Instant;

use super::{
    agreed_language, engine_error, is_near_silent, post_process_transcription_text,
    real_time_factor, BatchContext, Transcription, TranscriptionManager,
};
use crate::audio_toolkit::chunking::Chunker;
use crate::engine_supervisor::EngineError;
use crate::managers::audio::create_detector;

/// Tries per imported chunk when a dictation's cancel interrupts it.
/// Whole dictation retries survive foreign cancels until their own cancellation.
const MAX_CHUNK_ATTEMPTS: usize = 3;

/// Audio handed to the chunker at a time: the first chunk is transcribed
/// without scoring the whole file first. Cuts match the whole-file plan (a
/// rare short blip at a feed boundary can nudge one within its pause).
const FEED_SAMPLES: usize = 60 * 16_000;

fn background_uses_whole_clip(is_whisper: bool, source_name: Option<&str>) -> bool {
    is_whisper && source_name.is_none()
}

/// What a background transcription asks of whoever runs it.
pub trait ChunkControl {
    /// Records the actual mode chosen from the loaded model.
    fn started(&self, whole_clip: bool);
    /// Called before each feed to the chunker and before each chunk is
    /// decoded. Blocks while dictation is using the engine; `false` stops the
    /// transcription.
    fn proceed(&self) -> bool;
    /// `done` of `total` samples are transcribed.
    fn progress(&self, done: usize, total: usize);
}

/// The production feed/decode loop; callbacks keep native VAD and workers
/// outside the control-flow tests.
fn decode_background_chunks<T>(
    audio: &[f32],
    is_whisper: bool,
    source_name: Option<&str>,
    control: &dyn ChunkControl,
    mut feed: impl FnMut(Option<&[f32]>) -> Vec<Vec<f32>>,
    mut decode: impl FnMut(Vec<f32>) -> std::result::Result<T, EngineError>,
) -> Result<Option<Vec<T>>> {
    let whole_clip = background_uses_whole_clip(is_whisper, source_name);
    control.started(whole_clip);
    let total = audio.len();
    let mut pieces = audio.chunks(if whole_clip {
        total.max(1)
    } else {
        FEED_SAMPLES
    });

    let mut transcripts: Vec<T> = Vec::new();
    let mut done = 0;
    control.progress(done, total);
    loop {
        // Scoring waits out dictation too, and stops on cancel.
        if !control.proceed() {
            return Ok(None);
        }
        let (chunks, last) = match pieces.next() {
            Some(piece) if whole_clip => (vec![piece.to_vec()], true),
            Some(piece) => (feed(Some(piece)), false),
            None => (feed(None), true),
        };
        for chunk in chunks {
            let len = chunk.len();
            if is_near_silent(&chunk) {
                done += len;
                control.progress(done, total);
                continue;
            }
            let mut attempts = 0;
            let transcript = loop {
                if !control.proceed() {
                    return Ok(None);
                }
                attempts += 1;
                match decode(chunk.clone()) {
                    Ok(transcript) => break transcript,
                    Err(EngineError::Cancelled) if whole_clip || attempts < MAX_CHUNK_ATTEMPTS => {
                        warn!("A background chunk was cancelled; trying it again");
                    }
                    Err(e) => return Err(engine_error(e)),
                }
            };
            transcripts.push(transcript);
            done += len;
            control.progress(done, total);
        }
        if last {
            break;
        }
    }

    Ok(Some(transcripts))
}

impl TranscriptionManager {
    /// Transcribe `audio` chunk by chunk with the selected model, one
    /// paragraph per chunk. `None` if `control` stopped it.
    pub fn transcribe_in_chunks(
        &self,
        audio: Vec<f32>,
        source_name: Option<&str>,
        control: &dyn ChunkControl,
    ) -> Result<Option<Transcription>> {
        let started = Instant::now();
        let total = audio.len();
        if total == 0 || is_near_silent(&audio) {
            return Ok(Some(Transcription {
                text: String::new(),
                language: None,
            }));
        }

        self.touch_activity();
        self.initiate_model_load(false);
        // Read once: the model can't change until the job ends.
        let BatchContext {
            settings,
            active_model,
            setup,
            ..
        } = self.batch_context(&audio)?;

        let backend = settings.vad_backend;
        let app_handle = self.app_handle.clone();
        let mut chunker = Chunker::new(Box::new(move || create_detector(backend, &app_handle)));
        let Some(transcripts) = decode_background_chunks(
            &audio,
            setup.is_whisper,
            source_name,
            control,
            |piece| match piece {
                Some(piece) => chunker.push(piece),
                None => chunker.finish(),
            },
            |chunk| {
                let mark = self.engine.cancel_mark();
                // A cancel between the loop's check and this mark must not
                // start another whole-clip decode with a fresh generation.
                if !control.proceed() {
                    return Err(EngineError::Cancelled);
                }
                self.run_batch(mark, chunk, &setup.run_options, &active_model)
            },
        )?
        else {
            return Ok(None);
        };

        debug!("Transcribed {} chunks in the background", transcripts.len());
        let language = setup.spoken_language(agreed_language(&transcripts).as_deref());
        let paragraphs: Vec<String> = transcripts
            .into_iter()
            .map(|t| {
                let output_language = setup.output_language(&settings, t.language);
                post_process_transcription_text(
                    t.text,
                    &settings,
                    setup.custom_words_prompted(),
                    setup.is_whisper,
                    &output_language,
                    &setup.languages,
                )
            })
            .filter(|p| !p.trim().is_empty())
            .collect();

        let elapsed_secs = started.elapsed().as_secs_f64();
        let audio_secs = total as f64 / 16_000.0;
        info!(
            "Background transcription of {:.1}s of audio in {} paragraphs took {:.1}s ({:.1}x real time)",
            audio_secs,
            paragraphs.len(),
            elapsed_secs,
            real_time_factor(audio_secs, elapsed_secs)
        );
        Ok(Some(Transcription {
            text: paragraphs.join("\n\n"),
            language: Some(language),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[derive(Default)]
    struct Control {
        cancelled: Cell<bool>,
        whole_clip: Cell<bool>,
        done: Cell<usize>,
    }

    impl ChunkControl for Control {
        fn started(&self, whole_clip: bool) {
            self.whole_clip.set(whole_clip);
        }

        fn proceed(&self) -> bool {
            !self.cancelled.get()
        }

        fn progress(&self, done: usize, _total: usize) {
            self.done.set(done);
        }
    }

    fn feed(piece: Option<&[f32]>) -> Vec<Vec<f32>> {
        piece.map(|p| vec![p.to_vec()]).unwrap_or_default()
    }

    #[test]
    fn whisper_dictation_retry_preserves_context_but_imports_stay_chunked() {
        let audio = vec![0.1; FEED_SAMPLES + 16_000];
        for (whisper, source, whole) in [
            (true, None, true),
            (true, Some("interview.wav"), false),
            (true, Some(""), false),
            (false, None, false),
        ] {
            let control = Control::default();
            let feeds = Cell::new(0);
            let lengths = decode_background_chunks(
                &audio,
                whisper,
                source,
                &control,
                |piece| {
                    feeds.set(feeds.get() + 1);
                    feed(piece)
                },
                |chunk| Ok(chunk.len()),
            )
            .unwrap()
            .unwrap();
            if whole {
                assert_eq!(lengths, vec![audio.len()]);
                assert_eq!(feeds.get(), 0, "whole dictation must bypass VAD chunking");
            } else {
                assert_eq!(lengths, vec![FEED_SAMPLES, 16_000]);
                assert!(feeds.get() > 0);
            }
            assert_eq!(control.whole_clip.get(), whole);
            assert_eq!(control.done.get(), audio.len());
        }
    }

    #[test]
    fn background_loop_skips_silent_chunks_before_calling_engine() {
        for source in [None, Some("file.wav")] {
            for level in [0.004, 0.006] {
                let control = Control::default();
                let calls = Cell::new(0);
                let result =
                    decode_background_chunks(&[level; 64], true, source, &control, feed, |_| {
                        calls.set(calls.get() + 1);
                        Ok(())
                    })
                    .unwrap()
                    .unwrap();
                assert_eq!(calls.get(), usize::from(level > 0.005));
                assert_eq!(result.len(), calls.get());
                assert_eq!(control.done.get(), 64);
            }
        }
        // The file is audible overall; a silent chunk still must not decode.
        let audio = vec![0.1; 64];
        let result = decode_background_chunks(
            &audio,
            true,
            Some("file.wav"),
            &Control::default(),
            |piece| {
                piece
                    .map(|_| vec![vec![0.004; 32], vec![0.1; 32]])
                    .unwrap_or_default()
            },
            |chunk| Ok(chunk.len()),
        )
        .unwrap()
        .unwrap();
        assert_eq!(result, vec![32]);
    }

    #[test]
    fn foreign_cancels_do_not_fail_whole_retry_but_own_cancel_stops_it() {
        let control = Control::default();
        let mut attempts = 0;
        let result = decode_background_chunks(&[0.1; 64], true, None, &control, feed, |_| {
            attempts += 1;
            if attempts <= MAX_CHUNK_ATTEMPTS + 1 {
                Err(EngineError::Cancelled)
            } else {
                Ok("complete")
            }
        })
        .unwrap()
        .unwrap();
        assert_eq!(result, vec!["complete"]);
        assert_eq!(attempts, MAX_CHUNK_ATTEMPTS + 2);

        let mut calls = 0;
        let result = decode_background_chunks::<()>(&[0.1; 64], true, None, &control, feed, |_| {
            calls += 1;
            control.cancelled.set(true);
            Err(EngineError::Cancelled)
        })
        .unwrap();
        assert!(result.is_none());
        assert_eq!(calls, 1);
    }

    #[test]
    fn imported_file_keeps_bounded_cancellation_retries() {
        let mut calls = 0;
        assert!(decode_background_chunks::<()>(
            &[0.1; 64],
            true,
            Some("file.wav"),
            &Control::default(),
            feed,
            |_| {
                calls += 1;
                Err(EngineError::Cancelled)
            },
        )
        .is_err());
        assert_eq!(calls, MAX_CHUNK_ATTEMPTS);
    }
}
