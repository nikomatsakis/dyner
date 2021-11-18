pub struct Ref<T> {
    t: T,
}

impl<T> Ref<T> {
    pub fn new(t: T) -> Self {
        Ref { t }
    }
}

impl<T> std::ops::Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.t
    }
}
