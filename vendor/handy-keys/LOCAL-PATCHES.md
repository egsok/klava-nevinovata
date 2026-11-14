# Локальная база handy-keys

Источник: https://crates.io/api/v1/crates/handy-keys/0.3.5/download

SHA256 архива (проверен перед распаковкой): `a38835faa2713931599234a357985a051474c556bc19b8ce124ecb00bee89154`.
Исходный commit из `.cargo_vcs_info.json`: `bff289bd47e85f58677b8b6aaa9fdcda9c818d5a`.

Перенесена только существующая дельта форка `egsok/handy-keys-fork@2a1ae0b6d214470f3d95c4e5d71ce38f30e518ec`:

- `src/lib.rs`, `src/platform/windows/mod.rs`: экспорт `modifier_resync_count()`; на остальных ОС возвращает 0.
- `src/platform/windows/listener.rs`: атомарный счётчик фактических коррекций stale/adopt и приоритет потока `THREAD_PRIORITY_TIME_CRITICAL`.
- `Cargo.toml`: Windows feature `Win32_System_Threading`, необходимый существующему приоритету.

Формула upstream 0.3.5 сохранена: `stale_modifiers(ctx.current_modifiers, physical | ctx.swallowed_modifiers)`. Обработка проглоченных press/repeat/release и сброс после смены сессии не заменялись старой реализацией.

Получение и чтение исходников выполнены без запуска кода. Cargo build/test не выполнялись. `Cargo.toml.orig` и `.cargo_vcs_info.json` сохраняют исходное происхождение архива; действующий манифест — `Cargo.toml`.
