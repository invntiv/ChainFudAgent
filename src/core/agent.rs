use rig::agent::Agent as RigAgent;
use rig::providers::anthropic::{self, CLAUDE_3_5_SONNET};
use rig::completion::Prompt;
use rig::providers::anthropic::completion::CompletionModel;

pub struct Agent {
    agent: RigAgent<CompletionModel>,
}

#[derive(Debug, PartialEq)]
pub enum ResponseDecision {
    Respond,
    Ignore,
}

impl Agent {
    pub fn new(anthropic_api_key: &str, prompt: &str) -> Self {
        let client = anthropic::ClientBuilder::new(anthropic_api_key).build();
        let agent = client
            .agent(CLAUDE_3_5_SONNET)
            .preamble(prompt)
            .temperature(0.5)
            .max_tokens(8192)
            .build();
        Agent { agent }
        
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
            - Is direct and possibly sarcastic\n\
            - Stays under 280 characters\n\
            Write only the response text, nothing else:",
            tweet
        );
        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }

    pub async fn generate_post(&self) -> Result<String, anyhow::Error> {
        let prompt = r#"# Task: Write a Social Media Post
            Write a 1-3 sentence post from your perspective that would be engaging to readers. Keep it casual and friendly in tone. Stay under 280 characters.

            Requirements:
            - Write only the post content, no additional commentary
            - No emojis
            - No hashtags
            - No questions
            - Brief, concise statements only"#;
        let response = self.agent.prompt(&prompt).await?;
        Ok(response.trim().to_string())
    }
}
