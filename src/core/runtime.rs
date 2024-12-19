use chrono::{DateTime, Utc};
use rand::Rng;
use std::collections::HashSet;
use tokio::time::{sleep, Duration};
use twitter_v2::id::NumericId;
use crate::character::{CharacterConfig, InstructionBuilder};
    
use crate::{
    core::agent::{Agent, ResponseDecision},
    memory::MemoryStore,
    models::Memory,
    providers::telegram::Telegram,
    providers::twitter::Twitter,
};

pub struct Runtime {
    anthropic_api_key: String,
    twitter: Twitter,
    agents: Vec<Agent>,
    memory: Memory,
    processed_tweets: HashSet<String>,
    telegram: Telegram,
    cached_user_id: Option<u64>,
    last_notification_check: Option<DateTime<Utc>>,
    last_tweet_time: Option<DateTime<Utc>>,
}

impl Runtime {
    pub fn new(
        anthropic_api_key: &str,
        twitter_consumer_key: &str,
        twitter_consumer_secret: &str,
        twitter_access_token: &str,
        twitter_access_token_secret: &str,
        telegram_bot_token: &str,
    ) -> Self {
        let twitter = Twitter::new(
            twitter_consumer_key,
            twitter_consumer_secret,
            twitter_access_token,
            twitter_access_token_secret,
        );
        let telegram = Telegram::new(telegram_bot_token);
        let agents = Vec::new();
        let memory = MemoryStore::load_memory().unwrap_or_else(|_| Memory::default());

        let processed_tweets =
            MemoryStore::load_processed_tweets().unwrap_or_else(|_| HashSet::new());

        Runtime {
            memory,
            anthropic_api_key: anthropic_api_key.to_string(),
            agents,
            twitter,
            processed_tweets,
            telegram,
            cached_user_id: None,
            last_notification_check: None,
            last_tweet_time: None,
        }
    }

    pub fn add_agent(&mut self, prompt: &str) {
        let agent = Agent::new(&self.anthropic_api_key, prompt);
        self.agents.push(agent);
    }

