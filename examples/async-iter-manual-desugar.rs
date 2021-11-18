#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

mod async_iter {
    use std::{future::Future, intrinsics::transmute, marker::PhantomData, pin::Pin};

    pub trait AsyncIter {
        type Item;

        type Next<'me>: Future<Output = Option<Self::Item>>
        where
            Self: 'me;

        fn next(&mut self) -> Self::Next<'_>;
    }

    use private::ErasedData;
    mod private {
        pub struct ErasedData(());
    }

    pub struct DynAsyncIter<Item> {
        data: *mut ErasedData,
        vtable: &'static ErasedDynAsyncIterVtable,
        phantom: PhantomData<fn(Item) -> Item>,
    }

    type DropFnType = unsafe fn(*mut ErasedData);
    type NextFnType<Item> = for<'a> unsafe fn(
        &'a mut *mut ErasedData,
    )
        -> Pin<Box<dyn Future<Output = Option<Item>> + 'a>>;

    // struct DynAsyncIterVtable<Item> {
    //     drop_fn: DropFnType,
    //     next_fn: NextFnType<Item>,
    // }

    struct ErasedDynAsyncIterVtable {
        drop_fn: *mut (),
        next_fn: *mut (),
    }

    impl<Item> AsyncIter for DynAsyncIter<Item> {
        type Item = Item;

        type Next<'me>
        where
            Item: 'me,
        = Pin<Box<dyn Future<Output = Option<Item>> + 'me>>;

        fn next(&mut self) -> Self::Next<'_> {
            let next_fn: NextFnType<Item> = unsafe { transmute(self.vtable.next_fn) };
            unsafe { next_fn(&mut self.data) }
        }
    }

    impl<Item> Drop for DynAsyncIter<Item> {
        fn drop(&mut self) {
            let drop_fn: DropFnType = unsafe { transmute(self.vtable.drop_fn) };
            unsafe {
                drop_fn(self.data);
            }
        }
    }

    impl<Item> DynAsyncIter<Item> {
        pub fn new<T>(value: T) -> DynAsyncIter<Item>
        where
            T: AsyncIter<Item = Item>,
        {
            let boxed_value = Box::new(value);
            DynAsyncIter {
                data: Box::into_raw(boxed_value) as *mut ErasedData,
                vtable: dyn_async_iter_vtable::<T>(), // we’ll cover this fn later
                phantom: PhantomData,
            }
        }
    }

    // Safety conditions:
    //
    // The `*mut ErasedData` is actually the raw form of a `Box<T>`
    // that is valid for ‘a.
    unsafe fn next_wrapper<'a, T>(
        this: &'a mut *mut ErasedData,
    ) -> Pin<Box<dyn Future<Output = Option<T::Item>> + 'a>>
    where
        T: AsyncIter + 'a,
    {
        let this_raw: *mut *mut ErasedData = this;
        let this_raw: *mut Box<T> = this_raw as *mut Box<T>;
        let unerased_this: &mut Box<T> = &mut *this_raw;
        let future: T::Next<'_> = <T as AsyncIter>::next(unerased_this);
        Box::pin(future)
    }

    // Safety conditions:
    //
    // The `*mut ErasedData` is actually the raw form of a `Box<T>`
    // and this function is being given ownership of it.
    unsafe fn drop_wrapper<T>(this: *mut ErasedData)
    where
        T: AsyncIter,
    {
        let unerased_this = Box::from_raw(this as *mut T);
        drop(unerased_this); // Execute destructor as normal
    }

    fn dyn_async_iter_vtable<T>() -> &'static ErasedDynAsyncIterVtable
    where
        T: AsyncIter,
    {
        // (Generic) inline-`const` polyfill.
        trait GenericConstHelper<T> {
            const VTABLE: ErasedDynAsyncIterVtable;
        }
        impl<T: AsyncIter> GenericConstHelper<T> for () {
            const VTABLE: ErasedDynAsyncIterVtable = ErasedDynAsyncIterVtable {
                drop_fn: drop_wrapper::<T> as _,
                next_fn: next_wrapper::<T> as _,
            };
        }
        // FIXME: This would ideally be `&DynAsyncIterVtable<T>`,
        // but we have to hide the types from the compiler
        &<() as GenericConstHelper<T>>::VTABLE
    }
}

mod yielding_range {
    use crate::async_iter::AsyncIter;
    use std::future::Future;
    use tokio::task;

    pub struct YieldingRange {
        start: u32,
        stop: u32,
    }

    impl YieldingRange {
        pub fn new(start: u32, stop: u32) -> Self {
            Self { start, stop }
        }
    }

    impl AsyncIter for YieldingRange {
        type Item = u32;

        type Next<'me> = impl Future<Output = Option<Self::Item>> + 'me;

        fn next(&mut self) -> Self::Next<'_> {
            async move {
                task::yield_now().await;
                if self.start == self.stop {
                    None
                } else {
                    let p = self.start;
                    self.start += 1;
                    Some(p)
                }
            }
        }
    }
}

use async_iter::AsyncIter;

#[tokio::main]
async fn main() {
    let range = yielding_range::YieldingRange::new(0, 10);
    let mut boxed_range = async_iter::DynAsyncIter::new(range);
    while let Some(v) = boxed_range.next().await {
        println!("v={}", v);
    }
}
