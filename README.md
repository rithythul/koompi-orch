# koompi-orch

Desktop application for orchestrating multiple AI coding agents in parallel. Run Claude Code, Codex, Gemini CLI, and Aider side-by-side across isolated git worktrees with cost tracking, pipeline execution, and a unified control interface.

Built with Tauri 2, React, and Rust.

## Features

- **Multi-agent orchestration** — Spawn and manage multiple AI coding agents concurrently
- **Workspace Kanban** — Organize work across backlog, active, review, and done columns
- **Agent templates** — Pre-configured definitions for Claude Code, Codex, Gemini CLI, Aider, plus custom agents
- **Pipeline builder** — Chain agents in sequence (architect → implementer → reviewer) with configurable handoffs
- **Real-time streaming** — Agent output streams to the UI via PTY with event routing
- **Cost & token tracking** — Per-session and aggregate metrics for budget awareness
- **Role presets** — Assign system prompts per agent role (implementer, reviewer, architect, tester)
- **Dark/light themes** — Design token system with full theme support
- **Embedded database** — SurrealDB (SurrealKV engine) for workspaces, sessions, metrics, and pipelines

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   React Frontend                │
│  Zustand stores ← Tauri Events ← Agent Events  │
│  Sidebar │ CenterPanel (Kanban) │ RightPanel    │
└────────────────────┬────────────────────────────┘
                     │ IPC (invoke / events)
┌────────────────────┴────────────────────────────┐
│                  Rust Backend                   │
│  Engine ─── ProcessSpawner ─── AgentProcess     │
│    │              │                │            │
│  Sessions    PTY (portable-pty)  Event Parser   │
│    │                                            │
│  SurrealDB (embedded)                           │
│  ├── workspace, repo, session, pipeline         │
│  ├── metric, checkpoint, agent_template         │
│  └── role_preset, belongs_to, runs_in           │
└─────────────────────────────────────────────────┘
```

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) 22+ or [Bun](https://bun.sh/)
- System dependencies for Tauri: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development

```bash
# Install frontend dependencies
bun install

# Run in development mode (starts both Vite dev server and Tauri)
bun tauri dev

# Run frontend tests
bun run test

# Run Rust tests
cargo test --manifest-path src-tauri/Cargo.toml

# Type check
npx tsc --noEmit

# Build for production
bun tauri build
```

## Project Structure

```
src/                          # React frontend
├── app/App.tsx               # Root component, theme, event hooks
├── components/
│   ├── layout/               # ThreePanel, Sidebar, CenterPanel, RightPanel
│   ├── dashboard/            # ProjectDashboard with stats and session list
│   ├── settings/             # SettingsPage, ApiKeyManager, AgentTemplates
│   ├── agent/                # PipelineBuilder
│   └── plugins/              # PluginList
├── stores/                   # Zustand state management
│   ├── agentStore.ts         # Sessions, messages, metrics
│   ├── workspaceStore.ts     # Workspaces, Kanban state
│   └── settingsStore.ts      # Theme, templates, API keys, preferences
└── hooks/
    ├── useAgentEvents.ts     # Tauri event → store routing
    └── useBootstrap.ts       # Initial data load from backend

src-tauri/                    # Rust backend
├── src/
│   ├── lib.rs                # Tauri app setup, state management
│   ├── agent/
│   │   ├── process.rs        # PTY-based agent process management
│   │   ├── config.rs         # AgentConfig, AgentEvent types
│   │   └── registry.rs       # Built-in + custom agent templates
│   ├── orchestrator/
│   │   └── engine.rs         # Session lifecycle, concurrency control
│   ├── ipc/
│   │   ├── commands.rs       # DB query IPC handlers
│   │   ├── agent_commands.rs # Agent lifecycle + settings IPC
│   │   └── spawner.rs        # ProcessSpawner → PTY bridge
│   ├── db/
│   │   ├── schema.rs         # SurrealDB record types
│   │   ├── queries.rs        # Typed query helpers
│   │   └── migrations/       # SurrealQL schema migrations
│   ├── config.rs             # AppConfig (TOML)
│   └── workspace/            # Git worktree + status management
```

## Supported Agents

| Agent | Command | Input Mode | Output Mode |
|-------|---------|------------|-------------|
| Claude Code | `claude` | flag_message | json_stream |
| Codex | `codex` | flag_message | text_markers |
| Gemini CLI | `gemini` | pty_stdin | raw_pty |
| Aider | `aider` | pty_stdin | text_markers |
| Custom | configurable | configurable | configurable |

## Roadmap

See [open issues](https://github.com/rithythul/koompi-orch/issues) for planned features:

- Drag-and-drop Kanban board
- Terminal emulation for agent output (xterm.js)
- Multi-agent pipeline execution
- Git repo management with worktree auto-creation
- Agent stdin forwarding (interactive mode)
- Cost governor with auto-throttling
- Session history and replay
- Command palette (Ctrl+K)
- Toast notifications with OS-level alerts
- Multi-workspace split view

## License

MIT
