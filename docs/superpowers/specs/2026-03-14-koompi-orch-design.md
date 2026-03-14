# koompi-orch Design Specification

**Date:** 2026-03-14
**Status:** Approved
**Author:** System Architect

---

## 1. Overview

**koompi-orch** is a cross-platform desktop application for orchestrating multiple AI coding agents in parallel. Built with Tauri 2 (Rust backend + React frontend), it manages isolated git worktrees, pipelines, and review workflows for any CLI-based AI agent.

**Tagline:** "Conduct a team of AI agents from one desktop."

**Differentiators vs Conductor:**
- Cross-platform (Linux, macOS, Windows) — not macOS only
- Any CLI agent (Claude Code, Codex, Gemini CLI, aider, custom scripts)
- Pipeline orchestration (chained multi-agent workflows)
- Smart model routing per task type
- WASM plugin system
- Graph-native data (SurrealDB)
- Conflict detection across parallel agents
- Auto-review chains
- Open source

---

## 2. Technology Stack

| Component | Choice | Rationale |
|---|---|---|
| Desktop shell | Tauri 2 | Rust backend, web frontend, cross-platform, small binary |
| Frontend | React + Vite + TypeScript | Ecosystem for Monaco, xterm.js, diff rendering |
| Styling | Tailwind CSS | Fast iteration, dark/light theming, consistent |
| State management | Zustand | Lightweight, no boilerplate, good DX |
| Terminal emulator | xterm.js | Industry standard embedded terminal |
| Code editor | Monaco Editor | VSCode engine, syntax highlighting, built-in diff |
| Diff viewer | Monaco diff editor | No extra dependency, high quality |
| Database | SurrealDB embedded (surrealkv) | Graph relations, LIVE queries, no separate process |
| Git operations | git2-rs (libgit2) | Direct bindings for diff, merge, commit, worktree |
| GitHub/GitLab API | octocrab + gitlab crate | PR creation, CI status, issue linking |
| PTY management | portable-pty | Cross-platform pseudo-terminal for agent processes |
| Plugin host | wasmtime | Sandboxed WASM plugin execution |
| Notifications | tauri-plugin-notification | Native OS desktop notifications |
| Key storage | tauri-plugin-stronghold | OS-native encrypted secret storage |
| Keyboard shortcuts | Custom registry | Command palette, platform-adaptive bindings |

---

## 3. Architecture

```
+------------------------------------------------------+
|                   koompi-orch                          |
|                                                      |
|  +--------------------------------------------------+|
|  |          React Frontend (Vite + TS)               ||
|  |  +----------+ +----------+ +------------------+  ||
|  |  | Workspace | |  Agent   | |  Diff/Review     |  ||
|  |  | Sidebar   | |  Chat +  | |  Panel +         |  ||
|  |  | + Kanban  | | Terminal | |  Git Actions     |  ||
|  |  | + Files   | | + Editor | |  + PR/CI         |  ||
|  |  +----------+ +----------+ +------------------+  ||
|  +-------------------------+------------------------+|
|                            | Tauri IPC               |
|  +-------------------------+------------------------+|
|  |          Rust Backend (Tauri Core)                ||
|  |                                                  ||
|  |  +--------------+  +---------------+             ||
|  |  | Orchestrator |  | Worktree      |             ||
|  |  | Engine       |  | Manager       |             ||
|  |  | - spawn      |  | - create      |             ||
|  |  | - monitor    |  | - cleanup     |             ||
|  |  | - pipeline   |  | - snapshot    |             ||
|  |  | - routing    |  | - conflict    |             ||
|  |  +--------------+  +---------------+             ||
|  |                                                  ||
|  |  +--------------+  +---------------+             ||
|  |  | Agent        |  | State Store   |             ||
|  |  | Registry     |  | (SurrealDB)   |             ||
|  |  | - templates  |  | - sessions    |             ||
|  |  | - I/O proto  |  | - metrics     |             ||
|  |  | - presets    |  | - graph rels  |             ||
|  |  +--------------+  +---------------+             ||
|  |                                                  ||
|  |  +--------------+  +---------------+             ||
|  |  | Git Ops      |  | Plugin Host   |             ||
|  |  | (git2-rs)    |  | (wasmtime)    |             ||
|  |  | - diff/merge |  | - agent types |             ||
|  |  | Remote API   |  | - pipeline    |             ||
|  |  | (octocrab)   |  |   steps       |             ||
|  |  | - PR/CI      |  | - msg passing |             ||
|  |  +--------------+  +---------------+             ||
|  +--------------------------------------------------+|
+------------------------------------------------------+
```

---

## 4. Rust Backend Module Structure

