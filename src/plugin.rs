pub mod connect_cost;
pub mod input_text;
pub mod oov;
pub mod path_rewrite;

use libloading::Error as LLError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Libloading Error: {0}")]
    Libloading(#[from] LLError),
}
