use std::sync::MutexGuard;

use crate::database::get_database_count;
use libsql::{params, Connection, Database, Result as SQLResult};

#[derive(Debug, Clone)]

pub struct FileEntry {
    pub path: String,
    pub filename: String,
    pub extension: String,
    pub size: u64,
    pub modified_at: i64,
}

pub struct SearchQuery {
    original_query: String,
    query: String,
    operations: Vec<String>,
}
impl SearchQuery {
    pub fn new() -> Self {
        Self {
            original_query: String::new(),
            query: String::new(),
            operations: Vec::new(),
        }
    }

    pub fn replace_characters(&self, query: &str) -> String {
        query
            .to_string()
            .replace('"', "")
            // .replace('\\', "")
            .replace('?', "")
            .replace('(', "")
            .replace(')', "")
            .replace('[', "")
            .replace(']', "")
            .replace('{', "")
            .replace('}', "")
            .replace(';', "")
            .replace('!', "")
            .replace('@', "")
            .replace('#', "")
            .replace('$', "")
            .replace('&', "")
            .replace('|', "")
            .replace('<', "")
            .replace('>', "")
            .replace('=', "")
            .replace('/', " + ") // Replace slashes with plus for FTS5
    }

    pub fn make_query(&mut self, query: &str) -> String {
        // self.original_query = query.to_string();
        // self.query = query.to_string();
        // self.operations.clear();

        // Replace characters and return the modified query
        // self.replace_characters(query)
        query.to_string()
    }
    pub fn get_original_query(&self) -> &str {
        &self.original_query
    }
}

pub async fn search_files(
    query: &str,
    conn: MutexGuard<'_, Connection>,
) -> SQLResult<Vec<FileEntry>> {
    if query.starts_with("LIKE") {
        // If the query starts with "LIKE", we can skip FTS5 search
        let search_pattern = query.replace("LIKE", "").trim().to_string();
        println!(
            "Skipping FTS5 search, using LIKE search for pattern: {}",
            search_pattern
        );
        return search_normal(&search_pattern, &conn).await;
    }

    let search_pattern = query.to_string();

    let result = match search_fts5(&search_pattern, &conn).await {
        Ok(res) => res,

        // if fts5 search fails, fall back to normal LIKE search
        // this is a fallback to ensure that the search always works
        Err(e) => {
            eprintln!("FTS5 search failed: {}", e);
            eprintln!("Falling back to LIKE search...");
            match search_normal(&query, &conn).await {
                Ok(res) => res,
                Err(e2) => {
                    eprintln!("LIKE search also failed: {}", e2);
                    return Err(e2);
                }
            }
        }
    };

    let db_count = get_database_count(&conn).await;
    if let Err(e) = db_count {
        eprintln!("Failed to get database count: {}", e);
    } else {
        println!("Total files in database: {}", db_count.unwrap());
    }

    Ok(result)
}

// home/theo godot = home + theo godot
// matches to include home with theo close by, and godot somewhere in the path

// ^godot*
// matches to have godot at start

// col : query
// matches to have query in the column 'col'
// use - to exclude a column e.g. -col

// NOT to exclude

// Try FTS5 search - can fail gracefully

//**------------------------------------------------------------------------
//*
//*  Internal search functions, takes directly from the database
//*
//*------------------------------------------------------------------------**/
pub async fn search_fts5(search_pattern: &str, conn: &Connection) -> SQLResult<Vec<FileEntry>> {
    print!("Searching FTS5 for pattern: {}\n", search_pattern);
    let mut stmt = conn
        .prepare("SELECT path, filename FROM files_fts WHERE files_fts MATCH ?1 LIMIT 50")
        .await?;

    let mut rows = match stmt.query([search_pattern]).await {
        Ok(rows) => rows,
        Err(_) => return Ok(vec![]),
    };
    let mut count = 0;
    let mut entries: Vec<FileEntry> = Vec::new();

    while let Some(row) = rows.next().await? {
        let path: String = row.get(0)?;
        let filename: String = row.get(1)?;
        count += 1;

        entries.push(FileEntry {
            path,
            filename,
            extension: "test".to_string(), // Placeholder for extension
            size: 1,                       // Placeholder for size
            modified_at: 1,                // Placeholder for modified
        });
    }

    Ok(entries)
}

// Fallback LIKE search - always works
pub async fn search_normal(query: &str, conn: &Connection) -> SQLResult<Vec<FileEntry>> {
    let search_pattern = format!("%{}%", query);

    let mut stmt = conn
        .prepare("SELECT path, filename FROM files WHERE filename LIKE ?1 OR path LIKE ?1 LIMIT 50")
        .await?;

    let mut rows = stmt.query([search_pattern]).await?;
    let mut count = 0;
    let mut entries: Vec<FileEntry> = Vec::new();

    while let Some(row) = rows.next().await? {
        let path: String = row.get(0)?;
        let filename: String = row.get(1)?;

        entries.push(FileEntry {
            path,
            filename,
            extension: "test".to_string(), // Placeholder for extension
            size: 1,
            modified_at: 1,
        });

        count += 1;
    }

    Ok(entries)
}
