/*
 * Copyright (c) 2021 Works Applications Co., Ltd.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use nom::{bytes::complete::take, number::complete::le_u64};
use std::io::Write;
use std::time::{Duration, SystemTime};
use thiserror::Error;

use crate::error::{SudachiError, SudachiNomResult, SudachiResult};

/// Sudachi error
#[derive(Error, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum HeaderError {
    #[error("Invalid header version")]
    InvalidVersion,

    #[error("Invalid system dictionary version")]
    InvalidSystemDictVersion,

    #[error("Invalid user dictionary version")]
    InvalidUserDictVersion,

    #[error("Unable to parse")]
    CannotParse,
}

/// Header version
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

impl HeaderVersion {
    pub fn to_u64(&self) -> u64 {
        #[allow(unreachable_patterns)]
        match self {
            HeaderVersion::SystemDict(SystemDictVersion::Version1) => {
                HeaderVersion::SYSTEM_DICT_VERSION_1
            }
            HeaderVersion::SystemDict(SystemDictVersion::Version2) => {
                HeaderVersion::SYSTEM_DICT_VERSION_2
            }
            HeaderVersion::UserDict(UserDictVersion::Version1) => {
                HeaderVersion::USER_DICT_VERSION_1
            }
            HeaderVersion::UserDict(UserDictVersion::Version2) => {
                HeaderVersion::USER_DICT_VERSION_2
            }
            HeaderVersion::UserDict(UserDictVersion::Version3) => {
                HeaderVersion::USER_DICT_VERSION_3
            }
            _ => panic!("unknown version {:?}", self),
        }
    }
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
}

/// Dictionary header
///
/// Contains version, create_time, and description
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Header {
    pub version: HeaderVersion,
    pub create_time: u64,
    pub description: String,
}

impl Header {
    const DESCRIPTION_SIZE: usize = 256;
    pub const STORAGE_SIZE: usize = 8 + 8 + Header::DESCRIPTION_SIZE;

    /// Creates new system dictionary header
    /// Its version field should be modified to create user dictionary header
    pub fn new() -> Self {
        let unix_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("unix time error");

        Self {
            version: HeaderVersion::SystemDict(SystemDictVersion::Version2),
            create_time: unix_time.as_secs(),
            description: String::new(),
        }
    }

    /// Set creation time
    pub fn set_time(&mut self, time: SystemTime) -> SystemTime {
        let unix_time = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("unix time error");

        let old_unix_secs = std::mem::replace(&mut self.create_time, unix_time.as_secs());

        SystemTime::UNIX_EPOCH + Duration::from_secs(old_unix_secs)
    }

    /// Creates a new header from a dictionary bytes
    pub fn parse(bytes: &[u8]) -> Result<Header, HeaderError> {
        let (_rest, (version, create_time, description)) =
            header_parser(bytes).map_err(|_| HeaderError::CannotParse)?;

        let version = HeaderVersion::from_u64(version).ok_or(HeaderError::InvalidVersion)?;

        Ok(Header {
            version,
            create_time,
            description,
        })
    }

    /// Returns if this header version has grammar
    pub fn has_grammar(&self) -> bool {
        match self.version {
            HeaderVersion::SystemDict(_) => true,
            HeaderVersion::UserDict(UserDictVersion::Version2) => true,
            HeaderVersion::UserDict(UserDictVersion::Version3) => true,
            _ => false,
        }
    }

    /// Returns if this header version has synonym group ids
    pub fn has_synonym_group_ids(&self) -> bool {
        match self.version {
            HeaderVersion::SystemDict(SystemDictVersion::Version2) => true,
            HeaderVersion::UserDict(UserDictVersion::Version3) => true,
            _ => false,
        }
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> SudachiResult<usize> {
        if self.description.len() > Header::DESCRIPTION_SIZE {
            return Err(SudachiError::InvalidDataFormat(
                Header::DESCRIPTION_SIZE,
                self.description.clone(),
            ));
        }

        w.write_all(&self.version.to_u64().to_le_bytes())?;
        w.write_all(&self.create_time.to_le_bytes())?;
        w.write_all(&self.description.as_bytes())?;
        for _ in 0..Header::DESCRIPTION_SIZE - self.description.len() {
            w.write_all(&[0])?;
        }
        Ok(Header::STORAGE_SIZE)
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

fn description_parser(input: &[u8]) -> SudachiNomResult<&[u8], String> {
    let (rest, description_bytes) = take(Header::DESCRIPTION_SIZE)(input)?;
    Ok((rest, nul_terminated_str_from_slice(description_bytes)))
}

fn header_parser(input: &[u8]) -> SudachiNomResult<&[u8], (u64, u64, String)> {
    nom::sequence::tuple((le_u64, le_u64, description_parser))(input)
}

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

        Header::parse(&bytes)
    }

    #[test]
    fn graceful_failure() {
        // Too small
        assert_eq!(Header::parse(&[]), Err(HeaderError::CannotParse));

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

    #[test]
    fn write_system() {
        let header = Header::new();
        let mut data: Vec<u8> = Vec::new();
        assert_eq!(header.write_to(&mut data).unwrap(), Header::STORAGE_SIZE);
        let header2 = Header::parse(&data).unwrap();
        assert_eq!(header, header2);
    }

    #[test]
    fn write_user() {
        let mut header = Header::new();
        header.version = HeaderVersion::UserDict(UserDictVersion::Version3);
        header.description = String::from("some great header");
        let mut data: Vec<u8> = Vec::new();
        assert_eq!(header.write_to(&mut data).unwrap(), Header::STORAGE_SIZE);
        let header2 = Header::parse(&data).unwrap();
        assert_eq!(header, header2);
    }
}
