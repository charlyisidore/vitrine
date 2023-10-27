/// Make the lifetime of a variable `'static` (unsafe).
///
/// Some libraries require `'static` lifetime for function arguments, so we
/// cannot use values created at runtime. Therefore, this unsafe function can be
/// used as a workaround.
pub(crate) unsafe fn static_lifetime<T: ?Sized>(value: &T) -> &'static T {
    std::mem::transmute::<&T, &'static T>(value)
}
