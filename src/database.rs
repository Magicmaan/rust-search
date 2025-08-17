use std::sync::{Arc, Mutex, MutexGuard};

use jwalk::WalkDirGeneric;
use libsql::{params, Connection, Database, Result as SQLResult};

use crate::search;

pub async fn create_schema(connection: Connection, reset: Option<bool>) -> SQLResult<()> {
    if reset.unwrap_or(false) {
        if let Err(e) = connection
            .execute("DROP TABLE IF EXISTS files_fts", ())
            .await
        {
            eprintln!("Warning: Failed to drop files_fts table: {}", e);
        }
        if let Err(e) = connection.execute("DROP TABLE IF EXISTS files", ()).await {
            eprintln!("Warning: Failed to drop files table: {}", e);
        }
    }

    match connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS files (
        id          INTEGER PRIMARY KEY,
        path        TEXT NOT NULL,
        filename    TEXT NOT NULL,
        extension   TEXT,
        size        INTEGER NOT NULL,
        modified_at TEXT NOT NULL,
        UNIQUE(path)
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            filename, 
            path, 
            extension,
            content='files',
            content_rowid='id'
     
        );

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
        .await
    {
        Ok(_) => {
            println!("Database and FTS table created successfully.");
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to create database tables: {}", e);
            Err(e)
        }
    }
}

pub async fn get_database_count(connection: &Connection) -> SQLResult<usize> {
    let mut stmt = connection.prepare("SELECT COUNT(*) FROM files").await?;

    let mut rows = stmt.query(()).await?;
    let count = match rows.next().await? {
        Some(row) => row.get::<i64>(0)? as usize,
        None => 0,
    };

    Ok(count)
}

async fn debug_database_state(conn: &Connection) {
    // Print the number of rows in the 'files' table
    // (You can add debug logic here if needed)
}

