# klava-nevinovata 0.9.7-1

Обновление на базе Handy 0.9.7: новые режимы горячей клавиши, исправления записи и автозапуска Windows. Сохранены оформление klava-nevinovata, пользовательский промпт, обработка текста Whisper/Breeze, защита настроек и работа с буфером обмена.

## Что изменилось

- **Три режима горячей клавиши.** В «Общие → Поведение сочетания клавиш» можно выбрать удержание, переключение или автоматический режим: удерживать для записи либо нажимать для старта и остановки. При обновлении сохраняется прежний режим; автоматический включается по умолчанию только в новых установках.
- **Надёжнее быстрые нажатия.** Исправлена обработка коротких и последовательных нажатий, в том числе пока микрофон ещё запускается.
- **Сохраняется конец записи.** Обновлена обработка последних аудиоданных при остановке, чтобы не терять окончания слов.
- **Исправлен автозапуск Windows при пробелах в пути.** Путь к приложению теперь записывается в кавычках — в том числе для учётных записей вроде `Имя Фамилия`.
- **Исправлено масштабирование индикатора записи Windows.** Текст и элементы управления подстраиваются под размер оверлея.
- **Добавлен экспериментальный детектор речи Earshot.** Он доступен в экспериментальных настройках; Silero остаётся вариантом по умолчанию.
- **Добавлены каталонский и индонезийский языки интерфейса.** Часть дополнительных строк форка пока отображается на английском.

Настройки, загруженные модели и история сохраняются. На Windows версия прошла локальное тестирование в обычной работе. Сборки macOS и Linux доступны, но не проходили такую же пользовательскую проверку.

## Как обновиться

В установленной версии 0.9.5-3 или новее нажмите **«Проверить обновления» → «Доступно обновление»** внизу окна. Загрузка и установка начинаются после вашего нажатия.

**Portable-версию обновляйте вручную:** закройте приложение, замените файлы в той же папке и сохраните `Data/` с настройками, моделями и записями.

Установщики не подписаны сертификатом Windows/macOS; инструкции первого запуска — в [README](https://github.com/egsok/klava-nevinovata#download).

---

# klava-nevinovata 0.9.7-1 — English

Based on Handy 0.9.7, with new shortcut modes and recording and Windows autostart fixes. Preserves klava-nevinovata's design, custom transcription prompt, Whisper/Breeze text processing, settings safeguards, and clipboard handling.

## What's changed

- **Three shortcut modes.** Choose Hold, Toggle, or Auto in General → Shortcut Behavior. Auto supports both hold-to-record and tap-to-toggle. Updating preserves your existing mode; Auto is the default only for new installations.
- **More reliable quick presses.** Fixed short and successive key presses, including presses received while the microphone is starting.
- **Retained recording endings.** Updated stop-time audio handling to avoid losing the final sounds of a recording.
- **Fixed Windows autostart for paths containing spaces.** The executable path is now quoted in the startup entry.
- **Fixed Windows recording-overlay scaling.** Text and controls follow the overlay's size.
- **Experimental Earshot voice detector.** Available in Experimental settings; Silero remains the default.
- **Catalan and Indonesian interface translations.** Some additional fork strings still use English.

Settings, downloaded models, and history are preserved. This version has been tested in daily use on Windows. macOS and Linux builds have not received the same hands-on validation.

## Updating

In an installed copy of 0.9.5-3 or newer, click **Check for updates → Update available** at the bottom of the window. Downloading and installation start after your click.

**Update portable copies manually:** quit the app, replace the files in the same folder, and keep `Data/` with your settings, models, and recordings.

Installers are not Windows/macOS code-signed. See the [README](https://github.com/egsok/klava-nevinovata#download) for first-launch instructions.
