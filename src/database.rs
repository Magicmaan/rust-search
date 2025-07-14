use libsql::{Database, Result};

const SCHEMA: &str = "

    CREATE TABLE IF NOT EXISTS files (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        path TEXT NOT NULL,
        filename TEXT NOT NULL,
        extension TEXT NOT NULL,
        size INTEGER NOT NULL,
        modified_at TEXT NOT NULL
    );
    CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
        path,
        filename,
        extension,
        tokenize='trigram',
        prefix='2,3'
    );

";

pub async fn create_database(reset: Option<bool>) -> Result<()> {
    let db = libsql::Builder::new_local("search.db").build().await?;
    let conn = db.connect()?;

    if reset.unwrap_or(false) {
        conn.execute("DROP TABLE IF EXISTS files_fts", ()).await?;
        conn.execute("DROP TABLE IF EXISTS files", ()).await?;
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS files (
        id          INTEGER PRIMARY KEY,
        path        TEXT NOT NULL,
        filename    TEXT NOT NULL,
        extension   TEXT,
        size        INTEGER NOT NULL,
        modified_at TEXT NOT NULL,
        UNIQUE(path)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(filename, path, extension);


        PRAGMA journal_mode = OFF;
        PRAGMA synchronous = OFF;
        PRAGMA journal_size_limit = 1000000;
        PRAGMA cache_size = 100000;
        PRAGMA temp_store = memory;
        PRAGMA locking_mode = EXCLUSIVE;
        PRAGMA mmap_size = 268435456;
        PRAGMA optimize;
        ",
    )
    .await?;

    println!("Database and FTS table created successfully.");

    Ok(())
}

pub fn insert_files_to_database() -> Result<()> {
    !todo!("Implement file insertion logic here");
}

pub fn query_database(query: &str) -> Result<()> {
    !todo!("Implement database query logic here");
}

fn _query_files(query: &str) -> Result<()> {
    !todo!("Implement file search logic here");
}
fn _query_fts(query: &str) -> Result<()> {
    !todo!("Implement FTS search logic here");
}
