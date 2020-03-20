use crate::ibbridge;
use crate::time_buckets::Timestamp;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::convert::{TryFrom, TryInto};
use std::option::NoneError;
use std::{
    fmt::Debug,
    time::{Duration, SystemTime},
};

#[derive(Clone)]
pub(crate) struct Candle {
    pub timestamp: SystemTime,
    pub duration: Duration,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub trades: Decimal,
    pub vwap: Decimal,
}

impl TryFrom<ibbridge::Bar> for Candle {
    type Error = NoneError;

    fn try_from(bar: ibbridge::Bar) -> Result<Self, Self::Error> {
        Ok(Candle {
            timestamp: bar.timestamp?.clone().try_into().or(Err(NoneError))?,
            duration: Duration::from_secs(60),
            open: Decimal::from_f64(bar.open)?.round_dp(4),
            high: Decimal::from_f64(bar.high)?.round_dp(4),
            low: Decimal::from_f64(bar.low)?.round_dp(4),
            close: Decimal::from_f64(bar.close)?.round_dp(4),
            volume: Decimal::from_i64(bar.volume)?.round_dp(4),
            trades: Decimal::from_i64(bar.trades)?.round_dp(4),
            vwap: Decimal::from_f64(bar.vwap)?.round_dp(4),
        })
    }
}

impl Timestamp for Candle {
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
}

impl Debug for Candle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ts: DateTime<Utc> = self.timestamp.into();
        write!(
            f,
            "{:?}: O: {:>6.2}, H: {:>6.2}, L: {:>6.2}, C: {:>6.2}, V: {:>6.2}",
            ts, self.open, self.high, self.low, self.close, self.volume
        )
    }
}
