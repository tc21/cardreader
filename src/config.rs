use std::fs;

use enigo::Key;
use serde::Deserialize;


#[derive(Deserialize)]
pub struct Config {
    pub felica_file: String,
    pub login_key: Key,
}

const CONFIG_PATHS: &[&str] = &[
    "cardreader.json",
    "~/cardreader.json",
    "/cardreader.json"
];

pub fn get_config() -> Config {
    for path in CONFIG_PATHS {
        let file_contents = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let config = match serde_json::from_str(&file_contents) {
            Ok(c) => c,
            Err(e) => panic!("Failed to parse {}: {}", path, e)
        };

        validate_config(&config);

        return config;
    };

    panic!("Failed to get config; please create a cardreader.json");
}

fn validate_config(config: &Config) {
    let f = match fs::File::open(&config.felica_file) {
        Ok(f) => f,
        Err(_) => panic!("File not found: {}", &config.felica_file),
    };

    let metadata = match f.metadata() {
        Ok(f) => f,
        Err(_) => panic!("Failed to read file metadata: {}", &config.felica_file),
    };

    if metadata.permissions().readonly() {
        panic!("Felica file not writable: {}", &config.felica_file)
    };
}
