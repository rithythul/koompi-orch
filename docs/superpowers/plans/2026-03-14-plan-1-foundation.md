# Plan 1: Foundation — Tauri App Shell, SurrealDB, Config, Basic UI

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a buildable Tauri 2 desktop app with SurrealDB embedded, configuration system, and a basic React shell that renders three panels.

**Architecture:** Tauri 2 app with Rust backend providing SurrealDB access and config management via IPC commands. React + Vite + TypeScript frontend with Tailwind CSS and Zustand. The app starts, initializes the database with schema migrations, loads config, and renders the three-panel layout shell.

**Tech Stack:** Rust, Tauri 2, SurrealDB (surrealkv engine), React 18, Vite, TypeScript, Tailwind CSS, Zustand

**Spec Reference:** `/home/userx/projects/koompi-orch/docs/superpowers/specs/2026-03-14-koompi-orch-design.md`

---

## Chunk 1: Project Scaffolding

### Task 1: Initialize Tauri 2 project with React + TypeScript

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/Cargo.toml`
- Create: `~/projects/koompi-orch/src-tauri/src/main.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/lib.rs`
- Create: `~/projects/koompi-orch/src-tauri/tauri.conf.json`
- Create: `~/projects/koompi-orch/package.json`
- Create: `~/projects/koompi-orch/tsconfig.json`
- Create: `~/projects/koompi-orch/vite.config.ts`
- Create: `~/projects/koompi-orch/index.html`
- Create: `~/projects/koompi-orch/src/app/main.tsx`
- Create: `~/projects/koompi-orch/src/app/App.tsx`
- Create: `~/projects/koompi-orch/src/styles/globals.css`
- Create: `~/projects/koompi-orch/tailwind.config.js`
- Create: `~/projects/koompi-orch/postcss.config.js`

- [ ] **Step 1: Create Tauri 2 project**

Run from `~/projects/koompi-orch`:
```bash
pnpm create tauri-app . --template react-ts --manager pnpm
```
If the directory already has files, use `--force` or scaffold manually.

Expected: Project created with Tauri 2, React, TypeScript, Vite.

- [ ] **Step 1b: Create .gitignore**

Create `.gitignore`:
```
node_modules/
target/
dist/
.DS_Store
*.log
```

- [ ] **Step 1c: Fix entry point path**

The scaffolder creates `src/main.tsx`. We use `src/app/main.tsx` instead.

Move the file:
```bash
mkdir -p src/app
mv src/main.tsx src/app/main.tsx 2>/dev/null || true
```

Update `index.html` to reference the correct path:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>koompi-orch</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/app/main.tsx"></script>
  </body>
</html>
```

Update `src-tauri/tauri.conf.json` — set the app identifier and window title:
```json
{
  "$schema": "https://raw.githubusercontent.com/nicovrc/tauri-apps/tauri-v2/core/tauri-config-schema/schema.json",
  "productName": "koompi-orch",
  "identifier": "com.koompi.orch",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "title": "koompi-orch",
        "width": 1400,
        "height": 900,
        "resizable": true,
        "fullscreen": false
      }
    ]
  }
}
```

- [ ] **Step 2: Add Tailwind CSS v3 (pinned for stability)**

```bash
cd ~/projects/koompi-orch
pnpm add -D tailwindcss@3 postcss autoprefixer
npx tailwindcss init -p
```

Update `src/styles/globals.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --bg-primary: #0f0f0f;
  --bg-secondary: #1a1a1a;
  --bg-tertiary: #252525;
  --border: #333333;
  --text-primary: #e0e0e0;
  --text-secondary: #888888;
  --accent: #6366f1;
  --accent-hover: #818cf8;
  --success: #22c55e;
  --warning: #f59e0b;
  --error: #ef4444;
}

body {
  background-color: var(--bg-primary);
  color: var(--text-primary);
  font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  margin: 0;
  overflow: hidden;
  height: 100vh;
}
```

Update `tailwind.config.js`:
```js
/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        primary: 'var(--bg-primary)',
        secondary: 'var(--bg-secondary)',
        tertiary: 'var(--bg-tertiary)',
        border: 'var(--border)',
        'text-primary': 'var(--text-primary)',
        'text-secondary': 'var(--text-secondary)',
        accent: 'var(--accent)',
        'accent-hover': 'var(--accent-hover)',
      },
    },
  },
  plugins: [],
};
```

- [ ] **Step 3: Add Zustand**

```bash
pnpm add zustand
```

- [ ] **Step 4: Verify the app builds and opens a window**

```bash
cd ~/projects/koompi-orch
pnpm tauri dev
```

Expected: A Tauri window opens with the default React template content.

- [ ] **Step 5: Commit**

```bash
cd ~/projects/koompi-orch
git init
git add -A
git commit -m "chore: scaffold Tauri 2 + React + TypeScript + Tailwind project"
```

---

### Task 2: Add Rust dependencies to Cargo.toml

**Files:**
- Modify: `~/projects/koompi-orch/src-tauri/Cargo.toml`

- [ ] **Step 1: Add core Rust dependencies**

