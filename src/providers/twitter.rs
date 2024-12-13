use twitter_v2::{authorization::Oauth1aToken, TwitterApi, id::IntoNumericId};

pub struct Twitter {
    auth: Oauth1aToken,
}
impl Twitter {
    pub fn new(
        twitter_consumer_key: &str,
        twitter_consumer_secret: &str,
        twitter_access_token: &str,
        twitter_access_token_secret: &str,
    ) -> Self {
        let auth = Oauth1aToken::new(
            twitter_consumer_key.to_string(),
            twitter_consumer_secret.to_string(),
            twitter_access_token.to_string(),
            twitter_access_token_secret.to_string(),
        );

        Twitter { auth }
    }

    pub async fn tweet(&self, text: String) -> Result<(), anyhow::Error> {
        let tweet = TwitterApi::new(self.auth.clone())
            .post_tweet()
            .text(text)
            .send()
            .await?
            .into_data()
            .expect("this tweet should exist");
        println!("Tweet posted successfully with ID: {}", tweet.id);

        Ok(())
    }

    pub async fn reply_to_tweet(&self, tweet_id: &str, text: String) -> Result<(), anyhow::Error> {
        let tweet_id = tweet_id.parse::<u64>()?;
        let tweet = TwitterApi::new(self.auth.clone())
            .post_tweet()
            .in_reply_to_tweet_id(tweet_id)
            .text(text)
            .send()
            .await?
            .into_data()
            .expect("this tweet should exist");
        println!("Reply posted successfully with ID: {}", tweet.id);

        Ok(())
    }
    
    pub async fn get_notifications(&self, user_id: impl IntoNumericId) -> Result<Vec<twitter_v2::Tweet>, anyhow::Error> {
        let api = TwitterApi::new(self.auth.clone());
        let mentions = api
            .get_user_mentions(user_id)
            .send()
            .await?
            .into_data()
            .unwrap_or_default();

        Ok(mentions)
    }

    pub async fn get_user_id(&self) -> Result<impl IntoNumericId, anyhow::Error> {
        let api = TwitterApi::new(self.auth.clone());
        let me = api.get_users_me()
            .send()
            .await?
            .into_data()
            .expect("should have user data");
        
        Ok(me.id)
    }
}
