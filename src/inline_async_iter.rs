use crate::async_iter::AsyncIter;
use crate::dyner::InlineFuture;
use std::future::Future;
use std::mem::MaybeUninit;

struct InlineAsyncIterImpl<'me, I>
where
    I: AsyncIter + 'me,
{
    underlying_impl: I,
    next_future: MaybeUninit<I::Next<'me>>,
}

impl<'me, I> InlineAsyncIterImpl<'me, I>
where
    I: AsyncIter + 'me,
{
    pub fn new(underlying: I) -> Self {
        Self {
            underlying_impl: underlying,
            next_future: MaybeUninit::uninit(),
        }
    }
}

impl<'me, I> AsyncIter for InlineAsyncIterImpl<'me, I>
where
    I: AsyncIter + 'me,
{
    type Item = I::Item;

    type Next<'a>
    where
        Self: 'a,
    = crate::dyner::InlineFuture<'a, Option<Self::Item>>;

    fn next(&mut self) -> Self::Next<'_> {
        let f = self.underlying_impl.next();

        // Extend the lifetime of `f` artificially to `'me`.
        unsafe {
            let f_ptr = std::ptr::addr_of!(f) as *mut I::Next<'me>;
            self.next_future.write(std::ptr::read(f_ptr));
            std::mem::forget(f);
        }

        unsafe { InlineFuture::new(&mut self.next_future) }
    }

    type SizeHint<'a>
    where
        Self: 'a,
    = crate::dyner::InlineFuture<'a, Option<usize>>;

    fn size_hint(&self) -> Self::SizeHint<'_> {
        panic!()
    }
}
