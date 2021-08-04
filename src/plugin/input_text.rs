use crate::prelude::*;
use crate::utf8inputtext::Utf8InputTextBuilder;

pub mod prolonged_sound_mark;

pub trait InputTextPlugin {
    fn rewrite(&self, builder: &mut Utf8InputTextBuilder);
}

pub fn get_input_text_plugins() -> SudachiResult<Vec<Box<dyn InputTextPlugin>>> {
    // todo load from config
    let mut plugins: Vec<Box<dyn InputTextPlugin>> = vec![];

    plugins.push(Box::new(
        prolonged_sound_mark::ProlongedSoundMarkPlugin::new()?,
    ));

    Ok(plugins)
}
