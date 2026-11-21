## Что нового в 0.10.1-3

- Если выбрать русский или пару русский + английский, мастер первого запуска рекомендует **Breeze-ASR-25**. У Breeze в формате GGUF и её ранее загруженной версии .bin есть поддержка русского и оценка **A−** для русского и английского. Изначально Breeze оптимизировали для тайваньского мандаринского с английским, но она также хорошо распознаёт русскую речь с английскими терминами.
- У **Whisper Large v3 Turbo** теперь **A−** для русского. Чтобы получить сопоставимое с Breeze качество в смешанной русско-английской диктовке, выберите русский языком распознавания и используйте **Transcription Prompt**.
- На новой установке уже выбран пресет **V3 — Ru-En**, который просит модель писать английские термины латиницей. При обновлении сохраняется ваша прежняя настройка, в том числе пустой промпт.
- При первом запуске приложение открывается в светлой теме. Обновление сохраняет выбранную тему.
- В тёмной теме мастера легче читать сообщения, заметки и подсказки во время пробной диктовки. В карточке модели появилась подсказка о скорости распознавания на видеокартах и Mac с чипами серии M.

Оценки отражают опыт диктовки в форке. Список изменений предыдущей версии, включая расшифровку файлов и исправления вставки в Windows, есть в [описании 0.10.1-2](https://github.com/egsok/klava-nevinovata/releases/tag/v0.10.1-2).

## What's new in 0.10.1-3

- First-time setup recommends **Breeze-ASR-25** when you select Russian alone or together with English. Both the GGUF model and previously downloaded Breeze .bin models support Russian and show **A−** for Russian and English. Originally tuned for Taiwanese Mandarin mixed with English, Breeze also handles Russian dictation with English terms well.
- **Whisper Large v3 Turbo** now shows **A−** for Russian. For mixed Russian and English dictation comparable in quality to Breeze, select Russian as the transcription language and use a **Transcription Prompt**.
- New installations start with **V3 — Ru-En**, which asks the model to preserve English terms in Latin script. Updating keeps your existing prompt, including an empty one.
- New installations open in light mode. Updates preserve your theme preference.
- Messages, notes and practice hints are easier to read in dark mode during setup. Model speed descriptions now explain how hardware acceleration can affect the wait.