Add to `[dependencies]` in `src-tauri/Cargo.toml`:
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-notification = "2"
tauri-plugin-stronghold = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
surrealdb = { version = "2", features = ["kv-surrealkv"] }
# NOTE: Verify SurrealDB 2.x feature flags and engine type names against
# https://docs.rs/surrealdb/latest — the engine may be `SurrealKv` not `SurrealKV`
# and the feature may differ. Check `cargo doc --open` after adding.
git2 = "0.19"
octocrab = "0.41"
portable-pty = "0.8"
tokio = { version = "1", features = ["full"] }
toml = "0.8"
uuid = { version = "1", features = ["v4"] }
dirs = "6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 2: Verify it compiles**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo check
```

Expected: Compiles with no errors (warnings are OK).

- [ ] **Step 3: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add Rust dependencies (surrealdb, git2, portable-pty, etc.)"
```

---

## Chunk 2: Configuration System

### Task 3: Create the config module

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/config/mod.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/config/settings.rs`
- Modify: `~/projects/koompi-orch/src-tauri/src/lib.rs`

- [ ] **Step 1: Write the settings test**

Create `src-tauri/src/config/settings.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,
    pub defaults: DefaultSettings,
    pub notifications: NotificationSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub data_dir: PathBuf,
    pub max_concurrent_agents: u32,
    #[serde(default = "default_handoff_retention")]
    pub handoff_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultSettings {
    pub agent: String,
    pub role: String,
    pub auto_review: bool,
    pub auto_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub agent_completed: bool,
    pub agent_failed: bool,
    pub agent_needs_input: bool,
    pub ci_status: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// CostLimits is implemented in Plan 4 (Orchestration) when cost tracking is wired up

fn default_handoff_retention() -> u32 {
    30
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                theme: "dark".to_string(),
                data_dir: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".koompi-orch"),
                max_concurrent_agents: 10,
                handoff_retention_days: 30,
            },
            defaults: DefaultSettings {
                agent: "claude-code".to_string(),
                role: "implementer".to_string(),
                auto_review: true,
                auto_checkpoint: true,
            },
            notifications: NotificationSettings {
                agent_completed: true,
                agent_failed: true,
                agent_needs_input: true,
                ci_status: true,
            },
        }
    }
}

impl AppConfig {
    /// Load config from global path, falling back to defaults
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".koompi-orch")
            .join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to global path
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = self.app.data_dir.join("config.toml");
        std::fs::create_dir_all(config_path.parent().unwrap())?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Ensure data directories exist
    pub fn ensure_dirs(&self) -> Result<(), Box<dyn std::error::Error>> {
        let dirs = [
            self.app.data_dir.clone(),
            self.app.data_dir.join("db"),
            self.app.data_dir.join("worktrees"),
            self.app.data_dir.join("plugins"),
            self.app.data_dir.join("logs"),
            self.app.data_dir.join("handoffs"),
        ];
        for dir in &dirs {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.app.theme, "dark");
        assert_eq!(config.app.max_concurrent_agents, 10);
        assert_eq!(config.defaults.agent, "claude-code");
        assert!(config.defaults.auto_review);
    }

    #[test]
    fn test_roundtrip_toml() {
        let config = AppConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.app.theme, config.app.theme);
        assert_eq!(deserialized.defaults.agent, config.defaults.agent);
    }

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let mut config = AppConfig::default();
        config.app.data_dir = tmp.path().to_path_buf();
        config.save().unwrap();

        let loaded_content = fs::read_to_string(tmp.path().join("config.toml")).unwrap();
        let loaded: AppConfig = toml::from_str(&loaded_content).unwrap();
        assert_eq!(loaded.app.theme, "dark");
    }

    #[test]
    fn test_ensure_dirs() {
        let tmp = TempDir::new().unwrap();
        let mut config = AppConfig::default();
        config.app.data_dir = tmp.path().to_path_buf();
        config.ensure_dirs().unwrap();

        assert!(tmp.path().join("db").exists());
        assert!(tmp.path().join("worktrees").exists());
        assert!(tmp.path().join("plugins").exists());
        assert!(tmp.path().join("logs").exists());
        assert!(tmp.path().join("handoffs").exists());
    }
}
```

- [ ] **Step 2: Create config module file**

Create `src-tauri/src/config/mod.rs`:
```rust
pub mod settings;

pub use settings::AppConfig;
```

- [ ] **Step 3: Wire into lib.rs**

Update `src-tauri/src/lib.rs` to add the module:
```rust
pub mod config;
```

- [ ] **Step 4: Add tempfile dev dependency**

Add to `src-tauri/Cargo.toml`:
```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 5: Run tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test config
```

Expected: All 4 tests pass.

- [ ] **Step 6: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/config/ src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add configuration system with TOML persistence"
```

---

## Chunk 3: SurrealDB Embedded + Schema Migrations

### Task 4: Create the database module with schema and migrations

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/db/mod.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/db/schema.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/db/migrate.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/db/queries.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/db/live.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/db/migrations/001_initial_schema.surql`
- Modify: `~/projects/koompi-orch/src-tauri/src/lib.rs`

- [ ] **Step 1: Create the initial schema migration file**

Create `src-tauri/src/db/migrations/001_initial_schema.surql`:
```surql
-- Migration 001: Initial Schema
-- koompi-orch database tables, relations, and indexes

-- Migration tracking
DEFINE TABLE migration SCHEMAFULL;
DEFINE FIELD version ON migration TYPE int;
DEFINE FIELD name ON migration TYPE string;
DEFINE FIELD applied_at ON migration TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_migration_version ON migration FIELDS version UNIQUE;

