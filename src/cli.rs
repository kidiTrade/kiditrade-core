use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub enum KidiTrade {
    Import {},
    Backtest {},
}
