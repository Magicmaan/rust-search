use std::{
    io::{self, stdin},
    sync::{Arc, Mutex, MutexGuard},
    thread,
};

use diesel::row;
use jwalk::WalkDirGeneric;
use libsql::{params, Connection, Result as LibSqlResult};
use search::FileEntry;
use tokio;

mod database;
mod search;
use crate::database::create_schema;

fn get_directories() -> Result<WalkDirGeneric<(usize, bool)>, std::io::Error> {
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

// async fn _main() {
//     println!("Hello, world!");

//     // Initalise the main database connection for the application
//     // must be same object as the one used in the search module to avoid some collision or whatever its called.
//     let db = libsql::Builder::new_local("search.db")
//         .build()
//         .await
//         .expect("Failed to build database");

//     // Use an Arc<Mutex<Connection>> to share the connection across threads
//     // this is to allow the indexing to run in the background, incrementally adding the files to the database
//     //
//     // while the main thread is free to accept user input and perform searches on the partially indexed database
//     // this lets users search while the database is being indexed

//     // but why not just use a single connection?
//     // because indexing is slow, and users don't usually need the full indexed database to perform searches
//     // so we can use a separate connection for indexing and another for searching
//     // this is purely for startup performance and user experience

//     // conn_raw shouldn't be used directly, it should be wrapped in an Arc<Mutex<Connection>>
//     // to allow multiple threads to access it safely
//     let conn_raw = db.connect().expect("Failed to connect to database");
//     // have one connection for the main thread
//     // and one for the worker thread that will insert files into the database
//     // this is to avoid deadlocks and allow the main thread to continue accepting user input
//     let conn = Arc::new(Mutex::new(conn_raw.clone()));

//     let mut should_index = true; // Change this to false to skip indexing

//     use std::path::Path;
//     let db_path = "search.db";
//     if Path::new(db_path).exists() {
//         println!("Database '{}' already exists. Overwrite? (y/N): ", db_path);
//         let mut answer = String::new();
//         stdin().read_line(&mut answer).expect("Failed to read line");
//         if answer.trim().eq_ignore_ascii_case("y") {
//             should_index = true;
//         } else {
//             should_index = false;
//         }
//     }

//     if should_index {
//         println!("Starting indexing of files...");
//         let conn_worker = conn.clone();

//         // create the database
//         create_schema(conn_raw.clone(), Some(true))
//             .await
//             .expect("Failed to create database");

//         index_directories(
//             get_directories().expect("Failed to get directories"),
//             conn_worker,
//         )
//         .await;
//     } else {
//         println!("Skipping indexing of files.");
//     }

//     let mut query_formatter = search::SearchQuery::new();

//     loop {
//         print!("> ");

//         let mut input = String::new();
//         println!("\nEnter search query:");
//         stdin().read_line(&mut input).expect("Failed to read line");

//         if input == "exit" {
//             break;
//         }

//         // Lock connection exclusively, pauses indexing
//         if let Ok(mut conn) = conn.lock() {
//             println!("Locked connection successfully.");

//             let query = query_formatter.make_query(input.trim());

//             let res = search_files_timed(&query, conn)
//                 .await
//                 .expect("Failed to perform search");

//             if res.is_empty() {
//                 println!("No results found for query: {}", input.trim());
//             } else {
//                 for entry in &res {
//                     println!(
//                         "Path: {}\nFilename: {}\nExtension: {}\nSize: {}\nModified At: {}\n",
//                         entry.path, entry.filename, entry.extension, entry.size, entry.modified_at
//                     );
//                 }
//             }
//         } else {
//             println!("[Main] Could not acquire DB connection.");
//         }
//     }

//     // perform_search("config")
//     //     .await
//     //     .expect("Failed to perform search");

//     // use std::io::stdin;

//     // loop {
//     //     // Prompt user for input
//     //     let mut input = String::new();
//     //     println!("\nEnter search query:");
//     //     stdin().read_line(&mut input).expect("Failed to read line");

//     //     print!("Searching for: {}", input.trim());

//     //     if input.trim().is_empty() {
//     //         println!("❌ Empty search query, please try again.");
//     //         continue;
//     //     }
//     //     if input.trim() == "q" {
//     //         println!("Exiting search...");
//     //         break;
//     //     }

//     //     // Perform search
//     //     if let Err(e) = perform_search(input.trim()).await {
//     //         eprintln!("❌ Error during search: {}", e);
//     //     } else {
//     //         println!("✅ Search completed successfully.");
//     //     }
//     // }
// }

use rustsearch::SearchEngine;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Rust Search Engine");

    // Create search engine
    let engine = SearchEngine::new("search.db", Some(true)).await?;
    engine.start_watcher();

    // Check if database exists
    let mut should_index = true;
    let db_path = "search.db";

    if Path::new(db_path).exists() {
        println!("Database '{}' already exists. Overwrite? (y/N): ", db_path);
        let mut answer = String::new();
        stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            should_index = false;
        }
    }

    // Index files if needed
    if should_index {
        println!("Starting indexing of files...");
        engine.index_directories().await;
    } else {
        println!("Skipping indexing of files.");
    }

    // Command loop
    loop {
        println!("\nEnter search query (or 'exit' to quit):");
        let mut input = String::new();
        stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" {
            break;
        }

        // Search files
        let results = engine.search_files(input).await?;

        if results.is_empty() {
            println!("No results found for query: {}", input);
            continue;
        }

        println!("Found {} results:", results.len());
        for entry in &results {
            println!(
                "Path: {}\nFilename: {}\nExtension: {}\nSize: {}\nModified At: {}\n",
                entry.path, entry.filename, entry.extension, entry.size, entry.modified_at
            );
        }
    }

    Ok(())
}
