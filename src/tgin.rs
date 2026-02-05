use tokio::task::JoinHandle;
use tokio::sync::mpsc;


use crate::base::{Ingress, Egress, Envelope, Runnable, Forwardable};



pub struct Tgin<E> {
    egress: E,
    tasks: Vec<JoinHandle<()>>,


}

impl<E> Tgin<E> 
where E: Clone + Send + Sync + 'static
{
    pub fn new(egress: E) -> Self {
        Self {
            egress,
            tasks: Vec::new(),
        }
    }

    pub fn add_run<R>(&mut self, resource: R)
    where 
        R: Runnable + 'static 
    {
        let handle = tokio::spawn(async move {
            resource.run().await;
        });
        self.tasks.push(handle);
    }


    pub fn add_ingress<Ing, I, O>(&mut self, ingress: Ing) 
    where
        Ing: Ingress<I, O> + Send + Sync + 'static,
        I: Send + Sync + 'static,
        O: Send + Sync + 'static,
        Envelope<I, O>: Forwardable<E>, 
    {
        let (tx, mut rx) = mpsc::channel::<Envelope<I, O>>(1_000_000);
        let egress_clone = self.egress.clone();

        let t1 = tokio::spawn(async move {
            ingress.start(tx).await;
        });

        let t2 = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let e = egress_clone.clone();
                tokio::spawn(async move {
                    msg.process(&e).await;
                });
            }
        });

        self.tasks.push(t1);
        self.tasks.push(t2);
    }


    pub async fn run(self) {
        futures::future::join_all(self.tasks).await;
    }


}