#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod async_iter;
mod dyner;
mod yielding_range;

use async_iter::AsyncIter;

async fn do_loop(range: std::ops::Range<u32>, data: &mut async_iter::DynAsyncIter<'_, u32>) {
    for i in range {
        match data.next().await {
            Some(j) => assert_eq!(i, j),
            None => panic!("expected {} found None", i),
        }
    }
}

#[tokio::test]
async fn box_dyn_async_iter() {
    let range = yielding_range::YieldingRange::new(0, 10);
    let mut dyn_range = async_iter::DynAsyncIter::new(range);
    do_loop(0..10, &mut dyn_range).await;
}

#[tokio::test]
async fn ref_mut_dyn_async_iter() {
    let mut range = yielding_range::YieldingRange::new(0, 10);
    let mut dyn_range = async_iter::DynAsyncIter::from_ref_mut(&mut range);
    do_loop(0..10, &mut dyn_range).await;
}

///
/// ```compile_fail
/// let mut range = yielding_range::YieldingRange::new(0, 10);
/// let mut dyn_range = async_iter::DynAsyncIter::from_ref(&range);
/// dyn_range.next().await;
/// ```
#[tokio::test]
async fn ref_dyn_async_iter() {}

#[tokio::test]
async fn ref_dyn_async_iter_size_hint() {
    let range = yielding_range::YieldingRange::new(0, 10);
    let dyn_range = async_iter::DynAsyncIter::from_ref(&range);
    assert_eq!(dyn_range.size_hint().await, Some(10));
}
