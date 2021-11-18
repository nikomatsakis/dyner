#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod async_iter {
    use std::future::Future;
    use std::pin::Pin;

    pub trait AsyncIter {
        type Item;

        type Next<'me>: Future<Output = Option<Self::Item>>
        where
            Self: 'me;

        fn next(&mut self) -> Self::Next<'_>;
    }

    pub struct DynAsyncIter<'data, Item> {
        fatptr: Box<dyn ErasedAsyncIter<Item = Item> + 'data>,
    }

    trait ErasedAsyncIter {
        type Item;
        fn next<'me>(&'me mut self) -> Pin<Box<dyn Future<Output = Option<Self::Item>> + 'me>>;
    }

    impl<T> ErasedAsyncIter for T
    where
        T: AsyncIter,
    {
        type Item = T::Item;
        fn next<'me>(&'me mut self) -> Pin<Box<dyn Future<Output = Option<Self::Item>> + 'me>> {
            Box::pin(AsyncIter::next(self))
        }
    }

    impl<'data, Item> AsyncIter for DynAsyncIter<'data, Item> {
        type Item = Item;

        type Next<'me>
        where
            Item: 'me,
            'data: 'me,
        = Pin<Box<dyn Future<Output = Option<Item>> + 'me>>;

        fn next(&mut self) -> Self::Next<'_> {
            self.fatptr.next()
        }
    }

    impl<'data, Item> DynAsyncIter<'data, Item> {
        pub fn new<T>(value: T) -> DynAsyncIter<'data, Item>
        where
            T: AsyncIter<Item = Item> + 'data,
            Item: 'data,
        {
            DynAsyncIter {
                fatptr: Box::new(value),
            }
        }
    }
}

pub mod yielding_range {
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
    }
}

use async_iter::AsyncIter;

#[tokio::main]
async fn main() {
    let range = yielding_range::YieldingRange::new(0, 10);
    let mut boxed_range = async_iter::DynAsyncIter::new(range);
    while let Some(v) = boxed_range.next().await {
        println!("v={}", v);
    }
}
