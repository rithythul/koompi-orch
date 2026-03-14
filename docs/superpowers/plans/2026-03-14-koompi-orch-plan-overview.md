# koompi-orch Implementation Plan — Overview

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan.

This project is split into 7 sequential plans. Each produces working, testable software.

## Plan Sequence

| # | Plan | Depends On | Produces |
|---|---|---|---|
| 1 | **Foundation** | None | Tauri app shell, SurrealDB, config, basic UI shell |
| 2 | **Agent Engine** | Plan 1 | PTY process management, agent templates, output parsing, input injection |
| 3 | **Workspace & Git** | Plan 1 | Worktree lifecycle, diff, merge, commit, conflict detection, snapshots |
| 4A | **Orchestration Engine** | Plans 2+3 | Core engine (spawn/monitor agents), resource governor (cost limits) |
| 4B | **Pipelines & Recovery** | Plan 4A | Pipeline chaining, handoff protocol, smart routing, crash recovery |
| 5A | **Core UI** | Plans 1-4 | Workspace sidebar, kanban board, agent chat, command palette, stores |
| 5B | **Rich UI & Polish** | Plan 5A | Terminal, Monaco editor, diff viewer, dashboard, settings, plugin stubs, notifications |

## Build Order Rationale

- Plan 1 sets up the project skeleton so all subsequent plans have a buildable app
- Plans 2 and 3 can be worked on in parallel (independent subsystems)
- Plan 4A composes Plans 2+3 into the orchestration layer; 4B adds pipelines on top
- Plan 5A builds the core UI components; 5B adds the rich interactive components

## Plan Files

```
docs/superpowers/plans/
├── 2026-03-14-koompi-orch-plan-overview.md     (this file)
├── 2026-03-14-plan-1-foundation.md              (reviewed + fixed)
├── 2026-03-14-plan-2-agent-engine.md
├── 2026-03-14-plan-3-workspace-git.md
├── 2026-03-14-plan-4a-orchestration-engine.md
├── 2026-03-14-plan-4b-pipelines-recovery.md
├── 2026-03-14-plan-5a-core-ui.md
└── 2026-03-14-plan-5b-rich-ui.md
```

## Key Files (final state)

### Rust Backend (src-tauri/src/)
```
main.rs, lib.rs
config/mod.rs, settings.rs
db/mod.rs, schema.rs, queries.rs, live.rs, migrate.rs
agent/mod.rs, process.rs, registry.rs, parser.rs, input.rs, presets.rs, config.rs
workspace/mod.rs, manager.rs, snapshot.rs, conflict.rs, lock.rs, status.rs
git/mod.rs, diff.rs, merge.rs, commit.rs, remote.rs
orchestrator/mod.rs, engine.rs, pipeline.rs, router.rs, governor.rs, recovery.rs
plugin/mod.rs, host.rs, manifest.rs, api.rs
ipc/mod.rs, commands.rs, events.rs
notify/mod.rs, notifications.rs
```

### React Frontend (src/)
```
app/App.tsx, main.tsx, router.tsx
components/layout/ThreePanel.tsx, Sidebar.tsx, CenterPanel.tsx, RightPanel.tsx, CommandPalette.tsx
components/workspace/WorkspaceCard.tsx, KanbanBoard.tsx, WorkspaceCreate.tsx, MultiRepoSelector.tsx, FileTree.tsx
components/agent/ChatView.tsx, ChatInput.tsx, AgentPicker.tsx, PipelineBuilder.tsx, CostTracker.tsx, AgentStatus.tsx
components/terminal/Terminal.tsx
components/editor/CodeEditor.tsx
components/diff/DiffViewer.tsx, DiffComment.tsx, TurnDiff.tsx, MergeActions.tsx
components/dashboard/ProjectDashboard.tsx, MetricsChart.tsx, GlobalSearch.tsx
components/settings/SettingsPage.tsx, ApiKeyManager.tsx, AgentTemplates.tsx, ThemeToggle.tsx
hooks/useTauriCommand.ts, useTauriEvent.ts, useLiveQuery.ts, useWorkspace.ts, useAgent.ts, useKeyboard.ts
stores/workspaceStore.ts, agentStore.ts, settingsStore.ts, notificationStore.ts
lib/ipc.ts, keybindings.ts, theme.ts
styles/globals.css
```
