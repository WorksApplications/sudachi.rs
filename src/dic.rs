use nom::{le_u16, le_u8};

use crate::error::SudachiNomCustomError;

pub mod grammar;
pub mod header;
pub mod lexicon;

named!(
    utf16_string<&[u8], String>,
    do_parse!(
        length: le_u8 >>
        v: count!(le_u16, length as usize) >>

        (String::from_utf16(&v)
            .map_err(|_| nom::Err::Failure(
                nom::Context::Code(&[] as &[u8], nom::ErrorKind::Custom(SudachiNomCustomError::FromUtf16Nom as u32))))?
        )
    )
);
