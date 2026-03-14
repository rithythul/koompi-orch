pub mod migrate;
pub mod queries;
pub mod schema;

use surrealdb::engine::local::{Db, SurrealKv};
use surrealdb::Surreal;
use std::path::Path;
use tracing::info;

/// Initialize embedded SurrealDB with SurrealKV engine
pub async fn init_db(data_dir: &Path) -> Result<Surreal<Db>, Box<dyn std::error::Error>> {
    let db_path = data_dir.join("db");
    std::fs::create_dir_all(&db_path)?;

    info!("Opening SurrealDB at {:?}", db_path);
    let db = Surreal::new::<SurrealKv>(db_path.to_str().unwrap()).await?;
    db.use_ns("koompi").use_db("orch").await?;

    // Run migrations
    migrate::run_migrations(&db).await?;

    // Seed builtin data
    seed_builtins(&db).await?;

    info!("Database initialized successfully");
    Ok(db)
}

/// Seed built-in agent templates and role presets
async fn seed_builtins(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
    // Built-in agent templates (idempotent upserts)
    db.query(
        "CREATE agent_template SET name = 'claude-code', command = 'claude', \
         default_args = ['--print', '--output-format', 'json'], \
         input_mode = 'flag_message', output_mode = 'json_stream', \
         resume_support = true, builtin = true \
         ON DUPLICATE KEY UPDATE command = 'claude'"
    ).await?;

    db.query(
        "CREATE agent_template SET name = 'codex', command = 'codex', \
         default_args = [], \
         input_mode = 'flag_message', output_mode = 'text_markers', \
         resume_support = false, builtin = true \
         ON DUPLICATE KEY UPDATE command = 'codex'"
    ).await?;

    db.query(
        "CREATE agent_template SET name = 'gemini-cli', command = 'gemini', \
         default_args = [], \
         input_mode = 'pty_stdin', output_mode = 'raw_pty', \
         resume_support = false, builtin = true \
         ON DUPLICATE KEY UPDATE command = 'gemini'"
    ).await?;

    db.query(
        "CREATE agent_template SET name = 'aider', command = 'aider', \
         default_args = ['--no-auto-commits'], \
         input_mode = 'pty_stdin', output_mode = 'text_markers', \
         resume_support = false, builtin = true \
         ON DUPLICATE KEY UPDATE command = 'aider'"
    ).await?;

    // Built-in role presets
    db.query(
        "CREATE role_preset SET name = 'implementer', \
         system_prompt = 'You are a senior software engineer. Implement the task with clean, tested code.', \
         description = 'General implementation role', \
         injection_method = 'flag', builtin = true \
         ON DUPLICATE KEY UPDATE description = 'General implementation role'"
    ).await?;

    db.query(
        "CREATE role_preset SET name = 'reviewer', \
         system_prompt = 'You are a code reviewer. Review the code for bugs, security issues, and style.', \
         description = 'Code review role', \
         injection_method = 'flag', builtin = true \
         ON DUPLICATE KEY UPDATE description = 'Code review role'"
    ).await?;

    db.query(
        "CREATE role_preset SET name = 'architect', \
         system_prompt = 'You are a software architect. Design systems with clear boundaries and interfaces.', \
         description = 'Architecture and design role', \
         injection_method = 'flag', builtin = true \
         ON DUPLICATE KEY UPDATE description = 'Architecture and design role'"
    ).await?;

    Ok(())
}
