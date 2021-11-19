use crate::async_iter::AsyncIter;
use std::future::Future;

pub struct DynAsyncIter<'me, S: DynAsyncIterStrategy> {
    dyn_trait: DynAsyncIterPtr<'me, S>,
}

// May Athena forgive me for what I do here
union DynAsyncIterPtr<'me, S: DynAsyncIterStrategy> {
    raw: *mut (dyn DynAsyncIterTrait<S> + 'me),
    usizes: (usize, usize),
}

impl<'me, S> Copy for DynAsyncIterPtr<'me, S> where S: DynAsyncIterStrategy {}

impl<'me, S> Clone for DynAsyncIterPtr<'me, S>
where
    S: DynAsyncIterStrategy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'me, S> DynAsyncIterPtr<'me, S>
where
    S: DynAsyncIterStrategy,
{
    fn new(raw: *mut (dyn DynAsyncIterTrait<S> + 'me)) -> Self {
        DynAsyncIterPtr { raw }
    }

    unsafe fn untagged(mut self) -> *mut (dyn DynAsyncIterTrait<S> + 'me) {
        self.usizes.0 &= !1;
        self.raw
    }

    unsafe fn is_tagged(self) -> bool {
        (self.usizes.0 & 1) != 0
    }

    unsafe fn tagged(self) -> Self {
        let (data, vtable) = self.usizes;
        DynAsyncIterPtr {
            usizes: (data | 1, vtable),
        }
    }
}

trait DynAsyncIterTrait<S: DynAsyncIterStrategy> {
    fn next(&mut self) -> S::Next<'_>;
    fn size_hint(&self) -> S::SizeHint<'_>;
}

pub trait DynAsyncIterStrategy {
    type DynType: ?Sized;

    type Item;

    type Next<'a>: Future<Output = Option<Self::Item>>
    where
        Self: 'a;

    type SizeHint<'a>: Future<Output = Option<usize>>
    where
        Self: 'a;
}

impl<'me, S> AsyncIter for DynAsyncIter<'me, S>
where
    S: DynAsyncIterStrategy,
{
    type Item = S::Item;

    type Next<'a>
    where
        Self: 'a,
    = S::Next<'a>;

    fn next(&mut self) -> Self::Next<'_> {
        unsafe { DynAsyncIterTrait::next(&mut *self.dyn_trait.untagged()) }
    }

    type SizeHint<'a>
    where
        Self: 'a,
    = S::SizeHint<'a>;

    fn size_hint(&self) -> Self::SizeHint<'_> {
        unsafe { DynAsyncIterTrait::size_hint(&*self.dyn_trait.untagged()) }
    }
}
