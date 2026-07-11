use std::{
    cell::Cell,
    ffi::CString,
    sync::{Arc, atomic::AtomicBool, mpsc},
    thread,
    time::Duration,
};

use skilltap_test_support::TempRoot;

use super::*;

struct TempDirectory(TempRoot);

impl TempDirectory {
    fn new() -> Self {
        Self(TempRoot::new("skilltap-filesystem-test").unwrap())
    }

    fn path(&self, child: &str) -> AbsolutePath {
        AbsolutePath::new(self.0.join(child).to_str().unwrap()).unwrap()
    }
}

include!("tests/metadata.rs");
include!("tests/publication.rs");
include!("tests/ownership.rs");
include!("tests/locking.rs");