-- Repos
DEFINE TABLE repo SCHEMAFULL;
DEFINE FIELD path ON repo TYPE string;
DEFINE FIELD name ON repo TYPE string;
DEFINE FIELD remote_url ON repo TYPE option<string>;
DEFINE FIELD added_at ON repo TYPE datetime DEFAULT time::now();
DEFINE INDEX idx_repo_path ON repo FIELDS path UNIQUE;

-- Workspaces
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

-- Checkpoints
DEFINE TABLE checkpoint SCHEMAFULL;
DEFINE FIELD commit_sha ON checkpoint TYPE string;
DEFINE FIELD turn_number ON checkpoint TYPE int;
DEFINE FIELD description ON checkpoint TYPE option<string>;
DEFINE FIELD created_at ON checkpoint TYPE datetime DEFAULT time::now();
DEFINE TABLE checkpoint_of SCHEMAFULL TYPE RELATION IN checkpoint OUT workspace;

-- Metrics (append-only)
DEFINE TABLE metric SCHEMAFULL;
DEFINE FIELD tokens_in ON metric TYPE int DEFAULT 0;
DEFINE FIELD tokens_out ON metric TYPE int DEFAULT 0;
DEFINE FIELD cost_usd ON metric TYPE float DEFAULT 0.0;
DEFINE FIELD duration_ms ON metric TYPE int DEFAULT 0;
DEFINE FIELD turn_number ON metric TYPE int DEFAULT 0;
DEFINE FIELD recorded_at ON metric TYPE datetime DEFAULT time::now();
DEFINE TABLE metric_for SCHEMAFULL TYPE RELATION IN metric OUT session;

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

- [ ] **Step 2: Create the migration runner**

Create `src-tauri/src/db/migrate.rs`:
```rust
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tracing::info;

/// Embedded migration files
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial_schema", include_str!("migrations/001_initial_schema.surql")),
];

/// Run all pending migrations
pub async fn run_migrations(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
    // Get current migration version
    #[derive(serde::Deserialize)]
    struct MaxVersion { max_version: Option<i64> }

    let mut response = db
        .query("SELECT math::max(version) AS max_version FROM migration")
        .await?;
    let result: Vec<MaxVersion> = response.take(0)?;
    let current_version = result.first().and_then(|r| r.max_version).unwrap_or(0);

    info!("Current migration version: {}", current_version);

    for (i, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version > current_version {
            info!("Applying migration {}: {}", version, name);

            // Run migration in a transaction-like manner
            // SurrealDB executes multi-statement queries atomically per query call
            db.query(*sql).await?;

            // Record migration (skip for migration 001 since it defines the table)
            db.query("CREATE migration SET version = $version, name = $name")
                .bind(("version", version))
                .bind(("name", name.to_string()))
                .await?;

            info!("Migration {} applied successfully", version);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use surrealdb::engine::local::Mem;

    #[tokio::test]
    async fn test_run_migrations() {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        run_migrations(&db).await.unwrap();

        // Verify migration was recorded
        let result: Vec<serde_json::Value> = db
            .query("SELECT * FROM migration")
            .await
            .unwrap()
            .take(0)
            .unwrap();
        assert_eq!(result.len(), 1);

        // Verify tables exist by inserting a repo
        db.query("CREATE repo SET path = '/test', name = 'test'")
            .await
            .unwrap();

        let repos: Vec<serde_json::Value> = db
            .query("SELECT * FROM repo")
            .await
            .unwrap()
            .take(0)
            .unwrap();
        assert_eq!(repos.len(), 1);
    }

    #[tokio::test]
    async fn test_migrations_are_idempotent() {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();

        // Run twice — should not error
        run_migrations(&db).await.unwrap();
        run_migrations(&db).await.unwrap();

        let result: Vec<serde_json::Value> = db
            .query("SELECT * FROM migration")
            .await
            .unwrap()
            .take(0)
            .unwrap();
        assert_eq!(result.len(), 1); // Still only 1 migration record
    }
}
```

- [ ] **Step 3: Create the schema types module**

Create `src-tauri/src/db/schema.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: Option<Thing>,
    pub path: String,
    pub name: String,
    pub remote_url: Option<String>,
    pub added_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Option<Thing>,
    pub name: String,
    pub branch: String,
    pub worktree_path: String,
    pub status: WorkspaceStatus,
    pub locked_by: Option<Thing>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceStatus {
    Backlog,
    Active,
    Review,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<Thing>,
    pub agent_type: String,
    pub model: Option<String>,
    pub pid: Option<i64>,
    pub role_preset: Option<String>,
    pub status: SessionStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Paused,
    Completed,
    Crashed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub id: Option<Thing>,
    pub name: String,
    pub command: String,
    pub default_args: Vec<String>,
    pub env: Option<serde_json::Value>,
    pub input_mode: String,
    pub output_mode: String,
    pub resume_support: bool,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePreset {
    pub id: Option<Thing>,
    pub name: String,
    pub system_prompt: String,
    pub description: String,
    pub injection_method: String,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub id: Option<Thing>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f64,
    pub duration_ms: i64,
    pub turn_number: i64,
    pub recorded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: Option<Thing>,
    pub commit_sha: String,
    pub turn_number: i64,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: Option<Thing>,
    pub name: String,
    pub steps: Vec<serde_json::Value>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    pub id: Option<Thing>,
    pub current_step: i64,
    pub status: PipelineRunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineRunStatus {
    Running,
    Paused,
    Completed,
    Failed,
}
```

- [ ] **Step 4: Create the queries helper module**

