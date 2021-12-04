use std::{
    ops::{Deref, DerefMut},
    rc::Rc,
};
use crate::dyner::{Ref, RefMut};

trait Len {
    fn len(&self) -> usize;
    fn modify(&mut self);
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
    fn modify(&mut self);

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

impl<T> RawDeref for Box<T> {
    fn into_raw(this: Self) -> *const T {
        Box::into_raw(this)
    }

    unsafe fn from_raw(target: *const T) -> Self {
        Box::from_raw(target as *mut T)
    }
}

impl<T> RawDeref for &T {
    fn into_raw(this: Self) -> *const T {
        this
    }

    unsafe fn from_raw(target: *const T) -> Self {
        &*target
    }
}

impl<T> RawDeref for &mut T {
    fn into_raw(this: Self) -> *const T {
        this
    }

    unsafe fn from_raw(target: *const T) -> Self {
        // Cast to *mut is okay because this method's invariant is that target
        // will always come from Self::into_raw.
        &mut *(target as *mut T)
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
    T: RawDeref,
    T::Target: Len,
{
    fn len(&self) -> usize {
        Len::len(&self.t)
    }

    fn modify(&mut self) {
        Len::modify(&mut self.t)
    }

    // FIXME: This is probably UB, and should be *const self
    fn drop_me(&self) {
        unsafe {
            let _value: T = T::from_raw(std::ptr::addr_of!(self.t));
        }
    }
}

// dyn &Foo
// &Foo was shorthand for Deref<Target: Foo>

struct DynLen<'data> {
    ptr: *mut (dyn ErasedLen + 'data),
}

impl<'data> DynLen<'data> {
    #[allow(dead_code)]
    fn from_ref<P>(value: P) -> Ref<DynLen<'data>>
    where
        P: RawDeref + 'data,
        <P as Deref>::Target: Len + Sized,
    {
        // Cast to *mut is okay because we're guarding everything behind Ref.
        let v: *mut Remember<P> = Remember::new(value) as _;
        let v: *mut (dyn ErasedLen + 'data) = v;
        Ref::new(DynLen { ptr: v })
    }

    #[allow(dead_code)]
    fn from_mut<P>(value: P) -> RefMut<DynLen<'data>>
    where
        P: RawDeref + DerefMut + 'data,
        <P as Deref>::Target: Len + Sized,
    {
        // Cast to *mut is okay because P: DerefMut.
        let v: *mut Remember<P> = Remember::new(value) as _;
        let v: *mut (dyn ErasedLen + 'data) = v;
        RefMut::new(DynLen { ptr: v })
    }
}

impl Len for DynLen<'_> {
    fn len(&self) -> usize {
        unsafe { ErasedLen::len(&*self.ptr) }
    }

    fn modify(&mut self) {
        unsafe { ErasedLen::modify(&mut *self.ptr) }
    }
}

impl Drop for DynLen<'_> {
    fn drop(&mut self) {
        unsafe { ErasedLen::drop_me(&*self.ptr) }
    }
}

// FIXME: Get this working with [T].
// The unsized coercion from Remember<P> above doesn't support already-unsized targets.
impl<T: Default, const N: usize> Len for [T; N] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn modify(&mut self) {
        if self.len() != 0 {
            self[0] = Default::default();
        }
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use super::*;

    fn get_len(x: &dyn Len) -> usize {
        x.len()
    }

    #[derive(Clone, Debug)]
    struct DropCounter(Rc<RefCell<usize>>);
    impl DropCounter {
        fn new() -> Self {
            Self(Rc::new(RefCell::new(0)))
        }

        fn count(&self) -> usize {
            *self.0.borrow()
        }
    }

    impl Drop for DropCounter {
        fn drop(&mut self) {
            *self.0.borrow_mut() += 1;
        }
    }

    #[test]
    fn test_len() {
        let mut local_items = [1, 2, 3];
        assert_eq!(3, get_len(&*DynLen::from_ref(&local_items)));
        {
            let mut dyn_mut_items = DynLen::from_mut(&mut local_items);
            dyn_mut_items.modify();
            assert_eq!(3, dyn_mut_items.len());
        }
        assert_eq!(0, local_items[0]);

        let drop_counter = DropCounter::new();
        let box_items = Box::new([Some(drop_counter.clone()), None, Some(drop_counter.clone())]);
        {
            let mut dyn_items = DynLen::from_mut(box_items);
            assert_eq!(0, drop_counter.count());
            assert_eq!(3, get_len(&*dyn_items));
            dyn_items.modify(); // drops the first element
            assert_eq!(1, drop_counter.count());
        }
        assert_eq!(2, drop_counter.count());

        let drop_counter = DropCounter::new();
        let rc_items = Rc::new([None, None, Some(drop_counter.clone())]);
        let rc_items2 = Rc::clone(&rc_items);
        {
            let dyn_items = DynLen::from_ref(rc_items);
            assert_eq!(0, drop_counter.count());
            assert_eq!(3, get_len(&*dyn_items));
            // dyn_items.modify(); <-- does not compile
        }
        assert_eq!(0, drop_counter.count());
        assert_eq!(3, get_len(&*DynLen::from_ref(rc_items2)));
        assert_eq!(1, drop_counter.count());
    }
}
