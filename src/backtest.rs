use std::env;
use std::time::Duration;
// use std::convert::{TryInto, TryFrom};
use tokio_postgres::NoTls;
use tracing::{debug, error};
use crate::candle::Candle;

pub async fn backtest() -> Result<(), Box<dyn std::error::Error>> {
    let (db, connection) = tokio_postgres::connect(&env::var("DATABASE_URL")?, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("connection error: {}", e);
        }
    });

    let bars = db
        .query(
            r#"
            SELECT timestamp, duration, open, high, low, close, vwap, volume, trades
            FROM bars
            ORDER BY timestamp ASC
            LIMIT 1000;
            "#,
            &[],
        ).await?;

    for bar in bars {
        // let duration: String = bar.get(1);
        // let duration: Interval = Interval::from_postgres(&duration).unwrap();
        // let duration: std::time::Duration = std::time::Duration::from_micros(duration.microseconds.try_into().unwrap());

        let candle = Candle {
            timestamp: bar.get(0),
            duration: Duration::from_secs(60),
            open: bar.get(2),
            high: bar.get(3),
            low: bar.get(4),
            close: bar.get(5),
            volume: bar.get(6),
            trades: bar.get(7),
            vwap: bar.get(8),
        };

        debug!("{:?}", candle);
    }
    Ok(())
}