Create `src-tauri/src/db/queries.rs`:
```rust
use super::schema::*;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// Typed query helpers for common database operations
pub struct Queries<'a> {
    db: &'a Surreal<Db>,
}

impl<'a> Queries<'a> {
    pub fn new(db: &'a Surreal<Db>) -> Self {
        Self { db }
    }

    // -- Repos --

    pub async fn create_repo(&self, path: &str, name: &str, remote_url: Option<&str>) -> Result<Repo, surrealdb::Error> {
        let repo: Option<Repo> = self.db
            .query("CREATE repo SET path = $path, name = $name, remote_url = $remote_url")
            .bind(("path", path))
            .bind(("name", name))
            .bind(("remote_url", remote_url))
            .await?
            .take(0)?;
        Ok(repo.expect("repo should be created"))
    }

    pub async fn list_repos(&self) -> Result<Vec<Repo>, surrealdb::Error> {
        let repos: Vec<Repo> = self.db
            .query("SELECT * FROM repo ORDER BY name ASC")
            .await?
            .take(0)?;
        Ok(repos)
    }

    // -- Workspaces --

    pub async fn create_workspace(
        &self,
        name: &str,
        branch: &str,
        worktree_path: &str,
        repo_id: &str,
    ) -> Result<Workspace, surrealdb::Error> {
        let ws: Option<Workspace> = self.db
            .query(
                "CREATE workspace SET name = $name, branch = $branch, worktree_path = $path, status = 'backlog';
                 RELATE (SELECT id FROM workspace WHERE name = $name LIMIT 1)->belongs_to->(type::thing('repo', $repo_id))"
            )
            .bind(("name", name))
            .bind(("branch", branch))
            .bind(("path", worktree_path))
            .bind(("repo_id", repo_id))
            .await?
            .take(0)?;
        Ok(ws.expect("workspace should be created"))
    }

    pub async fn list_workspaces(&self) -> Result<Vec<Workspace>, surrealdb::Error> {
        let workspaces: Vec<Workspace> = self.db
            .query("SELECT * FROM workspace ORDER BY updated_at DESC")
            .await?
            .take(0)?;
        Ok(workspaces)
    }

    pub async fn update_workspace_status(
        &self,
        workspace_id: &str,
        status: &str,
    ) -> Result<(), surrealdb::Error> {
        self.db
            .query("UPDATE type::thing('workspace', $id) SET status = $status, updated_at = time::now()")
            .bind(("id", workspace_id))
            .bind(("status", status))
            .await?;
        Ok(())
    }

    // -- Agent Templates --

    pub async fn seed_builtin_templates(&self) -> Result<(), surrealdb::Error> {
        let templates = vec![
            ("claude-code", "claude", vec!["--dangerously-skip-permissions"], "pty_stdin", "json_stream", true),
            ("codex", "codex", vec![], "pty_stdin", "text_markers", false),
            ("gemini-cli", "gemini", vec![], "pty_stdin", "text_markers", false),
            ("aider", "aider", vec!["--no-auto-commits"], "pty_stdin", "text_markers", true),
        ];

        for (name, cmd, args, input, output, resume) in templates {
            self.db
                .query(
                    "CREATE agent_template SET name = $name, command = $cmd, default_args = $args, \
                     input_mode = $input, output_mode = $output, resume_support = $resume, builtin = true \
                     ON DUPLICATE KEY UPDATE command = $cmd, default_args = $args"
                )
                .bind(("name", name))
                .bind(("cmd", cmd))
                .bind(("args", args))
                .bind(("input", input))
                .bind(("output", output))
                .bind(("resume", resume))
                .await?;
        }
        Ok(())
    }

    pub async fn list_templates(&self) -> Result<Vec<AgentTemplate>, surrealdb::Error> {
        let templates: Vec<AgentTemplate> = self.db
            .query("SELECT * FROM agent_template ORDER BY name ASC")
            .await?
            .take(0)?;
        Ok(templates)
    }

    // -- Role Presets --

    pub async fn seed_builtin_presets(&self) -> Result<(), surrealdb::Error> {
        let presets = vec![
            ("architect", "first_message", "Think from first principles, design before coding, consider trade-offs", "System architect role"),
            ("implementer", "first_message", "Write production code, follow existing patterns, test as you go", "Implementation engineer role"),
            ("reviewer", "first_message", "Paranoid code review: race conditions, security, N+1 queries, trust boundaries", "Code reviewer role"),
            ("tester", "first_message", "Write comprehensive tests, edge cases, integration tests", "Test engineer role"),
            ("shipper", "first_message", "Final-mile: sync main, run tests, resolve comments, open PR", "Release engineer role"),
            ("fixer", "first_message", "Debug and fix: systematic root cause analysis, minimal changes", "Bug fixer role"),
        ];

        for (name, method, prompt, desc) in presets {
            self.db
                .query(
                    "CREATE role_preset SET name = $name, injection_method = $method, \
                     system_prompt = $prompt, description = $desc, builtin = true \
                     ON DUPLICATE KEY UPDATE system_prompt = $prompt, description = $desc"
                )
                .bind(("name", name))
                .bind(("method", method))
                .bind(("prompt", prompt))
                .bind(("desc", desc))
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate::run_migrations;
    use surrealdb::engine::local::Mem;

    async fn setup_db() -> Surreal<Db> {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        run_migrations(&db).await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_create_and_list_repos() {
        let db = setup_db().await;
        let q = Queries::new(&db);

        q.create_repo("/home/user/my-app", "my-app", Some("https://github.com/user/my-app")).await.unwrap();
        q.create_repo("/home/user/api", "api", None).await.unwrap();

        let repos = q.list_repos().await.unwrap();
        assert_eq!(repos.len(), 2);
        assert_eq!(repos[0].name, "api"); // alphabetical
        assert_eq!(repos[1].name, "my-app");
    }

    #[tokio::test]
    async fn test_seed_templates() {
        let db = setup_db().await;
        let q = Queries::new(&db);

        q.seed_builtin_templates().await.unwrap();
        let templates = q.list_templates().await.unwrap();
        assert_eq!(templates.len(), 4);
        assert!(templates.iter().any(|t| t.name == "claude-code"));
    }

    #[tokio::test]
    async fn test_seed_presets() {
        let db = setup_db().await;
        let q = Queries::new(&db);

        q.seed_builtin_presets().await.unwrap();

        let presets: Vec<RolePreset> = db
            .query("SELECT * FROM role_preset")
            .await
            .unwrap()
            .take(0)
            .unwrap();
        assert_eq!(presets.len(), 6);
    }
}
```

