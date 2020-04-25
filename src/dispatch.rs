use tokio::task;
use std::sync::Arc;

pub trait Worker {
    type Config;
    fn new(id: usize, cfg: Arc<Self::Config>) -> Self;
    fn start(self) -> task::JoinHandle<()>;
}

pub struct Dispatcher<W> 
where
W: Worker{
    workers: Vec<task::JoinHandle<()>>,
    worker_config: Arc<W::Config>,
}

impl<W>  Dispatcher<W> 
where
W: Worker{
    pub fn new(worker_config: W::Config) -> Self{
        Self {
            workers: Vec::new(),
            worker_config: Arc::from(worker_config),
        }
    }
    pub fn start_workers(&mut self, threads: usize){
        for i in 0..threads {
            let cfg = self.worker_config.clone();
            let worker = W::new(i, cfg);
            let start_worker_task = worker.start();
            self.workers.push(start_worker_task);
        }
    }

    pub async fn finish_and_wait(self) -> Result<(), Box<dyn std::error::Error>> {
        for w in self.workers {
            w.await?;
        }
        Ok(())
    }
}