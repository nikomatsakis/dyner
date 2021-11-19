use crate::dyner::{Ref, RefMut};
use std::future::Future;
use std::pin::Pin;

pub trait AsyncIter {
    type Item;

    type Next<'me>: Future<Output = Option<Self::Item>>
    where
        Self: 'me;

    fn next(&mut self) -> Self::Next<'_>;

    type SizeHint<'me>: Future<Output = Option<usize>>
    where
        Self: 'me;

    fn size_hint(&self) -> Self::SizeHint<'_>;
}

impl<T> AsyncIter for &mut T
where
    T: AsyncIter,
{
    type Item = T::Item;

    type Next<'me>
    where
        Self: 'me,
    = T::Next<'me>;

    fn next(&mut self) -> Self::Next<'_> {
        T::next(self)
    }

    type SizeHint<'me>
    where
        Self: 'me,
    = T::SizeHint<'me>;

    fn size_hint(&self) -> Self::SizeHint<'_> {
        T::size_hint(self)
    }
}

// May Athena forgive me for what I do here
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
    fn size_hint<'me>(&'me self) -> Pin<Box<dyn Future<Output = Option<usize>> + 'me>>;
}

impl<T> ErasedAsyncIter for T
where
    T: AsyncIter,
{
    type Item = T::Item;

    fn next<'me>(&'me mut self) -> Pin<Box<dyn Future<Output = Option<Self::Item>> + 'me>> {
        Box::pin(AsyncIter::next(self))
    }

    fn size_hint<'me>(&'me self) -> Pin<Box<dyn Future<Output = Option<usize>> + 'me>> {
        Box::pin(AsyncIter::size_hint(self))
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

    type SizeHint<'me>
    where
        Item: 'me,
        'data: 'me,
    = Pin<Box<dyn Future<Output = Option<usize>> + 'me>>;

    fn size_hint(&self) -> Self::SizeHint<'_> {
        unsafe { ErasedAsyncIter::size_hint(&*self.fatptr.untagged()) }
    }
}

impl<'data, Item> DynAsyncIter<'data, Item> {
    pub fn new<T>(value: T) -> DynAsyncIter<'data, Item>
    where
        T: AsyncIter<Item = Item> + 'data,
        Item: 'data,
    {
        let b: Box<dyn ErasedAsyncIter<Item = Item>> = Box::new(value);
        let raw: *mut dyn ErasedAsyncIter<Item = Item> = Box::into_raw(b);
        unsafe {
            DynAsyncIter {
                fatptr: FatPtr::new(raw).tagged(),
            }
        }
    }

    pub fn from_ref<T>(value: &'data T) -> Ref<DynAsyncIter<'data, Item>>
    where
        T: AsyncIter<Item = Item> + 'data,
        Item: 'data,
    {
        let v: &dyn ErasedAsyncIter<Item = Item> = value;
        let raw: *const dyn ErasedAsyncIter<Item = Item> = v;
        let raw: *mut dyn ErasedAsyncIter<Item = Item> = raw as *mut _;
        Ref::new(DynAsyncIter {
            fatptr: FatPtr::new(raw),
        })
    }

    pub fn from_ref_mut<T>(value: &'data mut T) -> RefMut<DynAsyncIter<'data, Item>>
    where
        T: AsyncIter<Item = Item> + 'data,
        Item: 'data,
    {
        let v: &mut dyn ErasedAsyncIter<Item = Item> = value;
        let raw: *mut dyn ErasedAsyncIter<Item = Item> = v;
        RefMut::new(DynAsyncIter {
            fatptr: FatPtr::new(raw),
        })
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