- [ ] **Step 5: Create the LIVE query module (stub)**

Create `src-tauri/src/db/live.rs`:
```rust
//! SurrealDB LIVE query subscriptions
//! Bridges LIVE queries to Tauri event emitter for real-time UI updates
//!
//! Implementation deferred to Plan 5 (Full UI) when the frontend
//! components that consume these events are built.

// Placeholder — will be implemented in Plan 5
```

- [ ] **Step 6: Create the db module root**

Create `src-tauri/src/db/mod.rs`:
```rust
pub mod live;
pub mod migrate;
pub mod queries;
pub mod schema;

use surrealdb::engine::local::{Db, SurrealKV};
use surrealdb::Surreal;
use std::path::Path;

/// Initialize the embedded SurrealDB database
pub async fn init_db(data_dir: &Path) -> Result<Surreal<Db>, Box<dyn std::error::Error>> {
    let db_path = data_dir.join("db").join("surrealkv");
    std::fs::create_dir_all(&db_path)?;

    let db = Surreal::new::<SurrealKV>(db_path).await?;
    db.use_ns("koompi_orch").use_db("main").await?;

    migrate::run_migrations(&db).await?;

    // Seed built-in data if empty
    let templates: Vec<schema::AgentTemplate> = db
        .query("SELECT * FROM agent_template WHERE builtin = true")
        .await?
        .take(0)?;

    if templates.is_empty() {
        let q = queries::Queries::new(&db);
        q.seed_builtin_templates().await?;
        q.seed_builtin_presets().await?;
        tracing::info!("Seeded built-in agent templates and role presets");
    }

    Ok(db)
}
```

- [ ] **Step 7: Wire db module into lib.rs**

Update `src-tauri/src/lib.rs`:
```rust
pub mod config;
pub mod db;
```

- [ ] **Step 8: Run all tests**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test
```

Expected: All tests pass (config + db modules).

- [ ] **Step 9: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/db/ src-tauri/src/lib.rs
git commit -m "feat: add SurrealDB embedded with schema migrations and typed queries"
```

---

## Chunk 4: Tauri IPC Commands + App Startup

### Task 5: Create IPC commands module and wire app startup

**Files:**
- Create: `~/projects/koompi-orch/src-tauri/src/ipc/mod.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/ipc/commands.rs`
- Create: `~/projects/koompi-orch/src-tauri/src/ipc/events.rs`
- Modify: `~/projects/koompi-orch/src-tauri/src/main.rs`
- Modify: `~/projects/koompi-orch/src-tauri/src/lib.rs`

- [ ] **Step 1: Create the events module**

Create `src-tauri/src/ipc/events.rs`:
```rust
//! Tauri event names — used by both backend emitters and frontend listeners
//!
//! Convention: snake_case event names, prefixed by domain

pub const WORKSPACE_CREATED: &str = "workspace_created";
pub const WORKSPACE_UPDATED: &str = "workspace_updated";
pub const WORKSPACE_DELETED: &str = "workspace_deleted";
pub const SESSION_STARTED: &str = "session_started";
pub const SESSION_OUTPUT: &str = "session_output";
pub const SESSION_COMPLETED: &str = "session_completed";
pub const SESSION_CRASHED: &str = "session_crashed";
pub const CONFLICT_WARNING: &str = "conflict_warning";
pub const METRIC_RECORDED: &str = "metric_recorded";
pub const PIPELINE_STEP_COMPLETED: &str = "pipeline_step_completed";
pub const NOTIFICATION: &str = "notification";
```

- [ ] **Step 2: Create the commands module**

Create `src-tauri/src/ipc/commands.rs`:
```rust
use crate::config::AppConfig;
use crate::db::queries::Queries;
use crate::db::schema::*;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tauri::State;

/// App state managed by Tauri
pub struct AppState {
    pub db: Surreal<Db>,
    pub config: AppConfig,
}

// -- Config commands --

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.clone())
}

// -- Repo commands --

#[tauri::command]
pub async fn list_repos(state: State<'_, AppState>) -> Result<Vec<Repo>, String> {
    let q = Queries::new(&state.db);
    q.list_repos().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_repo(
    state: State<'_, AppState>,
    path: String,
    name: String,
    remote_url: Option<String>,
) -> Result<Repo, String> {
    let q = Queries::new(&state.db);
    q.create_repo(&path, &name, remote_url.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// -- Workspace commands --

#[tauri::command]
pub async fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<Workspace>, String> {
    let q = Queries::new(&state.db);
    q.list_workspaces().await.map_err(|e| e.to_string())
}

// -- Template commands --

#[tauri::command]
pub async fn list_agent_templates(state: State<'_, AppState>) -> Result<Vec<AgentTemplate>, String> {
    let q = Queries::new(&state.db);
    q.list_templates().await.map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Create the ipc module root**

Create `src-tauri/src/ipc/mod.rs`:
```rust
pub mod commands;
pub mod events;

