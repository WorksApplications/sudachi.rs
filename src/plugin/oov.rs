pub mod mecab_oov;
pub mod simple_oov;

use crate::dic::grammar::Grammar;
use crate::lattice::node::Node;
use crate::prelude::*;
use crate::utf8inputtext::Utf8InputText;

pub trait OovProviderPlugin {
    fn provide_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>>;

    fn get_oov(
        &self,
        input_text: &Utf8InputText,
        offset: usize,
        has_other_words: bool,
    ) -> SudachiResult<Vec<Node>> {
        let mut nodes = self.provide_oov(input_text, offset, has_other_words)?;
        for node in &mut nodes {
            let length = node.word_info.as_ref().unwrap().head_word_length as usize;
            node.set_range(offset, offset + length);
        }
        Ok(nodes)
    }
}

pub fn get_oov_plugins(grammar: &Grammar) -> SudachiResult<Vec<Box<dyn OovProviderPlugin + Sync>>> {
    // todo load from config
    let mut oovs: Vec<Box<dyn OovProviderPlugin + Sync>> = vec![];

    oovs.push(Box::new(mecab_oov::MeCabOovPlugin::new(grammar)?));
    oovs.push(Box::new(simple_oov::SimpleOovPlugin::new(grammar)?));

    Ok(oovs)
}
