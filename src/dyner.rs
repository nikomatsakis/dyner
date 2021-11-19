use std::{future::Future, mem::MaybeUninit, pin::Pin};

pub struct Ref<T> {
    t: T,
}

impl<T> Ref<T> {
    pub fn new(t: T) -> Self {
        Self { t }
    }
}

impl<T> std::ops::Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.t
    }
}

/// Newtype that permits borrowed (`&mut T`) or shared (`&T`) access,
/// but nothing else.
pub struct RefMut<T> {
    t: T,
}

impl<T> RefMut<T> {
    pub fn new(t: T) -> Self {
        Self { t }
    }
}

impl<T> std::ops::Deref for RefMut<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.t
    }
}

impl<T> std::ops::DerefMut for RefMut<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.t
    }
}

pub struct InlineFuture<'me, Output> {
    future: &'me mut dyn Future<Output = Output>,
}

impl<'me, Output> InlineFuture<'me, Output> {
    /// Safe:
    ///
    /// * `*future` belongs to us for duration of `'me`
    ///
    /// Unsafe:
    ///
    /// * `*future` must be initialized
    /// * `*future` must not be used again without having been reinitialized (which must occur after `'me` ends)
    pub unsafe fn new(future: &'me mut MaybeUninit<impl Future<Output = Output>>) -> Self {
        Self {
            future: future.assume_init_mut(),
        }
    }
}

impl<'me, Output> Drop for InlineFuture<'me, Output> {
    fn drop(&mut self) {
        unsafe {
            std::ptr::drop_in_place(self.future);
        }
    }
}

impl<'me, Output> Future for InlineFuture<'me, Output> {
    type Output = Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        unsafe {
            let mu: &mut dyn Future<Output = Output> = &mut *self.get_unchecked_mut().future;
            let mut mu_pin: Pin<&mut dyn Future<Output = Output>> = Pin::new_unchecked(mu);
            <Pin<&mut dyn Future<Output = Output>> as Future>::poll(Pin::new(&mut mu_pin), cx)
        }
    }
}