```
src-tauri/src/
├── main.rs                         # Tauri app entry
├── lib.rs                          # Module exports
│
├── orchestrator/                   # Core engine
│   ├── mod.rs
│   ├── engine.rs                   # Spawn, monitor, kill agents
│   ├── pipeline.rs                 # Chain agents (plan->impl->review->test->ship)
│   ├── router.rs                   # Smart agent routing (see Section 15)
│   ├── governor.rs                 # Resource limits, concurrency control
│   └── recovery.rs                 # Crash recovery, session restore
│
├── agent/                          # Agent abstraction
│   ├── mod.rs
│   ├── process.rs                  # PTY-based child process management
│   ├── registry.rs                 # Agent templates (claude, codex, gemini, custom)
│   ├── parser.rs                   # Agent output parsing (see Section 16)
│   ├── input.rs                    # Agent input injection (see Section 17)
│   ├── presets.rs                  # Role presets (architect, reviewer, QA, shipper)
│   └── config.rs                   # Per-agent configuration
│
├── workspace/                      # Git worktree lifecycle
│   ├── mod.rs
│   ├── manager.rs                  # Create, list, cleanup worktrees
│   ├── snapshot.rs                 # Checkpoint/revert to any agent turn
│   ├── conflict.rs                 # Cross-workspace file conflict detection
│   ├── lock.rs                     # Workspace mutual exclusion
│   └── status.rs                   # Kanban state machine
│
├── git/                            # Git operations
│   ├── mod.rs
│   ├── diff.rs                     # Diff generation (git2-rs)
│   ├── merge.rs                    # Merge/rebase (git2-rs)
│   ├── commit.rs                   # Commit, push (git2-rs)
│   └── remote.rs                   # PR/CI via octocrab (GitHub) / gitlab crate
│
├── db/                             # SurrealDB embedded
│   ├── mod.rs
│   ├── schema.rs                   # Tables, relations, indexes
│   ├── queries.rs                  # Typed query helpers
│   ├── live.rs                     # LIVE query subscriptions -> Tauri events
│   └── migrate.rs                  # Schema migrations
│
├── plugin/                         # Plugin system (see Section 18)
│   ├── mod.rs
│   ├── host.rs                     # WASM runtime (wasmtime)
│   ├── manifest.rs                 # Plugin manifest parsing
│   └── api.rs                      # Plugin host functions exposed to WASM
│
├── ipc/                            # Tauri commands and events
│   ├── mod.rs
│   ├── commands.rs                 # #[tauri::command] handlers
│   └── events.rs                   # Event emitters to frontend
│
├── notify/                         # Desktop notifications
│   ├── mod.rs
│   └── notifications.rs           # Agent done, needs input, failed, CI
│
└── config/                         # App configuration
    ├── mod.rs
    └── settings.rs                 # Global + project-level config, key management
```

---

## 5. React Frontend Structure

```
src/
├── app/
│   ├── App.tsx                     # Root layout, panel manager
│   ├── main.tsx                    # Entry point
│   └── router.tsx                  # Route definitions
│
├── components/
│   ├── layout/
│   │   ├── ThreePanel.tsx          # Resizable three-panel shell
│   │   ├── Sidebar.tsx             # Left: repos + workspace list + file tree
│   │   ├── CenterPanel.tsx         # Chat + terminal + editor
│   │   ├── RightPanel.tsx          # Diff viewer + git actions
│   │   └── CommandPalette.tsx      # Mod+K global search/actions
│   │
│   ├── workspace/
│   │   ├── WorkspaceCard.tsx       # Workspace in sidebar
│   │   ├── KanbanBoard.tsx         # Drag-drop status columns
│   │   ├── WorkspaceCreate.tsx     # New workspace dialog
│   │   ├── MultiRepoSelector.tsx   # Pick repo for new workspace
│   │   └── FileTree.tsx            # File explorer per workspace
│   │
│   ├── agent/
│   │   ├── ChatView.tsx            # Agent conversation view
│   │   ├── ChatInput.tsx           # Message input with model selector
│   │   ├── AgentPicker.tsx         # Choose agent type + role preset
│   │   ├── PipelineBuilder.tsx     # Visual pipeline builder
│   │   ├── CostTracker.tsx         # Token/cost display per session
│   │   └── AgentStatus.tsx         # Running/paused/done indicator
│   │
│   ├── terminal/
│   │   └── Terminal.tsx            # xterm.js embedded terminal
│   │
│   ├── editor/
│   │   └── CodeEditor.tsx          # Monaco editor
│   │
│   ├── diff/
│   │   ├── DiffViewer.tsx          # Side-by-side or unified diff
│   │   ├── DiffComment.tsx         # Inline comment on diff lines
│   │   ├── TurnDiff.tsx            # Per-agent-turn checkpoint diffs
│   │   └── MergeActions.tsx        # Commit, push, merge, PR buttons
│   │
│   ├── dashboard/
│   │   ├── ProjectDashboard.tsx    # Overview: agents, costs, status
│   │   ├── MetricsChart.tsx        # Token usage, cost over time
│   │   └── GlobalSearch.tsx        # Search across all workspaces
│   │
│   └── settings/
│       ├── SettingsPage.tsx        # App settings
│       ├── ApiKeyManager.tsx       # BYOK key management via Stronghold
│       ├── AgentTemplates.tsx      # Manage agent templates
│       └── ThemeToggle.tsx         # Dark/light theme
│
├── hooks/
│   ├── useTauriCommand.ts          # Typed Tauri invoke wrapper
│   ├── useTauriEvent.ts            # Tauri event listener hook
│   ├── useLiveQuery.ts             # SurrealDB LIVE query -> React state
│   ├── useWorkspace.ts             # Workspace CRUD + status
│   ├── useAgent.ts                 # Agent lifecycle
│   └── useKeyboard.ts             # Global keyboard shortcuts
│
├── stores/
│   ├── workspaceStore.ts           # Zustand: derived workspace state
│   ├── agentStore.ts               # Zustand: running agent UI state
│   ├── settingsStore.ts            # Zustand: app config cache
│   └── notificationStore.ts       # Zustand: notification queue
│
├── lib/
│   ├── ipc.ts                      # Tauri command type definitions
│   ├── keybindings.ts              # Keyboard shortcut registry
│   └── theme.ts                    # Theme tokens
│
└── styles/
    └── globals.css                 # Tailwind CSS base
```

