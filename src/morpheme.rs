use crate::dic::grammar::Grammar;
use crate::dic::lexicon::word_infos::WordInfo;
use crate::dic::lexicon::Lexicon;

pub struct Morpheme<'a> {
    word_info: WordInfo,
    grammar: &'a Grammar<'a>,
}

impl<'a> Morpheme<'a> {
    pub fn new(word_id: usize, grammar: &'a Grammar<'a>, lexicon: &Lexicon) -> Morpheme<'a> {
        let word_info = lexicon.get_word_info(word_id);
        Morpheme { word_info, grammar }
    }

    pub fn surface(&self) -> &String {
        &self.word_info.surface
    }

    pub fn pos(&self) -> &Vec<String> {
        &self
            .grammar
            .pos_list
            .get(self.word_info.pos_id as usize)
            .unwrap()
    }

    pub fn normalized_form(&self) -> &String {
        &self.word_info.normalized_form
    }

    pub fn reading_form(&self) -> &String {
        &self.word_info.reading_form
    }

    pub fn dictionary_form(&self) -> &String {
        &self.word_info.dictionary_form
    }
}
