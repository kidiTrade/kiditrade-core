#![feature(try_trait)]

mod candle;
use candle::Candle;
use prost_types::Timestamp;
use std::convert::TryFrom;
use std::env;
use std::time::SystemTime;
use tokio_postgres::NoTls;
use tracing::{debug, error, Level};
use ibloader::ib_loader_client::IbLoaderClient;
use ibloader::GetStockHistoricalDataRequest;
use tonic::Request;

pub mod ibloader {
    tonic::include_proto!("ibloader");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::DEBUG)
        // builds the subscriber.
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let (db, connection) = tokio_postgres::connect(&env::var("DATABASE_URL")?, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("connection error: {}", e);
        }
    });

    let mut ibloader = IbLoaderClient::connect("http://localhost:8443").await?;

    let contracts = db
        .query(
            r#"
            SELECT c.id, c.symbol, c.exchange, c.currency, MIN(b.timestamp)
            FROM contracts c
            LEFT JOIN bars b ON b.contractId = c.id AND b.duration = INTERVAL '1 minutes'
            GROUP BY c.id;
        "#,
            &[],
        )
        .await?;

    let insert_bar = db
        .prepare(r#"
            INSERT INTO bars (contractId, timestamp, duration, open, high, low, close, vwap, volume, trades)
            VALUES ($1, $2, INTERVAL '1 minutes', $3, $4, $5, $6, $7, $8, $9)
        "#)
        .await?;

    for row in contracts {
        let id: i32 = row.get(0);
        let timestamp: Option<SystemTime> = row.get(4);

        let mut stream = ibloader
            .get_stock_historical_data(Request::new(GetStockHistoricalDataRequest {
                symbol: row.get::<_, &str>(1).to_string(),
                exchange: row.get::<_, &str>(2).to_string(),
                currency: row.get::<_, &str>(3).to_string(),
                end_date: timestamp.map(Timestamp::from),
            }))
            .await?
            .into_inner();

        while let Some(bar) = stream.message().await? {
            let candle: Candle = Candle::try_from(bar).unwrap();
            debug!("{:?}", candle);

            db.execute(
                &insert_bar,
                &[
                    &id,
                    &candle.timestamp,
                    &candle.open,
                    &candle.high,
                    &candle.low,
                    &candle.close,
                    &candle.vwap,
                    &candle.volume,
                    &candle.trades,
                ],
            )
            .await?;
        }
    }

    Ok(())
}
