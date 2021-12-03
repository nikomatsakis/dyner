use std::{
    future::Future,
    ops::{Deref, DerefMut},
    pin::Pin,
    rc::Rc,
};

trait Len {
    fn len(&self) -> usize;
}

// Given an Ptr<T> where T: Len...
//
// * where `Ptr<T>: Deref<Target = T>`
// * and `sizeof(Ptr<T>) == sizeof(*const T)`
//
// we want to have a trait

// `*const dyn ErasedLen`
//
// when we drop, we're going to call `drop_me()` which takes a `*const T`
//
// and it should know to transmute the `*const` to an `Rc<T>` and drop it

trait ErasedLen {
    fn len(&self) -> usize;

    fn drop_me(&self);
}

trait RawDeref: Deref {
    fn into_raw(this: Self) -> *const Self::Target;

    // Unsafe: `target` must have been returned from `into_raw`
    unsafe fn from_raw(target: *const Self::Target) -> Self;
}

impl<T> RawDeref for Rc<T> {
    fn into_raw(this: Self) -> *const T {
        Rc::into_raw(this)
    }

    unsafe fn from_raw(target: *const T) -> Self {
        Rc::from_raw(target)
    }
}

/// Remember<T> is a bit of a funky type. The idea is that you have a pointer
/// type like `Rc<T>` and you are going to transmute it to a `*const U`; but
/// you'd like to remember in the type of U what the real pointer type is (i.e,
/// that this `*const` is actually an `Rc`).
///
/// takes a pointer type T = Ptr<U> and
///
#[repr(transparent)]
struct Remember<T: RawDeref> {
    t: T::Target,
}

impl<T: RawDeref> Remember<T> {
    pub fn new(value: T) -> *const Self {
        let ptr: *const T::Target = RawDeref::into_raw(value);
        ptr as *const Self
    }
}

impl<T> ErasedLen for Remember<T>
where
    T: Deref,
    T::Target: Len,
{
    fn len(&self) -> usize {
        Len::len(&self.t)
    }

    // FIXME: This is probably UB, and should be *const self
    fn drop_me(&self) {
        unsafe {
            let value: T = T::from_raw(self.t);
        }
    }
}

// dyn &Foo
// &Foo was shorthand for Deref<Target: Foo>

struct DynLen {
    ptr: *const dyn ErasedLen,
}

impl<T> From<Rc<T>> for DynLen
where
    T: Len,
{
    fn from(value: Rc<T>) -> DynLen {
        unsafe {
            let v: *const Remember<Rc<T>> = Remember::new(value);
            let v: *const dyn ErasedLen = v;
            DynLen { ptr: v }
        }
    }
}

impl Len for DynLen {
    fn len(&self) -> usize {
        unsafe { ErasedLen::len(&*self.ptr) }
    }
}

impl Drop for DynLen {
    fn drop(&mut self) {
        unsafe { ErasedLen::drop_me(&*self.ptr) }
    }
}