pub use commands::AppState;
```

- [ ] **Step 4: Wire everything into main.rs**

Update `src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use koompi_orch::config::AppConfig;
use koompi_orch::db;
use koompi_orch::ipc::commands::*;
use koompi_orch::ipc::AppState;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "koompi_orch=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let config = AppConfig::load().unwrap_or_default();
            config.ensure_dirs().unwrap();

            // Block on DB init to ensure state is managed before any IPC calls
            let db = tauri::async_runtime::block_on(async {
                db::init_db(&config.app.data_dir).await
            }).map_err(|e| format!("Failed to initialize database: {}", e))?;

            app.manage(AppState { db, config });
            tracing::info!("Database initialized successfully");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            list_repos,
            add_repo,
            list_workspaces,
            list_agent_templates,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Update lib.rs with ipc module**

Update `src-tauri/src/lib.rs`:
```rust
pub mod config;
pub mod db;
pub mod ipc;
```

- [ ] **Step 6: Verify Rust compiles**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo check
```

Expected: Compiles with no errors.

- [ ] **Step 7: Commit**

```bash
cd ~/projects/koompi-orch
git add src-tauri/src/ipc/ src-tauri/src/main.rs src-tauri/src/lib.rs
git commit -m "feat: add Tauri IPC commands and app startup with SurrealDB init"
```

---

## Chunk 5: Basic React UI Shell

### Task 6: Create the three-panel layout shell

**Files:**
- Create: `~/projects/koompi-orch/src/components/layout/ThreePanel.tsx`
- Create: `~/projects/koompi-orch/src/components/layout/Sidebar.tsx`
- Create: `~/projects/koompi-orch/src/components/layout/CenterPanel.tsx`
- Create: `~/projects/koompi-orch/src/components/layout/RightPanel.tsx`
- Create: `~/projects/koompi-orch/src/hooks/useTauriCommand.ts`
- Create: `~/projects/koompi-orch/src/hooks/useTauriEvent.ts`
- Create: `~/projects/koompi-orch/src/hooks/useKeyboard.ts`
- Create: `~/projects/koompi-orch/src/stores/settingsStore.ts`
- Create: `~/projects/koompi-orch/src/lib/ipc.ts`
- Create: `~/projects/koompi-orch/src/lib/keybindings.ts`
- Create: `~/projects/koompi-orch/src/lib/theme.ts`
- Modify: `~/projects/koompi-orch/src/app/App.tsx`
- Modify: `~/projects/koompi-orch/src/app/main.tsx`

- [ ] **Step 1: Create the IPC type definitions**

Create `src/lib/ipc.ts`:
```typescript
import { invoke } from "@tauri-apps/api/core";

// --- Types matching Rust schema ---

export interface Repo {
  id?: string;
  path: string;
  name: string;
  remote_url?: string;
  added_at?: string;
}

export interface Workspace {
  id?: string;
  name: string;
  branch: string;
  worktree_path: string;
  status: "backlog" | "active" | "review" | "done" | "failed";
  locked_by?: string;
  created_at?: string;
  updated_at?: string;
}

export interface AgentTemplate {
  id?: string;
  name: string;
  command: string;
  default_args: string[];
  env?: Record<string, string>;
  input_mode: "pty_stdin" | "flag_message" | "file_prompt";
  output_mode: "json_stream" | "text_markers" | "raw_pty";
  resume_support: boolean;
  builtin: boolean;
}

export interface AppConfig {
  app: {
    theme: string;
    data_dir: string;
    max_concurrent_agents: number;
    handoff_retention_days: number;
  };
  defaults: {
    agent: string;
    role: string;
    auto_review: boolean;
    auto_checkpoint: boolean;
  };
  notifications: {
    agent_completed: boolean;
    agent_failed: boolean;
    agent_needs_input: boolean;
    ci_status: boolean;
  };
}

// --- IPC command wrappers ---

export const api = {
  getConfig: () => invoke<AppConfig>("get_config"),
  listRepos: () => invoke<Repo[]>("list_repos"),
  addRepo: (path: string, name: string, remote_url?: string) =>
    invoke<Repo>("add_repo", { path, name, remote_url }),
  listWorkspaces: () => invoke<Workspace[]>("list_workspaces"),
  listAgentTemplates: () => invoke<AgentTemplate[]>("list_agent_templates"),
};
```

- [ ] **Step 2: Create the Tauri hooks**

Create `src/hooks/useTauriCommand.ts`:
```typescript
import { useState, useEffect, useCallback } from "react";

/**
 * Hook for calling Tauri commands with loading/error state
 */