**State management boundary:** SurrealDB is the source of truth for all domain data (workspaces, sessions, metrics, pipelines, templates). Zustand holds derived/ephemeral UI state only (which panel is focused, notification queue, local UI toggles). Frontend hydrates from SurrealDB on load and stays in sync via LIVE query subscriptions piped through Tauri events.

---

## 6. SurrealDB Schema

```surql
-- Repos
DEFINE TABLE repo SCHEMAFULL;
DEFINE FIELD path ON repo TYPE string;
DEFINE FIELD name ON repo TYPE string;
DEFINE FIELD remote_url ON repo TYPE option<string>;
DEFINE FIELD added_at ON repo TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_repo_path ON repo FIELDS path UNIQUE;

-- Workspaces (one per agent worktree)
DEFINE TABLE workspace SCHEMAFULL;
DEFINE FIELD name ON workspace TYPE string;
DEFINE FIELD branch ON workspace TYPE string;
DEFINE FIELD worktree_path ON workspace TYPE string;
DEFINE FIELD status ON workspace TYPE string
    ASSERT $value IN ['backlog','active','review','done','failed'];
DEFINE FIELD locked_by ON workspace TYPE option<record<session>>;
DEFINE FIELD created_at ON workspace TYPE datetime DEFAULT time::now();
DEFINE FIELD updated_at ON workspace TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_workspace_status ON workspace FIELDS status;

-- Relation: workspace belongs to repo
DEFINE TABLE belongs_to SCHEMAFULL TYPE RELATION IN workspace OUT repo;

-- Agent sessions
DEFINE TABLE session SCHEMAFULL;
DEFINE FIELD agent_type ON session TYPE string;
DEFINE FIELD model ON session TYPE option<string>;
DEFINE FIELD pid ON session TYPE option<int>;
DEFINE FIELD role_preset ON session TYPE option<string>;
DEFINE FIELD status ON session TYPE string
    ASSERT $value IN ['running','paused','completed','crashed'];
DEFINE FIELD started_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD ended_at ON session TYPE option<datetime>;
-- Log path derived from session ID: logs/session-{id}.jsonl (no field needed)
-- Agent-specific config (intentionally flexible per agent type)
-- Expected shapes documented in Section 10 per agent template
DEFINE FIELD config ON session TYPE object;
DEFINE INDEX idx_session_status ON session FIELDS status;

-- Relation: session runs in workspace
DEFINE TABLE runs_in SCHEMAFULL TYPE RELATION IN session OUT workspace;

-- Pipeline definitions
DEFINE TABLE pipeline SCHEMAFULL;
DEFINE FIELD name ON pipeline TYPE string;
DEFINE FIELD steps ON pipeline TYPE array<object>;
DEFINE FIELD created_at ON pipeline TYPE datetime DEFAULT time::now();

-- Pipeline execution instances
DEFINE TABLE pipeline_run SCHEMAFULL;
DEFINE FIELD current_step ON pipeline_run TYPE int DEFAULT 0;
DEFINE FIELD status ON pipeline_run TYPE string
    ASSERT $value IN ['running','paused','completed','failed'];
DEFINE FIELD started_at ON pipeline_run TYPE datetime DEFAULT time::now();
DEFINE FIELD ended_at ON pipeline_run TYPE option<datetime>;
DEFINE TABLE instance_of SCHEMAFULL TYPE RELATION IN pipeline_run OUT pipeline;
DEFINE TABLE executes_in SCHEMAFULL TYPE RELATION IN pipeline_run OUT workspace;

-- Relation: session hands off to session
DEFINE TABLE hands_off_to SCHEMAFULL TYPE RELATION IN session OUT session;
DEFINE FIELD handoff_type ON hands_off_to TYPE string
    ASSERT $value IN ['summary','full_log','diff_only'];
DEFINE FIELD output_summary ON hands_off_to TYPE option<string>;
DEFINE FIELD context_file ON hands_off_to TYPE option<string>;
DEFINE FIELD handoff_at ON hands_off_to TYPE datetime DEFAULT time::now();

-- Checkpoints (workspace snapshots)
DEFINE TABLE checkpoint SCHEMAFULL;
DEFINE FIELD commit_sha ON checkpoint TYPE string;
DEFINE FIELD turn_number ON checkpoint TYPE int;
DEFINE FIELD description ON checkpoint TYPE option<string>;
DEFINE FIELD created_at ON checkpoint TYPE datetime DEFAULT time::now();
DEFINE TABLE checkpoint_of SCHEMAFULL TYPE RELATION IN checkpoint OUT workspace;

-- Metrics (append-only, one row per agent API call)
DEFINE TABLE metric SCHEMAFULL;
DEFINE FIELD tokens_in ON metric TYPE int DEFAULT 0;
DEFINE FIELD tokens_out ON metric TYPE int DEFAULT 0;
DEFINE FIELD cost_usd ON metric TYPE float DEFAULT 0.0;
DEFINE FIELD duration_ms ON metric TYPE int DEFAULT 0;
DEFINE FIELD turn_number ON metric TYPE int DEFAULT 0;
DEFINE FIELD recorded_at ON metric TYPE datetime DEFAULT time::now();
DEFINE TABLE metric_for SCHEMAFULL TYPE RELATION IN metric OUT session;
-- Aggregation: SELECT math::sum(tokens_in), math::sum(cost_usd) FROM metric WHERE ->metric_for->session = $session_id

-- Agent templates
DEFINE TABLE agent_template SCHEMAFULL;
DEFINE FIELD name ON agent_template TYPE string;
DEFINE FIELD command ON agent_template TYPE string;
DEFINE FIELD default_args ON agent_template TYPE array<string>;
DEFINE FIELD env ON agent_template TYPE option<object>;
DEFINE FIELD input_mode ON agent_template TYPE string
    ASSERT $value IN ['pty_stdin','flag_message','file_prompt'];
DEFINE FIELD output_mode ON agent_template TYPE string
    ASSERT $value IN ['json_stream','text_markers','raw_pty'];
DEFINE FIELD resume_support ON agent_template TYPE bool DEFAULT false;
DEFINE FIELD builtin ON agent_template TYPE bool DEFAULT false;
DEFINE INDEX idx_template_name ON agent_template FIELDS name UNIQUE;

-- Role presets
DEFINE TABLE role_preset SCHEMAFULL;
DEFINE FIELD name ON role_preset TYPE string;
DEFINE FIELD system_prompt ON role_preset TYPE string;
DEFINE FIELD description ON role_preset TYPE string;
DEFINE FIELD injection_method ON role_preset TYPE string
    ASSERT $value IN ['flag','env_var','config_file','first_message'];
DEFINE FIELD builtin ON role_preset TYPE bool DEFAULT false;
DEFINE INDEX idx_preset_name ON role_preset FIELDS name UNIQUE;
```

