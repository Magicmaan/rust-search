use std::{
    io::{self, stdin},
    sync::{Arc, Mutex, MutexGuard},
    thread,
};

use diesel::row;
use jwalk::WalkDirGeneric;
use libsql::{params, Connection, Result as LibSqlResult};
use tokio;

use crate::database::create_database;
mod database;
mod search;

#[derive(Debug)]
struct FileEntry {
    id: i32,
    path: String,
    filename: String,
    extension: String,
    size: u64,
    modified_at: String,
}

fn run_search() -> Result<WalkDirGeneric<(usize, bool)>, std::io::Error> {
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

async fn insert_files_to_db(
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
    }
    println!("Database insertion completed successfully.");

    Ok(())
}

// wrapper function for debugging and testing
// can just call search_files directly
async fn perform_search(
    query: &str,
    conn: MutexGuard<'_, Connection>,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;
    let now = Instant::now();
    search::search_files(query, conn).await?;
    let elapsed = now.elapsed();
    println!("Search completed in: {:.10?}", elapsed);
    Ok(())
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // create the database
    create_database(Some(true))
        .await
        .expect("Failed to create database");

    // Initalise the main database connection for the application
    // must be same object as the one used in the search module to avoid some collision or whatever its called.
    let db = libsql::Builder::new_local("search.db")
        .build()
        .await
        .expect("Failed to build database");

    // Use an Arc<Mutex<Connection>> to share the connection across threads
    // this is to allow the indexxing to run in the background, incrementally adding the files to the database
    //
    // while the main thread is free to accept user input and perform searches
    // this lets users search while the database is being indexed
    // this is a very simple way to do it, but it works for now.
    let conn_raw = db.connect().expect("Failed to connect to database");

    // have one connection for the main thread
    // and one for the worker thread that will insert files into the database
    // this is to avoid deadlocks and allow the main thread to continue accepting user input
    let conn = Arc::new(Mutex::new(conn_raw));
    let conn_worker = conn.clone();

    // spawn a background thread to run and index the files
    // this is to allow the main thread to continue accepting user input
    // and perform searches while the files are being indexed
    tokio::task::spawn_blocking(move || {
        // this technically doesn't need to be async, but it just makes it easier to work with
        // as the search function is async and we can use await on it
        let search_result = run_search().expect("Failed to run search");
        // Use a synchronous block to avoid holding MutexGuard across await
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

        rt.block_on(async {
            insert_files_to_db(search_result, conn_worker)
                .await
                .expect("Failed to insert files into database");
        });
    });

    loop {
        print!("> ");

        let mut input = String::new();
        println!("\nEnter search query:");
        stdin().read_line(&mut input).expect("Failed to read line");

        if input == "exit" {
            break;
        }

        // Lock connection exclusively, pauses indexing
        if let Ok(mut conn) = conn.lock() {
            println!("Locked connection successfully.");
            perform_search(input.trim(), conn)
                .await
                .expect("Failed to perform search");
        } else {
            println!("[Main] Could not acquire DB connection.");
        }
    }

    // perform_search("config")
    //     .await
    //     .expect("Failed to perform search");

    // use std::io::stdin;

    // loop {
    //     // Prompt user for input
    //     let mut input = String::new();
    //     println!("\nEnter search query:");
    //     stdin().read_line(&mut input).expect("Failed to read line");

    //     print!("Searching for: {}", input.trim());

    //     if input.trim().is_empty() {
    //         println!("❌ Empty search query, please try again.");
    //         continue;
    //     }
    //     if input.trim() == "q" {
    //         println!("Exiting search...");
    //         break;
    //     }

    //     // Perform search
    //     if let Err(e) = perform_search(input.trim()).await {
    //         eprintln!("❌ Error during search: {}", e);
    //     } else {
    //         println!("✅ Search completed successfully.");
    //     }
    // }
}
