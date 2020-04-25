mod scanner;
mod worker;
mod http_client_builder;

use clap::Clap;
pub use scanner::HttpDirScanner;
pub use worker::{WorkerConfig};

#[derive(Clap, Clone)]
pub struct HttpDirConfig {
    #[clap(name = "TARGET", parse(try_from_str))]
    pub target: reqwest::Url,

    /// Compresss requests qith gzip.
    #[clap(short = "g", long = "gzip")]
    pub gzip: bool,

    /// Http timeout in ms.
    #[clap(long = "timeout", default_value = "0")]
    pub timeout: i32,

    /// Basic auth username.
    #[clap(short = "u", long = "username")]
    pub username: Option<String>,

    /// Basic auth password.
    #[clap(short = "P", long = "password")]
    pub password: Option<String>,

    /// Request user agent.
    #[clap(long = "agent")]
    pub user_agent: Option<String>,

    /// Show full url (rather than /<word>).
    #[clap(short = "e", long = "expand-url")]
    pub expand_url_log: bool,

    /// List of file extentions to append to word.
    #[clap(short = "x", long = "extentions")]
    pub extentions: Option<Vec<String>>,

    /// Use default extention list (adds to -x if any)
    #[clap(long = "--default-ext")]
    pub default_extentions: bool,

    /// List of status codes to ignore.
    #[clap(long = "ignore-code")]
    pub ignore_codes: Option<Vec<String>>,

    /// Print/output non-success requests.
    #[clap(short = "f", long = "print-fails")]
    pub print_fails: bool,
}
