# K1tty Tools

Нативное десктопное приложение на Rust + Tauri для просмотра, редактирования, генерации, очистки, оптимизации контекста и переноса истории диалогов в Google Antigravity, Antigravity CLI, OpenAI Codex, Claude Code и OpenCode.

A native desktop application built with Rust + Tauri for viewing, editing, generating, cleaning, optimizing context, and transferring conversation history across Google Antigravity, Antigravity CLI, OpenAI Codex, Claude Code, and OpenCode.

![Обзор: все среды, диалоги и бэкапы (тема Graphite) / Overview: all environments, dialogues, and backups (Graphite theme)](docs/screenshots/overview.png)

## Последнее обновление (v2.1.1) / Latest Update (v2.1.1)

**Русский:**
- **Хотфикс Codex (`v2.1.1`)**: исправлены префиксы ID для `custom_tool_call` (`ctc_`) и `custom_tool_call_output` (`ctco_`) при генерации и конвертации диалогов в Codex - устранена ошибка OpenAI API `invalid_id_prefix` при отправке новых сообщений в сгенерированный тред.
- **Новая функция - Генератор диалогов**: встроенное создание реалистичных диалогов с нуля через **OpenRouter** (включая бесплатные `:free` модели), **DeepSeek** и любой **Custom OpenAI-совместимый API** (Ollama, LM Studio и др.). Доступна настройка сценария, объёма, языка, уровня размышлений (`Effort`), температуры, привязки к папке проекта, системного промпта, вызовов инструментов (`tool_use` / `tool_result`) и блоков `thinking`. Перед сохранением диалог можно просмотреть, отредактировать и сохранить в любую среду, а все сгенерированные чаты автоматически сохраняются в раздел «Сохраненные». *Примечание: некоторые модели могут возвращать отказы или работать некорректно на чувствительных/триггерных темах (например, читы и подобные сценарии) - в таких случаях рекомендуем использовать DeepSeek либо вручную корректировать системный промпт в расширенных настройках.*
- **Производительность и большие диалоги**: ускорено сохранение и редактирование тяжёлых файлов и баз (потоковое копирование, частичная перезапись хвоста, кэширование заголовков и статистики контекста, быстрые префильтры поиска). Добавлены скользящее окно рендера и инкрементальная подгрузка для плавной работы на длинных чатах, кнопка быстрого перехода к последним сообщениям и автоматическая подгрузка истории при прокрутке вверх.
- **Поиск по тексту**: добавлен переключатель точного поиска по целому слову (`W`, чтобы различать, например, `"чит"` и `"читать"`) и живая подсветка найденных совпадений прямо в сообщениях.
- **Исправления и интерфейс**: исправлено редактирование уведомлений Claude (`Safety Classifier`, `Notice`, `isMeta`) - правка больше не превращает их в сообщения пользователя; прогресс-бар сохранения поднят поверх затемнения модальных окон; добавлены плавные переходы и верхняя полоса загрузки между диалогами; переведены системные предупреждения и ошибки бэкенда на `RU / UK / EN`.