export function useTauriCommand<T>(
  commandFn: () => Promise<T>,
  deps: unknown[] = []
) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refetch = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await commandFn();
      setData(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, deps);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { data, loading, error, refetch };
}
```

Create `src/hooks/useTauriEvent.ts`:
```typescript
import { useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

/**
 * Hook for listening to Tauri events
 */
export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void
) {
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<T>(eventName, (event) => {
      handler(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [eventName, handler]);
}
```

- [ ] **Step 3: Create the keybindings system**

Create `src/lib/keybindings.ts`:
```typescript
type KeyHandler = () => void;

interface Keybinding {
  key: string;
  mod: boolean;
  shift: boolean;
  handler: KeyHandler;
  description: string;
}

const bindings: Keybinding[] = [];

export function registerKeybinding(
  key: string,
  handler: KeyHandler,
  description: string,
  options?: { mod?: boolean; shift?: boolean }
) {
  const k = key.toLowerCase();
  const mod = options?.mod ?? true;
  const shift = options?.shift ?? false;

  // Deduplicate: replace existing binding for same key combo
  const existing = bindings.findIndex(
    (b) => b.key === k && b.mod === mod && b.shift === shift
  );
  const binding = { key: k, mod, shift, handler, description };
  if (existing >= 0) {
    bindings[existing] = binding;
  } else {
    bindings.push(binding);
  }
}

export function handleKeyDown(e: KeyboardEvent) {
  const isMod = e.metaKey || e.ctrlKey;

  for (const binding of bindings) {
    if (
      e.key.toLowerCase() === binding.key &&
      isMod === binding.mod &&
      e.shiftKey === binding.shift
    ) {
      e.preventDefault();
      binding.handler();
      return;
    }
  }
}

export function getBindings() {
  return [...bindings];
}
```

Create `src/hooks/useKeyboard.ts`:
```typescript
import { useEffect } from "react";
import { handleKeyDown } from "../lib/keybindings";

export function useKeyboard() {
  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}
```

- [ ] **Step 4: Create theme tokens**

Create `src/lib/theme.ts`:
```typescript
export const theme = {
  dark: {
    bgPrimary: "#0f0f0f",
    bgSecondary: "#1a1a1a",
    bgTertiary: "#252525",
    border: "#333333",
    textPrimary: "#e0e0e0",
    textSecondary: "#888888",
    accent: "#6366f1",
    accentHover: "#818cf8",
    success: "#22c55e",
    warning: "#f59e0b",
    error: "#ef4444",
  },
  light: {
    bgPrimary: "#ffffff",
    bgSecondary: "#f5f5f5",
    bgTertiary: "#e8e8e8",
    border: "#d4d4d4",
    textPrimary: "#1a1a1a",
    textSecondary: "#666666",
    accent: "#6366f1",
    accentHover: "#4f46e5",
    success: "#16a34a",
    warning: "#d97706",
    error: "#dc2626",
  },
} as const;
```

- [ ] **Step 5: Create the settings store**

Create `src/stores/settingsStore.ts`:
```typescript
import { create } from "zustand";

interface SettingsState {
  leftSidebarOpen: boolean;
  rightSidebarOpen: boolean;
  theme: "dark" | "light";
  toggleLeftSidebar: () => void;
  toggleRightSidebar: () => void;
  toggleZenMode: () => void;
  setTheme: (theme: "dark" | "light") => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  leftSidebarOpen: true,
  rightSidebarOpen: true,
  theme: "dark",
  toggleLeftSidebar: () =>
    set((s) => ({ leftSidebarOpen: !s.leftSidebarOpen })),
  toggleRightSidebar: () =>
    set((s) => ({ rightSidebarOpen: !s.rightSidebarOpen })),
  toggleZenMode: () =>
    set(() => ({ leftSidebarOpen: false, rightSidebarOpen: false })),
  setTheme: (theme) => set({ theme }),
}));
```

- [ ] **Step 6: Create the layout components**

Create `src/components/layout/Sidebar.tsx`:
```tsx
export function Sidebar() {
  return (
    <div className="h-full bg-[var(--bg-secondary)] border-r border-[var(--border)] flex flex-col">
      <div className="p-3 border-b border-[var(--border)]">
        <h2 className="text-sm font-semibold text-[var(--text-secondary)] uppercase tracking-wider">
          Workspaces
        </h2>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        <p className="text-sm text-[var(--text-secondary)] p-2">
          No workspaces yet. Click + to create one.
        </p>
      </div>
      <div className="p-3 border-t border-[var(--border)]">
        <button className="w-full py-1.5 px-3 text-sm bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white rounded transition-colors">
          + New Workspace
        </button>
      </div>
    </div>
  );
}
```

Create `src/components/layout/CenterPanel.tsx`:
```tsx
export function CenterPanel() {
  return (
    <div className="h-full bg-[var(--bg-primary)] flex flex-col">
      <div className="p-3 border-b border-[var(--border)] flex items-center justify-between">
        <span className="text-sm text-[var(--text-secondary)]">
          Select a workspace to start
        </span>
      </div>
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <h1 className="text-2xl font-bold text-[var(--text-primary)] mb-2">
            koompi-orch
          </h1>
          <p className="text-[var(--text-secondary)]">
            Conduct a team of AI agents from one desktop
          </p>
          <p className="text-sm text-[var(--text-secondary)] mt-4">
            Press <kbd className="px-1.5 py-0.5 bg-[var(--bg-tertiary)] rounded text-xs">Mod+N</kbd> to create a workspace
          </p>
        </div>
      </div>
    </div>
  );
}
```

Create `src/components/layout/RightPanel.tsx`:
```tsx
export function RightPanel() {
  return (
    <div className="h-full bg-[var(--bg-secondary)] border-l border-[var(--border)] flex flex-col">
      <div className="p-3 border-b border-[var(--border)]">
        <h2 className="text-sm font-semibold text-[var(--text-secondary)] uppercase tracking-wider">
          Changes
        </h2>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        <p className="text-sm text-[var(--text-secondary)] p-2">
          No changes to display.
        </p>
      </div>
    </div>
  );
}
```

Create `src/components/layout/ThreePanel.tsx`:
```tsx
import { Sidebar } from "./Sidebar";
import { CenterPanel } from "./CenterPanel";
import { RightPanel } from "./RightPanel";
import { useSettingsStore } from "../../stores/settingsStore";

