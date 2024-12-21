// src/providers/tests/solanatracker_tests.rs

use super::super::solanatracker::{SolanaTracker, TokenResponse, TokenInfo, Pool, Liquidity};

#[test]
fn test_find_token_by_symbol() {
    // Create test data
    let tokens = vec![
        TokenResponse {
            token: TokenInfo { 
                symbol: "TEST".to_string(), 
                name: "Test Token 1".to_string(),
                mint: "mint1".to_string(),
                uri: None,
                description: None,
            },
            pools: vec![Pool {
                liquidity: Liquidity { 
                    usd: 1000.0, 
                    quote: 0.0, 
                    price: Default::default() 
                },
                price: Default::default(),
                events: Default::default(),
            }]
        },
        TokenResponse {
            token: TokenInfo { 
                symbol: "TEST".to_string(), 
                name: "Test Token 2".to_string(),
                mint: "mint2".to_string(),
                uri: None,
                description: None,
            },
            pools: vec![Pool {
                liquidity: Liquidity { 
                    usd: 5000.0, 
                    quote: 0.0, 
                    price: Default::default() 
                },
                price: Default::default(),
                events: Default::default(),
            }]
        },
    ];

    let result = SolanaTracker::find_token_by_symbol(&tokens, "TEST");
    assert!(result.is_some(), "Should find a token");
    
    let found_token = result.unwrap();
    assert_eq!(
        found_token.pools[0].liquidity.usd,
        5000.0,
        "Should return token with highest liquidity"
    );

    // Test case insensitivity
    let lowercase_result = SolanaTracker::find_token_by_symbol(&tokens, "test");
    assert!(lowercase_result.is_some(), "Should find token with case-insensitive search");

    // Test non-existent token
    let no_result = SolanaTracker::find_token_by_symbol(&tokens, "NONEXISTENT");
    assert!(no_result.is_none(), "Should return None for non-existent token");
}

#[test]
fn test_find_token_empty_pools() {
    let tokens = vec![
        TokenResponse {
            token: TokenInfo { 
                symbol: "TEST".to_string(), 
                name: "Test Token".to_string(),
                mint: "mint1".to_string(),
                uri: None,
                description: None,
            },
            pools: vec![] // Empty pools
        },
    ];

    let result = SolanaTracker::find_token_by_symbol(&tokens, "TEST");
    assert!(result.is_some(), "Should find token even with empty pools");
}