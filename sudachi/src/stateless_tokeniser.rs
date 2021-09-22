use crate::plugin::input_text::InputTextPlugin;
use crate::plugin::oov::OovProviderPlugin;
use crate::plugin::path_rewrite::PathRewritePlugin;
use crate::dic::lexicon_set::LexiconSet;
use crate::dic::grammar::Grammar;
use crate::prelude::{Tokenize, Mode};
use crate::error::SudachiResult;
use crate::morpheme::Morpheme;
use serde_json::ser::State;

pub trait DictionaryAccess {
    fn grammar(&self) -> &Grammar<'_>;
    fn lexicon(&self) -> &LexiconSet<'_>;
    fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin>];
    fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin>];
    fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin>];
}

pub struct StatelessTokenizer<'a, T: DictionaryAccess> {
    dict: &'a T,
}

impl<'a, T: DictionaryAccess> StatelessTokenizer<'a, T> {
    pub fn new(dict: &T) -> StatelessTokenizer<T> { StatelessTokenizer { dict } }
}

impl<'a, T: DictionaryAccess> Tokenize for StatelessTokenizer<'a, T> {
    fn tokenize(&self, input: &str, mode: Mode, enable_debug: bool) -> SudachiResult<Vec<Morpheme>> {
        todo!()
    }

    fn tokenize_sentences(&self, input: &str, mode: Mode, enable_debug: bool) -> SudachiResult<Vec<Vec<Morpheme>>> {
        todo!()
    }
}