use crate::utils;
use crate::CommonArgs;

use log::{info, warn};
use reqwest;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;

use super::{HttpDirConfig, worker::HttpDirWorker, WorkerConfig};
use crate::dispatch::{Dispatcher};

pub struct HttpDirScanner {
    common_args: CommonArgs,
    cfg: HttpDirConfig,
    client: reqwest::Client,
    extentions: Vec<String>,
    ignore_codes: Vec<u16>,
}

impl HttpDirScanner {
    pub async fn run_new(
        common_args: CommonArgs,
        cfg: HttpDirConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut s = Self::new(common_args, cfg)?;
        s.run().await
    }

    pub fn new(
        common_args: CommonArgs,
        cfg: HttpDirConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let extentions = utils::get_extention_list(&cfg.extentions, cfg.default_extentions);

        let ignore_codes = if let Some(ignore) = &cfg.ignore_codes {
            ignore
                .iter()
                .map(|i| i.parse::<u16>())
                .filter_map(Result::ok)
                .collect()
        } else {
            Vec::new()
        };

        Ok(HttpDirScanner {
            client: Self::make_http_client(&cfg)?,
            extentions: extentions,
            ignore_codes: ignore_codes,
            cfg: cfg,
            common_args: common_args,
        })
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting httpdir scan...");

        let threads = self.common_args.threads.clamp(1  , 300) as usize;
        let mut word_list = utils::get_word_list(&self.common_args.word_list).await?;

        info!("Word list: {}", &word_list.source);
        info!("Threads: {}", threads);

        self.dispatch_workers(threads, &mut word_list).await?;

        Ok(())
    }


    async fn dispatch_workers(&self, threads: usize, word_list: &mut utils::WordList) -> Result<(), Box<dyn std::error::Error>> {
        let (mut tx, rx) = mpsc::channel(1);

        let worker_config = WorkerConfig {
            client: self.client.clone(),
            target: self.cfg.target.clone(),
            rx: Mutex::new(rx),
            expand_url_log: self.cfg.expand_url_log,
            extentions: self.extentions.clone(),
            ignore_codes: self.ignore_codes.clone(),
            print_fails: self.cfg.print_fails,
        };

        let mut diapatcher = Dispatcher::<HttpDirWorker>::new(worker_config);
        diapatcher.start_workers(threads);

        let task_timeout = Duration::from_secs(20);
        
        loop {
            let mut line = String::new();
            if word_list.buff.as_mut().read_line(&mut line).await? <= 0 {
                break;
            }
            let word = String::from(line.trim());

            let send_result = timeout(task_timeout, tx.send(word)).await;

            if !send_result.is_ok() {
                warn!("Couldent talk to any workers within 20s. Exiting..");
                break;
            }

            if self.common_args.delay > 0 {
                tokio::time::delay_for(Duration::from_millis(self.common_args.delay as u64))
                    .await;
            }
        }
        
        diapatcher.finish_and_wait().await?;
        Ok(())
    }

    fn make_http_client(
        cfg: &HttpDirConfig,
    ) -> Result<reqwest::Client, Box<dyn std::error::Error>> {

        use super::http_client_builder::HttpClientBuilder;
        let mut builder = HttpClientBuilder::new();

        builder.gzip(cfg.gzip);
        builder.redirect_policy_keep_on_domain(&cfg.target, 5);
        builder.timeout_ms(cfg.timeout);

        if let Some(user_agent) = &cfg.user_agent {
            builder.user_agent(user_agent);
        }

        if let Some(username) = &cfg.username {
            builder.basic_auth(&username, cfg.password.as_ref().map(|s| s.as_str()))?;
        }

        let client = builder.build()?;
        Ok(client)
    }
}
