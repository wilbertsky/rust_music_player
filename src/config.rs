use std::fs;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub theme: String,
    pub mpd_address: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mpd_address: "127.0.0.1:6600".to_string(),
            theme: "Moonfly".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Config, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir().ok_or("no config dir")?;
        let config_path = config_dir.join("music_player").join("config.toml");
        let config_string = fs::read_to_string(config_path)?;
        let config_toml: Config = toml::from_str(&config_string)?;

        Ok(config_toml)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir().ok_or("no config dir")?;
        let config_path = config_dir.join("music_player").join("config.toml");
        let config_toml = toml::to_string(self)?;
        fs::create_dir_all(config_path.parent().unwrap())?;

        fs::write(config_path, config_toml)?;

        Ok(())
    }
}