export function ThreePanel() {
  const { leftSidebarOpen, rightSidebarOpen } = useSettingsStore();

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      {leftSidebarOpen && (
        <div className="w-64 flex-shrink-0">
          <Sidebar />
        </div>
      )}
      <div className="flex-1 min-w-0">
        <CenterPanel />
      </div>
      {rightSidebarOpen && (
        <div className="w-72 flex-shrink-0">
          <RightPanel />
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 7: Update App.tsx and main.tsx**

Update `src/app/App.tsx`:
```tsx
import { ThreePanel } from "../components/layout/ThreePanel";
import { useKeyboard } from "../hooks/useKeyboard";
import { registerKeybinding } from "../lib/keybindings";
import { useSettingsStore } from "../stores/settingsStore";
import { useEffect } from "react";

export default function App() {
  useKeyboard();

  const { toggleLeftSidebar, toggleRightSidebar, toggleZenMode } =
    useSettingsStore();

  useEffect(() => {
    registerKeybinding("[", toggleLeftSidebar, "Toggle left sidebar");
    registerKeybinding("]", toggleRightSidebar, "Toggle right sidebar");
    registerKeybinding("z", toggleZenMode, "Zen mode", { shift: true });
  }, []);

  return <ThreePanel />;
}
```

Update `src/app/main.tsx`:
```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "../styles/globals.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 8: Verify the full app builds and renders**

```bash
cd ~/projects/koompi-orch
pnpm tauri dev
```

Expected: Tauri window opens showing the three-panel layout with dark theme. Sidebar on left, empty center panel with "koompi-orch" heading, changes panel on right. Pressing Mod+[ and Mod+] toggles sidebars.

- [ ] **Step 9: Commit**

```bash
cd ~/projects/koompi-orch
git add src/ src-tauri/
git commit -m "feat: add three-panel layout shell with sidebar toggles and keyboard shortcuts"
```

---

## Chunk 6: End-to-End Verification

### Task 7: Verify IPC works end-to-end (frontend calls Rust backend)

**Files:**
- Modify: `~/projects/koompi-orch/src/components/layout/Sidebar.tsx`

- [ ] **Step 1: Add a repo list fetch to the sidebar**

Update `src/components/layout/Sidebar.tsx`:
```tsx
import { useTauriCommand } from "../../hooks/useTauriCommand";
import { api, Repo } from "../../lib/ipc";

export function Sidebar() {
  const { data: repos, loading } = useTauriCommand<Repo[]>(
    () => api.listRepos(),
    []
  );

  return (
    <div className="h-full bg-[var(--bg-secondary)] border-r border-[var(--border)] flex flex-col">
      <div className="p-3 border-b border-[var(--border)]">
        <h2 className="text-sm font-semibold text-[var(--text-secondary)] uppercase tracking-wider">
          Repos
        </h2>
      </div>
      <div className="flex-1 overflow-y-auto p-2">
        {loading ? (
          <p className="text-sm text-[var(--text-secondary)] p-2">Loading...</p>
        ) : repos && repos.length > 0 ? (
          repos.map((repo) => (
            <div
              key={repo.path}
              className="p-2 text-sm text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] rounded cursor-pointer"
            >
              {repo.name}
            </div>
          ))
        ) : (
          <p className="text-sm text-[var(--text-secondary)] p-2">
            No repos added yet.
          </p>
        )}
      </div>
      <div className="p-3 border-t border-[var(--border)]">
        <button className="w-full py-1.5 px-3 text-sm bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white rounded transition-colors">
          + New Workspace
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Run the full app and verify IPC**

```bash
cd ~/projects/koompi-orch
pnpm tauri dev
```

Expected: App opens, sidebar shows "No repos added yet." (confirming the Tauri IPC round-trip works: React calls `list_repos` → Rust queries SurrealDB → returns empty array → React renders message).

Check Rust logs in terminal for: "Database initialized successfully" and "Seeded built-in agent templates and role presets".

- [ ] **Step 3: Run all Rust tests one final time**

```bash
cd ~/projects/koompi-orch/src-tauri
cargo test
```

Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
cd ~/projects/koompi-orch
git add src/
git commit -m "feat: wire sidebar IPC to SurrealDB — end-to-end verification"
```

---

## Plan 1 Complete

At this point you have:
- A buildable Tauri 2 desktop app
- SurrealDB embedded with full schema and migrations
- Configuration system with TOML persistence
- Typed IPC commands (Rust → Frontend)
- Three-panel layout shell with keyboard shortcuts
- Built-in agent templates and role presets seeded in the database
- All tests passing

**Next:** Proceed to Plan 2 (Agent Engine) to add PTY process management, agent spawning, output parsing, and input injection.
