//! Clone of [Sudachi](https://github.com/WorksApplications/Sudachi),
//! a Japanese morphological analyzer

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate nom;

pub mod morpheme;
pub mod tokenizer;

pub mod dic;
pub mod lattice;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::tokenizer::{Mode, Tokenize, Tokenizer};
}
