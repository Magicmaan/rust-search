use jwalk::WalkDirGeneric;

use rusqlite::{Connection, Result};

use crate::FileEntry;


// use rusqlite::{Connection, Result};
pub fn search_db(query: &str, limit: Option<i32>) -> Result<()> {
    let conn = Connection::open("search.db")?;
    // in fts_path, fuzzy text match input in either path or filename
    let mut stmt = conn.prepare("
        SELECT path, filename 
        FROM files_fts 
        WHERE filename 
        MATCH ?1")?;
    
    let file_iter = stmt.query_map([query], |row| {
        Ok(FileEntry{
            id: 0, // Placeholder for ID, not used in this context
            path: row.get::<_, String>(0).unwrap().to_string(), // path
            filename: row.get::<_, String>(1).unwrap().to_string(), // filename
            extension: "".to_string(), // Placeholder for extension, not used in this context
            size: u64::default(), // Placeholder for size, not used in this context
            modified_at: "".to_string(), // Placeholder for size, not used in this context
            })
    })?;
    for file in file_iter {
        match file {
            Ok(file_entry) => {
                println!("📁 {}", file_entry.filename);
                println!("   📂 {}", file_entry.path);
            },
            Err(e) => eprintln!("❌ Error reading search result: {}", e),
        }
    }
    
    Ok(())
}


pub fn search_files(query: &str) -> Result<()> {
    let conn = Connection::open("search.db")?;
    
    // // in fts_path, fuzzy text match input in either path or filename
    // let mut stmt = conn.prepare("SELECT path, filename FROM files_fts WHERE path MATCH ?1 LIMIT 20")?;
    
    // let file_iter = stmt.query_map([query], |row| {
    //     Ok((
    //         row.get::<_, String>(0)?, // path
    //         row.get::<_, String>(1)?, // filename
    //     ))
    // }).expect("Failed to query database");
    
    // for file in file_iter {
    //     match file {
    //         Ok((path, filename)) => {
                
    //             // let line = format!("📁 {}\n   📂 {}\n", filename, path);
                
    //             // print!("{}", line);
                
                
                
    //         },
    //         Err(e) => eprintln!("❌ Error reading search result: {}", e),
    //     }
    // }
  

    // for some reason LIKE search is faster...
    // 1.8ms vs 1.6ms
    search_with_like(query, &conn)?;
   

    
    Ok(())
}

// Fallback function using LIKE search
fn search_with_like(query: &str, conn: &Connection) -> Result<()> {
    let search_pattern = format!("%{}%", query);
    let mut stmt = conn.prepare("SELECT path, filename FROM files WHERE filename LIKE ?1 OR path LIKE ?1 LIMIT 20")?;
    
    let file_iter = stmt.query_map([&search_pattern], |row| {
        Ok((
            row.get::<_, String>(0)?, // path
            row.get::<_, String>(1)?, // filename
        ))
    })?;

    println!("\n🔍 LIKE Search results for '{}':", query);
    let mut count = 0;
    
    for file in file_iter {
        match file {
            Ok((path, filename)) => {
                count += 1;
                println!("📁 {}", filename);
                println!("   📂 {}", path);
            },
            Err(e) => eprintln!("❌ Error: {}", e),
        }
    }
    
    if count == 0 {
        println!("❌ No files found with LIKE search either");
        
        // Debug info
        let mut debug_stmt = conn.prepare("SELECT COUNT(*) FROM files")?;
        let total_count: i64 = debug_stmt.query_row([], |row| row.get(0))?;
        println!("📊 Total files in database: {}", total_count);
        
        if total_count > 0 {
            let mut sample_stmt = conn.prepare("SELECT filename FROM files LIMIT 5")?;
            let sample_iter = sample_stmt.query_map([], |row| {
                Ok(row.get::<_, String>(0)?)
            })?;
            
            println!("📋 Sample files in database:");
            for sample in sample_iter {
                match sample {
                    Ok(filename) => println!("   - {}", filename),
                    Err(_) => break,
                }
            }
        }
    } else {
        println!("✅ LIKE search found {} matching file(s)", count);
    }
    
    Ok(())
}