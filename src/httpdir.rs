use crate::utils;
use crate::CommonArgs;
use base64::write::EncoderWriter as Base64Encoder;
use log::{debug, error, info, trace, warn};
use reqwest;
use reqwest::{header, redirect};
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
use tokio::time::timeout;

use clap::Clap;

#[derive(Clap, Clone)]
pub struct HttpDirConfig {
    #[clap(name = "TARGET", parse(try_from_str))]
    pub target: reqwest::Url,

    #[clap(short = "g", long = "gzip")]
    pub gzip: bool,

    #[clap(long = "timeout", default_value="0")]
    pub timeout: i32,

    #[clap(short = "u", long = "username")]
    pub username: Option<String>,

    #[clap(short = "P", long = "password")]
    pub password: Option<String>,

    #[clap(long = "agent")]
    pub user_agent: Option<String>,

    #[clap(short = "e", long = "expand-url")]
    pub expand_url_log: bool,

    #[clap(short = "x", long = "extentions")]
    pub extentions: Option<Vec<String>>,

    #[clap(long = "ignore-code")]
    pub ignore_codes: Option<Vec<String>>,

    #[clap(short = "f", long = "print-fails")]
    pub print_fails: bool,
}

pub struct WorkerConfig {
    client: reqwest::Client,
    target: reqwest::Url,
    extentions: Vec<String>,
    ignore_codes: Vec<u16>,
    rx: Mutex<mpsc::Receiver<String>>,
    expand_url_log: bool,
    print_fails: bool,
}

pub struct Worker {
    id: i32,
    config: Arc<WorkerConfig>,
}

impl Worker {
    fn new(id: i32, cfg: Arc<WorkerConfig>) -> Self {
        Self {
            id: id,
            config: cfg,
        }
    }

    async fn start(self) {
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
            .map(|e| String::from(&[word, e.as_str()].concat()));

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
        let extentions = match &cfg.extentions {
            Some(e) => e.clone(),
            None => vec![], //"php", "css", "js", "sql", "aspx", "asp", "txt", "php"
        }
        .iter()
        .map(|s| {
            let mut new_str = String::from(s.trim());
            if !new_str.starts_with(".") {
                new_str.insert(0, '.');
            }
            new_str
        })
        .collect();

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

        let threads = if self.common_args.threads < 1 {
            1
        } else {
            self.common_args.threads
        } as usize;

        let mut word_list = utils::get_word_list(&self.common_args.word_list).await?;

        info!("Word list: {}", &word_list.source);
        info!("Threads: {}", threads);

        let mut workers = Vec::with_capacity(threads);

        {
            let (mut tx, rx) = mpsc::channel(1);

            let worker_config = Arc::from(WorkerConfig {
                client: self.client.clone(),
                target: self.cfg.target.clone(),
                rx: Mutex::new(rx),
                expand_url_log: self.cfg.expand_url_log,
                extentions: self.extentions.clone(),
                ignore_codes: self.ignore_codes.clone(),
                print_fails: self.cfg.print_fails,
            });

            for i in 0..threads {
                let worker = Worker::new(i as i32, worker_config.clone());
                workers.push(task::spawn(worker.start()));
            }

            let mut line = String::new();
            while word_list.buff.as_mut().read_line(&mut line).await? > 0 {
                let word = line.as_str().trim();
                if !timeout(Duration::from_secs(20), tx.send(word.to_string()))
                    .await
                    .is_ok()
                {
                    warn!("Couldent talk to any workers within 20s. Exiting..");
                    break;
                }
                line.clear();

                if self.common_args.delay > 0 {
                    tokio::time::delay_for(Duration::from_millis(self.common_args.delay as u64))
                        .await;
                }
            }
        }

        for w in workers {
            w.await?;
        }

        Ok(())
    }

    fn create_auth_header<U, P>(
        username: U,
        password: Option<P>,
    ) -> Result<header::HeaderValue, header::InvalidHeaderValue>
    where
        U: std::fmt::Display,
        P: std::fmt::Display,
    {
        let mut header_value = b"Basic ".to_vec();
        {
            let mut encoder = Base64Encoder::new(&mut header_value, base64::STANDARD);
            write!(encoder, "{}:", username).unwrap();
            if let Some(password) = password {
                write!(encoder, "{}", password).unwrap();
            }
        }
        header::HeaderValue::from_bytes(&header_value)
    }

    fn make_http_client(
        cfg: &HttpDirConfig,
    ) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
        let mut headers = header::HeaderMap::new();
        if let Some(username) = &cfg.username {
            headers.append(
                header::AUTHORIZATION,
                Self::create_auth_header(username, cfg.password.as_ref())?,
            );
        }

        let redirect_target = cfg.target.clone();
        let redirect_rule = redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() > 5 {
                attempt.error("too many redirects")
            } else if attempt.url().host_str() != redirect_target.host_str() {
                attempt.stop() //Do not redirect off target
            } else {
                attempt.follow()
            }
        });

        let mut builder = reqwest::Client::builder()
            .gzip(cfg.gzip)
            .default_headers(headers)
            .redirect(redirect_rule);

        if cfg.timeout > 0 {
            builder = builder.timeout(Duration::from_millis(cfg.timeout as u64));
        }

        if let Some(user_agent) = &cfg.user_agent {
            builder = builder.user_agent(user_agent);
        }
        let client = builder.build()?;
        Ok(client)
    }
}