//
// FTS search stuff: https://www.sqlite.org/fts5.html#fts5_column_filters
// LIKE search stuff: https://www.sqlitetutorial.net/sqlite-like/
//
// / = +
// so home/theo = home + theo
pub async fn insert_files_to_db(
    search_result: WalkDirGeneric<(usize, bool)>,
    conn_thread: Arc<Mutex<Connection>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let now = std::time::Instant::now();
    println!("Inserting files into database...");
    println!("This may take a while depending on the number of files.");

    // Lock the connection when starting the transaction
    {
        let conn = conn_thread.lock().unwrap();

        conn.execute_batch(
            "PRAGMA journal_mode = OFF;
        PRAGMA synchronous = OFF;
        PRAGMA journal_size_limit = 1000000;
        PRAGMA cache_size = 100000;
        PRAGMA temp_store = memory;
        PRAGMA locking_mode = EXCLUSIVE;
        PRAGMA mmap_size = 268435456;
        PRAGMA optimize;",
        )
        .await?;

        conn.execute("BEGIN TRANSACTION;", params![]).await?;
    }

    let mut count = 0;
    let mut batch_count = 0;
    let batch_size = 500;
    let mut new_query = String::with_capacity(batch_size * 200); // Pre-allocate ~200 chars per record
    new_query.push_str(
        "INSERT OR REPLACE INTO files (path, filename, extension, size, modified_at) VALUES ",
    );
    let mut first = true;

    // for every file in the search result
    // insert it into the database in batches of X
    for entry in search_result {
        if let Ok(dir_entry) = entry {
            if let Ok(metadata) = dir_entry.metadata() {
                if metadata.is_file() {
                    // Get the path, filename, extension, size, and modified_at
                    let path_str = dir_entry.path().display().to_string();
                    let filename = dir_entry.file_name().to_string_lossy().to_string();
                    let extension = dir_entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_string();
                    let size = metadata.len() as i64;

                    // convert the modified time to seconds since UNIX epoch
                    // using the modified time as a fallback if not available
                    let modified_at = metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                        .duration_since(std::time::SystemTime::UNIX_EPOCH) // Change this line
                        .unwrap_or_default()
                        .as_secs() as i64;

                    // Escape the single quotes in path and filename as sqlite requires it
                    // this isn't done automatically as using compound parameters
                    let formatted = format!(
                        "('{}', '{}', '{}', {}, {})",
                        path_str.replace("'", "''"),
                        filename.replace("'", "''"),
                        extension.replace("'", "''"),
                        size,
                        modified_at
                    );

                    // append the values to the query
                    // query should be like:
                    // INSERT INTO files (path, filename, extension, size, modified_at) VALUES (v1), (v2), (v3), ...
                    // where v1, v2, v3 are the values for each file
                    if first {
                        new_query.push_str(&formatted);
                        first = false;
                    } else {
                        new_query.push_str(", ");
                        new_query.push_str(&formatted);
                    }

                    count += 1;
                    batch_count += 1;

                    // insert files in batches of X size
                    // very small batches are slow due to overhead of executing many small queries
                    // large batches don't seem to always work due to memory limits
                    if batch_count == batch_size {
                        // aquire lock again to execute the query
                        // this is to avoid holding the lock for too long
                        let conn = conn_thread.lock().unwrap();
                        // println!("Inserting batch of {} files into database...", batch_count);
                        let _res = conn.execute_batch(&new_query).await;

                        if let Err(e) = _res {
                            eprintln!("Error inserting batch: {}", e);
                            println!("Query: {}", new_query);
                        };

                        // Reset for next batch
                        new_query.clear();

                        new_query.push_str(
                            "INSERT OR REPLACE INTO files (path, filename, extension, size, modified_at) VALUES ",
                        );
                        first = true;
                        batch_count = 0;
                    }
                }
            }
        }
    }

    {
        // Lock the connection again to finalize the insertion
        let mut conn = conn_thread.lock().unwrap();

        // Insert any remaining files in the last batch
        if batch_count > 0 && !first {
            println!(
                "Inserting final batch of {} files into database...",
                batch_count
            );
            let _res = conn.execute_batch(&new_query).await;
        }

        let elapsed = now.elapsed();
        println!(
            "Inserted {} files into database in: {:.10?}",
            count, elapsed
        );

        conn.execute("COMMIT;", params![]).await?;
        conn.execute(
            "INSERT INTO files_fts(files_fts) VALUES('rebuild');",
            params![],
        )
        .await?;
    }
    println!("Database insertion completed successfully.");

    Ok(())
}

pub fn run_search() -> Result<WalkDirGeneric<(usize, bool)>, std::io::Error> {
    println!("Searching for files in home directory... ");
    let now = std::time::Instant::now();
    let walk_dir = WalkDirGeneric::<(usize, bool)>::new("/".to_string())
        .process_read_dir(|_depth, _path, _read_dir_state, children| {
            // 3. Custom skip
            children.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    if dir_entry.file_name() == "AppData" {
                        dir_entry.read_children_path = None;
                    }
                    if dir_entry.file_name() == "node_modules" {
                        dir_entry.read_children_path = None;
                    }
                    if dir_entry.file_name() == "target" {
                        dir_entry.read_children_path = None;
                    }
                    if dir_entry.file_name() == "vendor" {
                        dir_entry.read_children_path = None;
                    }
                    if dir_entry.file_name() == "build" {
                        dir_entry.read_children_path = None;
                    }
                    if dir_entry.file_name() == ".cache" {
                        dir_entry.read_children_path = None;
                    }
                    if dir_entry.depth == 10 {
                        dir_entry.read_children_path = None;
                    }
                }
                // println!("Processing directory: {}", _path.display());
            });
        })
        .skip_hidden(false);

    let elapsed = now.elapsed();
    println!("Search completed in: {:.10?}", elapsed);
    // print!("Search completed. ");
    // Implement search logic here

    Ok(walk_dir)
}