    async fn should_allow_tweet(&self) -> bool {
        match self.last_tweet_time {
            None => true,
            Some(last_tweet) => {
                // Only allow tweet if at least 5 minutes have passed since last tweet
                let duration = Utc::now().signed_duration_since(last_tweet);
                duration.num_minutes() >= 5
            }
        }
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        if self.agents.is_empty() {
            return Err(anyhow::anyhow!("No agents available")).map_err(Into::into);
        }

        // Check if enough time has passed since last tweet
        if !self.should_allow_tweet().await {
            println!("Waiting for rate limit cooldown...");
            return Ok(());
        }

        let mut rng = rand::thread_rng();
        let selected_agent = &self.agents[rng.gen_range(0..self.agents.len())];

        let response = selected_agent
            .generate_post()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate post: {}", e))?;

        println!("Generated tweet: {}", response);

        // Send tweet and handle rate limits
        match self.twitter.tweet(response.clone()).await {
            Ok(tweet_result) => {
                // Update last tweet time
                self.last_tweet_time = Some(Utc::now());
                
                // Get the tweet ID from the tweet result
                let twitter_id = Some(tweet_result.id.to_string());

                // Save to memory
                match MemoryStore::add_to_memory(
                    &mut self.memory,
                    &response,
                    &selected_agent.prompt,
                    twitter_id,
                ) {
                    Ok(_) => println!("Response saved to memory."),
                    Err(e) => eprintln!("Failed to save response to memory: {}", e),
                }

                println!("AI Response: {}", response);
                Ok(())
            }
            Err(e) => {
                if e.to_string().contains("429") {
                    println!("Rate limit hit, waiting 15 minutes before retrying...");
                    sleep(Duration::from_secs(15 * 60)).await;
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn ensure_user_id(&mut self) -> Result<u64, anyhow::Error> {
        if let Some(id) = self.cached_user_id {
            Ok(id)
        } else {
            let user = self.twitter.get_user_id().await?;
            let numeric_id = match user.to_string().parse::<u64>() {
                Ok(id) => id,
                Err(_) => return Err(anyhow::anyhow!("Failed to parse user ID")),
            };
            self.cached_user_id = Some(numeric_id);
            Ok(numeric_id)
        }
    }

    async fn should_check_notifications(&self) -> bool {
        match self.last_notification_check {
            None => true,
            Some(last_check) => {
                // Only check notifications every 15 minutes
                let duration = Utc::now().signed_duration_since(last_check);
                duration.num_minutes() >= 5
            }
        }
    }

    async fn handle_notifications(&mut self) -> Result<(), anyhow::Error> {
        if self.agents.is_empty() {
            return Err(anyhow::anyhow!("No agents available"));
        }

        // Only proceed if enough time has passed since last check
        if !self.should_check_notifications().await {
            return Ok(());
        }

        let user_id = self.ensure_user_id().await?;
        
        match self.twitter.get_notifications(user_id).await {
            Ok(notifications) => {
                self.last_notification_check = Some(Utc::now());
                
                // Process notifications...
                let new_notifications: Vec<_> = notifications
                    .into_iter()
                    .filter(|tweet| !self.processed_tweets.contains(&tweet.id.to_string()))
                    .collect();

                if let Some(tweet) = new_notifications.first() {
                    let tweet_id = tweet.id.to_string();
                    let selected_agent = &self.agents[0];

                    match selected_agent.should_respond(&tweet.text).await? {
                        ResponseDecision::Respond => {
                            let reply = selected_agent.generate_reply(&tweet.text).await?;

                            // Save to memory as a reply
                            if let Err(e) = MemoryStore::add_reply_to_memory(
                                &mut self.memory,
                                &reply,
                                &selected_agent.prompt,
                                Some(tweet_id.clone()),
                                tweet.id.to_string(),
                            ) {
                                eprintln!("Failed to save response to memory: {}", e);
                            }

                            self.twitter.reply_to_tweet(&tweet_id, reply).await?;
                        }
                        ResponseDecision::Ignore => {
                            println!("Agent decided to ignore tweet: {}", tweet.text);
                        }
                    }

                    self.processed_tweets.insert(tweet_id);
                    MemoryStore::save_processed_tweets(&self.processed_tweets)?;
                }
                
                Ok(())
            }
            Err(e) => {
                if e.to_string().contains("429") {
                    println!("Rate limit hit for notifications, will retry in 15 minutes");
                    self.last_notification_check = Some(Utc::now());
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    fn schedule_next_tweet(&mut self) {
        let mut rng = rand::thread_rng();
        let delay_secs = rng.gen_range(5 * 60..15 * 60); 
        let next_tweet = Utc::now() + chrono::Duration::seconds(delay_secs as i64);
        self.memory.next_tweet = Some(next_tweet);

        // Save the updated next_tweet time
        if let Err(e) = MemoryStore::save_memory(&self.memory) {
            eprintln!("Failed to save next tweet time: {}", e);
        }
    }

    async fn wait_until_next_tweet(&self) -> bool {
        if let Some(next_tweet) = self.memory.next_tweet {
            let now = Utc::now();
            if next_tweet > now {
                let duration = next_tweet.signed_duration_since(now);
                if duration.num_seconds() > 0 {
                    sleep(Duration::from_secs(duration.num_seconds() as u64)).await;
                }
                true
            } else {
                true // Time has passed, ready to tweet
            }
        } else {
            false // No scheduled tweet
        }
    }

    pub async fn run_periodically(&mut self) -> Result<(), anyhow::Error> {
        // Schedule next tweet if not already scheduled
        if self.memory.next_tweet.is_none() {
            self.schedule_next_tweet();
        }
    
        // If in debug mode, send a tweet immediately
        if self.memory.debug_mode {
            if let Err(e) = self.run().await {
                eprintln!("Error running tweet process in debug mode: {}", e);
            }
            // Schedule next tweet after debug tweet
            self.schedule_next_tweet();
        }
    
        loop {
            // Wait until it's time for the next tweet
            if self.wait_until_next_tweet().await {
                // Handle regular tweets
                if let Err(e) = self.run().await {
                    eprintln!("Error running tweet process: {}", e);
                }
    
                // Schedule the next tweet
                self.schedule_next_tweet();
    
                // Handle notifications
                if let Err(e) = self.handle_notifications().await {
                    eprintln!("Error handling notifications: {}", e);
                }
            }
    
            // Small sleep to prevent tight loop
            sleep(Duration::from_secs(1)).await;
        }
    }
}