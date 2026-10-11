# klava-nevinovata 0.9.8-1

Обновление на базе Handy 0.9.8: исправления горячих клавиш, записи без загруженной модели и первого запуска. Сохранены оформление klava-nevinovata, пользовательский промпт и словарь, обработка текста Whisper/Breeze, защита настроек и работа с буфером обмена.

## Что изменилось

- **Клавиша отмены больше не остаётся занятой.** После очень короткой записи сочетание отмены (по умолчанию Esc) могло оставаться перехваченным во всей системе до выхода из приложения.
- **Запись не начинается без модели.** Если выбранная модель ещё не скачана или её файлы удалены, горячая клавиша не включает микрофон: приложение сразу покажет ошибку загрузки модели вместо записи, которую нечем распознать.
- **Исправлены сочетания клавиш в реализации Tauri.** Клавиши вроде Scroll Lock снова назначаются. Сочетание, которое эта реализация не поддерживает, при переключении сбрасывается на стандартное с уведомлением и не оставляет приложение без горячей клавиши.
- **Аккуратнее удаление слов-паразитов.** Если удалённое слово начинало предложение, следующее слово получает заглавную букву. Английское «Ha» больше не удаляется как слово-паразит.
- **Быстрее первый запуск после установки или обновления.** Поиск видеокарт перенесён в фон и не задерживает открытие приложения; заметнее всего на macOS.
- **История подтверждает копирование только после успешной записи в буфер обмена.**
- **Модель выгружается и после записи без звука**, если в настройках выбрано «Выгрузить модель → Немедленно».
- **Для китайского языка добавлен выбор упрощённого или традиционного письма.** Прежний выбор переносится автоматически; другие языки не меняются.
- **Движок transcribe.cpp обновлён до 0.2.4.** При переключении разделов настроек прокрутка начинается сверху.

Настройки, загруженные модели и история сохраняются. На Windows версия несколько дней проверялась в обычной работе. Сборки macOS и Linux доступны, но не проходили такую же пользовательскую проверку.

## Как обновиться

В установленной версии 0.9.5-3 или новее нажмите **«Проверить обновления» → «Доступно обновление»** внизу окна. Загрузка и установка начинаются после вашего нажатия.

**Portable-версию обновляйте вручную:** закройте приложение, замените файлы в той же папке и сохраните `Data/` с настройками, моделями и записями.

Новая установка — ниже, в разделе «Какой файл скачать».

---

# klava-nevinovata 0.9.8-1 — English

Based on Handy 0.9.8, with fixes for shortcuts, recording without a loaded model, and first launch. Preserves klava-nevinovata's design, custom prompt and vocabulary, Whisper/Breeze text processing, settings safeguards, and clipboard handling.

## What's changed

- **The cancel shortcut no longer stays captured.** After a very short recording, the cancel shortcut (Esc by default) could remain captured system-wide until the app quit.
- **Recording doesn't start without a model.** If the selected model hasn't been downloaded yet or its files were deleted, the shortcut no longer opens the microphone; the app reports the model loading error right away instead of recording audio it cannot transcribe.
- **Fixed shortcuts in the Tauri keyboard implementation.** Keys such as Scroll Lock can be assigned again. When switching implementations, a shortcut that Tauri cannot handle is reset to the default with a notice instead of leaving the app without a working shortcut.
- **More careful filler-word removal.** When a removed filler opened a sentence, the next word is capitalized. The English "Ha" is no longer removed as a filler.
- **Faster first launch after installing or updating.** Compute device discovery moved to the background and no longer delays the app; most noticeable on macOS.
- **History confirms copying only after the clipboard write succeeds.**
- **The model is also unloaded after recordings without audio** when Unload Model is set to Immediately.
- **Simplified or traditional script for Chinese transcriptions.** The previous choice migrates automatically; other languages are unaffected.
- **transcribe.cpp has been updated to 0.2.4.** Switching settings sections now starts at the top of the page.

Settings, downloaded models, and history are preserved. This version has been tested in daily use on Windows for several days. macOS and Linux builds have not received the same hands-on validation.

## Updating

In an installed copy of 0.9.5-3 or newer, click **Check for updates → Update available** at the bottom of the window. Downloading and installation start after your click.

**Update portable copies manually:** quit the app, replace the files in the same folder, and keep `Data/` with your settings, models, and recordings.

For a new installation, see "Which file to download" below.
