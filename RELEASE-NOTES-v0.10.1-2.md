# klava-nevinovata 0.10.1-2

Большое обновление на базе Handy 0.10.0 и 0.10.1. Сохранены доработки klava-nevinovata: пользовательский промпт и словарь для Whisper/Breeze, обработка результатов распознавания, настройки истории и защита буфера обмена.

## Главное

- **Расшифровка аудио и видео из файлов.** Добавляйте файлы в «Истории» или перетаскивайте в окно приложения. Задания обрабатываются в очереди, результаты сохраняются в истории. Добавлена поддержка Opus, в том числе голосовых сообщений в `.opus`, `.ogg` и `.webm`; поддержка конкретного файла зависит от его аудиокодека.
- **Каталог моделей под ваши языки.** Выберите предпочитаемые языки, чтобы видеть оценки качества и рекомендации: подходящую модель, более быстрый и более точный варианты. Обновлён первый запуск с выбором модели и пробной диктовкой.
- **Автоопределение языка.** Для моделей, которым нужен внешний детектор, приложение предлагает скачать его при выборе «Авто». Экспериментально определение можно ограничить предпочитаемыми языками.
- **Устойчивее длинные расшифровки.** Длинное аудио по умолчанию разбивается по паузам; это можно отключить в дополнительных настройках. Движок распознавания работает в отдельном процессе с ограничением ожидания и восстановлением после ошибок.

## Важно при обновлении моделей

Старые ONNX-модели заменены вариантами GGUF и больше не запускаются. Экран миграции поможет скачать замену. **Файлы старых моделей автоматически не удаляются** — освободить место можно отдельной кнопкой в разделе моделей. Уже скачанные Whisper/Breeze в формате `.bin` продолжают поддерживаться.

## Вставка текста в Windows

Добавлены две доработки против появления `v` или `м` вместо текста при быстрой диктовке: отправка сочетания вставки одним пакетом и ожидание отпускания физических Ctrl/Shift/Alt/Win перед вставкой.

В предварительном пользовательском тесте с обеими правками ошибка больше не наблюдалась. Это обнадёживающий результат, но полное устранение редкого сбоя пока не подтверждено. Правки сохранены отдельными коммитами; история проверок и гипотеза о роли ожидания описаны в [технической заметке](https://github.com/egsok/klava-nevinovata/blob/v0.10.1-2/docs/paste-v0.10.1-2.md).

## Другие изменения

- Понятнее сообщения об отсутствующей модели, тишине с микрофона и попытке вставки в окно с повышенными правами в Windows.
- Улучшена фильтрация пустых результатов и повторения словаря. Если постобработка вернула пустой текст, сохраняется исходная диктовка; ожидание постобработки ограничено.
- Исправления истории, воспроизведения и перемотки аудио, горячих клавиш, Escape и клавиатурной навигации. Язык можно выбирать из меню трея.
- Linux: улучшения Shift+Insert, PRIMARY, запасных способов вставки и работы оверлея в X11/Wayland; исправления AppImage и Gamescope.
- macOS: исправления запросов разрешений и экспериментальный AirPods Mode.
- Пользовательский hook после транскрипции и поддержка Anthropic Messages API в пользовательском провайдере постобработки.

Полные списки изменений оригинала: [Handy 0.10.0](https://github.com/cjpais/Handy/releases/tag/v0.10.0), [Handy 0.10.1](https://github.com/cjpais/Handy/releases/tag/v0.10.1), [все изменения исходников с 0.9.8](https://github.com/cjpais/Handy/compare/v0.9.8...v0.10.1).

**При обновлении portable-версии сохраните папку `Data/` с настройками, моделями и записями.**

---

## English

Based on Handy 0.10.0 and 0.10.1, preserving klava-nevinovata's Whisper/Breeze prompt and vocabulary behavior, recognition cleanup, history preferences and clipboard protection.

- Transcribe audio/video files from History or by dropping them into the app. Includes queued jobs and Opus support, subject to the file's audio codec.
- Language-aware model recommendations, quality ratings and an updated first-run experience.
- Optional language detection for models that need an external detector.
- Pause-based chunking for long audio and an isolated recognition engine process with timeouts and recovery.
- Retired ONNX models require GGUF replacements. **Their files are retained until you explicitly delete them in Models.** Existing Whisper/Breeze `.bin` models remain supported.
- Two Windows paste mitigations: batched paste key events and waiting for physical modifiers to be released. Initial user testing with both changes showed no further `v`/`м` incidents; definitive resolution and each change's individual contribution remain unverified.
- Further history, hotkey, post-processing, accessibility and platform fixes; transcription hooks and Anthropic Messages API support for custom providers.

See the upstream release links above for the full lists. Preserve `Data/` when updating a portable installation.
