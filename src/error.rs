use std::convert::TryFrom;
use std::fmt::Debug;

use thiserror::Error;

use crate::dic::header::HeaderError;

pub type SudachiResult<T> = Result<T, SudachiError>;

/// Sudachi error
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SudachiError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid header: {0}")]
    InvalidHeader(#[from] HeaderError),

    #[error("Invalid header")]
    InvalidDictionaryGrammar,

    #[error("Error parsing nom: {}", .0.description())]
    NomParse(nom::ErrorKind<u32>),

    #[error("Missing word_id")]
    MissingWordId,

    #[error("Missing part of speech")]
    MissingPartOfSpeech,

    #[error("Missing latice path")]
    MissingLaticePath,

    #[error("Missing dictionary trie")]
    MissingDictionaryTrie,

    #[error("Invalid UTF-16: {0}")]
    FromUtf16(#[from] std::string::FromUtf16Error),

    #[error("Invalid UTF-16 from nom")]
    FromUtf16Nom,

    #[error("EOS isn't connected to BOS")]
    EosBosDisconnect,
}

/// Define `SudachiNomCustomError` error and conversion to `SudachiError`.
/// The variants that can be converted will have the same name.
macro_rules! define_nom_error_enum {
    (
        $( $name:ident = $value_int:literal , )*
    ) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        pub(crate) enum SudachiNomCustomError {
            $($name = $value_int , )*
        }

        impl TryFrom<u32> for SudachiError {
            type Error = ();
            fn try_from(x: u32) -> Result<Self, Self::Error> {
                match x {
                    $( $value_int => Ok(SudachiError::$name) , )*
                    _ => Err(()),
                }
            }
        }
    };
}

define_nom_error_enum! {
    FromUtf16Nom = 0,
}

impl<I> From<nom::Err<I, u32>> for SudachiError {
    fn from(err: nom::Err<I, u32>) -> Self {
        if let nom::Err::Failure(nom::Context::Code(_v, nom::ErrorKind::Custom(custom_error))) =
            &err
        {
            if let Ok(sudachi_error) = SudachiError::try_from(*custom_error) {
                return sudachi_error;
            }
        }
        SudachiError::NomParse(err.into_error_kind())
    }
}
