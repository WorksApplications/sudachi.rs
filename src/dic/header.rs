use nom::le_u64;
use thiserror::Error;

/// Sudachi error
#[derive(Error, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HeaderError {
    #[error("Invalid header")]
    InvalidVersion,

    #[error("Unable to parse")]
    CannotParse,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Header {
    pub version: u64,
    _create_time: u64,
    description: String,
}

impl Header {
    const DESCRIPTION_SIZE: usize = 256;
    const EXPECTED_VERSION: u64 = 0x7366d3f18bd111e7;
    pub const STORAGE_SIZE: usize = 8 + 8 + Header::DESCRIPTION_SIZE;

    pub fn new(bytes: &[u8]) -> Result<(&[u8], Header), HeaderError> {
        let (rest, header) = header_parser(bytes).map_err(|_| HeaderError::CannotParse)?;

        if header.version != Self::EXPECTED_VERSION {
            return Err(HeaderError::InvalidVersion);
        }

        Ok((rest, header))
    }
}

/// Create String from UTF-8 bytes up to NUL byte or end of slice (whichever is first)
fn nul_terminated_str_from_slice(buf: &[u8]) -> String {
    let str_bytes: &[u8] = if let Some(nul_idx) = buf.iter().position(|b| *b == 0) {
        &buf[..nul_idx]
    } else {
        &buf
    };
    String::from_utf8_lossy(str_bytes).to_string()
}

named_args!(
    header_parser()<&[u8], Header>,
    do_parse!(
        version: le_u64 >>
        create_time: le_u64 >>
        desc_buf: take!(Header::DESCRIPTION_SIZE) >>

        (Header{ version,
                 _create_time: create_time,
                 description: nul_terminated_str_from_slice(&desc_buf) })
    )
);

#[cfg(test)]
mod test {
    use super::*;

    fn header_from_parts<T: AsRef<[u8]>>(
        version: u64,
        create_time: u64,
        description: T,
    ) -> Result<Header, HeaderError> {
        let mut bytes = Vec::new();
        bytes.extend(&version.to_le_bytes());
        bytes.extend(&create_time.to_le_bytes());
        bytes.extend(description.as_ref());

        Header::new(&bytes).map(|(_rest, header)| header)
    }

    #[test]
    fn graceful_failure() {
        // Too small
        assert_eq!(Header::new(&[]), Err(HeaderError::CannotParse));

        assert_eq!(
            header_from_parts(42, 0, vec![0; Header::DESCRIPTION_SIZE]),
            Err(HeaderError::InvalidVersion)
        );
    }

    #[test]
    fn simple_header() {
        let mut description: Vec<u8> = Vec::new();
        let description_str = "My Description";
        description.extend(description_str.bytes());
        description.extend(&vec![0; Header::DESCRIPTION_SIZE]);

        assert_eq!(
            header_from_parts(Header::EXPECTED_VERSION, 1337, &description),
            Ok(Header {
                version: Header::EXPECTED_VERSION,
                description: description_str.to_string(),
                _create_time: 1337,
            })
        );
    }
}
