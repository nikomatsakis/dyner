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
