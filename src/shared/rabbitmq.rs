use std::sync::Arc;

use tokio::sync::Mutex;
use lapin::{Connection, ConnectionProperties};

struct Inner {
    url: String,
    connection: Mutex<Option<Arc<Connection>>>,
}

#[derive(Clone)]
pub struct Rabbit {
    inner: Arc<Inner>,
}

impl Rabbit {
    pub fn new(url: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                url: url.to_string(),
                connection: Mutex::new(None),
            }),
        }
    }

    pub async fn connection(&self) -> Option<Arc<Connection>> {
        let mut guard = self.inner.connection.lock().await;

        if let Some(connection) = guard.as_ref() {
            if connection.status().connected() {
                return Some(connection.clone());
            }
        }

        let connection = Arc::new(
            Connection::connect(&self.inner.url, ConnectionProperties::default())
                .await
                .ok()?,
        );
        *guard = Some(connection.clone());
        Some(connection)
    }
}