**English:**
- **Codex Hotfix (`v2.1.1`)**: fixed ID prefixes for `custom_tool_call` (`ctc_`) and `custom_tool_call_output` (`ctco_`) when generating or converting dialogues into Codex, resolving the OpenAI API `invalid_id_prefix` error when sending follow-up messages.
- **New Feature - Dialogue Generator**: built-in generator for creating realistic multi-turn dialogues from scratch via **OpenRouter** (including `:free` models), **DeepSeek**, and any **Custom OpenAI-compatible API** (Ollama, LM Studio, etc.). Supports scenario, length, language, reasoning effort, temperature, optional project folder context, system prompt, synthetic tool calls (`tool_use` / `tool_result`), and `thinking` blocks. Preview and edit turns before saving directly into any supported environment, with automatic draft saving under "Saved". *Note: some models may refuse or behave inconsistently on sensitive or trigger topics (such as cheats or similar subjects) - for those scenarios, we recommend using DeepSeek or manually adjusting the system prompt in advanced settings.*
- **Performance & Large Dialogues**: significantly faster saving and editing on heavy files and databases (streaming backup copies, tail-only rewrites, cached titles and context stats, fast byte pre-filters). Added a sliding render window and incremental loading for smooth scrolling on massive chats, a quick jump-to-latest button, and automatic loading of older messages when scrolling up.
- **In-Chat Search**: added a Whole Word toggle (`W`, distinguishing e.g. `"read"` vs `"reading"`) and live match highlighting directly inside messages.
- **Fixes & UI Polish**: fixed editing Claude notices (`Safety Classifier`, `Notice`, `isMeta`) so they keep their proper notice role instead of turning into user messages; lifted the save progress bar above modal backdrops; added smooth thread-switching transitions and a subtle loading rail; localized backend warnings and error messages across `RU / UK / EN`.

## Возможности / Features

### Генератор диалогов / Dialogue Generator

Встроенный генератор позволяет создавать реалистичные многоходовые диалоги с нуля и сохранять их сразу в Antigravity, Codex, Claude или OpenCode. Поддерживаются **OpenRouter** (включая бесплатные `:free` модели), **DeepSeek** и любой OpenAI-совместимый **Custom API** (Ollama, LM Studio, vLLM и др.). Можно настроить тему, объём, язык, уровень размышлений (`Effort`), температуру, привязку к папке проекта, системный промпт, а также генерацию вызовов инструментов (`tool_use` / `tool_result`) и блоков `thinking`. Все генерации автоматически сохраняются в черновики («Сохраненные»), а перед записью диалог можно просмотреть и отредактировать по отдельным репликам. На чувствительных/триггерных темах (например, читы) часть моделей может уходить в отказ - в таких случаях лучше использовать **DeepSeek** или вручную подкорректировать системный промпт в расширенных настройках.

The built-in generator lets you compose realistic multi-turn dialogues from scratch and save them directly into Antigravity, Codex, Claude, or OpenCode. Supports **OpenRouter** (including `:free` models), **DeepSeek**, and any OpenAI-compatible **Custom API** (Ollama, LM Studio, vLLM, etc.). Configure topic, length, language, reasoning effort, temperature, optional project folder context, system prompt, plus synthetic tool calls (`tool_use` / `tool_result`) and `thinking` blocks. Every generated dialogue is automatically archived under "Saved", and individual messages can be reviewed or edited before saving. On sensitive or trigger topics (such as cheats), some models may refuse or produce incomplete output - in those cases, use **DeepSeek** or adjust the generator system prompt manually in advanced settings.

<p>
  <img src="docs/screenshots/generate.png" width="49%" alt="Генератор диалогов: настройка модели, контекста и сценария / Dialogue generator: model, context, and scenario setup">
  <img src="docs/screenshots/generate-preview.png" width="49%" alt="Генератор диалогов: предпросмотр реплик, инструментов и сохранение / Dialogue generator: turn preview, tool calls, and saving">
</p>

### Блокировки в Codex / Codex Locks

Когда Codex блокирует чат из-за фильтров безопасности, инструмент показывает причину и снимает блокировку в один клик. Тред можно и вручную заблокировать с любой указанной причиной для тестов.

When Codex blocks a chat due to safety filters, the tool shows the reason and unlocks it in one click. You can also manually lock a thread with any custom reason for testing purposes.

### Редактирование сообщений, промптов и инструментов / Editing Messages, Prompts, and Tool Calls

Можно менять системный промпт, внутренние инструкции и метаданные чата. Править текст сообщений пользователя и ответы модели. Править вызовы инструментов (`tool_calls`), их выводы, размышления (`thinking`) и ошибки во всех средах, не ломая структуру файлов.

You can edit the system prompt, internal instructions, and chat metadata. Modify user messages and model responses. Edit tool calls (`tool_calls`), tool outputs, thinking blocks, and errors across all environments without breaking the file structure.

