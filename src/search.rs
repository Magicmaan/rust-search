use std::sync::MutexGuard;

use libsql::{params, Connection, Result};

use crate::FileEntry;

pub async fn search_files(query: &str, conn: MutexGuard<'_, Connection>) -> Result<()> {
    println!("🔍 LIKE Search results for '{}':", query);

    // Use LIKE search for better performance
    search_with_like(query, &conn).await?;

    // Print the number of rows in the 'files' table
    let mut stmt = conn.prepare("SELECT COUNT(*) FROM files").await?;
    let mut rows = stmt.query(()).await?;
    if let Some(row) = rows.next().await? {
        let count: i64 = row.get(0)?;
        println!("📊 Total files in database: {}", count);
    }

    Ok(())
}

// Function using LIKE search
async fn search_with_like(query: &str, conn: &Connection) -> Result<()> {
    let search_pattern = format!("%{}%", query);

    // select 50 results from db where filename matches the search pattern
    let mut stmt = conn
        .prepare(
            "
            SELECT path, filename 
            FROM files 
            WHERE filename 
            LIKE ?1 
            LIMIT 50",
        )
        .await?;

    let mut rows = stmt.query([search_pattern.as_str()]).await?;

    let mut count = 0;
    while let Some(row) = rows.next().await? {
        let path: String = row.get(0)?;
        let filename: String = row.get(1)?;

        println!("📁 {}", filename);
        println!("   📂 {}", path);
        count += 1;
    }

    // debug check if any files were found
    if count == 0 {
        println!("❌ No files found matching '{}'", query);

        // Debug: check if database has any files
        let mut debug_stmt = conn.prepare("SELECT COUNT(*) FROM files").await?;
        let mut debug_rows = debug_stmt.query(()).await?;

        if let Some(row) = debug_rows.next().await? {
            let file_count: i64 = row.get(0)?;
            println!("📊 Database contains {} files total", file_count);

            // Show some sample files
            let mut sample_stmt = conn.prepare("SELECT filename FROM files LIMIT 5").await?;
            let mut sample_rows = sample_stmt.query(()).await?;

            println!("📋 Sample files in database:");
            while let Some(row) = sample_rows.next().await? {
                let filename: String = row.get(0)?;
                println!("  - {}", filename);
            }
        }
    } else {
        println!("✅ LIKE search found {} matching file(s)", count);
    }

    Ok(())
}
