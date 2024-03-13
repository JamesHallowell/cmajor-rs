#[cfg(feature = "static")]
mod static_linkage {
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };

    fn build_cmajor_static_lib() -> PathBuf {
        let mut cmake = cmake::Config::new("static");
        cmake.build_target("cmajor-static");
        cmake.very_verbose(true);
        cmake.build()
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

    fn llvm_libs_path() -> PathBuf {
        let out_dir = env::var("OUT_DIR").unwrap();
        PathBuf::from(format!(
            "{out_dir}/build/_deps/cmajor-src/3rdParty/llvm/release/osx/universal/lib"
        ))
    }

    fn link_llvm_libs() {
        println!(
            "cargo:rustc-link-search=native={}",
            llvm_libs_path().display()
        );

        for entry in fs::read_dir(llvm_libs_path())
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
        link_cmajor_static_lib(&build_cmajor_static_lib());
        link_platform_libs();
        link_llvm_libs();
    }
}

fn main() {
    #[cfg(feature = "static")]
    static_linkage::link_cmajor();
}
