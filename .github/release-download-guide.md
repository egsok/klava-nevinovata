---

## 📦 Какой файл скачать

Файлы — ниже, в разделе **Assets**. Нужен только один:

| Ваш компьютер                      | Файл                                                                                                                                                                |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Windows**                        | [`klava-nevinovata_{{VERSION}}_x64-setup.exe`](https://github.com/egsok/klava-nevinovata/releases/download/v{{VERSION}}/klava-nevinovata_{{VERSION}}_x64-setup.exe) |
| **Mac с чипом Apple** (M1 и новее) | [`klava-nevinovata_{{VERSION}}_aarch64.dmg`](https://github.com/egsok/klava-nevinovata/releases/download/v{{VERSION}}/klava-nevinovata_{{VERSION}}_aarch64.dmg)     |
| **Mac с процессором Intel**        | [`klava-nevinovata_{{VERSION}}_x64.dmg`](https://github.com/egsok/klava-nevinovata/releases/download/v{{VERSION}}/klava-nevinovata_{{VERSION}}_x64.dmg)             |

Какой у вас Mac, видно в меню Apple → «Об этом Mac»: «Чип Apple M…» — первый вариант, «Процессор Intel» — второй.

Остальные файлы обычно не нужны: `.msi` — тот же установщик Windows в другом формате, `.deb`, `.rpm` и `.AppImage` — для Linux, а `.app.tar.gz`, `.sig` и `latest.json` приложение скачивает само при обновлении.

**Windows:** если при запуске установщика появится «Система Windows защитила ваш компьютер», нажмите «Подробнее» → «Выполнить в любом случае». Предупреждение появляется потому, что установщик не подписан платным сертификатом.

### ⚠️ Первый запуск на Mac

Сборки не подписаны сертификатом Apple Developer. После перетаскивания `klava-nevinovata.app` в «Программы» один раз снимите карантинный атрибут в Терминале, иначе macOS может ошибочно сообщить, что приложение повреждено:

```bash
sudo xattr -dr com.apple.quarantine /Applications/klava-nevinovata.app
```

Терминал попросит пароль от Mac; символы при вводе не отображаются. После обновления macOS может заново запросить разрешения на микрофон и Универсальный доступ — выдайте их ещё раз.

## 📦 Which file to download

The files are below, under **Assets**. You need just one:

| Your computer                       | File                                                                                                                                                                |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Windows**                         | [`klava-nevinovata_{{VERSION}}_x64-setup.exe`](https://github.com/egsok/klava-nevinovata/releases/download/v{{VERSION}}/klava-nevinovata_{{VERSION}}_x64-setup.exe) |
| **Apple Silicon Mac** (M1 or later) | [`klava-nevinovata_{{VERSION}}_aarch64.dmg`](https://github.com/egsok/klava-nevinovata/releases/download/v{{VERSION}}/klava-nevinovata_{{VERSION}}_aarch64.dmg)     |
| **Intel Mac**                       | [`klava-nevinovata_{{VERSION}}_x64.dmg`](https://github.com/egsok/klava-nevinovata/releases/download/v{{VERSION}}/klava-nevinovata_{{VERSION}}_x64.dmg)             |

To check which Mac you have, open the Apple menu → About This Mac: "Chip Apple M…" means Apple Silicon, "Processor Intel" means Intel.

You usually don't need the other files: `.msi` is the same Windows installer in another format, `.deb`, `.rpm` and `.AppImage` are for Linux, and the app downloads `.app.tar.gz`, `.sig` and `latest.json` itself when updating.

**Windows:** if the installer shows "Windows protected your PC", click **More info → Run anyway**. The warning appears because the installer isn't signed with a paid certificate.

### ⚠️ First launch on macOS

Builds are not signed with an Apple Developer ID. After dragging `klava-nevinovata.app` into Applications, clear the quarantine flag once in Terminal, otherwise macOS may incorrectly report that the app is damaged:

```bash
sudo xattr -dr com.apple.quarantine /Applications/klava-nevinovata.app
```

Terminal asks for your Mac password; nothing is shown while you type it. After an update, macOS may ask for Microphone and Accessibility permissions again — grant them once more.
