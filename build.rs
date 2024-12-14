#[cfg(feature = "static")]
mod static_linkage {
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };

    fn build_cmajor_static_lib() -> PathBuf {
        let mut cmake = cmake::Config::new("static");
        cmake.build_target("cmajor-static");
        if let Some(out_dir) = build_directory() {
            cmake.out_dir(out_dir);
        }
        cmake.very_verbose(true);
        cmake.build()
    }

    fn build_directory() -> Option<PathBuf> {
        env::var("CMAJOR_BUILD_DIR").map(PathBuf::from).ok()
    }

    fn link_cmajor_static_lib(path: &Path) {
        println!("cargo:rustc-link-search=native={}/build", path.display());
        println!("cargo:rustc-link-lib=static=cmajor-static");
    }

    fn link_platform_libs() {
        if cfg!(target_os = "macos") {
            for library in ["c++", "z"] {
                println!("cargo:rustc-link-lib={}", library);
            }

            for framework in [
                "Accelerate",
                "AudioToolbox",
                "Cocoa",
                "CoreAudio",
                "CoreFoundation",
                "CoreMIDI",
                "IOKit",
                "WebKit",
                "Security",
            ] {
                println!("cargo:rustc-link-lib=framework={}", framework);
            }
        }
    }

    fn link_llvm_libs(path: &Path) {
        let llvm_libs_path =
            path.join("build/_deps/cmajor-src/3rdParty/llvm/release/osx/universal/lib");

        println!(
            "cargo:rustc-link-search=native={}",
            llvm_libs_path.display()
        );

        for entry in fs::read_dir(llvm_libs_path)
            .unwrap()
            .map(Result::<_, _>::unwrap)
        {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let lib_name = file_name
                .as_ref()
                .strip_prefix("lib")
                .and_then(|str| str.strip_suffix(".a"))
                .unwrap();

            println!("cargo:rustc-link-lib=static={}", lib_name);
        }
    }

    pub fn link_cmajor() {
        let _ = dotenvy::dotenv();
        let dir = build_cmajor_static_lib();
        link_cmajor_static_lib(&dir);
        link_platform_libs();
        link_llvm_libs(&dir);
    }
}

fn link_math_lib() {
    if cfg!(target_os = "linux") {
        // see https://github.com/cmajor-lang/cmajor/issues/84
        println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
        println!("cargo:rustc-link-arg=-lm");
    }
}

fn main() {
    #[cfg(feature = "static")]
    static_linkage::link_cmajor();

    #[cfg(not(feature = "static"))]
    link_math_lib();
}
