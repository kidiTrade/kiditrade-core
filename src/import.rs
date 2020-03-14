use crate::candle::Candle;
use chrono::Duration;
use crate::ibbridge::ib_bridge_client::IbBridgeClient;
use crate::ibbridge::GetStockHistoricalDataRequest;
use pg_interval::Interval;
use prost_types::Timestamp;
use std::convert::TryFrom;
use std::env;
use std::time::SystemTime;
use tokio_postgres::NoTls;
use tonic::Request;
use tracing::{debug, error};

pub async fn import() -> Result<(), Box<dyn std::error::Error>> {
    let (db, connection) = tokio_postgres::connect(&env::var("DATABASE_URL")?, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("connection error: {}", e);
        }
    });

    let mut ibloader = IbBridgeClient::connect("http://localhost:8443").await?;

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
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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

            let duration: Duration = Duration::from_std(candle.duration)?;
            let duration: Interval = Interval::from_duration(duration).unwrap();

            db.execute(
                &insert_bar,
                &[
                    &id,
                    &candle.timestamp,
                    &duration.to_postgres(),
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
