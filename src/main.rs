use std::{env, process::ExitCode};

use butter::start;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

pub fn main() -> ExitCode {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let dir = env::args().skip(1).next().unwrap();
    env::set_current_dir(dir).unwrap();

    start()
}
