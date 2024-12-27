

## TLDR;
AI Agent framework with multi-profile support; 
- Create a new profile in ./characters
- Set API keys in .env (see Installation step #2 below)
- Run a specific character by typing:
  -- Powershell: `$env:CHARACTER_NAME="fud"; cargo run`
  -- Command prompt: `set CHARACTER_NAME=fud; cargo run`

## FudAI
![banner](https://i.ibb.co/fMJfKZB/testsubject125-by-terojako-dieqlzj-pre.jpg)  
  
A Rust-based AI agent implementation featuring [SolanaTracker API](https://www.solanatracker.io/) integration and [rig](https://github.com/0xPlaygrounds/rig) for AI functionality, powering an autonomous social media presence on X.

Follow our AI agent: [@FudAIAgent](https://x.com/FudAIAgent)

# Social Media AI Agent

A Rust-based autonomous social media agent that engages authentically across platforms through consistent personality traits and natural interaction patterns. Built using the rig framework for AI capabilities.

## Capabilities

The agent maintains engaging social media presences through:

**Dynamic Personality Engine**
- Creates consistent interactions through structured personality profiles
- Adapts writing style and topics based on configurable preferences
- Generates unique responses that align with the character's traits

**Automated Social Engagement** 
- Posts original content based on interests and context
- Responds thoughtfully to interactions and mentions
- Maintains natural conversation flows with intelligent filtering
- Introduces random timing delays to mirror human behavior
- Uploads engaging images to enhance posts
- Intelligently pairs visual content with text for maximum impact

**Example tweet media:**  
<img src="https://i.ibb.co/FxqJB0v/crash-chart-472.png" alt="crash" width="200"/>  


**Contextual Memory**
- Records and learns from past interactions
- Builds relationships with other users over time
- Leverages conversation history for relevant responses

**Technical Foundation**
- Full Twitter API v2 integration with built-in rate limiting
- Modular architecture separating core logic from platform specifics
- Extensible design for adding new traits and platform integrations
- Efficient Rust implementation prioritizing performance and reliability

Built for developers looking to create authentic, automated social media presences that engage meaningfully while maintaining consistent personalities.

## Prerequisites

- Rust (latest stable version)  
- API Keys:  
  - Anthropic Claude API access  
  - Twitter API v2 credentials (OAuth 1.0a)  
  - SolanaTracker API  

## Installation

1. Clone the repository:  
   `git clone https://github.com/invntiv/FudAIAgent`  
   `cd FudAIAgent`    

2. Create a `.env` file with required credentials:  
   ANTHROPIC_API_KEY=your_api_key  
   TWITTER_CONSUMER_KEY=your_key  
   TWITTER_CONSUMER_SECRET=your_secret  
   TWITTER_ACCESS_TOKEN=your_token  
   TWITTER_ACCESS_TOKEN_SECRET=your_token_secret  
   CHARACTER_NAME=your_character_name  
   SOLANA_TRACKER_API_KEY=your_solanatracker_api_key   

4. Configure your character:
   - Create a new directory: `characters/{CHARACTER_NAME}/`  
   - Add character definition in `character.json`  

## Character Configuration

Characters are defined using a structured JSON format:

{\
"instructions": {\
"base": "Base character instructions",\
"suffix": "Additional instructions"\
},\
"adjectives": ["trait1", "trait2"],\
"bio": {\
"headline": "Character headline",\
"key_traits": ["trait1", "trait2"]\
},\
"lore": ["background1", "background2"],\
"styles": ["style1", "style2"],\
"topics": ["topic1", "topic2"],\
"post_style_examples": ["example1", "example2"]\
}

## Usage

Run the agent:
`$env:CHARACTER_NAME="{character name}"; cargo run`

## Project Structure

FudAIAgent/\
├── src/\
│ ├── core/ # Core agent functionality\
│ ├── characteristics/ # Character trait implementations\
│ ├── providers/ # External service integrations\
│ └── memory/ # Persistence layer\
├── characters/ # Character definitions\
└── tests/ # Test suite\

## Dependencies

- [rig](https://github.com/0xPlaygrounds/rig) - AI agent framework
- `twitter-v2` - Twitter API client
- `tokio` - Async runtime
- `serde` - Serialization/deserialization
- `anyhow` - Error handling

## Acknowledgments

- [rig](https://github.com/0xPlaygrounds/rig) team for the AI agent framework
- Contributors and maintainers

## Support

For questions and support, please open an issue in the GitHub repository.
