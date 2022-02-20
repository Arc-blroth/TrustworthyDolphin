/// Kotlin-style `also` extension function.
pub trait Also<C> {
    fn also(self, callback: C) -> Self;
}

impl <T, C> Also<C> for T
where
    C: FnMut(&mut T)
{
    #[inline(always)]
    fn also(mut self, mut callback: C) -> Self {
        callback(&mut self);
        self
    }
}
