pub mod arena;
pub mod source;

#[macro_export]
macro_rules! static_assert {
    ($condition:expr) => {
        const _: () = assert!($condition);
    };
    ($condition:expr, $msg:expr) => {
        const _: () = assert!($condition, $msg);
    };
}

#[macro_export]
macro_rules! static_assert_eq {
    ($left:expr, $right:expr $(,)?) => {
        const _: [(); $left] = [(); $right];
    };
}

#[macro_export]
macro_rules! static_assert_size {
    ($left:ty, $right:expr $(,)?) => {
        const _: [(); std::mem::size_of::<$left>()] = [(); $right];
    };
}
