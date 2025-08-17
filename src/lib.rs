// //! # Rust Search Library
// //!
// //! A fast file search library that indexes files and provides search functionality
// //! using SQLite with FTS (Full-Text Search) capabilities.
// //!
// //! ## Features
// //!
// //! - Fast file system traversal with `jwalk`
// //! - SQLite-based storage with FTS support
// //! - Efficient batch insertions
// //! - LIKE-based search with pattern matching
// //! - Configurable directory skipping (node_modules, target, etc.)
// //!
// //! ## Example
// //!
// //! ```rust,no_run
// //! use rust_search::{SearchEngine, FileEntry};
// //!
// //! #[tokio::main]
// //! async fn main() -> Result<(), Box<dyn std::error::Error>> {
// //!     // Create a new search engine
// //!     let mut search_engine = SearchEngine::new("search.db").await?;
// //!
// //!     // Index files from a directory
// //!     search_engine.index_directory("/home/user/documents").await?;
// //!
// //!     // Search for files
// //!     let results = search_engine.search("config").await?;
// //!
// //!     for file in results {
// //!         println!("Found: {} at {}", file.filename, file.path);
// //!     }
// //!
// //!     Ok(())
// //! }
// //! ```

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use jwalk::{WalkDir, WalkDirGeneric};
use libsql::{params, Connection, Database, Result as SQLResult};

use crate::{config::get_config, database::create_schema, search::FileEntry};

mod config;
mod database;
mod search;
mod tests;

pub struct SearchEngine {
    database: Database,
    connection: Arc<Mutex<Connection>>,
    query_queue: Vec<String>,
    config: config::Config,
    debug: bool,
}
impl SearchEngine {
    pub async fn new(db_path: &str, debug: Option<bool>) -> Result<Self, std::io::Error> {
        let config = get_config();
        println!("Using config: {:?}", config);

        let database = libsql::Builder::new_local("search.db")
            .build()
            .await
            .expect("Failed to build database");

        // Use an Arc<Mutex<Connection>> to share the connection across threads
        // this is to allow the indexing to run in the background, incrementally adding the files to the database
        //
        // while the main thread is free to accept user input and perform searches on the partially indexed database
        // this lets users search while the database is being indexed

        // but why not just use a single connection?
        // because indexing is slow, and users don't usually need the full indexed database to perform searches
        // so we can use a separate connection for indexing and another for searching
        // this is purely for startup performance and user experience

        // conn_raw shouldn't be used directly, it should be wrapped in an Arc<Mutex<Connection>>
        // to allow multiple threads to access it safely
        let conn_raw = database.connect().expect("Failed to connect to database");
        create_schema(conn_raw.clone(), None)
            .await
            .expect("Failed to create schema");
        // have one connection for the main thread
        // and one for the worker thread that will insert files into the database
        // this is to avoid deadlocks and allow the main thread to continue accepting user input
        let connection = Arc::new(Mutex::new(conn_raw.clone()));

        Ok(Self {
            database,
            connection,
            query_queue: Vec::new(),
            config,
            debug: debug.unwrap_or(false),
        })
    }

    pub fn start_watcher(&self) {
        // Spawn a thread to run the file watcher so it doesn't block the main thread
        std::thread::spawn(move || {
            use notify::{Event, RecursiveMode, Result, Watcher};
            use std::{path::Path, sync::mpsc};

            let (tx, rx) = mpsc::channel::<Result<Event>>();

            // Use recommended_watcher() to automatically select the best implementation
            // for your platform. The `EventHandler` passed to this constructor can be a
            // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
            // another type the trait is implemented for.
            // Use polling-based watcher with higher interval to reduce overhead
            let mut watcher = notify::PollWatcher::new(
                tx,
                notify::Config::default().with_poll_interval(Duration::from_secs(30)),
            )
            .unwrap();
            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            watcher
                .watch(Path::new("/home/theo"), RecursiveMode::Recursive)
                .unwrap();
            // Block forever, printing out events as they come in
            for res in rx {
                match res {
                    Ok(event) => match event.kind {
                        // notify::EventKind::Create(_) => {
                        //     println!("File created: {:?}", event.paths);
                        // }
                        // notify::EventKind::Modify(_) => {
                        //     println!("File modified: {:?}", event.paths);
                        // }
                        // notify::EventKind::Remove(_) => {
                        //     println!("File removed: {:?}", event.paths);
                        // }
                        _ => {}
                    },
                    Err(_e) => {
                        // Ignore error
                    }
                }
            }
        });
    }

