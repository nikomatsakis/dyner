#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod async_iter;
mod dyner;
mod yielding_range;

use async_iter::AsyncIter;

#[tokio::main]
async fn main() {
    let range = yielding_range::YieldingRange::new(0, 10);
    let mut boxed_range = async_iter::DynAsyncIter::new(range);
    while let Some(v) = boxed_range.next().await {
        println!("v={}", v);
    }
}
