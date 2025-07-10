use jwalk::WalkDirGeneric;

use rusqlite::{Connection, Result, vtab};
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

fn create_db() -> Result<()> {
    let conn = Connection::open("search.db")?;

    conn.execute("DROP TABLE IF EXISTS files_fts", [])?;
    conn.execute("DROP TABLE IF EXISTS files", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id          INTEGER PRIMARY KEY,
            path        TEXT NOT NULL,
            filename    TEXT NOT NULL,
            extension   TEXT,
            size        INTEGER NOT NULL,
            modified_at TEXT NOT NULL,
            UNIQUE(path)
        )",
        [],
    )?;


    conn.execute("CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
        row,
        path,
        filename,
        extension,
        tokenize='trigram',
        prefix='2,3'
    )", [])?;
    println!("Database created successfully.");
    // Implement database creation logic here
    // This is a placeholder function

    conn.close().expect("Failed to close the database connection");
    Ok(())
}



fn run_search() -> Result<WalkDirGeneric<(usize, bool)>, std::io::Error> {
    let count = 0;
    

    
    let walk_dir = WalkDirGeneric::<(usize,bool)>::new(
        std::env::var("HOME").unwrap_or_else(|_| "/home".to_string()), )
        .process_read_dir(|_depth, _path, _read_dir_state, children| {
            // 3. Custom skip
            children.iter_mut().for_each(|dir_entry_result| {
                if let Ok(dir_entry) = dir_entry_result {
                    if dir_entry.file_name() == "AppData"  {
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
                    if dir_entry.depth == 7 {
                        dir_entry.read_children_path = None;
                    }

                    
                }
            });
        }).skip_hidden(false);
        println!("Total entries: {}", count);
        // Implement search logic here
        Ok(walk_dir)
}

fn insert_files_to_db(search_result: WalkDirGeneric<(usize, bool)>) -> Result<()> {
    let mut con = Connection::open("search.db")?;
    let tx = con.transaction()?;

    {
        let mut stmt = tx.prepare("
            INSERT OR IGNORE INTO files (
                path, 
                filename, 
                extension, 
                size, 
                modified_at 
            ) VALUES (
                ?1, 
                ?2, 
                ?3, 
                ?4,
                ?5
            )
        ")?;

        let mut fts_stmt = tx.prepare("
            INSERT OR IGNORE INTO files_fts (
                row, 
                path, 
                filename, 
                extension 
            ) VALUES (
                ?1, 
                ?2, 
                ?3,
                ?4
            )
        ")?;

        for entry in search_result {
            match entry {
                Ok(dir_entry) => {
                    let path_str = dir_entry.path().display().to_string();
                    let filename = dir_entry.file_name().to_string_lossy().to_string();
                    let extension = dir_entry.path().extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_string();

                    // Insert into main table
                    stmt.execute(rusqlite::params![&path_str, &filename, &extension, 1, "1"])?;
                    
                    // Get the row ID (either newly inserted or existing)
                    let row_id = if tx.changes() > 0 {
                        // New row was inserted
                        tx.last_insert_rowid()
                    } else {
                        // Row already existed, find its ID
                        let mut id_stmt = tx.prepare("SELECT id FROM files WHERE path = ?1")?;
                        id_stmt.query_row([&path_str], |row| row.get::<_, i64>(0))?
                    };

                    // Insert into FTS table
                    fts_stmt.execute(rusqlite::params![row_id, &path_str, &filename, &extension])?;
                },
                Err(e) => eprintln!("Error reading directory entry: {}", e),
            }
        }
    }

    tx.commit()?;
    con.close().expect("Failed to close database connection");
    Ok(())
}

fn perform_search(query: &str) -> Result<()> {
    use std::time::Instant;
    let now = Instant::now();
    search::search_files(query)?;
    let elapsed = now.elapsed();
    println!("Search completed in: {:.10?}", elapsed);
    Ok(())
}

fn main() {
    println!("Hello, world!");
    create_db().expect("Failed to create database");

    let search_result = run_search().expect("Failed to run search");
    insert_files_to_db(search_result).expect("Failed to insert files to database");
    // perform_search("").expect("Failed to perform search");

    use std::io::{stdin, stdout, Write};

    loop {// Prompt user for input
        let mut input = String::new();
        println!("\nEnter search query:");
        stdin().read_line(&mut input).expect("Failed to read line");

        print!("Searching for: {}", input.trim());

        if input.trim().is_empty() {
            println!("❌ Empty search query, please try again.");
            continue;
        }
        if input.trim() == "q" {
            println!("Exiting search...");
            break;
        }

        // Perform search
        if let Err(e) = perform_search(input.trim()) {
            eprintln!("❌ Error during search: {}", e);
        } else {
            println!("✅ Search completed successfully.");
        }
    }
}

