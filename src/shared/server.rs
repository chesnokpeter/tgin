use tokio::net::TcpListener; 
use axum::Router;



pub struct HttpServer {
    host: String, 
    port: u16,
    router: Router
}

impl HttpServer {
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host: host.clone(),
            port,
            router: Router::new()
        }
    }

    pub fn register_route(&mut self, new_route: Router) {
        let old_router = std::mem::take(&mut self.router);
        self.router = old_router.merge(new_route);
    }

    pub async fn run(&self) {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).await.expect("You can not use HttpServer for this host and port");

        axum::serve(listener, self.router.clone()).await.unwrap();
    }
}