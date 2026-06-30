# LLMtop

**Like [btop](https://github.com/aristocratos/btop), but for your local LLMs, runners, and AI coding agents.**

See every active local LLM session, model usage, context window, rate limits, child processes, open ports, and more at a glance. Supports Ollama, llama.cpp, vLLM, OpenCode, Odysseus, and any OpenAI-compatible server (LM Studio, LiteLLM, Open WebUI, KoboldCpp, TabbyAPI, etc.).

LLMtop auto-discovers active agents and servers from local process/file state and active network ports across macOS, Linux, and Windows.

```mermaid
graph TD
    A["llmtop TUI"] --> B["MultiCollector"]
    B --> C["OllamaCollector<br/>HTTP /api/ps (11434)"]
    B --> D["LlamaCppCollector<br/>HTTP /health (8080)"]
    B --> E["VllmCollector<br/>Prometheus /metrics (8000)"]
    B --> F["OpenCodeCollector<br/>SQLite opencode.db"]
    B --> G["OdysseusCollector<br/>SQLite app.db"]
    B --> H["AutoDiscoverCollector<br/>Port scan → /v1/models"]

    H --> I["LM Studio"]
    H --> J["LiteLLM"]
    H --> K["Open WebUI"]
    H --> L["LocalAI"]
    H --> M["KoboldCpp"]
    H --> N["TabbyAPI"]
    H --> O["Hermes Agent / API"]
    H --> P["Jan / text-gen-webui / others"]

    style A fill:#1a1a2e,color:#fff,stroke:#3a3a5e,stroke-width:2px
    style H fill:#2d4a22,color:#fff,stroke:#5a8a4e,stroke-width:2px
    style G fill:#4a2d22,color:#fff,stroke:#8a5e4e,stroke-width:2px
```

## Features

- **Multi-Inference Engine Monitoring**: Real-time stats from Ollama, llama.cpp, vLLM, and SQLite DB agent sessions.
- **Universal OpenAI Auto-Discovery**: Automatically scans local listening ports for OpenAI-compatible `/v1/models` endpoints to track server status.
- **Odysseus Workspace Support**: Deep integration with Odysseus AI workspace DB logs.
- **Orphan Port Detection**: Detects and highlights ports left behind by dead agent processes.
- **Context Window Gauges**: Displays context usage bars with alerts when models get full.
- **Fully Local & Read-Only**: No external API keys or authorization required.

## Installation

### From Cargo

```bash
cargo install --path .
```
*(Or `cargo install llmtop` once published)*

### Manual Build

Ensure you have Rust (v1.88+) installed:

```bash
git clone https://github.com/weby-homelab/LLMtop.git
cd LLMtop
cargo build --release
cp target/release/llmtop /usr/local/bin/
```

## Usage

```bash
llmtop                    # Launch TUI
llmtop --once             # Print snapshot and exit
llmtop --json             # Print one JSON snapshot and exit (for scripts)
llmtop --setup            # Configure setup hooks
llmtop --theme dracula    # Launch with a specific theme
```

## Key Bindings

| Key                | Action                               |
| ------------------ | ------------------------------------ |
| `↑`/`↓` or `k`/`j` | Select session                       |
| `Enter`            | Jump to session terminal             |
| `x`                | Kill selected session                |
| `X`                | Kill all orphan ports                |
| `t`                | Cycle theme                          |
| `1`–`5`            | Toggle panel visibility              |
| `Esc`              | Open/close config page               |
| `q`                | Quit                                 |
| `r`                | Force refresh                        |

## Privacy

LLMtop reads local files and local process/open-file metadata only. No API keys, no auth. In the TUI and `--once` output, tool names and file paths are shown, but file contents and prompt text are never displayed.

The JSON snapshot includes richer local dashboard data, including `summary`, `chat_messages`, working directories, config roots, tool-call previews, child process commands, token counts, and port metadata. Treat JSON snapshots as local/private data and avoid writing them to shared logs or exposing them on a network without your own access controls.

## License

GPL-3.0
