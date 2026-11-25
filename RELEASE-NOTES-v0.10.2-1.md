## Что изменилось в 0.10.2-1

- На некоторых Bluetooth-наушниках пропадали сигналы начала и конца диктовки. Теперь приложение использует размер звукового буфера, который выбирает устройство.
- В macOS мастер первого запуска больше не сбрасывает уже выданные разрешения при запросе доступа к микрофону или Универсальному доступу.
- Интерфейс доступен ещё на трёх языках — армянском, венгерском и тайском.

### Если вы пропустили 0.10.1-3

Для русской диктовки с английскими словами мастер рекомендует **Breeze-ASR-25**. Если вы пользуетесь **Whisper Large v3 Turbo**, выберите русский язык распознавания и пресет **V3 — Ru-En**. На новой установке этот пресет уже выбран, при обновлении сохраняется ваш выбор. [Подробнее о моделях и настройке](https://github.com/egsok/klava-nevinovata/releases/tag/v0.10.1-3).

## Changes in 0.10.2-1

- Some Bluetooth headsets would not play the sounds at the start and end of dictation. The app now lets the output device choose the audio buffer size.
- On macOS, first-time setup now preserves existing permissions when requesting microphone or Accessibility access.
- The interface is now available in Armenian, Hungarian and Thai.

### If you missed 0.10.1-3

Setup recommends **Breeze-ASR-25** for Russian dictation mixed with English. If you use **Whisper Large v3 Turbo**, select Russian as the transcription language and choose the **V3 — Ru-En** prompt. New installations already use this preset. Updates preserve your choice. [Model recommendations and setup details](https://github.com/egsok/klava-nevinovata/releases/tag/v0.10.1-3).
