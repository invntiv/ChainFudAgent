use std::env;
use std::fs;
use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterConfig {
    pub name: String,
    pub prompt: String,
    // Add any other character-specific fields here
}

pub fn load_character_config() -> Result<CharacterConfig> {
    // Get character name from environment variable, default to "rina" if not set
    let character_name = env::var("CHARACTER_NAME").unwrap_or_else(|_| "rina".to_string());
    
    // Construct path to character config
    let mut config_path = PathBuf::from("characters");
    config_path.push(&character_name);
    config_path.push("config.json");

    // Check if character directory exists
    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Character config not found for '{}' at {:?}",
            character_name,
            config_path
        ));
    }

    // Read and parse the config file
    let config_str = fs::read_to_string(&config_path)?;
    let config: CharacterConfig = serde_json::from_str(&config_str)?;

    println!("Loaded character profile: {}", config.name);
    Ok(config)
}

pub struct InstructionBuilder {
    character_config: CharacterConfig,
}

impl InstructionBuilder {
    pub fn new() -> Result<Self> {
        let character_config = load_character_config()?;
        Ok(Self { character_config })
    }

    pub fn get_instructions(&self) -> &str {
        &self.character_config.prompt
    }
}