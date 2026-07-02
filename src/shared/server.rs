use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use axum::Router;

use crate::base::Runnable;

struct Inner {
    addr: String,
    router: Mutex<Router>,
}

#[derive(Clone)]
pub struct HttpServer {
    inner: Arc<Inner>,
}

impl HttpServer {
    pub fn new(addr: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                addr: addr.to_string(),
                router: Mutex::new(Router::new()),
            }),
        }
    }

    pub async fn register(&self, route: Router) {
        let mut router = self.inner.router.lock().await;
        let current = std::mem::take(&mut *router);
        *router = current.merge(route);
    }

    pub async fn serve(&self, shutdown: CancellationToken) {
        let listener = TcpListener::bind(&self.inner.addr)
            .await
            .expect("HttpServer: cannot bind address");

        let router = self.inner.router.lock().await.clone();
        let app = router.into_make_service_with_connect_info::<SocketAddr>();

        axum::serve(listener, app)
            .with_graceful_shutdown(async move { shutdown.cancelled().await })
            .await
            .unwrap();

        *self.inner.router.lock().await = Router::new();
    }
}

#[async_trait]
impl Runnable for HttpServer {
    fn id(&self) -> Option<usize> {
        Some(Arc::as_ptr(&self.inner) as usize)
    }

    async fn run(&self, shutdown: CancellationToken) {
        self.serve(shutdown).await;
    }
}
