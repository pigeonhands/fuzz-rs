use log::{debug, error, info, trace, warn};
use reqwest;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::dispatch::Worker;
use tokio::task;

pub struct WorkerConfig {
    pub client: reqwest::Client,
    pub target: reqwest::Url,
    pub extentions: Vec<String>,
    pub ignore_codes: Vec<u16>,
    pub rx: Mutex<mpsc::Receiver<String>>,
    pub expand_url_log: bool,
    pub print_fails: bool,
}

pub struct HttpDirWorker {
    id: i32,
    config: Arc<WorkerConfig>,
}

impl Worker for HttpDirWorker {

    type Config = WorkerConfig;
    fn new(id: usize, cfg: Arc<Self::Config>) -> Self {
        Self {
            id: id as i32,
            config: cfg,
        }
    }

    fn start(self) -> task::JoinHandle<()>{
        task::spawn(self.execute())
    }
}


impl HttpDirWorker {

    pub async fn execute(self) {
        trace!("Worker {} started.", self.id);

        loop {
            let word = {
                let mut rx = self.config.rx.lock().await;
                match rx.recv().await {
                    Some(e) => e,
                    None => break,
                }
            };

            let _ = self.process_word(&word).await;
        }

        trace!("Worker {} ended.", self.id);
    }

    async fn process_word(&self, word: &str) -> Result<(), ()> {
        let base_url = self
            .config
            .target
            .join(&word)
            .map_err(|_| error!("Invalid word for a url: {}", &word))?;

        let default_suffixes = vec![&word];
        let default_suffixes_iter = default_suffixes.iter().map(|e| e.to_string());
        let word_with_extentions = self
            .config
            .extentions
            .iter()
            .map(|e| String::from(&[word, e.as_str().trim()].concat()));

        for suffix in default_suffixes_iter.chain(word_with_extentions) {
            let url = base_url.join(&suffix).map_err(|_| {
                error!("Invalid extention for a url: {}", &suffix);
            })?;

            debug!("{}] baseurl {}", self.id, &base_url.as_str());
            debug!("{}] Sending request to {}", self.id, &url.as_str());

            let resp = self.config.client.get(url).send().await.map_err(|e| {
                debug!("{}] Error while performing request. {}.", self.id, e);
                warn!("Error while performing request..");
            })?;

            let status_code = resp.status().as_u16();
            if self.config.ignore_codes.contains(&status_code) {
                trace!("Ignoring because {} is in ignore list.", status_code);
                continue;
            }

            debug!("{}] Request responded with {:?}", self.id, resp.status());
            if resp.status().is_success() {
                if self.config.expand_url_log {
                    info!("OK {} {}", status_code, resp.url().as_str());
                } else {
                    info!("OK {} /{}", status_code, &suffix);
                };
            } else if self.config.print_fails {
                if self.config.expand_url_log {
                    warn!("ERR {} {}", status_code, resp.url().as_str());
                } else {
                    warn!("ERR {} /{}", status_code, &suffix);
                };
            }
        }
        Ok(())
    }
}
