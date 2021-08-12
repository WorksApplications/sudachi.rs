#[macro_use]
extern crate lazy_static;

extern crate sudachi;
use sudachi::dic::header::{HeaderVersion, SystemDictVersion};

mod common;
use common::HEADER;

#[test]
fn version() {
    assert_eq!(
        HeaderVersion::SystemDict(SystemDictVersion::Version1),
        HEADER.version
    );
}

#[test]
fn create_time() {
    assert!(HEADER.create_time > 0);
}

#[test]
fn description() {
    // todo: this fails. load dict for test
    assert_eq!(
        "the system dictionary for the unit tests",
        HEADER.description
    );
}