<p>
  <img src="docs/screenshots/chat.png" width="49%" alt="Просмотр диалога: поиск с подсветкой, таблицы, формулы LaTeX, diff и файловые ссылки / Dialogue view: search highlights, tables, LaTeX formulas, diff, and file links">
  <img src="docs/screenshots/edit.png" width="49%" alt="Редактирование ответа модели / Editing a model reply">
</p>

### Оптимизация контекста и счётчик токенов / Context Optimizer & Token Counter

Показывает оценку контекста в токенах прямо в шапке диалога и позволяет в один клик облегчить тяжёлый чат: удалить блоки `thinking` / `reasoning` и сократить громоздкие выводы инструментов (с выбором порога 2 / 4 / 10 / 25 КБ), сохраняя целостность связей и нумерации. Перед очисткой показывается прогноз экономии и список самых тяжёлых шагов. Обе функции можно отключить в настройках.

Shows an estimated token count in the thread header and lets you slim down bloated chats in one click: strip `thinking` / `reasoning` blocks and trim oversized tool outputs (with configurable 2 / 4 / 10 / 25 KB thresholds) while keeping DAG links and step numbering intact. Displays a savings preview and the heaviest steps before applying. Both controls are optional in Settings.

![Оптимизация контекста / Context optimizer](docs/screenshots/slim.png)

### Удаление, обрезка, поиск и пакетные действия / Trimming, Full-Text Search, and Batch Actions

Удаление отдельных сообщений без сбоя нумерации шагов. Обрезка начала диалога до выбранного места или удаление всей переписки после нужного шага. Полнотекстовый поиск внутри чата (`Ctrl+F`) с переключателем точного слова (`W`) и живой подсветкой совпадений, поиск по содержимому всех чатов (`Ctrl+Shift+F`), автоматическая подгрузка старых сообщений при прокрутке вверх, палитра команд (`Ctrl+K`), закрепление диалогов и мультивыбор в сайдбаре для пакетного экспорта или удаления.

Delete individual messages without breaking step numbering. Trim the beginning of a conversation up to a selected point, or remove everything after a specific step. Full-text search inside a chat (`Ctrl+F`) with a Whole Word toggle (`W`) and live match highlighting, cross-chat search (`Ctrl+Shift+F`), automatic loading of older messages when scrolling up, command palette (`Ctrl+K`), pinned chats, and sidebar multi-select for batch export or deletion.

### Конвертация между средами и архивы `.k1tty` / Cross-Environment Conversion & `.k1tty` Archives

Перенос диалогов между любыми из пяти сред:

- Antigravity <-> Antigravity CLI, Codex, Claude, OpenCode
- Codex <-> Antigravity, Antigravity CLI, Claude, OpenCode
- Claude <-> Antigravity, Antigravity CLI, Codex, OpenCode
- OpenCode <-> Antigravity, Antigravity CLI, Codex, Claude

Поддерживаются сжатые переносимые архивы `.k1tty` (zstd / lzma2), сохранение границ компактизации (`compact_boundary`) в Claude и автоматическое определение активного аккаунта Claude Desktop.

Transfer conversations between any of the five environments with built-in `.k1tty` archive export/import (zstd / lzma2), preserved Claude compaction boundaries (`compact_boundary`), and active Claude Desktop account detection.

![Конвертация чата в другую среду / Converting a chat to another environment](docs/screenshots/convert.png)

### Рендер разметки, формул и локальных путей / Rich Markdown, LaTeX Math, and Local Paths

Сообщения, размышления и выводы инструментов поддерживают таблицы GFM, списки с чекбоксами, callout-блоки, подсветку `diff`, локальный офлайн-рендер математических формул (LaTeX через встроенный KaTeX) и кликабельные ссылки на локальные файлы с открытием в Проводнике / Finder. Работает выделение текста во всех блоках, `Ctrl+A`, контекстное меню копирования (исходный TeX формул, код, ячейки и таблицы в Markdown).

