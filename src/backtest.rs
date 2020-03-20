use crate::candle::Candle;
use crate::time_buckets::*;
use chrono::{DateTime, Utc};
use futures::future;
use futures::{stream::StreamExt, Stream};
use itertools::Itertools;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::env;
use std::{collections::HashMap, time::Duration};
use tokio_postgres::RowStream;
use tokio_postgres::{types::ToSql, Client, NoTls, Row};
use tracing::{debug, error};
use ta::indicators::ExponentialMovingAverage;
use ta::Next;

impl Timestamp for (i32, Candle) {
    fn timestamp(&self) -> std::time::SystemTime {
        self.1.timestamp
    }
}

pub async fn backtest() -> Result<(), Box<dyn std::error::Error>> {
    let (db, connection) = tokio_postgres::connect(&env::var("DATABASE_URL")?, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("connection error: {}", e);
        }
    });

    let candles = candles(&db).await;
    let mut contracts = HashMap::new();
    candles
        .time_buckets(Duration::from_secs(60 * 30))
        .map(|mut bucket: Vec<_>| {
            bucket.sort_by_key(|(contract_id, candle)| (contract_id.clone(), candle.timestamp));

            bucket
                .into_iter()
                .group_by(|(contract_id, _)| contract_id.clone())
                .into_iter()
                .filter_map(|(contract_id, bars)| {
                    let bars: Vec<Candle> = bars.map(|(_, b)| b).collect();

                    let candle = Candle {
                        timestamp: bars.first()?.timestamp,
                        duration: Duration::from_secs(60 * 30),
                        open: bars.first()?.open,
                        high: bars.iter().map(|b| b.high).max()?,
                        low: bars.iter().map(|b| b.low).min()?,
                        close: bars.last()?.close,
                        volume: bars.iter().map(|b| b.volume).sum(),
                        trades: bars.iter().map(|b| b.trades).sum(),
                        vwap: bars.iter().map(|b| b.vwap).last()?,
                    };

                    Some((contract_id, candle))
                })
                .collect::<Vec<(i32, Candle)>>()
        })
        .for_each(|bars| {
            for (contract_id, candle) in bars {
                let (contract, signal) = contracts
                    .entry(contract_id)
                    .or_insert_with(|| (Stock::new(contract_id.clone(), 10), EmaSignal::new()));
                contract.on_bar(&candle);
                if let Some(signal) = signal.on_bar(&candle) {
                    debug!("{:?}: {:?} - {:?}", contract_id, candle, signal);
                } else {
                    debug!("{:?}: {:?}", contract_id, candle);
                }
            }

            future::ready(())
        })
        .await;

    Ok(())
}

pub(crate) async fn candles(db: &Client) -> impl Stream<Item = (i32, Candle)> {
    let params: Vec<String> = vec![];
    let stream: RowStream = db
        .query_raw(
            r#"
            SELECT timestamp, duration, open, high, low, close, vwap, volume, trades, contractId
            FROM bars
            WHERE timestamp BETWEEN '2019-01-01' AND '2019-03-01'
            ORDER BY timestamp ASC -- LIMIT 10000
            ;
            "#,
            params.iter().map(|p| p as &dyn ToSql),
        )
        .await
        .unwrap();

    stream.map(|bar| bar.unwrap()).map(|bar: Row| {
        let contract_id = bar.get(9);
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

        (contract_id, candle)
    })
}

struct Stock {
    capacity: usize,
    _contract_id: i32,
    bars: Vec<Candle>,
}

impl Stock {
    fn new(contract_id: i32, capacity: usize) -> Self {
        Stock {
            capacity,
            _contract_id: contract_id,
            bars: Vec::with_capacity(capacity),
        }
    }

    fn on_bar(&mut self, bar: &Candle) {
        let _t: DateTime<Utc> = bar.timestamp.into();

        if self.bars.len() >= self.capacity {
            self.bars.pop();
        }

        self.bars.insert(0, bar.clone());

        let _close_prices: Vec<Decimal> = self.bars.iter().map(|b| b.close).collect();
        // debug!(
        //     "{} {}: {:?}",
        //     self.contract_id,
        //     t.to_rfc3339(),
        //     close_prices
        // );
    }
}

#[derive(Debug)]
enum Signal {
    Long,
    Short,
}

#[derive(Default)]
struct EmaSignal {
    faster: ExponentialMovingAverage,
    slower: ExponentialMovingAverage,
    current_faster: f64,
    current_slower: f64,
}

impl EmaSignal {
    fn new() -> Self {
        EmaSignal {
            faster: ExponentialMovingAverage::new(10).unwrap(),
            slower: ExponentialMovingAverage::new(50).unwrap(),
            current_faster: 0.0,
            current_slower: 0.0,
        }
    }

    fn on_bar(&mut self, bar: &Candle) -> Option<Signal> {
        let (prev_faster, prev_slower) = (self.current_faster, self.current_slower);
        self.current_faster = self.faster.next(bar.close.to_f64()?);
        self.current_slower = self.slower.next(bar.close.to_f64()?);

        if prev_faster < prev_slower && self.current_faster > self.current_slower {
            Some(Signal::Long)
        } else if prev_faster > prev_slower && self.current_faster < self.current_slower {
            Some(Signal::Short)
        } else {
            None
        }
    }
}
