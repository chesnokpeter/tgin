use crate::base::{RouteableComponent, UpdaterComponent};
use crate::lb::{roundrobin::RoundRobinLB, all::AllLB};
use crate::route::longpull::LongPollRoute;
use crate::route::webhook::WebhookRoute;
use crate::update::longpull::LongPollUpdate;
use crate::update::webhook::{WebhookUpdate, RegistrationWebhookConfig};
use crate::config::schema::{TginConfig, UpdateConfig, RouteStrategyConfig, RouteConfig};

use std::sync::Arc;
use std::fs;

use std::env;
use regex::Regex;

pub fn load_config(path: &str) -> TginConfig {
    let content = fs::read_to_string(path).expect("Failed to read config file");
    let processed_content = substitute_env_vars(&content);

    ron::from_str(&processed_content).expect("Failed to parse RON config")
}


fn substitute_env_vars(input: &str) -> String {
    let re = Regex::new(r"\$\{(\w+)\}").unwrap();

    re.replace_all(input, |caps: &regex::Captures| {
        let var_name = &caps[1];
        
        match env::var(var_name) {
            Ok(val) => val,
            Err(_) => {
                panic!("Environment variable '${}' is not set", var_name);
            }
        }
    }).to_string()
}

pub fn build_updates(configs: Vec<UpdateConfig>) -> Vec<Box<dyn UpdaterComponent>> {
    let mut result: Vec<Box<dyn UpdaterComponent>> = Vec::new();

    for cfg in configs {
        match cfg {
            UpdateConfig::LongPollUpdate { token, url, default_timeout_sleep, error_timeout_sleep } => {
                let mut up = LongPollUpdate::new(token);
                if let Some(u) = url {
                    up.set_url(u); 
                }
                up.set_timeouts(default_timeout_sleep, error_timeout_sleep); 
                result.push(Box::new(up));
            }
            UpdateConfig::WebhookUpdate { path, registration } => {
                let mut up = WebhookUpdate::new(path);
                if let Some(reg) = registration {
                    // up = up.with_registration(reg.token, reg.public_ip);
                }
                result.push(Box::new(up));
            }
        }
    }
    result
}

pub fn build_route(cfg: RouteConfig) -> Arc<dyn RouteableComponent> {
    match cfg {
        // Базовые роуты
        RouteConfig::LongPollRoute { path } => {
            Arc::new(LongPollRoute::new(path))
        }
        RouteConfig::WebhookRoute { url } => {
            Arc::new(WebhookRoute::new(url))
        }
        
        // Рекурсивная сборка RoundRobin
        RouteConfig::RoundRobinLB { routes } => {
            let built_routes: Vec<Arc<dyn RouteableComponent>> = routes
                .into_iter()
                .map(build_route) // Вызываем сами себя для каждого под-элемента
                .collect();
            
            Arc::new(RoundRobinLB::new(built_routes))
        }
        
        // Рекурсивная сборка AllLB
        RouteConfig::AllLB { routes } => {
            let built_routes: Vec<Arc<dyn RouteableComponent>> = routes
                .into_iter()
                .map(build_route) // Вызываем сами себя
                .collect();

            Arc::new(AllLB::new(built_routes))
        }
    }
}