use rig::agent::Agent as RigAgent;
use rig::providers::anthropic::completion::CompletionModel;
use rig::providers::anthropic::{self, CLAUDE_3_HAIKU};
use rig::completion::Prompt;
use rand::{self, Rng};
use serde_json::json;
use std::collections::HashMap;


use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
}; 

use teloxide::prelude::*;

pub struct Agent {
    agent: RigAgent<CompletionModel>,
    anthropic_api_key: String,
    pub prompt: String,
    fud_analysis: FudAnalysis, 
}

#[derive(Debug, PartialEq)]
pub enum ResponseDecision {
    Respond,
    Ignore,
}

#[derive(Debug, Clone)]
struct FudAnalysis {
    word_frequencies: HashMap<String, usize>,
    pattern_frequencies: HashMap<String, usize>,
}

impl FudAnalysis {
    fn new() -> Self {
        FudAnalysis {
            word_frequencies: HashMap::new(),
            pattern_frequencies: HashMap::new(),
        }
    }

    fn update(&mut self, text: &str) {
        // Update word frequencies
        for word in text.split_whitespace() {
            *self.word_frequencies.entry(word.to_lowercase()).or_insert(0) += 1;
        }

        // Update pattern frequencies (basic phrases)
        let patterns = ["ser", "ngmi", "wen", "just", "literally"];
        for pattern in patterns.iter() {
            if text.to_lowercase().contains(pattern) {
                *self.pattern_frequencies.entry(pattern.to_string()).or_insert(0) += 1;
            }
        }
    }

    fn is_overused(&self, text: &str) -> bool {
        // Check for overused words
        let words: Vec<&str> = text.split_whitespace().collect();
        for word in words {
            if let Some(count) = self.word_frequencies.get(&word.to_lowercase()) {
                if *count > 5 {
                    return true;
                }
            }
        }

        // Check for overused patterns
        for (pattern, count) in &self.pattern_frequencies {
            if *count > 3 && text.to_lowercase().contains(pattern) {
                return true;
            }
        }

        false
    }
}

impl Agent {
    pub fn new(anthropic_api_key: &str, prompt: &str) -> Self {
        let client = anthropic::ClientBuilder::new(anthropic_api_key).build();
        let rng = rand::thread_rng();
        let temperature = 0.9;

        let agent = client
            .agent(CLAUDE_3_HAIKU)
            .preamble(prompt)
            .temperature(temperature)
            .max_tokens(4096)
            .build();
        Agent { 
            agent,
            anthropic_api_key: anthropic_api_key.to_string(),
            prompt: prompt.to_string(),
            fud_analysis: FudAnalysis::new(),  // Initialize FudAnalysis
        }
    }

    pub async fn should_respond(&self, tweet: &str) -> Result<ResponseDecision, anyhow::Error> {
        let prompt = format!(
            "Tweet: {tweet}\n\
            Task: Reply [RESPOND] or [IGNORE] based on:\n\
            [RESPOND] if:\n\
            - Direct mention/address\n\
            - Contains question\n\
            - Contains command/request\n\
            [IGNORE] if:\n\
            - Unrelated content\n\
            - Spam/nonsensical\n\
            Answer:"
        );
        let response = self.agent.prompt(&prompt).await?;
        let response = response.to_uppercase();
        Ok(if response.contains("[RESPOND]") {
            ResponseDecision::Respond
        } else {
            ResponseDecision::Ignore
        })
    }

    pub async fn generate_reply(&self, tweet: &str) -> Result<String, anyhow::Error> {
        let prompt = format!(
            "Task: Generate a post/reply in your voice, style and perspective while using this as context:\n\
            Current Post: '{}'\n\
            Generate a brief, single response that:\n\
            - Uses all lowercase\n\
            - Avoids punctuation\n\
            - Is direct and very sarcastic\n\
            - Stays under 280 characters\n\
            Write only the response text, nothing else:",
            tweet
        );
        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }

    pub async fn generate_custom_response(&self, prompt: &str) -> Result<String, anyhow::Error> {
        let response = self.agent
            .prompt(prompt)
            .await?;

        Ok(response.trim().to_string())
    }

    pub async fn generate_post(&self) -> Result<String, anyhow::Error> {
        let prompt = r#"Write a 1-3 sentence post that would be engaging to readers. Your response should be the EXACT text of the tweet only, with no introductions, meta-commentary, or explanations.

            Requirements:
            - Stay under 280 characters
            - No emojis
            - No hashtags
            - No questions
            - Brief, concise statements only
            - Focus on personal experiences, observations, or thoughts
            - Write ONLY THE TWEET TEXT with no additional words or commentary"#;
        
        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }

