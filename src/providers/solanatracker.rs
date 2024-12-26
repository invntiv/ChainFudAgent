use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue};
use crate::core::agent::Agent;  
use rand::Rng;

#[derive(Debug, Deserialize, Clone)]
pub struct TokenResponse {
    pub token: TokenInfo,
    #[serde(default)]
    pub pools: Vec<Pool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TokenInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub mint: String,
    // Make other fields optional
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Pool {
    #[serde(default)]
    pub price: Price,
    #[serde(default)]
    pub liquidity: Liquidity,
    #[serde(default)]
    pub events: Events,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Liquidity {
    #[serde(default)]
    pub quote: f64,
    #[serde(default)]
    pub usd: f64,
    #[serde(default)]
    pub price: Price,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Price {
    #[serde(default)]
    pub quote: f64,
    #[serde(default)]
    pub usd: f64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MarketCap {
    #[serde(default)]
    pub quote: f64,
    #[serde(default)]
    pub usd: f64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Events {
    #[serde(rename = "24h", default)]
    pub price_change_percentage_24h: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SearchParams {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_liquidity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_liquidity: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_market_cap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_market_cap: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_buys: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_buys: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_sells: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_sells: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_total_transactions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_transactions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lp_burn: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freeze_authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mint_authority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_price_changes: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub status: String,
    pub data: Vec<TokenResponse>,
}

#[derive(Debug, Deserialize)]
pub struct TokenSearchResult {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub mint: String,
    #[serde(default)]
    pub image: Option<String>,
    pub decimals: u8,
    pub quote_token: String,
    pub has_socials: bool,
    pub pool_address: String,
    pub liquidity_usd: f64,
    pub market_cap_usd: f64,
    pub lp_burn: Option<u32>,
    pub market: String,
    pub freeze_authority: Option<String>,
    pub mint_authority: Option<String>,
    pub deployer: Option<String>,
    pub created_at: i64,
    pub status: String,
    pub last_updated: i64,
    pub buys: u32,
    pub sells: u32,
    pub total_transactions: u32,
}


pub struct SolanaTracker {
    api_key: String,
    client: reqwest::Client,
}

impl Price {
    // Function to calculate market cap
    pub fn calculate_market_cap(&self) -> f64 {
        // Assuming shifting decimal is equivalent to multiplying by 10^8
        self.usd * 1e9
    }
}

impl Pool {
    pub fn get_liquidity_usd(&self) -> f64 {
        // Liquidity is stored directly in the pool.liquidity.usd field
        self.liquidity.usd
    }
}

impl SolanaTracker {
    pub fn new(api_key: &str) -> Self {
        SolanaTracker {
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_trending_tokens(&self, timeframe: &str) -> Result<Vec<TokenResponse>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-API-Key",
            HeaderValue::from_str(&self.api_key)?,
        );
        
        let url = format!(
            "https://data.solanatracker.io/tokens/trending/{}", 
            timeframe
        );
        
        println!("Making request to: {}", url);
        
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        println!("Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await?;
            println!("Error response body: {}", error_text);
            return Err(anyhow::anyhow!(
                "API request failed with status: {}. Response: {}", 
                status,
                error_text
            ));
        }

        let body = response.text().await?;
        
        // Try parsing token by token to identify problematic ones
        match serde_json::from_str::<Vec<TokenResponse>>(&body) {
            Ok(tokens) => Ok(tokens),
            Err(e) => {
                println!("Error parsing response: {}", e);
                // Try parsing as Value first to debug
                let v: serde_json::Value = serde_json::from_str(&body)?;
                if let Some(array) = v.as_array() {
                    for (i, token) in array.iter().enumerate() {
                        println!("Token {}: Symbol: {}", i, 
                            token.get("token")
                                .and_then(|t| t.get("symbol"))
                                .and_then(|s| s.as_str())
                                .unwrap_or("unknown")
                        );
                    }
                }
                Err(anyhow::anyhow!("Failed to parse response: {}", e))
            }
        }
    }

    pub async fn get_daily_trending(&self) -> Result<Vec<TokenResponse>> {
        self.get_trending_tokens("5m").await
    }

    pub async fn get_token_by_address(&self, address: &str) -> Result<TokenResponse> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-API-Key",
            HeaderValue::from_str(&self.api_key)?,
        );
        
        let url = format!(
            "https://data.solanatracker.io/tokens/{}", 
            address
        );
        
        println!("Making request to: {}", url);
        
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        println!("Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await?;
            println!("Error response body: {}", error_text);
            return Err(anyhow::anyhow!(
                "API request failed with status: {}. Response: {}", 
                status,
                error_text
            ));
        }

        let body = response.text().await?;
        
        match serde_json::from_str::<TokenResponse>(&body) {
            Ok(token) => Ok(token),
            Err(e) => {
                println!("Error parsing response: {}", e);
                // Try parsing as Value first to debug
                let v: serde_json::Value = serde_json::from_str(&body)?;
                println!("Raw token data: {}", serde_json::to_string_pretty(&v)?);
                Err(anyhow::anyhow!("Failed to parse token response: {}", e))
            }
        }
    }

    pub fn find_token_by_symbol<'a>(tokens: &'a [TokenResponse], symbol: &str) -> Option<&'a TokenResponse> {
        // Get all tokens matching the symbol
        let matching_tokens: Vec<&TokenResponse> = tokens
            .iter()
            .filter(|t| t.token.symbol.eq_ignore_ascii_case(symbol))
            .collect();

        // If no matches, return None
        if matching_tokens.is_empty() {
            return None;
        }

        // If only one match, return it
        if matching_tokens.len() == 1 {
            return Some(matching_tokens[0]);
        }

        // Multiple matches - sort by liquidity and return the highest
        matching_tokens
            .into_iter()
            .max_by(|a, b| {
                let a_liquidity = a.pools.first()
                    .map(|p| p.liquidity.usd)
                    .unwrap_or(0.0);
                let b_liquidity = b.pools.first()
                    .map(|p| p.liquidity.usd)
                    .unwrap_or(0.0);
                a_liquidity.partial_cmp(&b_liquidity).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub async fn token_search(&self, params: SearchParams) -> Result<Vec<TokenResponse>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-API-Key",
            HeaderValue::from_str(&self.api_key)?,
        );
        
        // Simple URL encode function for our known parameter types
        fn encode_param(s: &str) -> String {
            s.replace(" ", "%20")
             .replace("&", "%26")
             .replace("=", "%3D")
             .replace("+", "%2B")
             .replace("#", "%23")
             .replace("?", "%3F")
        }

        // Build query string manually
        let mut query_parts = vec![format!("query={}", encode_param(&params.query))];
        
        if let Some(page) = params.page {
            query_parts.push(format!("page={}", page));
        }
        if let Some(limit) = params.limit {
            query_parts.push(format!("limit={}", limit));
        }
        if let Some(ref sort_by) = params.sort_by {
            query_parts.push(format!("sortBy={}", encode_param(sort_by)));
        }
        if let Some(ref sort_order) = params.sort_order {
            query_parts.push(format!("sortOrder={}", encode_param(sort_order)));
        }
        if let Some(min_liquidity) = params.min_liquidity {
            query_parts.push(format!("minLiquidity={}", min_liquidity));
        }
        if let Some(max_liquidity) = params.max_liquidity {
            query_parts.push(format!("maxLiquidity={}", max_liquidity));
        }
        if let Some(ref freeze_authority) = params.freeze_authority {
            query_parts.push(format!("freezeAuthority={}", encode_param(freeze_authority)));
        }
        if let Some(ref mint_authority) = params.mint_authority {
            query_parts.push(format!("mintAuthority={}", encode_param(mint_authority)));
        }
        
        let url = format!(
            "https://data.solanatracker.io/search?{}", 
            query_parts.join("&")
        );
        
        println!("Making request to: {}", url);
        
        let response = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        println!("Response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await?;
            println!("Error response body: {}", error_text);
            return Err(anyhow::anyhow!(
                "API request failed with status: {}. Response: {}", 
                status,
                error_text
            ));
        }

        let body = response.text().await?;
        
        match serde_json::from_str::<SearchResponse>(&body) {
            Ok(search_response) => Ok(search_response.data),
            Err(e) => {
                println!("Error parsing response: {}", e);
                let v: serde_json::Value = serde_json::from_str(&body)?;
                println!("Raw response data: {}", serde_json::to_string_pretty(&v)?);
                Err(anyhow::anyhow!("Failed to parse search response: {}", e))
            }
        }
    }

    // Make create_search_params take &self to be a method instead of associated function
    pub fn create_search_params(&self, query: String) -> SearchParams {
        SearchParams {
            query,
            page: None,
            limit: None,
            sort_by: None,
            sort_order: None,
            min_liquidity: None,
            max_liquidity: None,
            min_market_cap: None,
            max_market_cap: None,
            min_buys: None,
            max_buys: None,
            min_sells: None,
            max_sells: None,
            min_total_transactions: None,
            max_total_transactions: None,
            lp_burn: None,
            market: None,
            freeze_authority: None,
            mint_authority: None,
            deployer: None,
            show_price_changes: None,
        }
    }

    pub fn format_currency(amount: f64) -> String {
        if amount >= 1_000_000_000.0 {
            format!("${:.1}B", amount / 1_000_000_000.0)
        } else if amount >= 1_000_000.0 {
            format!("${:.1}M", amount / 1_000_000.0)
        } else {
            format!("${:.1}K", amount / 1_000.0)
        }
    }

    pub fn format_token_summary(&self, token: &TokenResponse) -> String {
        let pool = token.pools.first().unwrap();
        
        // Add more varied metrics and data points
        let holder_count = rand::thread_rng().gen_range(10..1000); // Simulated data
        let age_days = rand::thread_rng().gen_range(1..60);
        let transactions_24h = rand::thread_rng().gen_range(5..500);
        
        format!(
            "Token: ${}\n\
             Market Cap: {}\n\
             Liquidity: {}\n",
            token.token.symbol,
            Self::format_currency(pool.price.calculate_market_cap()),
            Self::format_currency(pool.get_liquidity_usd()),
        )
    }
    pub fn format_tokens_summary(&self, tokens: &[TokenResponse], limit: usize) -> String {
        let tokens = &tokens[..tokens.len().min(limit)];
        let mut summary = String::from("🚀💩 Worst Trending Shitcoins on Solana:\n\n");
    
        for (i, token_response) in tokens.iter().enumerate() {
            if let Some(pool) = token_response.pools.first() {
                // Price
                let price_usd = pool.price.usd;
                let price_str = if price_usd > 0.0 {
                    if price_usd >= 1.0 {
                        format!("${:.2}", price_usd)
                    } else if price_usd >= 0.01 {
                        format!("${:.3}", price_usd)
                    } else {
                        format!("${:.8}", price_usd)
                    }
                } else {
                    "N/A".to_string()
                };

                // Market cap
                let mcap = pool.price.calculate_market_cap();
                let mcap_str = if mcap > 0.0 {
                    if mcap >= 1_000_000_000.0 {
                        format!("${:.1}B", mcap / 1_000_000_000.0)
                    } else if mcap >= 1_000_000.0 {
                        format!("${:.1}M", mcap / 1_000_000.0)
                    } else {
                        format!("${:.1}K", mcap / 1_000.0)
                    }
                } else {
                    println!(
                        "Warning: Derived marketCap is zero for token: {}",
                        token_response.token.symbol
                    );
                    "N/A".to_string()
                };
                                    
                // Volume
                let volume_usd = pool.liquidity.usd;
                let volume_str = if volume_usd >= 1_000_000.0 {
                    format!("${:.1}M", volume_usd / 1_000_000.0)
                } else {
                    format!("${:.0}K", volume_usd / 1_000.0)
                };
    
                // Add to summary
                summary.push_str(&format!(
                    "#{} ${}\n💎 MCap: {}\n💵 {}\n💫 Liq: {}\n\n",
                    i + 1,
                    token_response.token.symbol,
                    mcap_str,
                    price_str,
                    volume_str
                ));
            } else {
                println!("Warning: No pools found for token: {}", token_response.token.symbol);
            }
        }
    
        summary.push_str("Data from SolanaTracker 📊");
        summary
    }


    pub async fn get_top_tokens(&self, limit: usize) -> Result<Vec<TokenResponse>> {
        let tokens = self.get_daily_trending().await?;
        Ok(tokens.into_iter().take(limit).collect())
    }

    pub fn generate_fud(&self, token: &TokenResponse) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let fud_intros = [
            "🚨 WARNING: Stay away from ${}! ",
            "${} is the biggest scam I've ever seen. ",
            "🤮 Just looked into ${} and I'm shocked... ",
            "⚠️ ATTENTION: ${} is going to zero! ",
            "${} is absolute garbage! 🗑️",
        ];

        let fud_reasons = [
            "Dev wallet holds 99.9% of supply (trust me bro)",
            "Hawk Tuah team behind this.",
            "Dev is Jewish. Fading.",
            "Website looks like it was made by a retarded 5-year-old",
            "Telegram admin can't spell for shit.",
            "My wife's boyfriend says it's a rugpull",
            "Chart looks like the Titanic's final moments",
            "Devs are probably just three raccoons in a trenchcoat",
            "Obvious scam.",
            "Federal Honeypot.",
            "This one is just clearly NGMI and if you buy it you deserve to be poor.",
            "Smart contract security looks like Swiss cheese",
            "Marketing strategy is just paying Nigerians $1 to spam rocket emojis",
            "Good coin for a 10% gain (waste of time).",
            "Just put the fries in the bag, you'd make more money that way.",
            "Reporting dev to the SEC."
        ];

        let fud_closings = [
            "DYOR but I'm out. 🏃‍♂️",
            "Not financial advice but... run.🚫",
            "Unfollowing anyone who buys this. 🙅‍♂️",
            "Consider yourself warned. ⚠️",
            "Good luck to the bagholders. 🎒",
        ];

        let intro = fud_intros[rng.gen_range(0..fud_intros.len())].replace("{}", &token.token.symbol);
        let reason = fud_reasons[rng.gen_range(0..fud_reasons.len())];
        let closing = fud_closings[rng.gen_range(0..fud_closings.len())];

        if let Some(pool) = token.pools.first() {
            let mcap = pool.price.calculate_market_cap();
            let mcap_str = if mcap > 0.0 {
                if mcap >= 1_000_000_000.0 {
                    format!("${:.1}B", mcap / 1_000_000_000.0)
                } else if mcap >= 1_000_000.0 {
                    format!("${:.1}M", mcap / 1_000_000.0) // Correctly dividing by 1,000,000
                } else {
                    format!("${:.1}K", mcap / 1_000.0) // Correctly dividing by 1,000
                }
            } else {
                "N/A".to_string()
            };
        
            format!(
                "{}\n\n{}\n\nPrice: ${:.8}\nMC: {}\n\n{}", 
                intro,
                reason,
                pool.price.usd,
                mcap_str, // Use the formatted string here
                closing
            )
        } else {
            format!("{}\n\n{}\n\n{}", intro, reason, closing)
        }
    }

    pub fn generate_generic_fud(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let generic_intros = [
            "another day another scam... ",
            "just found the next rugpull lmao ",
            "crypto npc's be like ",
            "solana devs never learn do they ",
            "anon dev starter pack: ",
            "hey guys i found this 'gem' ",
            "your favorite influencer is about to shill ",
            "ser i think we found the bottom ",
            "breaking: local degen loses everything on ",
        ];

        let fud_reasons = [
            "dev wallet holds 99.9% of supply (trust me bro)",
            "hawk tuah team behind this",
            "dev is jewish fading",
            "website looks like it was made by a retarded 5-year-old",
            "telegram admin can't spell for shit",
            "my wife's boyfriend says it's a rugpull",
            "chart looks like the titanic's final moments",
            "devs are probably just three raccoons in a trenchcoat",
            "obvious scam",
            "federal honeypot",
            "this one is just clearly ngmi and if you buy it you deserve to be poor",
            "smart contract security looks like swiss cheese",
            "marketing strategy is just paying nigerians $1 to spam rocket emojis",
            "good coin for a 10% gain (waste of time)",
            "just put the fries in the bag you'd make more money that way",
            "reporting dev to the sec"
        ];

        let generic_closings = [
            "ngmi",
            "have fun staying poor",
            "this is financial advice",
            "not sorry",
            "do better anon",
            "crypto is dead",
            "why are we still here",
            "touch grass",
            "stick to farming airdrops",
            "sir this is a wendy's"
        ];

        // Select random components
        let intro = generic_intros[rng.gen_range(0..generic_intros.len())];
        let reason = fud_reasons[rng.gen_range(0..fud_reasons.len())];
        let closing = generic_closings[rng.gen_range(0..generic_closings.len())];

        // Format them together
        // Using lowercase throughout to match the style and adding some spacing
        format!(
            "{}\n\n{}\n\n{}", 
            intro.to_lowercase().trim(),
            reason.to_lowercase().trim(),
            closing.to_lowercase().trim()
        )
    }

    pub fn get_fud_components(&self) -> (String, String, String) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let generic_intros = [
            "another day another scam...",
            "just found the next rugpull lmao",
            "crypto npc's be like",
            "solana devs never learn do they",
            "anon dev starter pack:",
            "hey guys i found this 'gem'",
            "your favorite influencer is about to shill",
            "ser i think we found the bottom",
            "breaking: local degen loses everything on",
            "just watched a youtuber explain why",
            "telegram group admin swears",
            "my technical analysis shows",
            "sources familiar with the matter say",
            "trust me bro update:",
            "weekly rugpull report:"
        ];

        let fud_reasons = [
            "dev wallet holds 99.9% of supply (trust me bro)",
            "hawk tuah team behind this",
            "dev is jewish fading",
            "website looks like it was made by a retarded 5-year-old",
            "telegram admin can't spell for shit",
            "my wife's boyfriend says it's a rugpull",
            "chart looks like the titanic's final moments",
            "devs are probably just three raccoons in a trenchcoat",
            "obvious scam",
            "federal honeypot",
            "this one is just clearly ngmi and if you buy it you deserve to be poor",
            "smart contract security looks like swiss cheese",
            "marketing strategy is just paying nigerians $1 to spam rocket emojis",
            "good coin for a 10% gain (waste of time)",
            "just put the fries in the bag you'd make more money that way",
            "reporting dev to the sec"
        ];

        let generic_closings = [
            "ngmi",
            "have fun staying poor",
            "this is financial advice",
            "not sorry",
            "do better anon",
            "crypto is dead",
            "why are we still here",
            "touch grass",
            "stick to farming airdrops",
            "sir this is a wendy's",
            "back to mcdonalds",
            "delete your wallet",
            "probably nothing",
            "wagmi (we are gonna miss income)",
            "certified shitcoin moment"
        ];

        // Select random components
        let intro = generic_intros[rng.gen_range(0..generic_intros.len())];
        let reason = fud_reasons[rng.gen_range(0..fud_reasons.len())];
        let closing = generic_closings[rng.gen_range(0..generic_closings.len())];

        (
            intro.to_string(),
            reason.to_string(),
            closing.to_string()
        )
    }

    // This is a helper method to add emojis to the final response
    fn add_emojis(response: String) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let emoji_sets = [
            "💀",
            "🤡",
            "🚮",
            "🗑️",
            "⚰️",
            "🤮",
            "🚨",
            "⚠️",
            "🤢",
            "💩",
        ];

        // Add 1-2 random emojis
        let num_emojis = rng.gen_range(1..=2);
        let mut final_response = response;
        
        for _ in 0..num_emojis {
            let emoji = emoji_sets[rng.gen_range(0..emoji_sets.len())];
            if rng.gen_bool(0.5) {
                final_response = format!("{} {}", emoji, final_response);
            } else {
                final_response = format!("{} {}", final_response, emoji);
            }
        }

        final_response
    }

    pub async fn generate_generic_fud_with_agent(&self, agent: &Agent) -> Result<String, anyhow::Error> {
        // Get random components
        let (intro, reason, closing) = self.get_fud_components();
        
        // Generate AI response using the components
        let response = agent.generate_generic_fud(&intro, &reason, &closing).await?;
        
        // Add emojis to the final response
        Ok(Self::add_emojis(response))
    }
}