use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::input_text::utf8_input_text::Utf8InputText;
use crate::lattice::{node::Node, Lattice};
use crate::prelude::*;

pub trait PathRewritePlugin {
    fn rewrite(
        &self,
        text: &Utf8InputText,
        path: Vec<Node>,
        lattice: &Lattice,
    ) -> SudachiResult<Vec<Node>>;

    fn concatenate(
        &self,
        mut path: Vec<Node>,
        begin: usize,
        end: usize,
        normalized_form: Option<String>,
    ) -> SudachiResult<Vec<Node>> {
        if begin >= end {
            return Err(SudachiError::InvalidRange(begin, end));
        }

        let b = path[begin].begin;
        let e = path[end - 1].end;
        let word_infos: Vec<_> = path[begin..end]
            .iter()
            .map(|node| node.word_info.clone())
            .collect::<Option<_>>()
            .ok_or(SudachiError::MissingWordInfo)?;
        let pos_id = word_infos[0].pos_id;
        let surface = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.surface);
        let head_word_length = word_infos
            .iter()
            .fold(0, |acc, wi| acc + wi.head_word_length);
        let normalized_form = normalized_form.unwrap_or_else(|| {
            word_infos
                .iter()
                .fold(String::new(), |acc, wi| acc + &wi.normalized_form)
        });
        let reading_form = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.reading_form);
        let dictionary_form = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.dictionary_form);

        let mut node = Node::new_default();
        node.set_range(b, e);
        node.set_word_info(WordInfo {
            surface,
            head_word_length,
            pos_id,
            normalized_form,
            reading_form,
            dictionary_form,
            ..Default::default()
        });

        path[begin] = node;
        path.drain(begin + 1..end);
        Ok(path)
    }

    fn concatenate_oov(
        &self,
        mut path: Vec<Node>,
        begin: usize,
        end: usize,
        pos_id: u16,
    ) -> SudachiResult<Vec<Node>> {
        if begin >= end {
            return Err(SudachiError::InvalidRange(begin, end));
        }

        let b = path[begin].begin;
        let e = path[end - 1].end;
        let word_infos: Vec<_> = path[begin..end]
            .iter()
            .map(|node| node.word_info.clone())
            .collect::<Option<_>>()
            .ok_or(SudachiError::MissingWordInfo)?;
        let surface = word_infos
            .iter()
            .fold(String::new(), |acc, wi| acc + &wi.surface);
        let head_word_length = word_infos
            .iter()
            .fold(0, |acc, wi| acc + wi.head_word_length);

        let mut node = Node::new_default();
        node.set_range(b, e);
        node.set_word_info(WordInfo {
            normalized_form: surface.clone(),
            dictionary_form: surface.clone(),
            surface,
            head_word_length,
            pos_id,
            ..Default::default()
        });

        path[begin] = node;
        path.drain(begin + 1..end);
        Ok(path)
    }
}

pub fn get_path_rewrite_plugins(
    _grammar: &Grammar,
) -> SudachiResult<Vec<Box<dyn PathRewritePlugin + Sync>>> {
    // todo load from config
    let mut plugins: Vec<Box<dyn PathRewritePlugin + Sync>> = vec![];

    // plugins.push(Box::new(join_katakana_oov::JoinKarakanaOovPlugin::new(
    //     grammar,
    // )?));
    // plugins.push(Box::new(join_numeric::JoinNumericPlugin::new(grammar)?));

    Ok(plugins)
}
