use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TweetType {
    Original,
    Reply
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Tweet {
    pub internal_id: u64,
    pub twitter_id: Option<String>,
    pub text: String,
    pub prompt: String,
    pub timestamp: DateTime<Utc>,
    pub tweet_type: TweetType,
    pub reply_to: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Memory {
    pub tweets: Vec<Tweet>,
    pub next_id: u64,
    pub next_tweet: Option<DateTime<Utc>>,
    pub debug_mode: bool,
    pub tweet_mode: bool
}

#[derive(Serialize, Deserialize, Default)]
pub struct ProcessedNotifications {
    pub tweet_ids: HashSet<String>,
}