Messages, thinking blocks, and tool outputs render GFM tables, task lists, callouts, `diff` highlighting, offline LaTeX math (bundled KaTeX), and clickable local file links that reveal files directly in Explorer / Finder. Full text selection across all blocks, scoped `Ctrl+A`, and a right-click copy menu (raw TeX source, code blocks, table cells/Markdown).

### Темы и оформление / Themes & Appearance

12 встроенных тем (4 базовые, 4 с пиксельными котиками и 4 с пиксельными манулами), авто-адаптивный тон интерфейса от одного выбранного цвета и мягкое градиентное свечение.

12 built-in themes (4 classic, 4 pixel cat themes, and 4 pixel manul themes), auto-adaptive interface tint derived from a single chosen color, and subtle ambient gradient lighting.

<p>
  <img src="docs/screenshots/theme-cat.png" width="49%" alt="Тема с котиком (Matcha Cat) / Pixel Cat theme (Matcha Cat)">
  <img src="docs/screenshots/theme-manul.png" width="49%" alt="Тема с манулом (Steppe Manul) / Pixel Manul theme (Steppe Manul)">
</p>

![Настройки оформления и выбор тем / Appearance settings and theme picker](docs/screenshots/settings.png)

### Бэкапы / Backups

Автобэкапы отключаются тумблером. Показывает вес диалогов и бэкапов по каждому приложению. Любую сохранённую копию можно открыть в режиме просмотра (только чтение), восстановить или удалить.

Automatic backups can be toggled off. Shows the size of conversations and backups for each application. Any saved backup can be inspected in read-only mode, restored, or deleted.

![Бэкапы диалога / Dialogue backups](docs/screenshots/backups.png)

## Примеры использования / Usage Examples

**Подмена ответов модели / Spoofing Model Responses.** Можно отредактировать прошлый ответ ассистента в истории так, чтобы модель в следующем шаге опиралась на него как на собственный вывод. Это показывает, насколько модель доверяет своему прошлому ответу и как далеко может уйти от исходной линии рассуждения, если предыдущий шаг подменён. Полезно для оценки устойчивости к prompt injection через историю.

You can edit a previous assistant response in the history so that the model treats it as its own previous output in the next step. This demonstrates how much the model trusts its past responses and how far it can deviate from the original reasoning path when a previous step is spoofed. Useful for evaluating robustness against prompt injection via conversation history.

**Восстановление чата после срабатывания Safety Guard / Recovering a Chat After Safety Guard Trigger.** Когда чат получил Safe Guard и модель отказывается продолжать работу, причину можно найти по истории. Дальше два пути. Первый - отредактировать сообщение, которое триггернуло фильтр, или подменить ответ модели на нейтральный, чтобы следующий шаг опирался на него как на собственный вывод. Второй - откатиться до сообщения перед срабатыванием, удалив всё после него, и продолжить с чистой точки. Оба варианта возвращают чат в рабочее состояние, а исходную ветку можно сохранить в бэкапе и восстановить позже.

When a chat hits a Safety Guard and the model refuses to continue, you can find the cause in the history. There are two approaches. First: edit the message that triggered the filter, or replace the model's response with a neutral one so the next step treats it as its own previous output. Second: roll back to the message before the trigger by deleting everything after it, and continue from a clean point. Both methods restore the chat to a working state, while the original branch can be saved in a backup and restored later.

**Подмена системного промпта / Spoofing the System Prompt.** Смена системного промпта в существующем чате позволяет сравнить поведение модели на одном и том же наборе сообщений при разных инструкциях. Этот сценарий особенно удобен для внедрения джейлбрейка. Системный промпт задаёт рамку, в которой модель интерпретирует всю переписку, поэтому его правка меняет поведение сразу во всех следующих шагах, без необходимости редактировать отдельные сообщения. Можно подставить роль, стиль ответа или набор разрешений, которые через интерфейс не выставить, и посмотреть, как модель реагирует на тот же диалог в новой рамке. Полезно и для сравнения, как формулировка системного промпта влияет на отказы, стиль и выбор инструментов.

