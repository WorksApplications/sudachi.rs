/*
 * Copyright (c) 2026 Works Applications Co., Ltd.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::sync::Arc;

use crate::config::ConfigBuilder;
use crate::dic::grammar::Grammar;
use crate::dic::DictionaryAccess;
use crate::input_text::InputBuffer;
use crate::plugin::input_text::default_input_text::DefaultInputTextPlugin;
use crate::plugin::input_text::InputTextPlugin;
use crate::prelude::*;

/// Applies input-text normalization used by tokenizer input processing.
///
/// By default, this uses `DefaultInputTextPlugin`. When built from a dictionary,
/// it applies that dictionary's configured input-text plugins.
pub struct TextNormalizer<D = DefaultInputTextPlugin> {
    source: D,
    input: InputBuffer,
}

impl TextNormalizer<DefaultInputTextPlugin> {
    /// Create a text normalizer using the default input-text plugin.
    pub fn new(grammar: &Grammar) -> SudachiResult<Self> {
        Ok(Self {
            source: set_up_default_plugin(grammar)?,
            input: InputBuffer::new(),
        })
    }

    /// Create a text normalizer using the default input-text plugin and an empty grammar.
    pub fn try_default() -> SudachiResult<Self> {
        let grammar = Grammar::empty();
        Self::new(&grammar)
    }

    pub fn normalize(&mut self, text: &str) -> SudachiResult<String> {
        self.input.reset().push_str(text);
        self.input.start_build()?;
        self.source.rewrite(&mut self.input)?;
        Ok(self.input.current().to_owned())
    }
}

impl<D> TextNormalizer<D>
where
    D: DictionaryAccess,
{
    /// Create a text normalizer using the input-text plugins from a dictionary.
    pub fn from_dictionary(dictionary: D) -> Self {
        Self {
            source: dictionary,
            input: InputBuffer::new(),
        }
    }

    pub fn normalize(&mut self, text: &str) -> SudachiResult<String> {
        self.input.reset().push_str(text);
        self.input.start_build()?;
        rewrite_with_dictionary(&self.source, &mut self.input)?;
        Ok(self.input.current().to_owned())
    }
}

impl<D> TextNormalizer<Arc<D>>
where
    D: DictionaryAccess + ?Sized,
{
    /// Create a text normalizer using the input-text plugins from a shared dictionary handle.
    pub fn from_shared_dictionary(dictionary: Arc<D>) -> Self {
        TextNormalizer::from_dictionary(dictionary)
    }
}

fn set_up_default_plugin(grammar: &Grammar) -> SudachiResult<DefaultInputTextPlugin> {
    let mut plugin = DefaultInputTextPlugin::default();
    let cfg = ConfigBuilder::empty().push_embedded().build();
    plugin.set_up(
        &serde_json::Value::Object(serde_json::Map::default()),
        &cfg,
        grammar,
    )?;
    Ok(plugin)
}

fn rewrite_with_dictionary<D>(dictionary: &D, input: &mut InputBuffer) -> SudachiResult<()>
where
    D: DictionaryAccess + ?Sized,
{
    for plugin in dictionary.input_text_plugins() {
        plugin.rewrite(input)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::config::Config;
    use crate::dic::lexicon_set::LexiconSet;
    use crate::dic::LexiconAccess;
    use crate::input_text::InputEditor;
    use crate::plugin::oov::OovProviderPlugin;
    use crate::plugin::path_rewrite::PathRewritePlugin;

    struct ReplaceAllPlugin;

    impl InputTextPlugin for ReplaceAllPlugin {
        fn set_up(
            &mut self,
            _settings: &Value,
            _config: &Config,
            _grammar: &Grammar,
        ) -> SudachiResult<()> {
            Ok(())
        }

        #[allow(deprecated)]
        fn rewrite_impl<'a>(
            &'a self,
            input: &InputBuffer,
            mut edit: InputEditor<'a>,
        ) -> SudachiResult<InputEditor<'a>> {
            edit.replace_ref(0..input.current().len(), "rewritten");
            Ok(edit)
        }
    }

    struct MockDictionary {
        grammar: Grammar<'static>,
        input_text_plugins: Vec<Box<dyn InputTextPlugin + Sync + Send>>,
        oov_provider_plugins: Vec<Box<dyn OovProviderPlugin + Sync + Send>>,
        path_rewrite_plugins: Vec<Box<dyn PathRewritePlugin + Sync + Send>>,
    }

    impl MockDictionary {
        fn new(input_text_plugins: Vec<Box<dyn InputTextPlugin + Sync + Send>>) -> Self {
            Self {
                grammar: Grammar::empty(),
                input_text_plugins,
                oov_provider_plugins: Vec::new(),
                path_rewrite_plugins: Vec::new(),
            }
        }
    }

    impl LexiconAccess for MockDictionary {
        fn lexicon(&self) -> &LexiconSet<'_> {
            unimplemented!("text normalization does not use lexicon access")
        }
    }

    impl DictionaryAccess for MockDictionary {
        fn grammar(&self) -> &Grammar<'_> {
            &self.grammar
        }

        fn input_text_plugins(&self) -> &[Box<dyn InputTextPlugin + Sync + Send>] {
            &self.input_text_plugins
        }

        fn oov_provider_plugins(&self) -> &[Box<dyn OovProviderPlugin + Sync + Send>] {
            &self.oov_provider_plugins
        }

        fn path_rewrite_plugins(&self) -> &[Box<dyn PathRewritePlugin + Sync + Send>] {
            &self.path_rewrite_plugins
        }
    }

    #[test]
    fn default_normalizer_works() {
        let mut normalizer = TextNormalizer::try_default().unwrap();

        assert_eq!("abc", normalizer.normalize("ＡＢＣ").unwrap());
        assert_eq!("", normalizer.normalize("").unwrap());
        assert_eq!("ガヴ", normalizer.normalize("ｶﾞウ゛").unwrap());
    }

    #[test]
    fn dictionary_normalizer_uses_dictionary_plugins() {
        let dictionary = MockDictionary::new(vec![Box::new(ReplaceAllPlugin)]);
        let mut normalizer = TextNormalizer::from_dictionary(&dictionary);

        assert_eq!("rewritten", normalizer.normalize("abc").unwrap());
        assert_eq!("rewritten", normalizer.normalize("ＡＢＣ").unwrap());
    }

    #[test]
    fn shared_dictionary_normalizer_uses_dictionary_plugins() {
        let dictionary = Arc::new(MockDictionary::new(vec![Box::new(ReplaceAllPlugin)]));
        let mut normalizer = TextNormalizer::from_shared_dictionary(dictionary);

        assert_eq!("rewritten", normalizer.normalize("abc").unwrap());
        assert_eq!("rewritten", normalizer.normalize("ＡＢＣ").unwrap());
    }
}
