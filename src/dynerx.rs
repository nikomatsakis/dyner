use std::{
    ops::Deref,
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
        this as _
    }

    unsafe fn from_raw(target: *const T) -> Self {
        std::mem::transmute(target)
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

    // FIXME: This is probably UB, and should be *const self
    fn drop_me(&self) {
        unsafe {
            let _value: T = T::from_raw(&self.t as *const _);
        }
    }
}

// dyn &Foo
// &Foo was shorthand for Deref<Target: Foo>

struct DynLen<'data> {
    ptr: *const (dyn ErasedLen + 'data),
}

impl<'data, T, P: 'data> From<P> for DynLen<'data>
where
    T: Len,
    P: RawDeref<Target = T>,
{
    fn from(value: P) -> DynLen<'data> {
        let v: *const Remember<P> = Remember::new(value);
        let v: *const (dyn ErasedLen + 'data) = v;
        DynLen { ptr: v }
    }
}

impl Len for DynLen<'_> {
    fn len(&self) -> usize {
        unsafe { ErasedLen::len(&*self.ptr) }
    }
}

impl Drop for DynLen<'_> {
    fn drop(&mut self) {
        unsafe { ErasedLen::drop_me(&*self.ptr) }
    }
}

// FIXME: Get this working with [T].
// The unsized coercion from Remember<P> above doesn't support already-unsized targets.
impl<T, const N: usize> Len for [T; N] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use super::*;

    fn get_len(x: DynLen<'_>) -> usize {
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
        let local_items = [1, 2, 3];
        let dyn_items = DynLen::from(&local_items);
        assert_eq!(3, get_len(dyn_items));

        let drop_counter = DropCounter::new();
        let box_items = Box::new([None, None, Some(drop_counter.clone())]);
        let dyn_items = DynLen::from(box_items);
        assert_eq!(0, drop_counter.count());
        assert_eq!(3, get_len(dyn_items));
        assert_eq!(1, drop_counter.count());

        let drop_counter = DropCounter::new();
        let rc_items = Rc::new([None, None, Some(drop_counter.clone())]);
        let rc_items2 = Rc::clone(&rc_items);
        let dyn_items = DynLen::from(rc_items);
        assert_eq!(0, drop_counter.count());
        assert_eq!(3, get_len(dyn_items));
        assert_eq!(0, drop_counter.count());
        assert_eq!(3, get_len(DynLen::from(rc_items2)));
        assert_eq!(1, drop_counter.count());
    }
}
