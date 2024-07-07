use {
    crate::value::types::{Primitive, Type},
    std::{any::Any, cell::RefCell, ffi::c_void, panic::UnwindSafe, ptr::null_mut},
};

pub fn get_external_function(name: &str, signature: &[Type]) -> *mut c_void {
    match (name, signature) {
        ("rust::test::assert", &[Type::Primitive(Primitive::Bool)]) => rust_assert as *mut c_void,
        (
            "rust::test::assertEqual",
            &[Type::Primitive(Primitive::Int32), Type::Primitive(Primitive::Int32)],
        ) => rust_assert_eq_i32 as *mut c_void,
        (
            "rust::test::assertEqual",
            &[Type::Primitive(Primitive::Int64), Type::Primitive(Primitive::Int64)],
        ) => rust_assert_eq_i64 as *mut c_void,
        (
            "rust::test::assertEqual",
            &[Type::Primitive(Primitive::Float32), Type::Primitive(Primitive::Float32)],
        ) => rust_assert_eq_f32 as *mut c_void,
        (
            "rust::test::assertEqual",
            &[Type::Primitive(Primitive::Float64), Type::Primitive(Primitive::Float64)],
        ) => rust_assert_eq_f64 as *mut c_void,
        ("rust::debug::print", &[Type::Primitive(Primitive::Bool)]) => {
            rust_print_bool as *mut c_void
        }
        ("rust::debug::print", &[Type::Primitive(Primitive::Int32)]) => {
            rust_print_i32 as *mut c_void
        }
        ("rust::debug::print", &[Type::Primitive(Primitive::Int64)]) => {
            rust_print_i64 as *mut c_void
        }
        ("rust::debug::print", &[Type::Primitive(Primitive::Float32)]) => {
            rust_print_f32 as *mut c_void
        }
        ("rust::debug::print", &[Type::Primitive(Primitive::Float64)]) => {
            rust_print_f64 as *mut c_void
        }
        _ => null_mut(),
    }
}

pub fn check_for_panic() {
    PANIC.with(|panic| {
        if let Some(err) = panic.borrow_mut().take() {
            std::panic::resume_unwind(err);
        }
    });
}

thread_local! {
    static PANIC: RefCell<Option<Box<dyn Any + Send>>> = RefCell::new(None);
}

fn catch_unwind_and_store_panic<F: FnOnce() -> R + UnwindSafe, R>(f: F) {
    let panic = std::panic::catch_unwind(f);

    if let Err(err) = panic {
        PANIC.with(|panic| {
            if panic.borrow().is_none() {
                panic.replace(Some(err));
            }
        });
    }
}

extern "C" fn rust_assert(condition: bool) {
    catch_unwind_and_store_panic(|| {
        assert!(condition, "cmajor assertion failed");
    });
}

macro_rules! make_assert_eq_fn {
    ($name:ident, $t:ty) => {
        extern "C" fn $name(a: $t, b: $t) {
            catch_unwind_and_store_panic(|| {
                assert_eq!(a, b, "cmajor assertion failed");
            });
        }
    };
}

make_assert_eq_fn!(rust_assert_eq_i32, i32);
make_assert_eq_fn!(rust_assert_eq_i64, i64);
make_assert_eq_fn!(rust_assert_eq_f32, f32);
make_assert_eq_fn!(rust_assert_eq_f64, f64);

macro_rules! make_print_fn {
    ($name:ident, $t:ty) => {
        extern "C" fn $name(value: $t) {
            catch_unwind_and_store_panic(|| {
                println!("{}", value);
            });
        }
    };
}

make_print_fn!(rust_print_bool, bool);
make_print_fn!(rust_print_i32, i32);
make_print_fn!(rust_print_i64, i64);
make_print_fn!(rust_print_f32, f32);
make_print_fn!(rust_print_f64, f64);
