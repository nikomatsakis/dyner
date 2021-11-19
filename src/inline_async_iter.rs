use crate::async_iter::AsyncIter;
use crate::dyner::InlineFuture;
use std::cell::RefCell;
use std::mem::MaybeUninit;

pub struct InlineDynAsyncIter<'me, Item> {
    obj: &'me mut dyn InlineAsyncIter<Item = Item>,
}

trait InlineAsyncIter {
    type Item;
    fn next(&mut self) -> crate::dyner::InlineFuture<'_, Option<Self::Item>>;
    fn size_hint(&self) -> crate::dyner::InlineRefCellFuture<'_, Option<usize>>;
}

impl<'me, Item> AsyncIter for InlineDynAsyncIter<'me, Item> {
    type Item = Item;

    type Next<'a>
    where
        Self: 'a,
    = crate::dyner::InlineFuture<'a, Option<Self::Item>>;

    fn next(&mut self) -> Self::Next<'_> {
        InlineAsyncIter::next(self.obj)
    }

    type SizeHint<'a>
    where
        Self: 'a,
    = crate::dyner::InlineRefCellFuture<'a, Option<usize>>;

    fn size_hint(&self) -> Self::SizeHint<'_> {
        InlineAsyncIter::size_hint(self.obj)
    }
}

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

    pub fn as_dyn(&mut self) -> InlineDynAsyncIter<'_, I::Item> {
        InlineDynAsyncIter { obj: self }
    }
}

impl<'me, I> InlineAsyncIter for InlineAsyncIterImpl<'me, I>
where
    I: AsyncIter + 'me,
{
    type Item = I::Item;

    fn next(&mut self) -> crate::dyner::InlineFuture<'_, Option<Self::Item>> {
        let f: I::Next<'_> = self.underlying_impl.next();

        // Extend the lifetime of `f` artificially to `'me`.
        unsafe {
            let f_ptr = std::ptr::addr_of!(f) as *mut I::Next<'me>;
            self.next_future.write(std::ptr::read(f_ptr));
            std::mem::forget(f);
        }

        unsafe { InlineFuture::new(&mut self.next_future) }
    }

    fn size_hint(&self) -> crate::dyner::InlineRefCellFuture<'_, Option<usize>> {
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
    let mut inline_dyn_range: InlineDynAsyncIter<'_, u32> = inline_range.as_dyn();
    for i in 0..10 {
        assert_eq!(inline_dyn_range.next().await, Some(i));
    }
    assert_eq!(inline_dyn_range.next().await, None);
}

// #[tokio::test]
// async fn next_error() {
//     let range = crate::yielding_range::YieldingRange::new(0, 10);
//     let mut inline_range = InlineAsyncIterImpl::new(range);
//     let mut dyn_range = dyn_Range.as_dyn();
//     let n1 = inline_range.next();
//     let n2 = inline_range.next();
// }

#[tokio::test]
async fn inline_size_hint() {
    let range = crate::yielding_range::YieldingRange::new(0, 10);
    let mut inline_range = InlineAsyncIterImpl::new(range);
    let inline_dyn_range: InlineDynAsyncIter<'_, u32> = inline_range.as_dyn();
    assert_eq!(inline_dyn_range.size_hint().await, Some(10));
}

#[tokio::test]
#[should_panic(expected = "already borrowed: BorrowMutError")]
async fn inline_size_hint_error() {
    let range = crate::yielding_range::YieldingRange::new(0, 10);
    let mut inline_range = InlineAsyncIterImpl::new(range);
    let inline_dyn_range: InlineDynAsyncIter<'_, u32> = inline_range.as_dyn();
    let _s1 = inline_dyn_range.size_hint();
    inline_dyn_range.size_hint(); // panics
}

#[tokio::test]
async fn inline_size_hint_ok() {
    let range = crate::yielding_range::YieldingRange::new(0, 10);
    let mut inline_range = InlineAsyncIterImpl::new(range);
    let inline_dyn_range: InlineDynAsyncIter<'_, u32> = inline_range.as_dyn();
    let s1 = inline_dyn_range.size_hint().await;
    let s2 = inline_dyn_range.size_hint().await;
    assert_eq!(s1, s2);
    assert_eq!(s1, Some(10));
}
