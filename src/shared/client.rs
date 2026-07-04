use std::time::Duration;

use reqwest::Client;
use reqwest::redirect::Policy;

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    pub fn new() -> Self {
        Self::builder().build()
    }

    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder { builder: Client::builder().redirect(Policy::none()) }
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }
}

pub struct HttpClientBuilder {
    builder: reqwest::ClientBuilder,
}

impl HttpClientBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.builder = self.builder.timeout(timeout);
        self
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.builder = self.builder.connect_timeout(timeout);
        self
    }

    pub fn pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.builder = self.builder.pool_max_idle_per_host(max);
        self
    }

    pub fn user_agent(mut self, user_agent: &str) -> Self {
        self.builder = self.builder.user_agent(user_agent.to_string());
        self
    }

    pub fn build(self) -> HttpClient {
        HttpClient {
            client: self.builder.build().expect("HttpClient: invalid configuration"),
        }
    }
}