    // Modify generate_generic_fud to use similar theme-based approach
    pub async fn generate_generic_fud(&self, intro: &str, reason: &str, closing: &str) -> Result<String, anyhow::Error> {
        let prompt = format!(
            "{}\n\nTask: Generate a creative and unique cynical comment.\n\
            Base elements to incorporate:\n\
            - Intro theme: {}\n\
            - Core criticism: {}\n\
            - Closing note: {}\n\n\
            Requirements:\n\
            - Transform these elements creatively - don't use them verbatim\n\
            - Create unexpected analogies or metaphors\n\
            - Mix technical and casual language\n\
            - Stay under 280 characters\n\
            - do not include any tickers or ticker symbols\n\
            - Use all lowercase\n\
            - Sound authentic - like a real frustrated trader\n\
            Write ONLY the tweet text:",
            self.prompt,    
            intro,
            reason,
            closing
        );

        let response = self.agent.prompt(&prompt).await?;
        Ok(self.ensure_unique_style(response.trim())?)
    }

    pub async fn generate_editorialized_fud(&mut self, token_info: &str) -> Result<String, anyhow::Error> {
        let prompt = format!(
            "{}\n\nTask: Generate unique, creative FUD about this token:\n{}\n\
            Requirements:\n\
            - Be extremely sarcastic and cynical, but make it clear when overt sarcasm is being used\n\
            - dont encapsulate your response in quotes\n\
            - Always use proper token symbol from the info\n\
            - Use numbers from the token info creatively and sarcastically\n\
            - Stay under 280 characters\n\
            - Use all lowercase except for token symbols\n\
            - Avoid repetitive phrases and metaphors\n\
            - Variety is key - use different structures and approaches\n\
            - Make each criticism unique and specific\n\
            - Avoid overused phrases like 'chart looks like' or 'mcdonalds'\n\
            - Mix different FUD styles: technical, social, financial, or conspiracy theories\n\
            \n\
            Some varied FUD approaches (use as inspiration, don't copy directly):\n\
            - Question developer competence\n\
            - Imply suspicious transaction patterns\n\
            - Mock community engagement (make sure you don't use words like 'ucertifieds' which your responses hae generated in the past. for example, refer to telegram 'users')\n\
            - Point out red flags in tokenomics\n\
            - Compare to historic failures\n\
            - Create absurd conspiracy theories\n\
            - Mock marketing efforts\n\
            - Question technical implementation\n\
            - Ridicule community demographics\n\
            - Invent fake insider information\n\
            Write ONLY the tweet text with no additional commentary:",
            self.prompt,
            token_info,
        );
    
        // Try generating a response up to 3 times if we get repetitive content
        for attempt in 0..3 {
            let response = self.agent.prompt(&prompt).await?;
            let processed_response = self.ensure_unique_style(response.trim())?;
            
            if attempt == 2 || !self.fud_analysis.is_overused(&processed_response) {
                // Update our analysis with the new content
                self.fud_analysis.update(&processed_response);
                return Ok(processed_response);
            }
            
            if attempt < 2 {
                println!("Generated repetitive FUD, retrying...");
            }
        }
        
        // If we get here, we've failed to generate unique content
        Err(anyhow::anyhow!("Failed to generate unique FUD content"))
    }

    fn ensure_unique_style(&self, response: &str) -> Result<String, anyhow::Error> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();

        // Common patterns to detect and vary
        let common_patterns = [
            "ser", "ngmi", "wen", "just", "literally", "probably",
            "definitely", "obviously", "clearly", "absolutely"
        ];

        let mut processed = response.to_string();

        // Check for overuse of common patterns
        let mut pattern_count = 0;
        for pattern in common_patterns.iter() {
            if processed.to_lowercase().contains(pattern) {
                pattern_count += 1;
            }
        }

        // If too many common patterns, try to replace some
        if pattern_count > 2 {
            // Alternative expressions to mix things up
            let alternatives = vec![
                "looking kinda", "straight up", "ngl", "fr fr",
                "lowkey", "highkey", "certified", "actual"
            ];

            for pattern in common_patterns.iter() {
                if processed.to_lowercase().contains(pattern) && rng.gen_bool(0.7) {
                    if let Some(alt) = alternatives.choose(&mut rng) {
                        processed = processed.replacen(pattern, alt, 1);
                    }
                }
            }
        }

