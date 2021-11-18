use std::future::Future;
use std::mem::ManuallyDrop;
use std::pin::Pin;

pub trait AsyncIter {
    type Item;

    type Next<'me>: Future<Output = Option<Self::Item>>
    where
        Self: 'me;

    fn next(&mut self) -> Self::Next<'_>;
}

// May Athena forgive me for what I do here]
union FatPtr<'data, Item> {
    raw: *mut (dyn ErasedAsyncIter<Item = Item> + 'data),
    usizes: (usize, usize),
}

impl<'data, Item> Copy for FatPtr<'data, Item> {}

impl<'data, Item> Clone for FatPtr<'data, Item> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'data, Item> FatPtr<'data, Item> {
    fn new(raw: *mut (dyn ErasedAsyncIter<Item = Item> + 'data)) -> Self {
        FatPtr { raw }
    }

    unsafe fn untagged(mut self) -> *mut (dyn ErasedAsyncIter<Item = Item> + 'data) {
        self.usizes.0 &= !1;
        self.raw
    }

    unsafe fn is_tagged(self) -> bool {
        (self.usizes.0 & 1) != 0
    }

    unsafe fn tagged(self) -> Self {
        let (data, vtable) = self.usizes;
        FatPtr {
            usizes: (data | 1, vtable),
        }
    }
}

pub struct DynAsyncIter<'data, Item> {
    fatptr: FatPtr<'data, Item>,
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
        unsafe { ErasedAsyncIter::next(&mut *self.fatptr.untagged()) }
    }
}

impl<'data, Item> DynAsyncIter<'data, Item> {
    pub fn new<T>(value: T) -> DynAsyncIter<'data, Item>
    where
        T: AsyncIter<Item = Item> + 'data,
        Item: 'data,
    {
        unsafe {
            let b: Box<dyn ErasedAsyncIter<Item = Item>> = Box::new(value);
            DynAsyncIter {
                fatptr: FatPtr::new(Box::into_raw(b)).tagged(),
            }
        }
    }
}

impl<'data, Item> Drop for DynAsyncIter<'data, Item> {
    fn drop(&mut self) {
        unsafe {
            if self.fatptr.is_tagged() {
                drop(Box::from_raw(self.fatptr.untagged()));
            }
        }
    }
}