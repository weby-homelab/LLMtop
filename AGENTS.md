# LLMtop

AI runner and agent monitor for your terminal. Like btop++, but for local LLMs, runners, and workspace sessions.

Supports Ollama, llama.cpp, vLLM, OpenCode, Odysseus, and OpenAI-compatible API servers.

## Language Policy

English is mandatory for all project-facing work and communication.

- Write all source code, comments, tests, fixtures, documentation, examples, configuration text, scripts, and user-facing strings in English.
- Use English for every GitHub artifact: issue titles and bodies, issue comments, pull request titles and descriptions, review comments, commit messages, branch names, release notes, changelogs, discussions, labels, milestones, and workflow or CI messages.
- Do not use non-English text in repository content or GitHub communication unless it is an exact external identifier, a required protocol value, or a direct quote needed for context.
- When quoting or preserving non-English input, add an English explanation and keep the non-English text as short as possible.
- If a contributor opens an issue, comment, or review in another language, respond in English and continue the thread in English.

## Architecture

```
src/
├── main.rs                 # Thin entry point calling llmtop::run()
├── lib.rs                  # TUI loop, CLI args routing, self-update and update logic
├── app.rs                  # Core TUI App state, tick orchestrator, token rate counters
├── config.rs               # TOML configuration loader/saver (~/.config/llmtop/config.toml)
├── theme.rs                # Theme configuration, color definitions, and layout styles
├── setup.rs                # Setup utilities and CLI flags processing (--setup)
├── host_info.rs            # Retrieves OS name, CPU cores, RAM, and system specs
├── locale.rs               # Multi-language translation tables (English & Chinese)
├── snapshot.rs             # JSON snapshot serialization logic (--json / --once)
├── demo.rs                 # Synthetic session generators for demo mode (--demo)
├── ui/                     # UI components divided into separate modules:
│   ├── mod.rs              # UI layout composer and drawing dispatch
│   ├── config.rs           # Config menu drawer
│   ├── context.rs          # Context window gauges and sparkline drawer
│   ├── quota.rs            # Quota panel (rate limit gauges/stubs)
│   ├── tokens.rs           # Token count breakdown panels
│   ├── projects.rs         # Active projects status drawer
│   ├── ports.rs            # Port allocations and orphan ports panel
│   ├── sessions.rs         # Active session table and detail panels
│   ├── mcp.rs              # MCP server connection status panel
│   ├── header.rs           # Top dashboard header
│   ├── footer.rs           # Interactive key-helper bottom footer
│   ├── help.rs             # In-app help overlays and keyboard shortcuts popup
│   └── view_menu.rs        # Panel visibility toggles dropdown menu
├── collector/              # Backends that collect LLM metrics and status:
│   ├── mod.rs              # Orchestrates the MultiCollector query pipeline
│   ├── process.rs          # Parses OS processes, children, git stats, and port mappings
│   ├── rate_limit.rs       # Handles rate limit parsing
│   ├── ollama.rs           # Polls Ollama's local endpoints (/api/ps)
│   ├── llama_cpp.rs        # Scrapes llama.cpp's slots (/slots) & health (/health)
│   ├── vllm.rs             # Reads vLLM's Prometheus /metrics endpoint
│   ├── opencode.rs         # Scrapes OpenCode SQLite DB (~/.local/share/opencode/opencode.db)
│   ├── odysseus.rs         # Queries Odysseus workspace SQLite DB (data/app.db)
│   ├── auto_discover.rs    # Auto-probes active local ports for OpenAI /v1/models compatible APIs
│   └── mcp.rs              # Scrapes running Model Context Protocol (MCP) server statuses
└── jump/                   # Adapters to focus terminal running the selected session
    ├── mod.rs              # Dispatches process focus/jump attempts
    ├── cmux.rs             # cmux terminal jumper
    ├── tmux.rs             # tmux terminal jumper
    └── iterm2.rs           # iTerm2 macOS AppleScript terminal jumper
```

## Layout