---

## 7. Key Interactions

### 7.1 Spawning an Agent

1. User clicks "New Workspace" in sidebar
2. Picks repo, names branch, selects agent template + role preset
3. Frontend invokes Tauri command: `create_workspace(repo, branch, agent, role)`
4. Rust backend:
   - `git2-rs` creates worktree at `~/.koompi-orch/worktrees/{repo}/{branch}-{workspace_id}`
   - Unique suffix prevents path collisions when same repo+branch is used multiple times
   - SurrealDB: INSERT workspace, session, belongs_to, runs_in relations
   - Sets `workspace.locked_by = session_id` (mutual exclusion)
   - Spawns agent CLI process with PTY (see Section 17 for input injection)
   - Streams PTY output via Tauri events to frontend ChatView (see Section 16 for parsing)
   - Emits `workspace_created` event, sidebar updates via LIVE query

### 7.2 Pipeline Execution

1. User creates pipeline: [architect, implementer, reviewer, tester]
2. Creates `pipeline_run` record, linked to workspace via `executes_in`
3. Step 1: spawn "architect" agent with task prompt
4. Agent completes → auto-commit checkpoint → session status = 'completed'
5. Handoff process (see Section 19 for details):
   a. Capture agent output: full session log saved to `logs/session-{id}.jsonl`
   b. Generate handoff context based on `handoff_type`:
      - `summary`: LLM-generated summary of what was done + key decisions (via a lightweight summarizer call)
      - `full_log`: entire conversation history (truncated to fit next agent's context window)
      - `diff_only`: `git diff` of all changes made during the step
   c. Write handoff context to `~/.koompi-orch/handoffs/{pipeline_run_id}/step-{n}.md`
   d. Create `hands_off_to` relation in SurrealDB
6. Step 2: spawn "implementer" in same worktree
   - Inject handoff context via the agent's input mode (see Section 17)
   - If context exceeds agent's context window: fall back to `diff_only` + summary header
7. Continues through chain, incrementing `pipeline_run.current_step`
8. Any step fails: set `pipeline_run.status = 'paused'`, notify user, await decision
   - User can: retry step, skip step, edit handoff context and retry, or abort pipeline

### 7.3 Conflict Detection

1. Background watcher runs every 5 seconds for all 'active' workspaces grouped by repo
2. For each repo group: run `git status` in each worktree via git2-rs to get uncommitted changed file paths
3. Compare file sets across worktrees — if any file path appears in 2+ worktrees:
   - Emit `conflict_warning` event with affected workspaces and overlapping file paths
   - Frontend shows warning badge on affected workspace cards
4. User can:
   - Ignore the warning (agents may be editing different parts of the same file)
   - Pause one of the conflicting agents
   - View a side-by-side diff of the conflicting changes
5. Performance: for 10 concurrent worktrees, git status via libgit2 is ~5ms per worktree = ~50ms total per cycle

### 7.4 Crash Recovery

1. App starts
2. Query SurrealDB: `SELECT * FROM session WHERE status = 'running'`
3. For each: check if PID is still alive (`kill(pid, 0)` on Unix, `OpenProcess` on Windows)
4. Dead PIDs: set `session.status = 'crashed'`, `workspace.locked_by = NONE`
5. Frontend shows "Crashed" badge with "Resume" button on affected workspaces
6. Resume process per agent type:
   - Agents with native resume (Claude Code `--resume`): re-spawn with resume flag + session ID
   - Agents without resume: re-spawn in same worktree, inject last N messages from session log as first message context
   - Session log format (JSONL): `{"ts": "...", "role": "user|assistant|tool", "content": "...", "turn": 1}`

### 7.5 Auto-Review Chain

1. Agent finishes implementation (session status → 'completed')
2. Orchestrator checks workspace config for `auto_review` flag (from global or project config)
3. If enabled:
   - Wait for workspace lock release
   - Spawn reviewer agent in same worktree (acquire lock)
   - Inject the implementation diff as context (via agent's input mode)
4. Reviewer reads diff, generates review comments
5. Review output parsed and stored as checkpoint
6. If reviewer reports issues: notify user, set workspace status = 'review'
7. If clean: auto-advance workspace status to 'done', notify user

### 7.6 Workspace Mutual Exclusion

1. Each workspace has a `locked_by` field pointing to the active session
2. Before spawning an agent in a workspace, check lock:
   - If `locked_by = NONE`: acquire lock, proceed
   - If locked by a running session: reject with error "workspace is in use by session X"
   - If locked by a dead session: clear stale lock (crash recovery), then acquire
3. Lock released when session completes, crashes, or is killed
4. Pipelines hold the lock across all steps (each step inherits the pipeline's lock)

---

## 8. UI Layout

```
+-----------------------------------------------------------+
| koompi-orch                          [Mod+K] Search   [=] |
+----------+---------------------------+--------------------+
|          |                           |                    |
| REPOS    | CHAT / TERMINAL / EDITOR  | CHANGES            |
|          |                           |                    |
| > my-app | [Claude Code] opus-4.6    | M  src/main.rs     |
|   |- feat|                           | A  src/new.rs      |
|   |- fix | > Implement the auth      | D  src/old.rs      |
|          |   module with JWT tokens  |                    |
| > api    |                           | --- DIFF ---       |
|   |- ref | I'll create the auth      | -old line          |
|          | module. Let me start by   | +new line           |
| -------- | analyzing the codebase... |                    |
| KANBAN   |                           | [Comment...]       |
| -------- | [Tool: Read src/lib.rs]   |                    |
|          | [Tool: Write auth.rs]     | [Commit] [Push]    |
| Backlog  |                           | [Merge]  [PR]      |
|  task-3  | ...                       |                    |
| Active   |                           | METRICS            |
|  feat [R]| _________________________ | Tokens: 45.2k      |
|  fix  [R]| |> Type a message...    | | Cost: $1.23        |
| Review   | |  [Claude v] [Send]    | | Duration: 4m 32s   |
|  refac   | +-------------------------+                    |
| Done     |                           |                    |
|  auth    | [Chat] [Terminal] [Files]  |                    |
+----------+---------------------------+--------------------+
```

---

## 9. Keyboard Shortcuts

All shortcuts use `Mod` which maps to `Cmd` on macOS and `Ctrl` on Linux/Windows.

| Shortcut | Action |
|---|---|
| Mod+K | Command palette |
| Mod+N | New workspace |
| Mod+T | New chat tab |
| Mod+P | File picker (fuzzy search) |
| Mod+Shift+M | Merge current workspace |
| Mod+Shift+Y | Commit and push |
| Mod+[ | Toggle left sidebar |
| Mod+] | Toggle right sidebar |
| Mod+Shift+Z | Zen mode (hide both sidebars) |
| Mod+/ | Show keyboard shortcuts |
| Mod+1-9 | Switch workspace by index |
| Mod+Shift+C | Copy chat as markdown |
| Escape | Close modal/palette |

---

## 10. Agent Templates (Built-in)

| Template | Command | Input Mode | Output Mode | Resume | Default Args |
|---|---|---|---|---|---|
| claude-code | `claude` | `pty_stdin` | `json_stream` | Yes (`--resume`) | `["--dangerously-skip-permissions"]` |
| codex | `codex` | `pty_stdin` | `text_markers` | No | `[]` |
| gemini-cli | `gemini` | `pty_stdin` | `text_markers` | No | `[]` |
| aider | `aider` | `pty_stdin` | `text_markers` | Yes (`--restore-chat-history`) | `["--no-auto-commits"]` |
| custom | (user-defined) | (configurable) | (configurable) | (configurable) | (configurable) |

**Input modes:**
- `pty_stdin`: Write user messages directly to PTY stdin (most agents)
- `flag_message`: Pass message via CLI flag (e.g., `--message "..."`)
- `file_prompt`: Write prompt to a temp file, pass path as arg

**Output modes:**
- `json_stream`: Agent emits structured JSON lines (Claude Code)
- `text_markers`: Parse plain text for known patterns (cost lines, tool use blocks)
- `raw_pty`: No parsing, display raw terminal output

---

## 11. Role Presets (Built-in, inspired by gstack)

| Preset | Injection Method | System Prompt Summary |
|---|---|---|
| architect | `first_message` | Think from first principles, design before coding, consider trade-offs |
| implementer | `first_message` | Write production code, follow existing patterns, test as you go |
| reviewer | `first_message` | Paranoid code review: race conditions, security, N+1 queries, trust boundaries |
| tester | `first_message` | Write comprehensive tests, edge cases, integration tests |
| shipper | `first_message` | Final-mile: sync main, run tests, resolve comments, open PR |
| fixer | `first_message` | Debug and fix: systematic root cause analysis, minimal changes |

**Injection methods:**
- `flag`: Pass via `--system-prompt "..."` flag (if agent supports it)
- `env_var`: Set `SYSTEM_PROMPT` environment variable
- `config_file`: Write to agent's config file before spawning
- `first_message`: Prepend role instructions to the user's first message (most portable, works with all agents)

---

## 12. Configuration

Two config levels:
- **Global:** `~/.koompi-orch/config.toml` — app-wide defaults, API keys, themes
- **Project:** `.orch.toml` in repo root — per-project overrides, agent preferences, pipeline definitions

Project config overrides global config where both define the same key.

```toml
# ~/.koompi-orch/config.toml (global)
[app]
theme = "dark"
data_dir = "~/.koompi-orch"
max_concurrent_agents = 10

[defaults]
agent = "claude-code"
role = "implementer"
auto_review = true
auto_checkpoint = true

[agents.claude-code]
command = "claude"
args = ["--dangerously-skip-permissions"]
env = {}

[agents.codex]
command = "codex"
args = []
env = {}

[agents.gemini-cli]
command = "gemini"
args = []
env = {}

# API keys stored via tauri-plugin-stronghold (OS keychain)
# NOT in this file — managed through UI ApiKeyManager

[notifications]
agent_completed = true
agent_failed = true
agent_needs_input = true
ci_status = true

[pipeline.default]
steps = ["architect", "implementer", "reviewer", "tester"]
```

```toml
# .orch.toml (project-level, committed to repo)
[defaults]
agent = "claude-code"
role = "implementer"
auto_review = true
max_concurrent_agents = 5

[pipeline.feature]
steps = ["architect", "implementer", "reviewer"]

[pipeline.hotfix]
steps = ["fixer", "tester", "shipper"]
```

---

## 13. Data Directory Layout

```
~/.koompi-orch/
├── config.toml                    # Global app configuration
├── db/                            # SurrealDB data files
│   └── surrealkv/
├── worktrees/                     # Git worktrees organized by repo
│   ├── my-app/
│   │   ├── feat-auth-ws3a7b/      # branch + workspace ID suffix
│   │   └── fix-bug-123-ws9c2d/
│   └── api-service/
│       └── refactor-db-wsf4e1/
├── plugins/                       # WASM plugins
│   └── my-plugin/
│       ├── manifest.json
│       └── plugin.wasm
├── logs/                          # Agent session logs (JSONL)
│   └── session-{id}.jsonl
├── handoffs/                      # Pipeline handoff context files
│   └── {pipeline_run_id}/
│       ├── step-0.md
│       └── step-1.md
└── stronghold/                    # Encrypted key storage (tauri-plugin-stronghold)
    └── vault.hold
```

**Session log format (JSONL):**
```json
{"ts":"2026-03-14T10:00:01Z","role":"user","content":"Implement JWT auth","turn":1}
{"ts":"2026-03-14T10:00:05Z","role":"assistant","content":"I'll create the auth module...","turn":1}
{"ts":"2026-03-14T10:00:05Z","role":"tool","content":"Read src/lib.rs","turn":1}
{"ts":"2026-03-14T10:00:10Z","role":"assistant","content":"Here's the implementation...","turn":2}
{"ts":"2026-03-14T10:00:10Z","role":"metric","tokens_in":1200,"tokens_out":3400,"cost_usd":0.05,"turn":2}
```

---

## 14. Future Considerations (post v1)

- Team collaboration (shared orchestrations via server mode)
- Remote/self-hosted deployment (access via browser)
- Voice control integration
- Integration with koompi-candle for local AI inference
- Mobile companion app (view agent status)
- Marketplace for community plugins and role presets
- AI-powered conflict resolution (agent auto-merges)

---

## 15a. Cost Guardrails

To prevent runaway agents on expensive models:

```toml
# In config.toml or .orch.toml
[limits]
max_cost_per_session_usd = 10.0    # Pause agent if session exceeds this
max_tokens_per_session = 500000     # Pause agent if total tokens exceed this
max_cost_per_pipeline_usd = 50.0   # Pause pipeline if total cost exceeds this
warn_at_percent = 80                # Emit warning notification at 80% of limit
```

When a limit is hit:
1. Agent is paused (SIGSTOP on Unix, suspend on Windows)
2. Notification sent: "Agent paused: cost limit reached ($9.80/$10.00)"
3. User can: increase limit, resume once, or kill the agent

If cost data is unavailable (agent output mode does not provide it), limits based on cost are skipped and only token limits apply (estimated from PTY output character count as a rough heuristic).

---

## 18a. Plugin Capability: shell_exec

The `exec_command` host function requires a separate `shell_exec` capability that is NOT included by default:

```json
{
  "capabilities": ["agent_type", "shell_exec"]
}
```

When a plugin with `shell_exec` is installed:
1. User sees a warning: "This plugin requests shell execution access within your workspace"
2. User must explicitly approve
3. Commands are sandboxed to the workspace worktree directory (cannot escape via `../` or absolute paths)
4. Blocked commands: `rm -rf /`, `sudo`, `chmod 777`, and other dangerous patterns
5. Without `shell_exec` capability, calling `exec_command` returns `Err("capability not granted")`

---

## 18b. Plugin WASM Serialization Boundary

WASM's linear memory model does not support Rust's `&str` and `&[&str]` directly. The actual ABI uses JSON serialization over shared memory:

1. Host writes JSON-encoded arguments to WASM linear memory
2. Plugin reads and deserializes
3. Plugin writes JSON-encoded return value to linear memory
4. Host reads and deserializes

The `PluginHost` trait in Section 18 is the high-level Rust interface. The actual WASM imports use:
```
fn host_call(func_id: u32, payload_ptr: u32, payload_len: u32) -> u64  // returns ptr|len packed
```

A code-generated shim layer (via `wit-bindgen` or manual) translates between the high-level trait and the low-level ABI.

---

## 19a. Pipeline Lock vs Auto-Review

Auto-review is treated as an implicit final pipeline step:

- If a pipeline is running and `auto_review = true`, the reviewer is appended as the last step
- The pipeline lock is held through the auto-review step
- The pipeline is only marked `completed` after auto-review finishes
- If no pipeline is running (standalone agent completes), auto-review acquires a new lock independently

This means:
- Pipeline: `[architect, implementer, reviewer(auto)]` — lock held throughout, no gap
- Standalone: agent completes → lock released → auto-review acquires new lock → reviews → releases

---

## 17a. flag_message Input Mode Lifecycle

For agents using `flag_message` input mode:

- **Session**: A single session record is created and persists across all messages
- **Lock**: Workspace lock is held for the duration of the session (not per-message)
- **Conversation continuity**: Each message spawns a new process, but the session log accumulates all turns. Previous turns are displayed in ChatView from the session log, not from the agent's memory.
- **Process lifecycle**: Process starts → runs to completion → exits. Next message → new process. The workspace lock prevents concurrent processes.
- **When to use**: Agents designed for single-shot tasks (e.g., a linter, a formatter, a custom script). Not recommended for conversational agents.

---

## 7.7 Workspace Status Transitions

Valid state transitions:

```
              ┌──────────┐
              │ backlog  │
              └────┬─────┘
                   │ user starts agent
                   v
              ┌──────────┐
         ┌────│  active   │────┐
         │    └────┬─────┘    │
         │         │           │
    agent fails    │ agent     │ user moves
         │         │ completes │ back manually
         v         v           │
    ┌────────┐ ┌──────────┐   │
    │ failed │ │  review   │◄──┘
    └───┬────┘ └────┬─────┘
        │           │ merge/approve
        │ retry     v
        │      ┌──────────┐
        └─────►│   done    │
               └──────────┘
```

- `backlog → active`: User spawns an agent or manually moves
- `active → review`: Agent completes (or auto-review triggers)
- `active → failed`: Agent crashes or is killed
- `review → done`: User merges or approves
- `review → active`: User requests rework (spawns new agent)
- `failed → active`: User retries (spawns new agent)
- `done → active`: User reopens (spawns new agent, rare)
- Any state → `backlog`: User moves back manually

---

## 20. Database Migration Strategy

Migrations use a sequential version number stored in SurrealDB itself:

```surql
DEFINE TABLE migration SCHEMAFULL;
DEFINE FIELD version ON migration TYPE int;
DEFINE FIELD name ON migration TYPE string;
DEFINE FIELD applied_at ON migration TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_migration_version ON migration FIELDS version UNIQUE;
```

Migration files are embedded in the binary at compile time (via `include_str!`):

```
src-tauri/src/db/migrations/
├── 001_initial_schema.surql
├── 002_add_pipeline_run.surql
├── 003_add_cost_limits.surql
└── ...
```

On app startup:
1. Query `SELECT max(version) FROM migration` to find current version
2. Apply all migrations with version > current, in order
3. INSERT into `migration` table for each applied migration
4. If any migration fails: rollback (SurrealDB transaction), show error, refuse to start

---

## 21. text_markers Parser Robustness

To reduce false positives in `text_markers` output parsing:

1. **Code fence exclusion**: Track markdown code fence state (` ``` ` open/close). Lines inside code fences are never parsed for markers.
2. **Line-start anchoring**: Cost/token patterns only match at the start of a line (after optional whitespace).
3. **Agent-specific patterns**: Each agent template can define custom regex patterns in the template config:
   ```toml
   [agents.aider]
   cost_pattern = "^Tokens: ([\\d,]+) sent, ([\\d,]+) received\\. Cost: \\$([\\d.]+)"
   ```
4. **Confidence threshold**: If a pattern matches but looks suspicious (e.g., cost > $100 in a single turn), flag it as uncertain and show raw text alongside the parsed metric.
5. **Fallback**: Any unrecognized output is passed through as plain text. False positives in metrics are correctable by the user via a "dismiss metric" action in CostTracker.

---

## 15. Smart Agent Routing

The router selects the optimal agent type and model for a given task based on configurable rules.

**Routing signals (checked in order):**
1. **User override**: If user explicitly picks an agent/model, use it (always wins)
2. **Pipeline step**: Each pipeline step specifies a role preset which maps to a preferred agent
3. **Task keyword classification**: Simple keyword matching on the task description:
   - Keywords like "review", "audit", "security" → reviewer preset
   - Keywords like "test", "spec", "coverage" → tester preset
   - Keywords like "fix", "bug", "error" → fixer preset
   - Default → implementer preset
4. **Cost tier**: For each role, a preferred model tier:
   - architect/reviewer → expensive model (Opus, GPT-5.4) — quality matters
   - implementer → mid-tier (Sonnet, GPT-5.2) — balance of speed and quality
   - tester/shipper → cheap model (Haiku, GPT-5.1) — routine tasks
5. **Fallback**: Use `[defaults].agent` and `[defaults].role` from config

The router is a simple rule engine, not ML-based. Rules are configurable in `.orch.toml`:

```toml
[routing]
architect = { agent = "claude-code", model = "opus" }
implementer = { agent = "claude-code", model = "sonnet" }
reviewer = { agent = "claude-code", model = "opus" }
tester = { agent = "claude-code", model = "haiku" }
shipper = { agent = "claude-code", model = "haiku" }
```

---

## 16. Agent Output Parsing

Each agent template declares an `output_mode` that determines how its stdout is parsed.

### json_stream (Claude Code)

Claude Code can emit structured JSON lines. The parser watches for:
- **Tool use blocks**: `{"type":"tool_use","name":"Read","input":{...}}`
- **Cost/token data**: `{"type":"usage","input_tokens":N,"output_tokens":N}`
- **Text output**: `{"type":"text","content":"..."}`
- **Errors**: `{"type":"error","message":"..."}`

These are parsed into a unified `AgentEvent` enum:
```rust
enum AgentEvent {
    Text { content: String },
    ToolUse { name: String, input: serde_json::Value },
    ToolResult { name: String, output: String },
    Usage { tokens_in: u64, tokens_out: u64, cost_usd: f64 },
    Error { message: String },
    Completed,
    NeedsInput,  // agent is waiting for user input
}
```

### text_markers (Codex, Gemini CLI, aider, others)

For agents that output plain text, the parser uses regex-based heuristic detection:
- Cost lines: patterns like `Cost: $0.05` or `Tokens: 1,200 in / 3,400 out`
- Tool use blocks: indented code blocks, file path headers
- Completion markers: agent returns to prompt or exits
- Error patterns: stack traces, error keywords

This is best-effort. Unrecognized output is passed through as raw text to the ChatView.

### raw_pty

No parsing. Full PTY output rendered in xterm.js terminal. Used for custom/unknown agents where structured parsing is not possible.

**CostTracker behavior:** If an agent's output mode does not provide cost data, CostTracker shows "N/A" for that session. The UI does not guess or estimate.

---

## 17. Agent Input Protocol

How user messages reach the agent process:

### pty_stdin (default, most agents)

1. User types message in ChatInput.tsx
2. Frontend invokes Tauri command: `send_message(session_id, message)`
3. Rust backend writes `message + "\n"` to the PTY stdin of the agent process
4. For role preset injection: the first message is prepended with the role's system prompt:
   ```
   [Role: Architect] Think from first principles, design before coding...

   User task: Implement JWT authentication
   ```

### flag_message

1. Agent is spawned per-message with the message as a CLI flag
2. Example: `claude --message "Implement JWT auth" --print`
3. Agent runs to completion, output captured, process exits
4. New message = new process spawn

### file_prompt

1. Write the prompt to a temp file: `/tmp/koompi-orch-prompt-{uuid}.md`
2. Spawn agent with file path as argument
3. Agent reads the file and processes it
4. Useful for agents that accept file-based input

**Handoff injection for pipelines:** When a pipeline step receives handoff context, it is prepended to the user's task prompt regardless of input mode:

```
## Context from previous step (architect)
[handoff content here]

## Your task
[user's original task description]
```

---

## 18. Plugin System (WASM)

### Plugin Manifest (manifest.json)

```json
{
  "name": "my-plugin",
  "version": "0.1.0",
  "description": "Adds custom agent type and pipeline step",
  "author": "author-name",
  "capabilities": ["agent_type", "pipeline_step"],
  "wasm": "plugin.wasm",
  "config_schema": {
    "api_key": { "type": "string", "required": true, "secret": true }
  }
}
```

### Host Functions (exposed to WASM plugins)

Plugins run in a sandboxed wasmtime environment with access to:

```rust
// Host functions available to plugins
trait PluginHost {
    // Read/write to plugin's own namespaced storage in SurrealDB
    fn store_get(key: &str) -> Option<String>;
    fn store_set(key: &str, value: &str);

    // Emit events to the frontend
    fn emit_event(event_type: &str, payload: &str);

    // Read files within the current workspace worktree (sandboxed)
    fn read_file(relative_path: &str) -> Result<String>;

    // Execute a shell command within the workspace (sandboxed, requires capability)
    fn exec_command(command: &str, args: &[&str]) -> Result<String>;

    // Log messages
    fn log(level: &str, message: &str);
}
```

### Plugin Capabilities

- `agent_type`: Register a new agent template (custom command, input/output modes)
- `pipeline_step`: Register a custom pipeline step (runs between standard steps)
- `event_handler`: React to orchestrator events (agent completed, conflict detected, etc.)

### UI Integration

Plugins do NOT render custom UI panels in v1. Plugin output is displayed in the existing ChatView as text/markdown. Custom UI panels are deferred to v2 to avoid the complexity of sandboxed rendering in the webview.

---

## 19. Pipeline Handoff Protocol

The handoff between pipeline steps is the core value proposition. Here's the detailed protocol:

### Handoff Types

| Type | When to Use | Content |
|---|---|---|
| `summary` | Default. When next agent needs high-level context but not every detail | LLM-generated summary: what was done, key decisions, file list, open questions |
| `full_log` | When next agent needs full conversation context (e.g., reviewer needs to see architect's reasoning) | Complete session JSONL, truncated to fit context window |
| `diff_only` | When next agent only needs to see code changes (e.g., tester just needs the diff) | Output of `git diff` for all changes in the worktree |

### Summary Generation

When `handoff_type = summary`, the orchestrator generates a summary by:
1. Reading the session log JSONL
2. Extracting: files modified, tools used, key decisions stated by the agent
3. Formatting into a structured markdown document:
   ```markdown
   ## Step Summary: Architect
   ### Task
   [original task prompt]
   ### What was done
   [bullet points of actions taken]
   ### Files modified
   - src/auth.rs (created)
   - src/lib.rs (modified)
   ### Key decisions
   - Chose JWT over session tokens for statelessness
   - Used RS256 algorithm for asymmetric signing
   ### Open questions
   - Should token expiry be configurable? (defaulted to 1 hour)
   ```
4. If a summarizer LLM call is too expensive, fall back to a mechanical extraction (file list + tool use log without LLM interpretation)

### Context Window Management

If handoff content exceeds the next agent's estimated context window:
1. Try `summary` (shortest)
2. If summary is still too long: truncate to last N turns of the session log
3. Always include: task prompt + file list + diff stats (even if conversation is truncated)

### Handoff File Storage

All handoff artifacts are persisted at `~/.koompi-orch/handoffs/{pipeline_run_id}/step-{n}.md` for debugging and replay.

**Retention policy:**
- Handoffs for completed pipelines older than 30 days are auto-pruned on app startup
- Configurable via `[app] handoff_retention_days = 30` in config.toml
- Set to `0` to disable auto-pruning (manual cleanup via settings UI)
