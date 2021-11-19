#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod async_iter;
mod dyn_async_iter;
mod dyner;
mod inline_async_iter;
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

/// Test that we get this error
///
/// ```notrust
/// error[E0596]: cannot borrow data in a dereference of `dyner::Ref<DynAsyncIter<'_, u32>>` as mutable
///   --> src/main.rs:43:5
///    |
/// 43 |     dyn_range.next().await;
///    |     ^^^^^^^^^^^^^^^^ cannot borrow as mutable
///    |
///    = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `dyner::Ref<DynAsyncIter<'_, u32>>`
/// ```
///
/// when we compile
///
/// ```compile_fail
/// let range = yielding_range::YieldingRange::new(0, 10);
/// let mut dyn_range = async_iter::DynAsyncIter::from_ref(&range);
/// dyn_range.next().await;
/// ```
///
/// because `next` is an `&mut self` method.
#[tokio::test]
async fn ref_dyn_async_iter() {}

#[tokio::test]
async fn ref_dyn_async_iter_size_hint() {
    let range = yielding_range::YieldingRange::new(0, 10);
    let dyn_range = async_iter::DynAsyncIter::from_ref(&range);
    assert_eq!(dyn_range.size_hint().await, Some(10));
}

fn main() {}
