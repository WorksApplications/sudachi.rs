use crate::dic::grammar::Grammar;
use crate::input_text::utf8_input_text_builder::Utf8InputTextBuilder;
use crate::prelude::*;

pub trait InputTextPlugin {
    fn rewrite(&self, builder: &mut Utf8InputTextBuilder);
}

pub fn get_input_text_plugins(
    _grammar: &Grammar,
) -> SudachiResult<Vec<Box<dyn InputTextPlugin + Sync>>> {
    // todo load from config
    let mut plugins: Vec<Box<dyn InputTextPlugin + Sync>> = vec![];

    // plugins.push(Box::new(default_input_text::DefaultInputTextPlugin::new()?));
    // plugins.push(Box::new(
    //     prolonged_sound_mark::ProlongedSoundMarkPlugin::new()?,
    // ));
    // plugins.push(Box::new(ignore_yomigana::IgnoreYomiganaPlugin::new(
    //     grammar,
    // )?));

    Ok(plugins)
}
