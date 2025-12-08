use crate::base::{UpdaterComponent, RouteableComponent};
use axum::Router;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;

use std::net::SocketAddr;
use axum_server::tls_rustls::RustlsConfig;

use tokio::runtime::Builder; 

pub struct Tgin {
    updates: Vec<Box<dyn UpdaterComponent>>,
    route: Arc<dyn RouteableComponent>,
    dark_threads: usize,
    server_port: Option<u16>,

    pub ssl_cert: Option<String>,
    pub ssl_key: Option<String>,  
}

impl Tgin {
    pub fn new(
        updates: Vec<Box<dyn UpdaterComponent>>,
        route: Arc<dyn RouteableComponent>,
        dark_threads: usize,
        server_port: Option<u16>,
    ) -> Self {
        Self {
            updates,
            route,
            dark_threads,
            server_port,
            ssl_cert: None,
            ssl_key: None
        }
    }

    pub fn set_tsl(&mut self, ssl_cert: String, ssl_key: String) {
        self.ssl_cert = Some(ssl_cert);
        self.ssl_key = Some(ssl_key);
    }


    pub fn run(self) {
        println!("STARTED TGIN with {} worker threads\n", &self.dark_threads);

        println!("CATCH UPDATES FROM\n");

        for update in &self.updates {
            println!("{}\n", update.print());
        }

        println!("\nRUTE TO\n");

        println!("{}", &self.route.print());

        let runtime = Builder::new_multi_thread()
            .worker_threads(self.dark_threads)
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime");

        runtime.block_on(self.run_async());
    }


    pub async fn run_async(self) {
        let (tx, mut rx) = mpsc::channel::<Value>(10000);

        if let Some(port) = self.server_port {
            let mut router: Router<Sender<Value>> = Router::new();

            for provider in &self.updates {
                router = provider.set_server(router);
            }

            router = self.route.set_server(router);

            let app = router.with_state(tx.clone());
            let addr = SocketAddr::from(([0, 0, 0, 0], port));

            match (self.ssl_cert.clone(), self.ssl_key.clone()) {
                (Some(cert_path), Some(key_path)) => {
                    let config = RustlsConfig::from_pem_file(
                        cert_path, 
                        key_path
                    )
                    .await
                    .expect("Failed to load SSL certificates");

                    tokio::spawn(async move {
                        axum_server::bind_rustls(addr, config)
                            .serve(app.into_make_service())
                            .await
                            .unwrap();
                    });
                }
                _ => {
                    tokio::spawn(async move {
                        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
                        axum::serve(listener, app).await.unwrap();
                    });
                }
            }

        }

        for provider in self.updates {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                provider.start(tx_clone).await;
            });
        }
        
        drop(tx);

        while let Some(update) = rx.recv().await {
            let route_clone = self.route.clone();
            tokio::spawn(async move {
                route_clone.process(update).await;
            });
        }
    }
    
}