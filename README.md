<p align="center">
  <img src=".github/assets/logo.svg" width="104" alt="">
</p>
<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset=".github/assets/wordmark-dark.svg">
    <img src=".github/assets/wordmark-light.svg" width="430" alt="klava-nevinovata">
  </picture>
</p>

> Personal fork of [cjpais/Handy](https://github.com/cjpais/Handy), tuned for Russian-language transcription.
> Unsigned installers are available on [Releases](https://github.com/egsok/klava-nevinovata/releases).
> [English] · [Русский](README.ru.md)

`klava-nevinovata` is my daily-driver fork of Handy, the offline speech-to-text desktop app. It stays close to upstream while adding fixes for recognition quality and reliability issues I encounter in everyday Russian dictation on Windows.

Current fork release: `0.9.7-1`, based on upstream `v0.9.7`.

## What's different from upstream

### Recognition quality

Every change in this section directly improves the transcription itself by fixing recurring recognition failures.

- **Fewer Whisper hallucinations.** Silence and uncertain audio are much less likely to turn into repeated sentences or subtitle-like phrases such as “Продолжение следует...”. The decoder carries at most 128 previous-context tokens, collapses repeated sentences, and removes known hallucinations only when a whole sentence matches, preserving real speech that happens to contain the same words.
- **Custom transcription prompt improves Whisper recognition and controls punctuation style.** A per-language prompt helps Whisper models recognize names and specialized terminology, improves punctuation, and lets you specify how that punctuation should look. Whisper Turbo makes the effect especially visible: without a prompt, it often turns Russian dictation into an almost punctuation-free wall of text. The default Russian prompt fixes this behavior, and you can customize it in Settings → Advanced.
- **Punctuation that survives long dictations.** `condition_on_prev_tokens=true` keeps decoder context between whisper.cpp's 30-second windows, so punctuation and sentence continuity do not fall apart after the first window.
- **Readable Cyrillic output from Breeze ASR.** Deterministic post-processing restores spaces between Cyrillic/Cyrillic and Cyrillic/Latin words that Breeze can glue together, while preserving common abbreviations such as `.NET` and `PDF`.

#### Measured impact

The [Russian IT-speech benchmark](https://egorsokolov.ru/ai/whisper-asr-benchmark-russian-it/) compares raw models with their best tuned configurations. On its test corpus, original Whisper Turbo rose from `83.5` to `89.6` Q (`+6.2`), while weaker Turbo RU variants gained up to `+19.2`. The chart measures the complete tuning stack — prompt, anti-hallucination defenses, and capglue — rather than the prompt in isolation. The article contains the full methodology and results in Russian.

[![Q-score improvement from tuning across Whisper and Breeze ASR models](.github/assets/benchmark-tuning-en.png)](https://egorsokolov.ru/ai/whisper-asr-benchmark-russian-it/)

### Reliability and quality of life

- **Full clipboard preservation in the standard paste path.** Files, images, HTML, and text are restored after the transcript is pasted. Upstream PR [#1231](https://github.com/cjpais/Handy/pull/1231) improved the legacy path, but it still snapshots only text or an image there; the fork keeps the broader implementation. Upstream's newer debug-gated reliable-paste path remains available as well.
- **Clipboard timeout protection.** A slow or suspended clipboard owner cannot block the app's main thread indefinitely. If the snapshot times out, the transcript is still pasted and the unavailable old clipboard is not mistaken for an intentionally empty one.
- **Hotkey watchdog and high-priority Windows hook.** The fork tracks modifier resyncs, monitors the hotkey pipeline for stalls, and runs the low-level keyboard hook at time-critical priority. Official `handy-keys` 0.3.3 already includes the live modifier-state correction; the bundled fork carries the additional diagnostics and scheduling safeguards.
- **Atomic settings updates.** Read-modify-write operations are serialized, preventing concurrent changes from silently resetting settings such as history retention.

### Visual identity

- **Two-ink design.** The interface uses the print-workshop language of the [«Нейросеть не виновата»](https://t.me/neiroset_ne_vinovata) channel: magenta and deep violet ink on kraft paper in the light theme, an ink wall in the dark theme, IBM Plex typography, and a matching recording overlay. The upstream information architecture remains intact.

## Download

**Already using an installed copy of 0.9.5-3 or newer?** Click **Check for updates → Update available** at the bottom of the app window. Downloading and installation start only after your click. Portable copies open a manual download link; keep your `Data/` folder when updating.

Pre-built unsigned installers are published on [Releases](https://github.com/egsok/klava-nevinovata/releases). The older `0.8.3-N` stable line remains there as a fallback.

- **Windows:** download `klava-nevinovata_0.9.7-1_x64-setup.exe` (NSIS) or the `.msi`. If SmartScreen shows “Windows protected your PC”, select **More info → Run anyway**.
- **Linux:** download the `.deb`, `.AppImage`, or `.rpm` build for your distribution.
- **macOS:** use `aarch64.dmg` for Apple Silicon or `x64.dmg` for Intel Macs, then drag `klava-nevinovata.app` to `/Applications`.

Because the macOS build is unsigned, the first launch may incorrectly say the app is damaged. Remove the quarantine attribute in Terminal — recursively, because current macOS versions set it on files inside the bundle too, and a non-recursive `xattr -d` leaves the app "damaged":

```bash
sudo xattr -dr com.apple.quarantine /Applications/klava-nevinovata.app
```

If the app still won't open, use `sudo xattr -cr /Applications/klava-nevinovata.app`. The app also needs Microphone and Accessibility permissions under **System Settings → Privacy & Security**. If the Accessibility permission won't stick, see [Troubleshooting](#troubleshooting).

## Troubleshooting

**Previous clipboard content is pasted instead of the transcription.** If History contains the correct transcript, open Debug with `Ctrl+Shift+D` (Windows/Linux) or `Cmd+Shift+D` (macOS). On Windows and macOS, try **Reliable Paste (Beta)** with a clipboard paste method. If it is disabled or unavailable, increase **Paste Delay (After)** and test in the receiving application. This setting delays restoring the old clipboard; **Paste Delay (Before)** delays sending the paste shortcut. These delay settings affect the standard paste path, not Reliable Paste. See upstream [issue #502](https://github.com/cjpais/Handy/issues/502).

**macOS: the Accessibility checkbox won't stick / the app keeps asking for access.** macOS ties this permission to the app's identity, and a stale entry — left by the original Handy app or by an older version of this app — blocks the new one. Fix:

1. Open **System Settings → Privacy & Security → Accessibility** and remove any Handy / klava-nevinovata entries with the **−** button.
2. If the original Handy (or an old version of this app) is still in `/Applications` and you don't use it, delete it.
3. Reset the stored permission in Terminal (the onboarding screen has a "Reset permission" button that runs this first command for you):

   ```bash
   tccutil reset Accessibility ru.egorsokolov.klava-nevinovata
   ```

   If the original Handy or any earlier klava-nevinovata release was ever installed (they used the old identifier), also run `tccutil reset Accessibility com.pais.handy` (note: this resets the permission for the original Handy too, if you use it).

4. Relaunch the app and grant the permission again.

**macOS: permissions are asked again after an update.** Expected for now: the builds are not signed with an Apple Developer certificate, so macOS treats every updated binary as a new applicant. Re-grant and continue.

## Build

For platform prerequisites and packaging details, see [BUILD.md](BUILD.md). The short version is:

```bash
bun install
bun run tauri build
```

On Windows, if MSVC runs out of heap while compiling generated shader sources, retry with `CARGO_BUILD_JOBS=8`. A custom short `CARGO_TARGET_DIR` is no longer required by default.

## Upstream

This fork tracks [cjpais/Handy](https://github.com/cjpais/Handy). For general usage, model management, troubleshooting, platform notes, and CLI flags, see the [upstream README](https://github.com/cjpais/Handy/blob/main/README.md).

For the official supported app, use [handy.computer](https://handy.computer) or [upstream releases](https://github.com/cjpais/Handy/releases).

## Author

Built by [Egor Sokolov](https://egorsokolov.ru/). I write about practical AI tooling in the Telegram channel [«Нейросеть не виновата»](https://t.me/neiroset_ne_vinovata).

Other open experiments:

- [plan-tango](https://github.com/egsok/plan-tango) — a Claude ↔ Codex review loop for plans.
- [press-1](https://github.com/egsok/press-1) — answer Claude Code permission prompts with one keypress from any window.
- [napotom](https://github.com/egsok/napotom) — a desktop video downloader and friendly GUI for yt-dlp.

## License

MIT, inherited from cjpais/Handy — see [LICENSE](LICENSE). Original work © cjpais and contributors. Fork changes © 2026 Egor Sokolov.
