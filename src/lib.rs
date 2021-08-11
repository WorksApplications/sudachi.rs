//! Clone of [Sudachi](https://github.com/WorksApplications/Sudachi),
//! a Japanese morphological analyzer
//!
//! The main entry point of the library is the
//! [`Tokenizer`](tokenizer/struct.Tokenizer.html) struct, which
//! implements [`Tokenize`](tokenizer/trait.Tokenize.html).

#[macro_use]
extern crate nom;

pub mod dic;
pub mod error;
pub mod lattice;
pub mod morpheme;
pub mod plugin;
pub mod tokenizer;
pub mod utf8inputtext;

pub use error::*;

pub mod prelude {
    pub use crate::{
        morpheme::Morpheme,
        tokenizer::{dictionary_bytes_from_path, Mode, Tokenize, Tokenizer},
        SudachiError, SudachiResult,
    };
}
