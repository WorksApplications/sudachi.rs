//! Clone of [Sudachi](https://github.com/WorksApplications/Sudachi),
//! a Japanese morphological analyzer

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate nom;

pub mod dic;
pub mod error;
pub mod lattice;
pub mod morpheme;
pub mod tokenizer;

pub use error::*;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::{
        tokenizer::{Mode, Tokenize, Tokenizer},
        SudachiError, SudachiResult,
    };
}
