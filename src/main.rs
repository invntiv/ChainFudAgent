mod characteristics;
mod core;
mod memory;
mod providers;
use core::{instruction_builder::InstructionBuilder, runtime::Runtime};
extern crate dotenv;
pub mod models;
pub mod character;
use crate::models::CharacterConfig;  // Add this import at the top of main.rs

use dotenv::dotenv;
use std::env;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if let Err(e) = dotenv() {
        eprintln!("Error loading .env file: {}", e);
    }

    let character_config = CharacterConfig {
            name: "fud".to_string(),
        };

    let mut runtime = Runtime::new(
        &env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set"),
        &env::var("TWITTER_CONSUMER_KEY").expect("TWITTER_CONSUMER_KEY not set"),
        &env::var("TWITTER_CONSUMER_SECRET").expect("TWITTER_CONSUMER_SECRET not set"),
        &env::var("TWITTER_ACCESS_TOKEN").expect("TWITTER_ACCESS_TOKEN not set"),
        &env::var("TWITTER_ACCESS_TOKEN_SECRET").expect("TWITTER_ACCESS_TOKEN_SECRET not set"),
        &env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set"),
        &env::var("SOLANA_TRACKER_API_KEY").expect("SOLANA_TRACKER_API_KEY not set"),
        character_config,
    );

    let mut instruction_builder = InstructionBuilder::new();
    let character_name = env::var("CHARACTER_NAME")
        .expect("CHARACTER_NAME not set")
        .trim()
        .to_string();

    println!("Running character: {}", character_name);

    if let Err(e) = instruction_builder.build_instructions(&character_name) {
        eprintln!("Error building instructions: {}", e);
        return Err(anyhow::anyhow!("Failed to build instructions"));
    }
    runtime.add_agent(instruction_builder.get_instructions());

    runtime.run_periodically().await?;

    Ok(())
}