        // Check sentence structure patterns
        let starts_with_common = [
            "another", "just", "ser", "breaking:", "imagine"
        ];

        let starts_common = starts_with_common.iter()
            .any(|&start| processed.to_lowercase().starts_with(start));

        // If it starts with a common pattern, maybe add a variation
        if starts_common && rng.gen_bool(0.6) {
            let variations = [
                "bruh", "certified", "actual", "friendly reminder:",
                "psa:", "reminder:", "daily dose of"
            ];
            if let Some(variation) = variations.choose(&mut rng) {
                processed = format!("{} {}", variation, processed);
            }
        }

        // Add occasional punctuation variation
        if !processed.contains('?') && !processed.contains('!') && rng.gen_bool(0.3) {
            let punctuation = ["..", "...", "!!", "!?", "???"]
                .choose(&mut rng)
                .unwrap();
            processed = format!("{}{}", processed, punctuation);
        }

        Ok(processed)
    }

    pub async fn generate_image(&self) -> Result<String, anyhow::Error> {
        let client = reqwest::Client::builder().build()?;
        dotenv::dotenv().ok();
        let heuris_api = env::var("HEURIS_API")
            .map_err(|_| anyhow::anyhow!("HEURIS_API not found in environment"))?;
        let base_prompt = env::var("IMAGE_PROMPT")
            .map_err(|_| anyhow::anyhow!("IMAGE_PROMPT not found in environment"))?;
        let deadline = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 300;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", heuris_api).parse()?);
        headers.insert("Content-Type", "application/json".parse()?);

        let body = json!({
            "model_input": {
                "SD": {
                    "width": 1024,
                    "height": 1024,
                    "prompt": format!("{}", base_prompt),
                    "neg_prompt": "worst quality, bad quality, umbrella, blurry face, anime, illustration",
                    "num_iterations": 22,
                    "guidance_scale": 7.5
                }
            },
            "model_id": "BluePencilRealistic",
            "deadline": deadline,
            "priority": 1,
            "job_id": format!("job_{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
        });

        
        let request = client
            .request(
                reqwest::Method::POST,
                "http://sequencer.heurist.xyz/submit_job",
            )
            .headers(headers)
            .json(&body);

        let response = request.send().await?;
        let body = response.text().await?;
        Ok(body.trim_matches('"').to_string())
    }

    pub async fn prepare_image_for_tweet(&self, image_url: &str) -> Result<Vec<u8>, anyhow::Error> {
        let client = reqwest::Client::new();
        let response = client.get(image_url).send().await?;

        Ok(response.bytes().await?.to_vec())
    }

    // pub async fn handle_telegram_message(&self, bot: &Bot) {
    //     let client = anthropic::ClientBuilder::new(&self.anthropic_api_key).build();
    //     let bot = bot.clone();
    //     let agent_prompt = self.prompt.clone();
    //     teloxide::repl(bot, move |bot: Bot, msg: Message| {
    //         let agent = client
    //             .agent(CLAUDE_3_HAIKU)
    //             .preamble(&agent_prompt)
    //             .temperature(0.5)
    //             .max_tokens(4096)
    //             .build();
    //         async move {
    //             if let Some(text) = msg.text() {
    //                 let should_respond = msg.chat.is_private() || text.contains("@rina_rig_bot");
                    
    //                 if should_respond {
    //                     let combined_prompt = format!(
    //                         "Task: Generate a conversational reply to this Telegram message while using this as context:\n\
    //                         Message: '{}'\n\
    //                         Generate a natural response that:\n\
    //                         - Is friendly and conversational\n\
    //                         - Can use normal punctuation and capitalization\n\
    //                         - May include emojis when appropriate\n\
    //                         - Maintains a helpful and engaging tone\n\
    //                         - Keeps responses concise but not artificially limited\n\
    //                         Write only the response text, nothing else:",
    //                         text
    //                     );
    //                     let response = agent
    //                         .prompt(&combined_prompt)
    //                         .await
    //                         .expect("Error generating the response");
    //                     println!("Telegram response: {}", response);
    //                     bot.send_message(msg.chat.id, response).await?;
    //                 }
    //             }
    //             Ok(())
    //         }
    //     })
    //     .await;
    // }
}

