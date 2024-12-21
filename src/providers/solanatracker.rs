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
        self.get_trending_tokens("24h").await
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

    pub fn format_token_summary(&self, token: &TokenResponse) -> String {
        let pool = token.pools.first().unwrap();

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
                token.token.symbol  // Fixed variable reference
            );
            "N/A".to_string()
        };

        let liquidity = pool.get_liquidity_usd();
        let liquidity_formatted = if liquidity >= 1_000_000.0 {
            format!("${:.1}M", liquidity / 1_000_000.0)
        } else if liquidity >= 1_000.0 {
            format!("${:.1}K", liquidity / 1_000.0)
        } else {
            format!("${:.2}", liquidity)
        };

        let price = pool.price.usd;
        let price_formatted = if price >= 1.0 {
            format!("${:.2}", price)
        } else {
            format!("${:.8}", price)
        };

        format!(
            "Token: ${}\nMarket cap: {}\nPrice: {}\nLiquidity: {}\n24h Change: {}%",
            token.token.symbol,
            mcap_str,  // Added comma here
            price_formatted,
            liquidity_formatted,
            pool.events.price_change_percentage_24h.map_or("N/A".to_string(), |c| format!("{:.1}", c))
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
}