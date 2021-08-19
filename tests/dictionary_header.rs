#[macro_use]
extern crate lazy_static;

extern crate sudachi;
use sudachi::dic::header::{HeaderVersion, SystemDictVersion};

mod common;
use common::HEADER;

#[test]
fn version() {
    assert_eq!(
        HeaderVersion::SystemDict(SystemDictVersion::Version2),
        HEADER.version
    );
}

#[test]
fn create_time() {
    assert!(HEADER.create_time > 0);
}

#[test]
fn description() {
    assert_eq!(
        "the system dictionary for the unit tests",
        HEADER.description
    );
}
