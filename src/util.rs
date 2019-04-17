use std::env;
use std::error;

use crate::cli::Cli;

fn use_dotenv() -> () {
    dotenv::dotenv().ok();
}

fn init_env_logger() -> () {
    if let Ok(log_file) = env::var("RUST_LOG_FILE") {
        use log::LevelFilter;
        use log4rs::append::file::FileAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;
        use std::str::FromStr;

        let level = env::var("RUST_LOG").ok().unwrap_or("info".to_owned());
        let level_filter = LevelFilter::from_str(&level).expect(&format!(
            "failed to parse log level (env-var: RUST_LOG='{}')",
            level
        ));

        let log_file = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build(log_file)
            .expect("failed to configure FileAppender");
        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(log_file)))
            .build(Root::builder().appender("logfile").build(level_filter))
            .expect("failed to create log4rs config");

        log4rs::init_config(config).expect("failed to init log4rs from config");
    } else {
        use env_logger::{Builder, Target};
        Builder::from_default_env().target(Target::Stderr).init()
    }
}

fn init() -> Result<(), Box<dyn error::Error>> {
    init_env_logger();

    Ok(())
}

pub fn run<C>(cli: C) -> Result<(), Box<dyn error::Error>>
where
    C: Cli,
{
    debug!("Running '{}'", C::name());
    if cli.use_dotenv() {
        use_dotenv();
    }

    init()?;

    debug!("Running [{}]", C::name());
    let () = cli.run()?;
    Ok(())
}
