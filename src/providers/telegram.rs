use teloxide::Bot;

pub struct Telegram {
    pub bot: Bot,
}

impl Telegram {
    pub fn new(token: &str) -> Self {
        Telegram {
            bot: Bot::new(token),
        }
    }
}