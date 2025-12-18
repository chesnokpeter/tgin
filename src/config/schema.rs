use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TginConfig {
    #[serde(default = "default_workers")]
    pub dark_threads: usize,
    pub server_port: Option<u16>,
    #[serde(default)]
    pub ssl: Option<SslConfig>,
    pub updates: Vec<UpdateConfig>,
    pub route: RouteConfig,
    pub api: Option<ApiConfig>,
}

fn default_workers() -> usize {
    4
}

#[derive(Deserialize, Debug)]
pub struct SslConfig {
    pub cert: String,
    pub key: String,
}

#[derive(Deserialize, Debug)]
pub struct ApiConfig {
    pub base_path: String,
}

#[derive(Deserialize, Debug)]
pub enum UpdateConfig {
    LongPollUpdate {
        token: String,
        url: Option<String>,
        #[serde(default = "default_timeout")]
        default_timeout_sleep: u64,
        #[serde(default = "default_timeout")]
        error_timeout_sleep: u64,
    },
    WebhookUpdate {
        path: String,
        registration: Option<RegistrationWebhookConfig>,
    },
}

fn default_timeout() -> u64 {
    100
}

#[derive(Deserialize, Debug)]
pub struct RegistrationWebhookConfig {
    pub public_ip: String,
    pub set_webhook_url: Option<String>,
    pub token: String,
}
#[derive(Deserialize, Debug)]
pub enum RouteStrategyConfig {
    RoundRobinLB { routes: Vec<RouteConfig> },
    AllLB { routes: Vec<RouteConfig> },
}





#[derive(Deserialize, Debug)]
pub enum RouteConfig {
    LongPollRoute { path: String },
    WebhookRoute { url: String },
    
    RoundRobinLB { routes: Vec<RouteConfig> },
    AllLB { routes: Vec<RouteConfig> },
}