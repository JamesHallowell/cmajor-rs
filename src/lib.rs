mod ffi;
use ffi::Library;

pub struct CMajor {
    library: Library,
}

impl CMajor {
    pub fn new() -> Self {
        let library = Library::load("libCmajPerformer.dylib").unwrap();

        Self { library }
    }
}
