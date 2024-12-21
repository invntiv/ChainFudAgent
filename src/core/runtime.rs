use chrono::{DateTime, Timelike, Utc};
use rand::Rng;
use std::collections::HashSet;
use tokio::time::{sleep, Duration};

use crate::{
    core::agent::{Agent, ResponseDecision},
    memory::MemoryStore,
    models::Memory,
    models::CharacterConfig,
    providers::telegram::Telegram,
    providers::twitter::Twitter,
    providers::solanatracker::SolanaTracker,
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
    solana_tracker: SolanaTracker,
    character_config: CharacterConfig,
}

impl Runtime {
    pub fn new(
        anthropic_api_key: &str,
        twitter_consumer_key: &str,
        twitter_consumer_secret: &str,
        twitter_access_token: &str,
        twitter_access_token_secret: &str,
        telegram_bot_token: &str,
        solana_tracker_api_key: &str,
        character_config: CharacterConfig,
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
        let processed_tweets = MemoryStore::load_processed_tweets().unwrap_or_else(|_| HashSet::new());
        let solana_tracker = SolanaTracker::new(solana_tracker_api_key);
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
            solana_tracker,
            character_config
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

    //  Method to check if it's time for scheduled actions
    async fn should_run_scheduled_action(&self, minutes: &[u32]) -> bool {
        let now = Utc::now();
        minutes.contains(&now.minute()) && now.second() == 0
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        //FUD First get and show trending summary
        let summary = self.get_trending_solana_summary().await?;
        println!("Solana trending summary: {}", summary);
    
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
        
        // This is where we decide what to tweet
        let response = if rng.gen_bool(0.5) {
            // Use the agent's normal post
            selected_agent
                .generate_post()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to generate post: {}", e))?
        } else {
            // Get tokens and generate FUD
            let tokens = self.solana_tracker.get_top_tokens(20).await?;
            let random_token = tokens.get(rng.gen_range(0..tokens.len()))
                .ok_or_else(|| anyhow::anyhow!("No tokens available"))?;
            self.solana_tracker.generate_fud(random_token)
        };
        
        let tokens = self.solana_tracker.get_top_tokens(20).await?;
        let random_token = tokens
            .get(rng.gen_range(0..tokens.len()))
            .ok_or_else(|| anyhow::anyhow!("No tokens available"))?;
        
        let fud = self.solana_tracker.generate_fud(random_token);
        println!("{}", fud);
    
        // Only proceed with tweeting if tweet_mode is true
        if self.memory.tweet_mode {
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
        } else {
            // If tweet_mode is false, just save to memory without tweeting
            match MemoryStore::add_to_memory(
                &mut self.memory,
                &response,
                &selected_agent.prompt,
                None,
            ) {
                Ok(_) => println!("Response saved to memory (tweet_mode disabled)."),
                Err(e) => eprintln!("Failed to save response to memory: {}", e),
            }
            Ok(())
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

    async fn handle_notifications_fud(&mut self) -> Result<(), anyhow::Error> {
        if self.agents.is_empty() {
            return Err(anyhow::anyhow!("No agents available"));
        }

        let user_id = self.ensure_user_id().await?;
        
        match self.twitter.get_notifications(user_id).await {
            Ok(notifications) => {
                self.last_notification_check = Some(Utc::now());
                
                let new_notifications: Vec<_> = notifications
                    .into_iter()
                    .filter(|tweet| !self.processed_tweets.contains(&tweet.id.to_string()))
                    .collect();

                for tweet in new_notifications {
                    let tweet_id = tweet.id.to_string();
                    
                    // Check for $ symbol followed by uppercase letters
                    if let Some(ticker) = Self::extract_ticker_symbol(&tweet.text) {
                        let selected_agent = &self.agents[0];
                        
                        // Get author ID and handle the case where it might be None
                        let author_reference = match tweet.author_id {
                            Some(author_id) => format!("@{}", author_id),
                            None => "anon".to_string()
                        };
                        
                        let fud_response = format!("yo {} {} is definitely going to zero, straight up ponzi vibes", 
                            author_reference,
                            ticker
                        );

                        // Save to memory as a reply
                        if let Err(e) = MemoryStore::add_reply_to_memory(
                            &mut self.memory,
                            &fud_response,
                            &selected_agent.prompt,
                            Some(tweet_id.clone()),
                            tweet.id.to_string(),
                        ) {
                            eprintln!("Failed to save response to memory: {}", e);
                        }

                        if self.memory.tweet_mode {
                            self.twitter.reply_to_tweet(&tweet_id, fud_response).await?;
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

    pub async fn get_trending_solana_summary(&self) -> Result<String, anyhow::Error> {
        let tokens = self.solana_tracker.get_top_tokens(5).await?;
        Ok(self.solana_tracker.format_tokens_summary(&tokens, 5))
    }

    pub async fn run_periodically(&mut self) -> Result<(), anyhow::Error> {
        println!("Starting periodic run loop...");
        
        loop {
            let now = Utc::now();
            
            // For FUD character specific timing
            if self.character_config.name == "fud" {
                // Check for trending summary times (HH:15 and HH:30)
                if self.should_run_scheduled_action(&[15, 30]).await {
                    println!("Running trending summary at {}:{:02}", now.hour(), now.minute());
                    let summary = self.get_trending_solana_summary().await?;
                    if self.memory.tweet_mode {
                        self.twitter.tweet(summary).await?;
                    }
                }

                // Check for FUD generation times (HH:00 and HH:45)
                if self.should_run_scheduled_action(&[0, 45]).await {
                    println!("Generating FUD at {}:{:02}", now.hour(), now.minute());
                    let tokens = self.solana_tracker.get_top_tokens(20).await?;
                    let mut rng = rand::thread_rng();
                    if let Some(random_token) = tokens.get(rng.gen_range(0..tokens.len())) {
                        let fud = self.solana_tracker.generate_fud(random_token);
                        if self.memory.tweet_mode {
                            self.twitter.tweet(fud).await?;
                        }
                    }
                }

                // Handle notifications every 5 minutes
                if self.should_check_notifications().await {
                    if let Err(e) = self.handle_notifications_fud().await {
                        eprintln!("Error handling FUD notifications: {}", e);
                    }
                }
            } else {
                // Original behavior for non-FUD characters
                if self.wait_until_next_tweet().await {
                    if let Err(e) = self.run().await {
                        eprintln!("Error running tweet process: {}", e);
                    }
                    self.schedule_next_tweet();
                }
            }

            // Sleep for 1 second before next check
            sleep(Duration::from_secs(1)).await;
        }
    }

        // Helper function to extract ticker symbols
    fn extract_ticker_symbol(text: &str) -> Option<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            if word.starts_with('$') && word.len() > 1 {
                let ticker = word[1..].to_string();
                if ticker.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
                    return Some(ticker);
                }
            }
        }
        None
    }
}

