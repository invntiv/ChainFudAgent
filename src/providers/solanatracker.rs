use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue};

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
    pub market: String,
    #[serde(default)]
    pub liquidity: Liquidity,
    #[serde(default)]
    pub events: Events,
    #[serde(default)]
    pub market_cap: MarketCap,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MarketCap {
    pub quote: f64,
    pub usd: f64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Events {
    #[serde(rename = "24h", default)]
    pub change_24h: f64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Price {
    #[serde(default)]
    pub quote: f64,
    #[serde(default)]
    pub usd: f64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Liquidity {
    #[serde(default)]
    pub quote: f64,
    #[serde(default)]
    pub usd: f64,
}

pub struct SolanaTracker {
    api_key: String,
    client: reqwest::Client,
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
        self.get_trending_tokens("24h").await
    }

    pub fn format_tokens_summary(&self, tokens: &[TokenResponse], limit: usize) -> String {
        let tokens = &tokens[..tokens.len().min(limit)];
        let mut summary = String::from("💩 Trending Shitcoins on Solana:\n\n");
        
        for (i, token_response) in tokens.iter().enumerate() {
            if let Some(pool) = token_response.pools.first() {
                // Format price based on its value
                let price_str = if pool.price.usd >= 1.0 {
                    format!("${:.2}", pool.price.usd)
                } else if pool.price.usd >= 0.01 {
                    format!("${:.3}", pool.price.usd)
                } else {
                    format!("${:.8}", pool.price.usd)
                };

                // Format volume
                let volume = if pool.liquidity.usd >= 1_000_000.0 {
                    format!("${:.1}M", pool.liquidity.usd / 1_000_000.0)
                } else {
                    format!("${:.0}K", pool.liquidity.usd / 1_000.0)
                };

                // Format 24h change with emoji
                let change_24h = pool.events.change_24h;
                let change_emoji = if change_24h >= 0.0 { "📈" } else { "📉" };
                let change_str = format!("{}{:.1}%", change_emoji, change_24h);

                // Format market cap
                let mcap = if pool.market_cap.usd >= 1_000_000_000.0 {
                    format!("${:.1}B", pool.market_cap.usd / 1_000_000_000.0)
                } else {
                    format!("${:.1}M", pool.market_cap.usd / 1_000_000.0)
                };

                summary.push_str(&format!(
                    "#{} ${}\n💎 MCap: {}\n💵 {}\n💫 Vol: {}\n{}\n\n",
                    i + 1,
                    token_response.token.symbol,
                    mcap,
                    price_str,
                    volume,
                    change_str
                ));
            }
        }
        
        summary.push_str("Data from SolanaTracker 📊");
        summary
    }

    pub async fn get_top_tokens(&self, limit: usize) -> Result<Vec<TokenResponse>> {
        let tokens = self.get_daily_trending().await?;
        Ok(tokens.into_iter().take(limit).collect())
    }
}