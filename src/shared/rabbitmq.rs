use std::sync::Arc;
use tokio::sync::OnceCell; // Обрати внимание: OnceCell из tokio, не Mutex
use async_trait::async_trait;
use lapin::{Connection, ConnectionProperties};
use crate::base::Runnable;

#[derive(Clone)]
pub struct RabbitmqBroker {
    url: String,
    connection: Arc<OnceCell<Arc<Connection>>>,
}

impl RabbitmqBroker {
    pub fn new(url: String) -> Self {
        Self {
            url,
            connection: Arc::new(OnceCell::new()),
        }
    }

    pub async fn get_connection(&self) -> Arc<Connection> {
        let url = self.url.clone();
        
        let conn_arc = self.connection.get_or_init(|| async move {
            let conn = Connection::connect(&url, ConnectionProperties::default())
                .await
                .expect("Failed to connect to RabbitMQ");
            
            Arc::new(conn)
        }).await;

        conn_arc.clone()
    }
}

#[async_trait]
impl Runnable for RabbitmqBroker {
    async fn run(&self) {
        let _ = self.get_connection().await;
    }
}