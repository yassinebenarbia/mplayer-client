use std::path::PathBuf;

use basic_toml;
use serde::{self, Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Copy, Clone, Default)]
pub enum Sorting {
    #[default]
    ByTitleAscending,
    ByTitleDescending,
    ByDurationAscending,
    ByDurationDescending,
    Shuffle,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Scripts {
    pub list: Option<PathBuf>,
    pub lyrics: Option<PathBuf>,
    pub actions: Option<PathBuf>,
    pub seeker: Option<PathBuf>,
    pub volume: Option<PathBuf>,
    pub all: Option<PathBuf>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "config")]
pub struct Config {
    pub fps: Option<u32>,
    pub lyrics: Option<bool>,
    pub genre: Option<bool>,
    pub scripts: Option<Scripts>,
    pub step_size: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fps: Some(30),
            lyrics: Some(false),
            genre: Some(false),
            scripts: None,
            step_size: Some(7),
        }
    }
}

impl Config {
    pub fn parse_config(_path: &str) -> std::io::Result<Self> {
        let conf_content = std::fs::read_to_string(_path)?;

        let config: Config = basic_toml::from_str(&conf_content)
            .map_err(|e| std::io::Error::other(format!("{}", e)))?;
        Ok(config)
    }
}

mod test {
    #[allow(unused_imports)]
    use super::*;
    #[test]
    fn read_config() {
        match Config::parse_config("./config.example.toml") {
            Ok(_) => {}
            Err(_) => panic!(),
        }
    }
}
