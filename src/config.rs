use std::io::Read;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub max_file_size_mb: u64,
    pub max_depth: u64,
    pub include_hidden: bool,
    pub skip_binary: bool,
    pub index_limit: u64,
    pub skip_directories: Vec<String>,
    pub skip_extensions: Vec<String>,
    pub skip_patterns: Vec<String>,
    pub force_include: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_file_size_mb: 100, // Default to 100MB
            max_depth: 10,         // Default to 10 levels deep
            include_hidden: false, // Do not include hidden files by default
            skip_binary: true,     // Skip binary files by default
            index_limit: 100_000,  // Default limit for indexed files
            skip_directories: vec!["node_modules".to_string(), "target".to_string()],
            skip_extensions: vec!["exe".to_string(), "dll".to_string()],
            skip_patterns: vec![],
            force_include: vec![],
        }
    }
}

const DEFAULT_PATH: &str = "/home/theo/Documents/github/rust-search/config.toml";

#[derive(Deserialize, Debug)]
struct ConfigFile {
    settings: Config,
}

pub fn get_config() -> Config {
    let file = std::fs::File::open(DEFAULT_PATH).expect("Failed to open config file");
    let mut buf = String::new();
    let mut reader = std::io::BufReader::new(file);
    reader
        .read_to_string(&mut buf)
        .expect("Failed to read config file");

    let config_file: ConfigFile = toml::from_str(&buf).expect("Failed to parse config file");
    config_file.settings
}
