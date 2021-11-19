use crate::async_iter::AsyncIter;
use crate::dyner::InlineFuture;
use std::cell::RefCell;
use std::mem::MaybeUninit;

struct InlineAsyncIterImpl<'me, I>
where
    I: AsyncIter + 'me,
{
    underlying_impl: I,
    next_future: MaybeUninit<I::Next<'me>>,
    size_hint_future: RefCell<MaybeUninit<I::SizeHint<'me>>>,
}

impl<'me, I> InlineAsyncIterImpl<'me, I>
where
    I: AsyncIter + 'me,
{
    pub fn new(underlying: I) -> Self {
        Self {
            underlying_impl: underlying,
            next_future: MaybeUninit::uninit(),
            size_hint_future: RefCell::new(MaybeUninit::uninit()),
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
        let f: I::Next<'_> = self.underlying_impl.next();

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
    = crate::dyner::InlineRefCellFuture<'a, Option<usize>>;

    fn size_hint(&self) -> Self::SizeHint<'_> {
        let f: I::SizeHint<'_> = self.underlying_impl.size_hint();

        // Extend the lifetime of `f` artificially to `'me`.
        let maybe_uninit;
        unsafe {
            let f_ptr = std::ptr::addr_of!(f) as *mut I::SizeHint<'me>;
            maybe_uninit = MaybeUninit::new(std::ptr::read(f_ptr));
            std::mem::forget(f);
        }

        let mut r = self.size_hint_future.borrow_mut();
        *r = maybe_uninit;
        unsafe { crate::dyner::InlineRefCellFuture::new(r) }
    }
}

#[tokio::test]
async fn inline_next() {
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

#[tokio::test]
async fn inline_size_hint() {
    let range = crate::yielding_range::YieldingRange::new(0, 10);
    let dyn_range = InlineAsyncIterImpl::new(range);
    assert_eq!(dyn_range.size_hint().await, Some(10));
}

#[tokio::test]
#[should_panic(expected = "already borrowed: BorrowMutError")]
async fn inline_size_hint_error() {
    let range = crate::yielding_range::YieldingRange::new(0, 10);
    let dyn_range = InlineAsyncIterImpl::new(range);
    let _s1 = dyn_range.size_hint();
    dyn_range.size_hint(); // panics
}

#[tokio::test]
async fn inline_size_hint_ok() {
    let range = crate::yielding_range::YieldingRange::new(0, 10);
    let dyn_range = InlineAsyncIterImpl::new(range);
    let s1 = dyn_range.size_hint().await;
    let s2 = dyn_range.size_hint().await;
    assert_eq!(s1, s2);
    assert_eq!(s1, Some(10));
}
