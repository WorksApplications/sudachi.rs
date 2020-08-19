use std::str;

use nom::le_u64;

pub struct Header {
    pub version: u64,
    _create_time: u64,
    _description: String,
}

impl Header {
    const DESCRIPTION_SIZE: usize = 256;
    pub const STORAGE_SIZE: usize = 8 + 8 + Header::DESCRIPTION_SIZE;

    pub fn new(bytes: &[u8], offset: usize) -> Header {
        let (_rest, header) = header_parser(bytes, offset).unwrap();
        assert_eq!(header.version, 0x7366d3f18bd111e7);

        header
    }
}

named_args!(
    header_parser(offset: usize)<&[u8], Header>,
    do_parse!(
        _seek: take!(offset) >>
        version: le_u64 >>
        create_time: le_u64 >>
        desc_buf: take!(Header::DESCRIPTION_SIZE) >>

        (Header{ version,
                 _create_time: create_time,
                 _description: str::from_utf8(&desc_buf).unwrap().to_string() })
                 // alternative: lossy_utf8, unchecked
    )
);
