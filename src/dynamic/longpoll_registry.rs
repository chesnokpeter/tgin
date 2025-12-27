use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;


use crate::route::longpull::LongPollRoute;

pub static LONGPOLL_REGISTRY: Lazy<RwLock<HashMap<String, Arc<LongPollRoute>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