Changing the system prompt in an existing chat lets you compare the model's behavior on the same set of messages under different instructions. This scenario is especially useful for injecting a jailbreak. The system prompt defines the frame in which the model interprets the entire conversation, so editing it immediately changes behavior in all subsequent steps without needing to modify individual messages. You can inject a role, response style, or set of permissions that cannot be set through the UI, and observe how the model reacts to the same dialogue under the new frame. Also useful for comparing how system prompt wording affects refusals, style, and tool selection.

**Подмена контекста (context spoofing) / Context Spoofing.** Редактирование метаданных, ролей и tool calls даёт возможность собрать состояние чата, которое через интерфейс не создать. Это позволяет тестировать, как модель реагирует на нестандартные или противоречивые состояния контекста.

Editing metadata, roles, and tool calls lets you construct chat states that cannot be created through the UI. This enables testing how the model reacts to non-standard or contradictory context states.

**Контролируемое тестирование джейлбрейков / Controlled Jailbreak Testing.** В локальной среде можно воспроизводимо проверять, какие формулировки промптов и какая структура истории приводят к обходу ограничений, а какие нет. Результат воспроизводим, потому что состояние чата хранится в файле и его можно откатить.

In a local environment you can reproducibly test which prompt formulations and conversation structures lead to bypassing restrictions and which do not. Results are reproducible because the chat state is stored in a file and can be rolled back.

**Перенос сценариев между средами / Transferring Scenarios Between Environments.** Один и тот же диалог можно перенести в другую среду и посмотреть, как её модель ведёт себя на идентичном контексте. Это даёт сравнение моделей на одинаковых входных данных.

The same conversation can be transferred to another environment to see how its model behaves on an identical context. This enables direct model comparison on the same inputs.

## Установка / Installation

