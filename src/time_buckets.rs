use chrono::{DateTime, TimeZone};
use core::pin::Pin;
use futures::{
    self,
    stream::{Fuse, Stream, StreamExt},
    task::{Context, Poll},
};
use pin_utils::{unsafe_pinned, unsafe_unpinned};
use std::convert::TryInto;
use std::mem;
use std::time::{Duration, SystemTime};

pub trait TimeStreamExt {
    fn time_buckets(self, period: Duration) -> TimeBuckets<Self>
    where
        Self: Stream + Sized,
        Self::Item: Timestamp,
    {
        TimeBuckets::new(self, period)
    }
}

impl<T: Stream> TimeStreamExt for T {}

pub trait Timestamp {
    fn timestamp(&self) -> SystemTime;
}

impl Timestamp for SystemTime {
    fn timestamp(&self) -> SystemTime {
        *self
    }
}

impl<T: TimeZone> Timestamp for DateTime<T> {
    fn timestamp(&self) -> SystemTime {
        let ts: SystemTime = self.clone().try_into().unwrap();
        ts
    }
}

impl<A> Timestamp for (SystemTime, A) {
    fn timestamp(&self) -> SystemTime {
        self.0
    }
}

impl<A, B> Timestamp for (SystemTime, A, B) {
    fn timestamp(&self) -> SystemTime {
        self.0
    }
}

pub struct TimeBuckets<St>
where
    St: Stream,
    St::Item: Timestamp,
{
    stream: Fuse<St>,
    bucket: Vec<St::Item>,
    period: Duration,
}

impl<St> Unpin for TimeBuckets<St>
where
    St: Stream + Unpin,
    St::Item: Timestamp + Unpin,
{
}

impl<St> TimeBuckets<St>
where
    St: Stream,
    St::Item: Timestamp,
{
    unsafe_pinned!(stream: Fuse<St>);
    unsafe_unpinned!(bucket: Vec<St::Item>);
    unsafe_pinned!(period: Duration);

    fn new(stream: St, period: Duration) -> TimeBuckets<St> {
        assert!(period > Duration::from_secs(0));

        TimeBuckets {
            stream: stream.fuse(),
            bucket: Vec::new(),
            period,
        }
    }

    fn replace(mut self: Pin<&mut Self>, item: St::Item) -> Vec<St::Item> {
        mem::replace(self.as_mut().bucket(), vec![item])
    }
}

impl<St> Stream for TimeBuckets<St>
where
    St: Stream,
    St::Item: Timestamp,
{
    type Item = Vec<St::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match futures::ready!(self.as_mut().stream().poll_next(cx)) {
                Some(item) => {
                    if let Some(first) = self.bucket.first() {
                        if first.timestamp() + self.period <= item.timestamp() {
                            return Poll::Ready(Some(self.as_mut().replace(item)));
                        }
                    }

                    self.as_mut().bucket().push(item);
                }

                None => {
                    let last = if self.bucket.is_empty() {
                        None
                    } else {
                        let bucket = mem::replace(self.as_mut().bucket(), Vec::new());
                        Some(bucket)
                    };

                    return Poll::Ready(last);
                }
            }
        }
    }
}

mod test {
    pub use super::{TimeBuckets, TimeStreamExt, Timestamp};
    pub use chrono::{Duration, Utc};
    pub use futures::executor::block_on;
    pub use futures::stream::{self, Stream, StreamExt};

    #[test]
    fn test_time_buckets() {
        let now = Utc::now();
        let items = vec![
            now,
            now + Duration::seconds(10),
            now + Duration::seconds(30),
            now + Duration::seconds(60),
            now + Duration::seconds(70),
        ];

        let expected = vec![items[0..3].to_vec(), items[3..].to_vec()];

        let result = block_on(
            stream::iter(items)
                .time_buckets(Duration::seconds(60).to_std().unwrap())
                .collect::<Vec<_>>(),
        );

        assert_eq!(expected, result);
    }
}
