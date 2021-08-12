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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderVersion {
    SystemDict(SystemDictVersion),
    UserDict(UserDictVersion),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemDictVersion {
    // we cannot set value since value can be larger than isize
    Version1,
    Version2,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserDictVersion {
    Version1,
    Version2,
    Version3,
}
impl HeaderVersion {
    /// the first version of system dictionaries
    const SYSTEM_DICT_VERSION_1: u64 = 0x7366d3f18bd111e7;
    /// the second version of system dictionaries
    const SYSTEM_DICT_VERSION_2: u64 = 0xce9f011a92394434;
    /// the first version of user dictionaries
    const USER_DICT_VERSION_1: u64 = 0xa50f31188bd211e7;
    /// the second version of user dictionaries
    const USER_DICT_VERSION_2: u64 = 0x9fdeb5a90168d868;
    /// the third version of user dictionaries
    const USER_DICT_VERSION_3: u64 = 0xca9811756ff64fb0;

    pub fn from_u64(v: u64) -> Option<Self> {
        match v {
            HeaderVersion::SYSTEM_DICT_VERSION_1 => {
                Some(Self::SystemDict(SystemDictVersion::Version1))
            }
            HeaderVersion::SYSTEM_DICT_VERSION_2 => {
                Some(Self::SystemDict(SystemDictVersion::Version2))
            }
            HeaderVersion::USER_DICT_VERSION_1 => Some(Self::UserDict(UserDictVersion::Version1)),
            HeaderVersion::USER_DICT_VERSION_2 => Some(Self::UserDict(UserDictVersion::Version2)),
            HeaderVersion::USER_DICT_VERSION_3 => Some(Self::UserDict(UserDictVersion::Version3)),
            _ => None,
        }
    }

    pub fn has_grammar(&self) -> bool {
        match self {
            HeaderVersion::UserDict(UserDictVersion::Version2) => true,
            HeaderVersion::UserDict(UserDictVersion::Version3) => true,
            _ => false,
        }
    }

    pub fn has_synonym_group_ids(&self) -> bool {
        match self {
            HeaderVersion::SystemDict(SystemDictVersion::Version2) => true,
            HeaderVersion::UserDict(UserDictVersion::Version3) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Header {
    pub version: HeaderVersion,
    pub create_time: u64,
    pub description: String,
}

impl Header {
    const DESCRIPTION_SIZE: usize = 256;
    const EXPECTED_VERSION: HeaderVersion = HeaderVersion::SystemDict(SystemDictVersion::Version1);
    pub const STORAGE_SIZE: usize = 8 + 8 + Header::DESCRIPTION_SIZE;

    pub fn new(bytes: &[u8]) -> Result<Header, HeaderError> {
        let (_rest, (version, create_time, description)) =
            header_parser(bytes).map_err(|_| HeaderError::CannotParse)?;

        let version = match HeaderVersion::from_u64(version) {
            Some(v) => {
                if Header::EXPECTED_VERSION != v {
                    return Err(HeaderError::InvalidVersion);
                }
                v
            }
            None => {
                return Err(HeaderError::InvalidVersion);
            }
        };

        Ok(Header {
            version,
            create_time,
            description,
        })
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
    header_parser()<&[u8], (u64, u64, String)>,
    do_parse!(
        version: le_u64 >>
        create_time: le_u64 >>
        desc_buf: take!(Header::DESCRIPTION_SIZE) >>

        (version, create_time, nul_terminated_str_from_slice(&desc_buf))
    )
);

#[cfg(test)]
mod tests {
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

        Header::new(&bytes)
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
            header_from_parts(HeaderVersion::SYSTEM_DICT_VERSION_1, 1337, &description),
            Ok(Header {
                version: HeaderVersion::SystemDict(SystemDictVersion::Version1),
                description: description_str.to_string(),
                create_time: 1337,
            })
        );
    }
}
