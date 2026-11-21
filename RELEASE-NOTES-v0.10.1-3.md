## Что нового в 0.10.1-3

- Если выбрать русский или пару русский + английский, мастер первого запуска рекомендует **Breeze-ASR-25**. Обе версии Breeze в каталоге поддерживают русский и получили **A−** для русского и английского. Изначально Breeze оптимизировали для тайваньского мандаринского с английским, но она также хорошо распознаёт русскую речь с английскими терминами.
- У **Whisper Large v3 Turbo** теперь **A−** для русского. Чтобы получить сопоставимое с Breeze качество в смешанной русско-английской диктовке, выберите русский языком распознавания и используйте **Transcription Prompt**.
- На новой установке уже выбран пресет **V3 — Ru-En**, который просит модель писать английские термины латиницей. При обновлении сохраняется ваша прежняя настройка, в том числе пустой промпт.

Изменения касаются каталога и настроек. Оценки отражают опыт диктовки в форке. Список изменений предыдущей версии, включая расшифровку файлов и исправления вставки в Windows, есть в [описании 0.10.1-2](https://github.com/egsok/klava-nevinovata/releases/tag/v0.10.1-2).

## What's new in 0.10.1-3

- First-time setup recommends **Breeze-ASR-25** when you select Russian alone or together with English. Both Breeze models in the catalog now list Russian and show **A−** for Russian and English. Originally tuned for Taiwanese Mandarin mixed with English, Breeze also handles Russian dictation with English terms well.
- **Whisper Large v3 Turbo** now shows **A−** for Russian. For mixed Russian and English dictation comparable in quality to Breeze, select Russian as the transcription language and use a **Transcription Prompt**.
- New installations start with **V3 — Ru-En**, which asks the model to preserve English terms in Latin script. Updating keeps your existing prompt, including an empty one.