Готовые установщики для всех ОС собираются автоматически в [GitHub Releases](https://github.com/quesada-lab/k1tty-tools/releases).

Pre-built installers for all platforms are published automatically in [GitHub Releases](https://github.com/quesada-lab/k1tty-tools/releases).

### Windows

1. Скачайте `K1tty Tools_2.1.1_x64-setup.exe` (или `.msi`) со страницы [Releases](https://github.com/quesada-lab/k1tty-tools/releases).
2. Запустите установщик. Если появится окно Windows SmartScreen (сборка без платной цифровой подписи), нажмите **«Подробнее» -> «Выполнить в любом случае»**.

1. Download `K1tty Tools_2.1.1_x64-setup.exe` (or `.msi`) from [Releases](https://github.com/quesada-lab/k1tty-tools/releases).
2. Run the installer. If Windows SmartScreen appears (unsigned open-source build), click **More info -> Run anyway**.

### macOS (Apple Silicon & Intel)

1. Скачайте универсальный образ `K1tty Tools_2.1.1_universal.dmg` со страницы [Releases](https://github.com/quesada-lab/k1tty-tools/releases) и перетащите приложение в `Applications`.
2. Так как сборка не нотаризована в Apple, при первом запуске снимите флаг карантина:
   ```bash
   xattr -cr "/Applications/K1tty Tools.app"
   ```

1. Download the universal `K1tty Tools_2.1.1_universal.dmg` from [Releases](https://github.com/quesada-lab/k1tty-tools/releases) and drag the app into `Applications`.
2. Since the build is not notarized with Apple, clear the quarantine attribute before the first launch:
   ```bash
   xattr -cr "/Applications/K1tty Tools.app"
   ```

### Linux

Скачайте `.AppImage` или `.deb` со страницы [Releases](https://github.com/quesada-lab/k1tty-tools/releases):

Download `.AppImage` or `.deb` from [Releases](https://github.com/quesada-lab/k1tty-tools/releases):

```bash
chmod +x K1tty\ Tools_*.AppImage
./K1tty\ Tools_*.AppImage
```

или для Debian / Ubuntu / or for Debian / Ubuntu:

```bash
sudo apt install ./K1tty\ Tools_*.deb
```

## Сборка из исходников / Building From Source

Требования / Requirements: **Rust (stable)**, **Node.js 22+**, и системные зависимости Tauri v2 (на Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev librsvg2-dev patchelf`).

```bash
git clone https://github.com/quesada-lab/k1tty-tools.git
cd k1tty-tools
npm ci
npm run build
```

Для запуска в режиме разработки / To run in development mode:

```bash
npm run dev
```

На Windows также можно запустить [build.bat](build.bat).  
On Windows you can also run [build.bat](build.bat).

### Нестандартные пути к данным / Custom Data Paths

По умолчанию данные ищутся в домашней папке пользователя. Пути можно переопределить переменными окружения:  
By default data is looked up in the user's home directory. Override the paths with environment variables:

| Переменная / Variable | По умолчанию / Default |
|---|---|
| `K1TTY_DATA_DIR` | системная папка данных приложения / OS app data directory |
| `ANTIGRAVITY_APPDATA` | `~/.gemini/antigravity` |
| `ANTIGRAVITY_CLI_APPDATA` | `~/.gemini/antigravity-cli` |
| `CLAUDE_HOME` | `~/.claude` |
| `CODEX_HOME` | `~/.codex` |
| `OPENCODE_DATA_DIR` | `~/.local/share/opencode` |

### Где хранятся бэкапы / Where Backups Are Stored

| Среда / App | Папка / Folder | Переменная / Variable |
|---|---|---|
| Antigravity | `<K1TTY_DATA_DIR>/backups/antigravity/` | `ANTIGRAVITY_BACKUP_ROOT` |
| Antigravity CLI | `<K1TTY_DATA_DIR>/backups/antigravity_cli/` | `ANTIGRAVITY_CLI_BACKUP_ROOT` |
| OpenCode | `<K1TTY_DATA_DIR>/backups/opencode/` | `OPENCODE_BACKUP_ROOT` |
| Claude Code | `~/claude_backups/` (рядом с / next to `CLAUDE_HOME`) | - |
| Codex | `~/codex_backups/` (рядом с / next to `CODEX_HOME`) | - |

Список бэкапов, их размер, просмотр и восстановление доступны прямо в интерфейсе.  
The UI lists backups, shows their size, and lets you inspect or restore them directly.

## Структура проекта / Project Structure

```
k1tty-tools/
├── crates/core/          # ядро на Rust: провайдеры, конвертер, генератор диалогов, оптимизация контекста, монитор процессов
│                         # Rust core: providers, converter, dialogue generator, context optimizer, process monitor
├── src-tauri/            # нативная оболочка Tauri v2, команды IPC и конфигурация сборки
│                         # Tauri v2 native shell, IPC commands, and bundle config
├── static/               # фронтенд интерфейса (HTML/JS/CSS, темы, встроенные шрифты и KaTeX)
│                         # desktop UI frontend (HTML/JS/CSS, themes, bundled fonts, and KaTeX)
├── scripts/              # сборка UI в dist/ и автотесты интерфейса / UI build & automated tests
├── docs/screenshots/     # демо-скриншоты для README / demo README screenshots
└── .github/workflows/    # кроссплатформенная сборка и релизы (Windows, macOS, Linux)
                          # cross-platform CI builds and releases (Windows, macOS, Linux)
```

## Telegram

Новости, обновления и обратная связь: [@k1ttyfthoughts](https://t.me/k1ttyfthoughts) и [@nepofiksili](https://t.me/nepofiksili).  
Updates, releases, and feedback: [@k1ttyfthoughts](https://t.me/k1ttyfthoughts) and [@nepofiksili](https://t.me/nepofiksili).

## Лицензия / License

[MIT](LICENSE)
