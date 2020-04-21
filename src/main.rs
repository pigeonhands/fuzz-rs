mod httpdir;
mod utils;

use chrono;
use clap::{App, AppSettings, Arg, SubCommand};
use fern;
use fern::colors::{Color, ColoredLevelConfig};
use log;
use log::error;
use tokio::runtime::Runtime;

pub struct CommonArgs<'a> {
    pub word_list: Option<&'a str>,
    pub threads: u16,
    pub delay: i32,
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("rscan")
        .about("Network scanner/buster written in rust.")
        .version("0.1")
        .author("pigenhands")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(Arg::from_usage("-v, --verbose 'Level of logging. E.g. -vvv'").multiple(true))
        .arg(
            Arg::from_usage("-s, --silent  'Turn off all console logging'")
                .global(true),
        )
        .arg(
            Arg::from_usage("-t, --threads=[int] 'Maximun number of parralel requests (default 10).'")
                .global(true),
        )
        .arg(
            Arg::from_usage("-d, --delay=[int] 'Minimum delay in ms between new requests.'")
                .global(true),
        )
        .arg(
            Arg::from_usage("-o, --out-file=[FILE] 'Output console to specified file.'")
                .global(true),
        )
        .arg(
            Arg::from_usage("-w, --word-list=[FILE] 'Word list to use for finding directories.'")
                .global(true),
        )
        .subcommand(
            SubCommand::with_name("httpdir")
                .about("Scan for http directories.")
                .setting(AppSettings::ArgRequiredElseHelp)
                .arg_from_usage("<TARGET>")
                .arg_from_usage("--timeout=[INT] 'Connection timeout in milliseconds.")
                .arg_from_usage("-g, --gzip, 'Compress requests with gzip'")
                .arg_from_usage("-u, --username=[str] 'Authentication username'")
                .arg_from_usage("-P, --password=[str] 'Authentication password'")
                .arg_from_usage("-a, --useragent=[agent] 'Custom user agent'")
                .arg_from_usage("-e, --expand-url 'Show full url in logs.'")
                .arg_from_usage("-f, --print-fails 'Print requests that are not success.'")
                .arg(
                    Arg::from_usage("--ignore-code=[codes] 'Ignore list. e.g. 401 402 403")
                        .multiple(true),
                )
                .arg(
                    Arg::from_usage("-x, --extentions=[ext] 'Extention list. -e php js css")
                        .multiple(true),
                ),
        )
        .get_matches();

    let log_level = match matches.occurrences_of("verbose") {
        1 => log::LevelFilter::Debug,
        2 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    };

    let colors = ColoredLevelConfig::new()
        .info(Color::Green)
        .warn(Color::BrightYellow)
        .error(Color::BrightRed);

    let mut fern_logger = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                colors.color(record.level()),
                message
            ))
        })
        .level(log_level);

    if !matches.is_present("silent") {
        fern_logger = fern_logger.chain(std::io::stdout());
    }
    if let Some(log_file) = matches.value_of("out-file") {
        fern_logger = fern_logger.chain(fern::log_file(log_file)?);
    }
    fern_logger.apply().expect("Failed to initilize logging!");

    let common_args = CommonArgs {
        word_list: matches.value_of("word-list"),
        threads: matches
            .value_of("threads")
            .unwrap_or_default()
            .parse()
            .unwrap_or(10),
        delay: matches
            .value_of("delay")
            .unwrap_or_default()
            .parse()
            .unwrap_or(0),
    };

    let app_run = match matches.subcommand() {
        ("httpdir", Some(cfg)) => Ok(httpdir::HttpDirScanner::run_new(httpdir::HttpDirConfig {
            common: common_args,
            target: reqwest::Url::parse(cfg.value_of("TARGET").unwrap())?,
            timeout: cfg.value_of("timeout"),
            gzip: cfg.is_present("gzip"),
            username: cfg.value_of("username"),
            password: cfg.value_of("password"),
            user_agent: cfg.value_of("useragent"),
            expand_url_log: cfg.is_present("expand-url"),
            extentions: cfg.values_of("extentions").map(|e| e.collect()),
            ignore_codes: cfg.values_of("ignore-code").map(|e| e.collect()),
            print_fails: cfg.is_present("print-fails"),
        })),
        _ => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
    }?;

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
