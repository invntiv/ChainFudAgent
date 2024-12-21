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
            let tokens = self.solana_tracker.get_top_tokens(35).await?;
            let random_token = tokens.get(rng.gen_range(0..tokens.len()))
                .ok_or_else(|| anyhow::anyhow!("No tokens available"))?;
            self.solana_tracker.generate_fud(random_token)
        };
        
        let tokens = self.solana_tracker.get_top_tokens(35).await?;
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
            None => {
                println!("First notification check");
                true
            },
            Some(last_check) => {
                let duration = Utc::now().signed_duration_since(last_check);
                let should_check = duration.num_minutes() >= 5;
                println!(
                    "Time since last notification check: {} minutes. Should check: {}", 
                    duration.num_minutes(),
                    should_check
                );
                should_check
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
                // Generate FUD every quarter hour (00, 15, 30, 45)
                if self.should_run_scheduled_action(&[0, 15, 30, 45]).await {
                    if let Err(e) = self.generate_and_post_fud().await {
                        eprintln!("Error generating FUD: {}", e);
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
        
        // First try to find a $ prefixed ticker
        for word in words.iter() {
            let trimmed = word.trim();
            if trimmed.starts_with('$') && trimmed.len() > 1 {
                let ticker = trimmed[1..].to_string();
                if ticker.chars().any(|c| c.is_ascii_alphanumeric()) {
                    println!("Found $ prefixed ticker: {}", ticker);
                    return Some(ticker);
                }
            }
        }

        // If no $ ticker found, look for keywords followed by potential tickers
        let text_lower = text.to_lowercase();
        let trigger_words = ["thoughts on", "think of", "about"];
        
        for trigger in trigger_words.iter() {
            if let Some(pos) = text_lower.find(trigger) {
                let after_trigger = &text[pos + trigger.len()..];
                let potential_ticker = after_trigger
                    .split_whitespace()
                    .next()
                    .map(|w| w.trim_matches(|c: char| !c.is_ascii_alphanumeric()));
                
                if let Some(ticker) = potential_ticker {
                    if !ticker.is_empty() {
                        println!("Found implied ticker from '{}': {}", trigger, ticker);
                        return Some(ticker.to_string());
                    }
                }
            }
        }
        
        None
    }
    ////////////////////////
    /// FUD-SPECIFIC ACTIONS
    ////////////////////////
    fn format_ticker_for_response(ticker: &str) -> String {
        ticker.to_uppercase()
    }

    async fn generate_and_post_fud(&mut self) -> Result<(), anyhow::Error> {
        println!("Generating FUD at {}:{:02}", Utc::now().hour(), Utc::now().minute());
        let tokens = self.solana_tracker.get_top_tokens(30).await?;  // Updated to 30 tokens
        let mut rng = rand::thread_rng();
        
        if let Some(random_token) = tokens.get(rng.gen_range(0..tokens.len())) {
            let token_summary = self.solana_tracker.format_token_summary(random_token);
            println!("Token Summary: {}", token_summary);

            let selected_agent = &self.agents[0];
            let fud = selected_agent.generate_editorialized_fud(&token_summary).await?;

            println!("Generated FUD: {}", fud);
            
            if self.memory.tweet_mode {
                match self.twitter.tweet(fud.clone()).await {
                    Ok(_) => println!("Successfully posted FUD tweet"),
                    Err(e) => println!("Failed to post FUD tweet: {}", e),
                }
            } else {
                println!("Tweet mode disabled, skipping tweet");
            }
        }
        
        Ok(())
    }

    async fn handle_notifications_fud(&mut self) -> Result<(), anyhow::Error> {
        println!("Starting FUD notification check...");
        
        if self.agents.is_empty() {
            println!("No agents available");
            return Err(anyhow::anyhow!("No agents available"));
        }

        // Check if we should process notifications
        if !self.should_check_notifications().await {
            println!("Skipping notification check - too soon since last check");
            return Ok(());
        }

        println!("Getting user ID...");
        let user_id = self.ensure_user_id().await?;
        println!("User ID: {}", user_id);
        
        match self.twitter.get_notifications(user_id).await {
            Ok(notifications) => {
                println!("Got {} notifications", notifications.len());
                self.last_notification_check = Some(Utc::now());
                
                // Instead of just getting new notifications, get all unresponded ones
                let unresponded_notifications: Vec<_> = notifications
                    .into_iter()
                    .filter(|tweet| {
                        // Check if we've already responded to this tweet
                        !self.memory.tweets.iter().any(|t| 
                            t.reply_to.as_ref().map_or(false, |reply_id| reply_id == &tweet.id.to_string())
                        )
                    })
                    .collect();
                
                println!("Found {} unresponded notifications", unresponded_notifications.len());

                // Randomly select up to 2 notifications to process
                let mut rng = rand::thread_rng();
                let notifications_to_process: Vec<_> = if unresponded_notifications.len() > 2 {
                    use rand::seq::SliceRandom;
                    let mut selected = unresponded_notifications.clone();
                    selected.shuffle(&mut rng);
                    selected.truncate(2);
                    selected
                } else {
                    unresponded_notifications
                };

                println!("Processing {} notifications", notifications_to_process.len());
                
                for tweet in notifications_to_process {
                    println!("Processing tweet: {}", tweet.text);
                    let tweet_id = tweet.id.to_string();
                    
                    // Check for $ symbol followed by uppercase letters with improved extraction
                    if let Some(ticker) = Self::extract_ticker_symbol(&tweet.text) {
                        let formatted_ticker = Self::format_ticker_for_response(&ticker);
                        println!("Found ticker in tweet: {}", formatted_ticker);
                        let selected_agent = &self.agents[0];
                        
                        // Get token data for the mentioned ticker if available
                        let tokens = self.solana_tracker.get_top_tokens(30).await?;
                        println!("Got {} tokens from tracker", tokens.len());
                        
                        let token_info = if let Some(token) = SolanaTracker::find_token_by_symbol(&tokens, &formatted_ticker) {
                            println!(
                                "Found token {} with liquidity ${:.2}", 
                                token.token.symbol,
                                token.pools.first().map(|p| p.liquidity.usd).unwrap_or(0.0)
                            );
                            self.solana_tracker.format_token_summary(token)
                        } else {
                            println!("No token found for ticker {}", formatted_ticker);
                            format!("Token: ${}", formatted_ticker)
                        };

                        println!("Generated token info: {}", token_info);
                        let prompt = format!(
                            "{}\n\nTask: Generate a toxic, cynical commentary about this token:\n{}\n\
                            Requirements:\n\
                            - Be extremely sarcastic and cynical\n\
                            - Always use ${} when referring to the token\n\
                            - Include specific numbers from the token info\n\
                            - Be creative with metaphors about scams, rugpulls, or dev behavior\n\
                            - Stay under 280 characters\n\
                            - Use all lowercase except for the token symbol\n\
                            - Avoid hashtags\n\
                            - Make it personal and specific to this token",
                            selected_agent.prompt,
                            token_info,
                            formatted_ticker
                        );
                        // Generate FUD response using the token info
                        let fud_response = selected_agent.generate_editorialized_fud(&token_info).await?;
                        println!("Generated FUD response: {}", fud_response);

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
                            println!("Tweet mode is enabled, posting reply...");
                            match self.twitter.reply_to_tweet(&tweet_id, fud_response).await {
                                Ok(_) => {
                                    println!("Successfully replied to tweet {}", tweet_id);
                                    sleep(Duration::from_secs(30)).await;
                                }
                                Err(e) => {
                                    println!("Failed to reply to tweet: {}", e);
                                    if e.to_string().contains("429") {
                                        println!("Rate limit hit, stopping notification processing");
                                        break;
                                    }
                                }
                            }
                        } else {
                            println!("Tweet mode is disabled, skipping reply");
                        }
                    } else {
                        println!("No ticker found in tweet: {}", tweet.text);
                    }
                }
                
                Ok(())
            }
            Err(e) => {
                if e.to_string().contains("429") {
                    println!("Rate limit hit for notifications, will retry in 15 minutes");
                    self.last_notification_check = Some(Utc::now());
                    Ok(())
                } else {
                    println!("Error getting notifications: {}", e);
                    Err(e)
                }
            }
        }
    }
}

