# cmajor-rs

[![Build](https://github.com/JamesHallowell/cmajor-rs/actions/workflows/build.yml/badge.svg)](https://github.com/JamesHallowell/cmajor-rs/actions/workflows/build.yml)
[![Crates.io](https://img.shields.io/crates/v/cmajor.svg)](https://crates.io/crates/cmajor)
[![Docs.rs](https://docs.rs/cmajor/badge.svg)](https://docs.rs/cmajor)

**Rust bindings for the [Cmajor](https://cmajor.dev/) JIT engine.**

## Overview

Work-in-progress bindings for the [Cmajor](https://cmajor.dev/) JIT engine, to enable embedding Cmajor programs in Rust
apps.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
cmajor = "0.4"
```

You will also need to [download the Cmajor library](https://github.com/SoundStacks/cmajor/releases) and tell the crate
where
to find it, either by:

1. Passing the path on construction:

    ```rust
    use {cmajor::Cmajor, std::error::Error};
    
    fn main() -> Result<(), Box<dyn Error>> {
        let cmajor = Cmajor::new("path/to/libCmajPerformer.so")?;
    }
    ```

2. Setting the `CMAJOR_LIB_PATH` environment variable (`.env` files are supported):

    ```
    CMAJOR_LIB_PATH=path/to/libCmajPerformer.so
    ```

    ```rust
    use cmajor::{Cmajor, std::error::Error};
    
    fn main() -> Result<(), Box<dyn Error>> {
        let cmajor = Cmajor::new_from_env()?;
    }
    ```

## Crate Features

### `static` (Experimental)

It is possible to statically link to Cmajor to avoid having to load the library dynamically at runtime. This will build
the library from source, so you'll need to have the necessary build tools installed. This feature is disabled by
default, and only has experimental support on macOS.

## License

Licensed under GPLv3 (or later). Refer to the [Cmajor licensing terms](https://cmajor.dev/docs/Licence) for more
information.
