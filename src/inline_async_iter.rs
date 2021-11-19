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

#[tokio::test]
async fn next() {
    let range = crate::yielding_range::YieldingRange::new(0, 10);
    let mut inline_range = InlineAsyncIterImpl::new(range);
    for i in 0..10 {
        assert_eq!(inline_range.next().await, Some(i));
    }
    assert_eq!(inline_range.next().await, None);
}

// #[tokio::test]
// async fn next_error() {
//     let range = crate::yielding_range::YieldingRange::new(0, 10);
//     let mut inline_range = InlineAsyncIterImpl::new(range);
//     let n1 = inline_range.next();
//     let n2 = inline_range.next();
// }