    pub async fn index_directories(&self) {
        let conn_worker = self.connection.clone();
        let config = self.config.clone();
        let directories = get_directories(&config).expect("Failed to run search");

        tokio::task::spawn_blocking(move || {
            // this technically doesn't need to be async, but it just makes it easier to work with
            // as the search function is async and we can use await on it

            // Use a synchronous block to avoid holding MutexGuard across await
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

            rt.block_on(async {
                database::insert_files_to_db(directories, conn_worker)
                    .await
                    .expect("Failed to insert files into database");
            });
        });
    }

    // Add implementation for the search method to use search::search_files under the hood
    pub async fn search_files(&self, query: &str) -> SQLResult<Vec<FileEntry>> {
        // Lock connection exclusively, pauses indexing
        if let Ok(mut conn) = self.connection.lock() {
            println!("Locked connection successfully.");
            let mut query_formatter = search::SearchQuery::new();
            let query = query_formatter.make_query(query.trim());

            let res = search::search_files(&query, conn)
                .await
                .expect("Failed to perform search");

            if res.is_empty() {
                println!("No results found for query: {}", query.trim());
                return Ok(vec![]);
            } else {
                Ok(res)
            }
        } else {
            println!("[Main] Could not acquire DB connection.");
            return Err(libsql::Error::ConnectionFailed(
                "Failed to acquire database connection".into(),
            ));
        }
    }
}

fn get_directories(
    config: &config::Config,
) -> Result<WalkDirGeneric<(usize, bool)>, std::io::Error> {
    println!("Searching for files in home directory... ");
    let now = std::time::Instant::now();

    // Clone the skip_directories so it can be moved into the closure
    let skip_directories = config.skip_directories.clone();
    let skip_extensions = config.skip_extensions.clone();
    let skip_patterns = config.skip_patterns.clone();
    let force_include = config.force_include.clone();

    let walk_dir =
        WalkDirGeneric::<(usize, bool)>::new("/".to_string())
            .process_read_dir(move |_depth, _path, _read_dir_state, children| {
                // 3. Custom skip
                let skip_directories = &skip_directories;
                children.iter_mut().for_each(|dir_entry_result| {
                    if let Ok(dir_entry) = dir_entry_result {
                        // include force include patterns
                        if force_include.iter().any(|pattern| {
                            dir_entry.file_name().to_string_lossy().contains(pattern)
                        }) {
                            print!(
                                "Forcing include for file: {}\n",
                                dir_entry.file_name().to_string_lossy()
                            );
                            // Do not skip this entry, but continue to check others
                        } else {
                            // Only skip directories if they match skip_directories
                            if dir_entry.file_type().is_dir()
                                && skip_directories.iter().any(|dir| {
                                    dir_entry.file_name().to_string_lossy().contains(dir)
                                })
                            {
                                dir_entry.read_children_path = None;
                                print!(
                                    "Skipping directory: {}\n",
                                    dir_entry.file_name().to_string_lossy()
                                );
                            }

                            // Only skip files if they match skip_extensions or skip_patterns
                            if dir_entry.file_type().is_file() {
                                if skip_extensions.iter().any(|ext| {
                                    dir_entry.file_name().to_string_lossy().ends_with(ext)
                                }) {
                                    dir_entry.read_children_path = None;
                                    print!(
                                        "Skipping file with extension: {}\n",
                                        dir_entry.file_name().to_string_lossy()
                                    );
                                } else if skip_patterns.iter().any(|pattern| {
                                    dir_entry.file_name().to_string_lossy().contains(pattern)
                                }) {
                                    dir_entry.read_children_path = None;
                                    print!(
                                        "Skipping file matching pattern: {}\n",
                                        dir_entry.file_name().to_string_lossy()
                                    );
                                }
                            }
                        }
                    }
                });
            })
            .skip_hidden(false);

    let elapsed = now.elapsed();
    println!("Search completed in: {:.10?}", elapsed);
    // print!("Search completed. ");
    // Implement search logic here

    Ok(walk_dir)
}
