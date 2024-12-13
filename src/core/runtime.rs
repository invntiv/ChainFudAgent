use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::time::{sleep, Duration};

use crate::{
    core::agent::{Agent, ResponseDecision},
    memory::MemoryStore,
    providers::twitter::Twitter,
};

#[derive(Serialize, Deserialize, Default)]
pub struct ProcessedNotifications {
    tweet_ids: HashSet<String>,
}

pub struct Runtime {
    anthropic_api_key: String,
    twitter: Twitter,
    agents: Vec<Agent>,
    memory: Vec<String>,
    processed_tweets: HashSet<String>,
}

impl Runtime {
    pub fn new(
        anthropic_api_key: &str,
        twitter_consumer_key: &str,
        twitter_consumer_secret: &str,
        twitter_access_token: &str,
        twitter_access_token_secret: &str,
    ) -> Self {
        let twitter = Twitter::new(
            twitter_consumer_key,
            twitter_consumer_secret,
            twitter_access_token,
            twitter_access_token_secret,
        );

        let agents = Vec::new();
        let memory: Vec<String> = MemoryStore::load_memory().unwrap_or_else(|_| Vec::new());

        let processed_tweets =
            MemoryStore::load_processed_tweets().unwrap_or_else(|_| HashSet::new());

        Runtime {
            memory,
            anthropic_api_key: anthropic_api_key.to_string(),
            agents,
            twitter,
            processed_tweets,
        }
    }

    pub fn add_agent(&mut self, prompt: &str) {
        let agent = Agent::new(&self.anthropic_api_key, prompt);
        self.agents.push(agent);
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        if self.agents.is_empty() {
            return Err(anyhow::anyhow!("No agents available")).map_err(Into::into);
        }

        let mut rng = rand::thread_rng();
        let selected_agent = &self.agents[rng.gen_range(0..self.agents.len())];
        let response = selected_agent.generate_post().await?;

        match MemoryStore::add_to_memory(&mut self.memory, &response) {
            Ok(_) => println!("Response saved to memory."),
            Err(e) => eprintln!("Failed to save response to memory: {}", e),
        }

        println!("AI Response: {}", response);
        self.twitter.tweet(response).await?;
        Ok(())
    }

    async fn handle_notifications(&mut self) -> Result<(), anyhow::Error> {
        if self.agents.is_empty() {
            return Err(anyhow::anyhow!("No agents available"));
        }

        let user_id = self.twitter.get_user_id().await?;
        let notifications = self.twitter.get_notifications(user_id).await?;

        // Take only the latest 5 notifications
        for tweet in notifications.into_iter().take(5) {
            let tweet_id = tweet.id.to_string();

            if self.processed_tweets.contains(&tweet_id) {
                continue;
            }
            let selected_agent = &self.agents[0];

            // Check if we should respond
            match selected_agent.should_respond(&tweet.text).await? {
                ResponseDecision::Respond => {
                    let reply = selected_agent.generate_reply(&tweet.text).await?;

                    // Save to memory
                    if let Err(e) = MemoryStore::add_to_memory(&mut self.memory, &reply) {
                        eprintln!("Failed to save response to memory: {}", e);
                    }

                    // Send the reply
                    self.twitter.reply_to_tweet(&tweet_id, reply).await?;
                }
                ResponseDecision::Ignore => {
                    println!("Agent decided to ignore tweet: {}", tweet.text);
                }
            }
            self.processed_tweets.insert(tweet_id);
            if let Err(e) = MemoryStore::save_processed_tweets(&self.processed_tweets) {
                eprintln!("Failed to save processed tweets: {}", e);
            }
            Self::random_delay(30, 60).await;
        }

        Ok(())
    }

    async fn random_delay(min_secs: u64, max_secs: u64) {
        let mut rng = rand::thread_rng();
        let delay_secs = rng.gen_range(min_secs..max_secs);
        sleep(Duration::from_secs(delay_secs)).await;
    }

    pub async fn run_periodically(&mut self) -> Result<(), anyhow::Error> {
        loop {
            //Handle regular tweets
            if let Err(e) = self.run().await {
                eprintln!("Error running tweet process: {}", e);
            }

            // Handle notifications
            if let Err(e) = self.handle_notifications().await {
                eprintln!("Error handling notifications: {}", e);
            }

            //Random delay between 5-15 minutes
            Self::random_delay(5 * 60, 15 * 60).await;
        }
    }
}