```
┌─ ¹context (token rate sparkline + per-session context bars) ─────────┐
│  ▁▃▅▇█▇▅▃▁▃▅▇██                       O1 llama3      ████████ 82%  │
│  token rate (200pt history)            V1 qwen2.5     █████████91%⚠ │
│                                        L1 slot-0      ███      22%  │
└──────────────────────────────────────────────────────────────────────┘
┌─ ²quota ─────┐┌─ ³tokens ───┐┌─ projects ───┐┌─ ⁴ports ──────────┐
│ OLLAMA       ││ Total  1.2M ││ LLMtop       ││ PORT  SESSION  CMD │
│ 5h ████ 35%  ││ Input  402k ││  main +3 ~18 ││ :11434 Ollama  ollm│
│   resets 2h  ││ Output  89k ││              ││ :8080 llama.c  llma│
│ 7d ██ 12%    ││ Cache  710k ││ prediction   ││                    │
│              ││ ▁▃▅▇█▇▅▃▁▃▅││  feat/x +1~2 ││ ORPHAN PORTS       │
│ LLAMA.CPP    ││ Turns: 48   ││              ││ :4000 old-prj node│
│ 5h █ 9%      ││ Avg: 25k/t  ││ api-server   ││                    │
│ 7d ██ 14%    ││             ││  main ✓clean ││                    │
└──────────────┘└─────────────┘└──────────────┘└────────────────────┘
┌─ ⁵sessions ─────────────────────────────────────────────────────────┐
│ ►*OL 7336 ollama  ● Work llama3 82% 1.2M  48  VRAM: 6.1 GB / 8 GB   │
│  >LC 8840 llama.c ◌ Wait mistrl 91% 340k  12  waiting                │
│ ─────────────────────────────────────────────────────────────────── │
│  SESSION ollama · localhost:11434                                   │
│  Running llama3:latest model.                                       │
│  └─ Active processing                                                │
│  CHILDREN: 7401 ollama-runner                                       │
│  MEM 6144MB | 8192 context window | loaded in memory                │
└──────────────────────────────────────────────────────────────────────┘
```

Panel rendering priority (top to bottom):
1. **Sessions** — always visible, gets priority allocation (min 5 rows, ideal = 2/session + 7)
2. **Mid-tier** (quota, tokens, projects, ports) — split equally, shown if space allows
3. **Context** — only renders when sessions have ideal height AND surplus >= 5 rows
4. **Header** (1 row) + **Footer** (1 row) — always present

Panel descriptions:
- **¹context**: Left = token rate braille sparkline (200-point history). Right = per-session context % bars with yellow/red warning.
- **²quota**: Local inference server rate limit gauges side-by-side (5h and 7d windows with reset countdown). This panel is disabled by default for purely local LLM runs.
- **³tokens**: Total token breakdown (in/out/cache) + per-turn sparkline for selected session.
- **projects** (always visible): Per-project git branch + added/modified file counts.
- **⁴ports**: Agent-spawned open ports + orphan ports (from dead sessions). Conflict detection.
- **⁵sessions**: Full-width panel below mid row. Session list table (top) + selected session detail (bottom), separated by divider.

## Data Sources

All data is gathered read-only from the local filesystem, active ports, `ps` process trees, and local HTTP sockets. No API keys or external authorizations are needed.

### 1. Ollama Collector
- **Endpoint**: Local HTTP GET on `127.0.0.1:11434/api/ps` (raw `TcpStream` client).
- **Process Matching**: Searches `ps` output for `ollama` binaries.
- **Information Extracted**: Loaded model name, quantization type, parameter size, active size in memory/VRAM, and expiry details. Maps CPU usage (>10%) to estimate activity status.

### 2. llama.cpp Collector
- **Endpoints**: Local HTTP GET on `127.0.0.1:8080/health` and `/slots`.
- **Process Matching**: Searches `ps` output for `llama-server` or `llama` binaries, matching active configuration arguments (`-m` / `--model`).
- **Information Extracted**: Active slots count, processing state (idle vs active decoding), prompt and generated token counts, memory consumption (RSS).

