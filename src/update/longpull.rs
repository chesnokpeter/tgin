use crate::utils::fun::{hide_segment, mask_secret};

use crate::base::{Serverable, Printable};
use crate::update::base::Updater;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tokio::sync::mpsc::Sender;
use tokio::time::{sleep, Duration};

pub struct LongPollUpdate {
    client: Client,
    url: String,
    default_timeout_sleep: u64,
    error_timeout_sleep: u64,
    token: String
}

impl LongPollUpdate {
    pub fn new(token: String) -> Self {
        Self {
            client: Client::new(),
            url: format!("https://api.telegram.org/bot{}/getUpdates", token),
            default_timeout_sleep: 100,
            error_timeout_sleep: 200,
            token
        }
    }

    pub fn set_client(&mut self, client: Client) {
        self.client = client;
    }

    pub fn set_url(&mut self, url: String) {
        self.url = url;
    }

    pub fn set_timeouts(&mut self, default_timeout_sleep: u64, error_timeout_sleep: u64) {
        self.default_timeout_sleep = default_timeout_sleep;
        self.error_timeout_sleep = error_timeout_sleep;
    }
}

#[async_trait]
impl Updater for LongPollUpdate {
    async fn start(&self, tx: Sender<Value>) {
        let mut offset = 0;

        loop {
            let params = [("offset", offset.to_string()), ("timeout", "30".to_string())];
            match self.client.get(&self.url).query(&params).send().await {
                Ok(res) => {
                    match res.json::<Value>().await {
                        Ok(json) => {
                            if let Some(result) = json.get("result").and_then(|r| r.as_array()) {
                                for update in result {
                                    if let Some(id) = update.get("update_id").and_then(|i| i.as_i64()) {
                                        offset = id + 1;
                                        if tx.send(update.clone()).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                            sleep(Duration::from_millis(self.default_timeout_sleep)).await;
                        }
                        Err(err) => {
                            eprintln!("JSON parse error: {:?}", err);
                            sleep(Duration::from_millis(self.error_timeout_sleep)).await;
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Network error: {:?}", err);

                    sleep(Duration::from_millis(self.error_timeout_sleep)).await;
                }
            }
        }
    }
}

impl Serverable for LongPollUpdate {}



impl Printable for LongPollUpdate {
    fn print(&self) -> String {

        let timeout_text = if self.default_timeout_sleep == 100 && self.error_timeout_sleep == 200 {format!("timeouts: {} {}", self.default_timeout_sleep, self.error_timeout_sleep)} else {"".to_string()};

        format!("longpull: {} {}", mask_secret(&hide_segment(&self.url), &self.token), timeout_text)
    }
}