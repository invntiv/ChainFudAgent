use std::fs;
use std::io::{self, Write};
use std::path::Path;
use crate::models::{Memory, Tweet, ProcessedNotifications, TweetType};
use std::collections::HashSet;
use chrono::{DateTime, Utc};

pub struct MemoryStore;

impl MemoryStore {
    const FILE_PATH: &'static str = "./storage/memory.json";

    // Load memory from file
    pub fn load_memory() -> io::Result<Memory> {
        if Path::new(Self::FILE_PATH).exists() {
            let data = fs::read_to_string(Self::FILE_PATH)?;
            let memory: Memory = serde_json::from_str(&data)?;
            Ok(memory)
        } else {
            Ok(Memory::default())
        }
    }

    // Add to memory for original tweets
    pub fn add_to_memory(memory: &mut Memory, text: &str, prompt: &str, twitter_id: Option<String>) -> Result<(), String> {
        let tweet = Tweet {
            internal_id: memory.next_id,
            twitter_id,
            text: text.to_string(),
            prompt: prompt.to_string(),
            timestamp: Utc::now(),
            tweet_type: TweetType::Original,
            reply_to: None,
        };
        
        memory.tweets.push(tweet);
        memory.next_id += 1;
        
        let _ = Self::save_memory(memory);
        Ok(())
    }

    // Add a new method specifically for replies
    pub fn add_reply_to_memory(
        memory: &mut Memory,
        text: &str,
        prompt: &str,
        twitter_id: Option<String>,
        reply_to: String,
    ) -> Result<(), String> {
        let tweet = Tweet {
            internal_id: memory.next_id,
            twitter_id,
            text: text.to_string(),
            prompt: prompt.to_string(),
            timestamp: Utc::now(),
            tweet_type: TweetType::Reply,
            reply_to: Some(reply_to),
        };
        
        memory.tweets.push(tweet);
        memory.next_id += 1;
        
        let _ = Self::save_memory(memory);
        Ok(())
    }

    // Update next tweet time
    pub fn update_next_tweet_time(memory: &mut Memory, next_tweet: DateTime<Utc>) -> io::Result<()> {
        memory.next_tweet = Some(next_tweet);
        Self::save_memory(memory)
    }

    // Get next tweet time
    pub fn get_next_tweet_time(memory: &Memory) -> Option<DateTime<Utc>> {
        memory.next_tweet
    }

    // Save memory to file
    pub fn save_memory(memory: &Memory) -> io::Result<()> {
        fs::create_dir_all("./storage")?;
        let data = serde_json::to_string_pretty(memory)?;
        let mut file = fs::File::create(Self::FILE_PATH)?;
        file.write_all(data.as_bytes())?;
        Ok(())
    }

    pub fn load_processed_tweets() -> Result<HashSet<String>, anyhow::Error> {
        match fs::read_to_string("storage/processed_tweets.json") {
            Ok(contents) => {
                let data: ProcessedNotifications = serde_json::from_str(&contents)?;
                Ok(data.tweet_ids)
            }
            Err(_) => Ok(HashSet::new())
        }
    }

    // Get Tweeting mode status
    pub fn get_tweet_mode(memory: &Memory) -> bool {
        memory.tweet_mode
    }

    // Get debug mode status
    pub fn get_debug_mode(memory: &Memory) -> bool {
        memory.debug_mode
    }

    // Set debug mode status
    pub fn set_debug_mode(memory: &mut Memory, debug: bool) -> io::Result<()> {
        memory.debug_mode = debug;
        Self::save_memory(memory)
    }

    pub fn save_processed_tweets(processed_tweets: &HashSet<String>) -> Result<(), anyhow::Error> {
        let data = ProcessedNotifications {
            tweet_ids: processed_tweets.clone(),
        };
        let json = serde_json::to_string_pretty(&data)?;
        fs::create_dir_all("storage")?;
        fs::write("storage/processed_tweets.json", json)?;
        Ok(())
    }
}