#![feature(try_trait)]

mod backtest;
mod candle;
mod cli;
mod import;
mod ibbridge {
    tonic::include_proto!("ibbridge");
}

use cli::KidiTrade;
use std::process;
use structopt::StructOpt;
use tokio::runtime::Runtime;
use tracing::{error, Level};

fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // builds the subscriber.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let mut rt = Runtime::new().unwrap();

    let args = KidiTrade::from_args();
    let exec = async {
        match args {
            KidiTrade::Import {} => import::import().await,
            KidiTrade::Backtest {} => backtest::backtest().await,
        }
    };

    if let Err(e) = rt.block_on(exec) {
        error!("Error: {}", e);
        process::exit(1);
    }
}
