# Rust Search 🦀🔍

A fast, lightweight file search tool built in Rust with SQLite indexing. Think of it as a cross-platform alternative to Windows "Everything" search tool.

Its quite terrible right now, but hey I'm an idiot. :)

## 🚀 Features

### Current

- **Fast indexing** of your home directory using parallel file traversal
- **SQLite database** with FTS5 full-text search for lightning-fast queries
- **Smart filtering** - skips heavy directories like `node_modules`, `.git`, `target`
- **Cross-platform (ish)** - works on Linux, macOS, and Windows

### Planned

- [ ] File change monitoring with real-time index updates
- [x] Fuzzy search algorithms
- [ ] Command-line interface with arguments
- [ ] TUI (Terminal User Interface)
- [ ] Multiple search strategies (regex, glob patterns)
- [ ] Index multiple directories
- [ ] File metadata search (size, date, type) (technically supported)
- [ ] Daemonise process (fits in with auto updates)
- [ ] IPC / piping

## 🛠️ Installation

### Prerequisites

- Rust 1.70+ (`rustup` recommended)
- SQLite support (included via `rusqlite` bundled feature)

### Build from source

```bash
git clone https://github.com/yourusername/rust-search.git
cd rust-search
cargo build --release
```

### Run

```bash
cargo run
```

## 📖 Usage

### Basic Search

1. Run the program: `cargo run`
2. Wait for indexing to complete (first run takes longer)
3. Enter search terms when prompted
4. Type `q` to quit

### Search Examples

```
Enter search query:
> main.rs          # Find files named main.rs
> .config          # Find dotfiles/directories
> rust             # Find anything containing "rust"
> *.py             # Find Python files (coming soon)
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   File Walker   │───▶│  SQLite Database │───▶│  Search Engine  │
│  (jwalk crate)  │    │   - files table  │    │   (FTS5 index)  │
│                 │    │   - fts5 index   │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Database Schema

```sql
-- Main files table
CREATE TABLE files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    filename TEXT NOT NULL,
    extension TEXT,
    size INTEGER NOT NULL,
    modified_at TEXT NOT NULL
);

-- FTS5 search index
CREATE VIRTUAL TABLE files_fts USING fts5(
    row,
    path,
    filename,
    extension
);
```

## 🔧 Configuration

Currently configured to:

- Index your `$HOME` directory
- Skip common build/cache directories
- Limit traversal depth to prevent infinite loops
- Store database as `search.db` in project root

## 🚫 Excluded Directories

The indexer automatically skips:

- `node_modules/` (npm packages)
- `target/` (Rust build artifacts)
- `.git/` (Git repositories)
- `build/` (Build outputs)
- `vendor/` (Dependencies)
- `AppData/` (Windows app data)
- `.cache/`, `.cargo/`, `.rustup/` (Heavy cache dirs)

## 📊 Performance

**Initial Results** (on typical home directory):

- ~50,000 files indexed in ~2-3 seconds
- Search queries return in <2ms (for 20 results, 50ms uncapped)
- Database size: ~5-10MB for typical home directory
- Memory usage: <50MB during indexing

## 🤝 Contributing

This is a learning project, but contributions are welcome!

### Development Setup

```bash
git clone https://github.com/yourusername/rust-search.git
cd rust-search
cargo build
cargo test
```

### Code Structure

```
src/
├── main.rs     # Main application, indexing logic
└── search.rs   # Search functionality, database queries
```

## 🐛 Known Issues

- [ ] FTS5 sync issues with manual index management
- [ ] No incremental updates (rebuilds entire index on run)
- [ ] Limited error handling for permission-denied files
- [ ] Search syntax not documented (its just a raw SQL query.. brilliant I know)

## 📋 Dependencies

| Crate      | Purpose                        |
| ---------- | ------------------------------ |
| `rusqlite` | SQLite database with FTS5      |
| `jwalk`    | Parallel file system traversal |

## 🎯 Goals vs. Everything Search

| Feature           | Everything | Rust Search | Status    |
| ----------------- | ---------- | ----------- | --------- |
| Speed             | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐  | 🟡 Good   |
| Cross-platform    | ❌         | ✅          | ✅ Done   |
| Real-time updates | ✅         | ❌          | 🔴 TODO   |
| Memory usage      | ⭐⭐⭐     | ⭐⭐⭐⭐    | ✅ Better |
| Fuzzy search      | ⭐⭐       | ❌          | 🔴 TODO   |

## 💡 Why This Project?

- **Learning Rust** - Great way to explore systems programming
- **Database skills** - SQLite, FTS, indexing strategies
- **Cross-platform** - Works everywhere, not just Windows
- **Performance** - Modern hardware deserves modern tools
- **Control** - No external dependencies or proprietary formats

---

_"I (foolishly) believe I can do better."_ 😄

**Status**: 🚧 Work in Progress - Basic functionality working, many features planned

## Development Notes

- **LIKE vs MATCH** - LIKE statement appears to be marginally faster in small datasets
