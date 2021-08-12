use std::collections::HashSet;

use sudachi::declare_input_text_plugin;
use sudachi::dic::grammar::Grammar;
use sudachi::input_text::utf8_input_text_builder::Utf8InputTextBuilder;
use sudachi::plugin::input_text::InputTextPlugin;
use sudachi::prelude::*;

declare_input_text_plugin!(ProlongedSoundMarkPlugin, ProlongedSoundMarkPlugin::default);

#[derive(Default)]
pub struct ProlongedSoundMarkPlugin {
    psm_set: HashSet<char>,
    replace_symbol: String,
}

impl InputTextPlugin for ProlongedSoundMarkPlugin {
    fn set_up(&mut self, _grammar: &Grammar) -> SudachiResult<()> {
        // todo: load from config
        let psm_set: HashSet<_> = ['ー', '-', '⁓', '〜', '〰'].iter().map(|v| *v).collect();
        let replace_symbol = String::from('ー');

        self.psm_set = psm_set;
        self.replace_symbol = replace_symbol;

        Ok(())
    }

    fn rewrite(&self, builder: &mut Utf8InputTextBuilder) {
        let text = builder.modified.clone();
        let n = builder.modified.chars().count();
        let mut offset = 0;
        let mut is_psm = false;
        let mut m_start_idx = n;
        for (i, c) in text.chars().enumerate() {
            if !is_psm && self.psm_set.contains(&c) {
                is_psm = true;
                m_start_idx = i;
            } else if is_psm && !self.psm_set.contains(&c) {
                if i > m_start_idx + 1 {
                    builder.replace(m_start_idx - offset..i - offset, &self.replace_symbol);
                    offset += i - m_start_idx - 1;
                }
                is_psm = false;
            }
        }
        if is_psm && n > m_start_idx + 1 {
            builder.replace(m_start_idx - offset..n - offset, &self.replace_symbol);
        }
    }
}
