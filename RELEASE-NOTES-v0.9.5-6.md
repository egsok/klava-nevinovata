# v0.9.5-6 — фиксы обновления по горячим следам: инсталлер и футер

> Черновик для описания GitHub-релиза. English version below.

## Главное

Маленький фикс-релиз по первым впечатлениям от автообновления на v0.9.5-5.

## Что исправлено

**Установщик больше не пугает ошибкой во время автообновления.** На Windows установщик мог стартовать раньше, чем система отпустит файл ещё закрывающегося приложения, — и показывал «Error opening file for writing» с кнопками Abort/Retry/Ignore (Retry помогал, но выглядело это страшно). Теперь установщик сам дожидается, пока файл освободится, — до 10 секунд, чего хватает с запасом.

**Футер перестал слипаться.** Статус проверки обновлений («Checking for updates…», прогресс загрузки) упирался вплотную в ссылку на канал, а при скачивании апдейта наезжал на неё. Теперь между блоками фиксированный зазор: длинный статус обрезается многоточием, ссылка на канал и номер версии всегда на месте.

Замечание: фикс установщика приезжает вместе с новым инсталлером, поэтому при обновлении _на эту версию_ диалог ошибки ещё может мелькнуть один последний раз — Retry решает. Все следующие обновления пройдут уже тихо.

---

# v0.9.5-6 — quick fixes after the first auto-update: installer and footer (English)

A small fix release based on first impressions of the v0.9.5-5 auto-update.

**The installer no longer scares users during auto-update:** on Windows it could start before the OS released the still-closing app's executable and popped "Error opening file for writing" with Abort/Retry/Ignore (Retry worked, but it looked alarming). The installer now waits for the file lock to clear — up to 10 seconds, plenty in practice.

**The footer no longer crowds itself:** the update status ("Checking for updates…", download progress) ran flush into the channel link and overlapped it during downloads. There is now a fixed gap — a long status truncates with an ellipsis while the channel link and version number stay put.

Note: the installer fix ships inside the new installer, so while updating _to_ this version the error dialog may still flash one last time — Retry resolves it. Every update after this one is covered.
