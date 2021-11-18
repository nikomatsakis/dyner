use crate::async_iter::AsyncIter;
use std::future::Future;
use tokio::task;

pub struct YieldingRange {
    start: u32,
    stop: u32,
}

impl YieldingRange {
    pub fn new(start: u32, stop: u32) -> Self {
        Self { start, stop }
    }
}

impl AsyncIter for YieldingRange {
    type Item = u32;

    type Next<'me> = impl Future<Output = Option<Self::Item>> + 'me;

    fn next(&mut self) -> Self::Next<'_> {
        async move {
            task::yield_now().await;
            if self.start == self.stop {
                None
            } else {
                let p = self.start;
                self.start += 1;
                Some(p)
            }
        }
    }

    type SizeHint<'me> = impl Future<Output = Option<usize>> + 'me;

    fn size_hint(&self) -> Self::SizeHint<'_> {
        async move {
            task::yield_now().await;
            let hint: usize = (self.stop - self.start) as usize;
            Some(hint)
        }
    }
}
