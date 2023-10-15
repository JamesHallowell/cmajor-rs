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
cmajor = "0.1"
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

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE))
* MIT license
  ([LICENSE-MIT](LICENSE-MIT))

at your option.

The end-user license for the Cmajor redistributable library can be
found [here](https://github.com/SoundStacks/cmajor/blob/main/EULA.md#cmajor-end-user-license-agreement).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