### 3. vLLM Collector
- **Endpoints**: Local HTTP GET on `127.0.0.1:8000/metrics` (Prometheus exposition) and `/v1/models`.
- **Process Matching**: Searches `ps` output for commands containing `vllm`.
- **Information Extracted**: KV cache utilization percentage (`vllm:kv_cache_usage_perc`), running request queues (`vllm:num_requests_running`), waiting queues (`vllm:num_requests_waiting`), and currently served model ids.

### 4. OpenCode Collector
- **Source**: SQLite database located at `~/.local/share/opencode/opencode.db` (safely read in read-only WAL mode via `sqlite3` CLI).
- **Process Matching**: Cross-references running `opencode` processes by directory matching.
- **Information Extracted**: Workspace paths, conversation messages, tool executions, token usage histories, and active project information.

### 5. Odysseus Collector
- **Source**: SQLite database located at `data/app.db` or typical Odysseus directories.
- **Process Matching**: Detects running `odysseus` or `uvicorn` processes hosting the workspace.
- **Information Extracted**: Chat sessions, model selections, thread titles, message tallies, and token usage estimates.

### 6. Auto-Discovery Collector
- **Discovery Strategy**: Inspects local listening TCP ports in the ranges `1024..=49151` from the shared system port map.
- **Probing**: Probes each port using an HTTP GET for `/v1/models`.
- **Server Identification**: Parses HTTP headers and body structures to automatically identify LM Studio, LiteLLM, Open WebUI, KoboldCpp, TabbyAPI, Jan, text-generation-webui, LocalAI, and others.

## Session Status Detection

```
● Working  = Process active + high CPU usage / engine reports active generation or decoding slots
◌ Waiting  = Process active but idle (waiting for requests)
✗ Error    = Process encountered an error / failed to serve requests
✓ Done     = Process terminated normally (no longer listening on its port)
```

**PID verification**: Checked by inspecting `/proc` or `ps` commands to verify the PID still belongs to the expected server binary.

## Context Window Calculation

- **Ollama**: Default context window (e.g. 8192) or specified by model defaults.
- **llama.cpp / vLLM / Odysseus**: Derived from metrics or hardcoded defaults (e.g., 32,768 for vLLM, 128,000 for Odysseus).
- **Warning levels**: Threshold alerts highlight yellow at 80% and red with a warning icon (⚠) at 90%+ context consumption.

## Key Bindings

| Key | Action |
|-----|--------|
| `↑`/`↓` or `k`/`j` | Select session in list |
| `Enter` | Jump to session terminal (cmux / tmux / iTerm2) |
| `x` | Kill selected session process (SIGKILL) |
| `X` | Force close all detected orphan ports |
| `t` | Cycle through visual TUI themes |
| `1`–`5` | Toggle individual panels on/off |
| `Esc` | Open/close local configuration menu |
| `r` | Force refresh stats immediately |
| `q` | Quit LLMtop |

## Tech Stack

- **Rust** (2021 edition, MSRV 1.88+)
- **ratatui** + **crossterm** for TUI drawing and inputs
- **serde** + **serde_json** for configuration parsing
- **chrono** for timestamps
- **dirs** for configuration folder resolution

## Commands

```bash
llmtop                         # Launch TUI
llmtop --once                  # Print snapshot and exit
llmtop --json                  # Print one JSON snapshot and exit (for scripts)
llmtop --theme dracula         # Launch with a specific theme
llmtop --demo                  # Start TUI in demo mode with synthetic sessions
llmtop --setup                 # Setup CLI environment configuration
```

## Release Process

1. Update package version in `Cargo.toml` and run `cargo build` to update `Cargo.lock`.
2. Run standard local validations:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo build --release
   ```
3. Commit and push/PR your bump to `main`:
   ```bash
   git add Cargo.toml Cargo.lock
   git commit -S -m "chore: bump version to X.Y.Z"
   git push origin main
   ```
4. Push a signed tag:
   ```bash
   git tag -a vX.Y.Z -S -m "vX.Y.Z"
   git push origin vX.Y.Z
   ```
5. GitHub Actions release workflows will build cross-platform binaries via `cargo-dist` and publish to crates.io automatically.
