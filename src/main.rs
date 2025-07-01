use jwalk::{WalkDir, WalkDirGeneric, DirEntry};
use std::cmp::Ordering;

use rusqlite::{Connection, Result};

#[derive(Debug)]
struct FileEntry {
    id: i32,
    path: String,
    size: u64,
    modified_at: String,
}

fn create_db() -> Result<()> {
    let conn = Connection::open("search.db")?;

    conn.execute("create table if not exists files
                (id integer primary key, path text not null, size integer not null, modified_at text not null)
    ", [])?;
    println!("Database created successfully.");
    // Implement database creation logic here
    // This is a placeholder function

    conn.close().expect("Failed to close the database connection");
    Ok(())
}

fn add_quasi_data() -> Result<()> {
    let conn = Connection::open("search.db")?;
    let mut stmt = conn.prepare("INSERT INTO files (path, size, modified_at) VALUES (?1, ?2, ?3)")?;

    // Example data
    let entries = vec![
        FileEntry { id: 1, path: "C:\\example\\file1.txt".to_string(), size: 1024, modified_at: "2023-10-01T12:00:00Z".to_string() },
        FileEntry { id: 2, path: "C:\\example\\file2.txt".to_string(), size: 2048, modified_at: "2023-10-02T12:00:00Z".to_string() },
    ];
    

    for entry in entries {
        stmt.execute(rusqlite::params![entry.path, entry.size, entry.modified_at])?;
    }

    println!("Data added successfully.");
    Ok(())
}

fn print_db() -> Result<()> {
    let conn = Connection::open("search.db")?;
    let mut stmt = conn.prepare("SELECT id, path, size, modified_at FROM files")?;
    let file_iter = stmt.query_map([], |row| {
        Ok(FileEntry {
            id: row.get(0)?,
            path: row.get(1)?,
            size: row.get(2)?,
            modified_at: row.get(3)?,
        })
    })?;

    for file in file_iter {
        match file {
            Ok(file_entry) => println!("ID: {}, Path: {}, Size: {}, Modified At: {}", file_entry.id, file_entry.path, file_entry.size, file_entry.modified_at),
            Err(e) => eprintln!("Error reading file entry: {}", e),
        }
    }
    Ok(())
}

fn run_search() -> Result<WalkDirGeneric<(usize, bool)>, std::io::Error> {
      let mut count = 0;
    
let walk_dir = WalkDirGeneric::<((usize),(bool))>::new("C:\\")
    .process_read_dir(|depth, path, read_dir_state, children| {
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
                if dir_entry.depth == 5 {
                    dir_entry.read_children_path = None;
                }
            }
        });
    });
    // println!("{}",walk_dir.try_into_iter().iter().count());
    // for entry in walk_dir {
    //     match entry {
    //         Ok(dir_entry) => {
    //             count += 1;
    //         },
    //         Err(e) => eprintln!("Error: {}", e),
    //     }
    // }
    println!("Total entries: {}", count);
    // Implement search logic here
    Ok(walk_dir)
}

fn main() {
    println!("Hello, world!");
    create_db().expect("Failed to create database");
    add_quasi_data().expect("Failed to add data to database");
    print_db().expect("Failed to print database");

    let search_result = run_search();

    let 
    match search_result {
        Ok(data) => {
            for entry in data {
                match entry {
                    Ok(dir_entry) => {
                        let file = dir_entry.path().display();
                        



                        // println!("Found: {}", dir_entry.path().display());
                    },
                    Err(e) => eprintln!("Error reading directory entry: {}", e),
                }
            }

        },
        Err(e) => eprintln!("Error during search: {}", e),
    }
    
    
}


