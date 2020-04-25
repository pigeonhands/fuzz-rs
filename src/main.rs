#![feature(clamp)]

mod httpdir;
mod utils;
mod dispatch;

use chrono;
use fern;
use log;
use log::error;
use tokio::runtime::Runtime;

use clap::Clap;

#[derive(Clap, Clone)]
#[clap(version = "1.0", author = "Sam M.")]
pub struct CommonArgs {
    /// Save output to specified file.
    #[clap(short = "o", long = "out-file", global = true)]
    pub out_file: Option<String>,

    /// Input work list used to fuzz.
    #[clap(short = "w", long = "word-list", global = true)]
    pub word_list: Option<String>,

    /// Number of threads to use for fuzzing.
    #[clap(short = "t", long = "threads", global = true, default_value = "10")]
    pub threads: u16,

    /// Minimum delay between word processing.
    #[clap(short = "d", long = "delay", global = true, default_value = "0")]
    pub delay: i32,

    /// Disable console output.
    #[clap(long = "silent", global = true)]
    pub silent: bool,

    /// Verbose level. e.g. -vvv
    #[clap(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: i32,

    #[clap(subcommand)]
    subcmd: Subcommand,
}

#[derive(Clap, Clone)]
pub enum Subcommand {
    #[clap(name = "httpdir", version = "1.0")]
    HttpDir(httpdir::HttpDirConfig),
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let args = CommonArgs::parse();

    let log_level = match args.verbose {
        1 => log::LevelFilter::Debug,
        2 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };

    let mut fern_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                record.level(),
                message
            ))
        })
        .level(log_level);

    if !args.silent {
        fern_logger = fern_logger.chain(std::io::stdout());
    }
    if let Some(log_file) = &args.out_file {
        fern_logger = fern_logger.chain(fern::log_file(log_file)?);
    }
    fern_logger.apply().expect("Failed to initilize logging!");

    let common_args = args.clone();
    let app_run = match args.subcmd {
        Subcommand::HttpDir(http_cfg) => httpdir::HttpDirScanner::run_new(common_args, http_cfg),
    };

    let mut rt = Runtime::new()?;
    rt.block_on(app_run)
}

fn main() {
    std::process::exit(match run_app() {
        Ok(_) => 0,
        Err(err) => {
            error!("{}", err);
            1
        }
    });
}
