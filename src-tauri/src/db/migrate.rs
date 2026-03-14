use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tracing::info;

/// Embedded migration files
const MIGRATIONS: &[(&str, &str)] = &[
    ("001_initial_schema", include_str!("migrations/001_initial_schema.surql")),
];

#[derive(serde::Deserialize)]
struct MigrationRecord {
    version: i64,
}

/// Run all pending migrations
pub async fn run_migrations(db: &Surreal<Db>) -> Result<(), Box<dyn std::error::Error>> {
    // Get current migration version by selecting all and finding max in Rust
    // (avoids SurrealDB math::max quirks with empty/single-element sets)
    let mut response = db
        .query("SELECT version FROM migration ORDER BY version DESC LIMIT 1")
        .await?;
    let result: Vec<MigrationRecord> = response.take(0)?;
    let current_version = result.first().map(|r| r.version).unwrap_or(0);

    info!("Current migration version: {}", current_version);

    for (i, (name, sql)) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version > current_version {
            info!("Applying migration {}: {}", version, name);

            // SurrealDB executes multi-statement queries atomically per query call
            db.query(*sql).await?;

            // Record migration
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

    async fn test_db() -> Surreal<Db> {
        let db: Surreal<Db> = Surreal::new::<Mem>(()).await.unwrap();
        db.use_ns("test").use_db("test").await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_run_migrations() {
        let db = test_db().await;

        run_migrations(&db).await.unwrap();

        // Verify migration was recorded
        let mut response = db
            .query("SELECT * FROM migration")
            .await
            .unwrap();
        let result: Vec<MigrationRecord> = response.take(0).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].version, 1);

        // Verify tables exist by inserting a repo
        db.query("CREATE repo SET path = '/test', name = 'test'")
            .await
            .unwrap();

        // Query and check count via a simple count approach
        let mut response = db
            .query("SELECT count() AS total FROM repo GROUP ALL")
            .await
            .unwrap();

        #[derive(serde::Deserialize)]
        struct CountResult { total: i64 }
        let counts: Vec<CountResult> = response.take(0).unwrap();
        assert_eq!(counts.first().map(|c| c.total).unwrap_or(0), 1);
    }

    #[tokio::test]
    async fn test_migrations_are_idempotent() {
        let db = test_db().await;

        // Run twice — should not error
        run_migrations(&db).await.unwrap();
        run_migrations(&db).await.unwrap();

        let mut response = db
            .query("SELECT * FROM migration")
            .await
            .unwrap();
        let result: Vec<MigrationRecord> = response.take(0).unwrap();
        assert_eq!(result.len(), 1);
    }
}
