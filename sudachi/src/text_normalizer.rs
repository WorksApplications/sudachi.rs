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
use crate::dic::connect::ConnectionMatrix;
use crate::dic::grammar::Grammar;
use crate::dic::pos::PosList;
use crate::dic::DictionaryAccess;
use crate::input_text::InputBuffer;
use crate::plugin::input_text::default_input_text::DefaultInputTextPlugin;
use crate::plugin::input_text::InputTextPlugin;
use crate::prelude::*;

const ZERO_CONNECTION_BYTES: &[u8] = &[0, 0, 0, 0];

/// Applies input-text normalization used by tokenizer input processing.
///
/// By default, this uses `DefaultInputTextPlugin`. When built from a dictionary,
/// it applies that dictionary's configured input-text plugins.
pub struct TextNormalizer<'a> {
    source: TextNormalizerSource<'a>,
    input: InputBuffer,
}

enum TextNormalizerSource<'a> {
    Default(DefaultInputTextPlugin),
    BorrowedDictionary(&'a (dyn DictionaryAccess + Sync)),
    SharedDictionary(Arc<dyn DictionaryAccess + Sync + Send>),
}

impl<'a> TextNormalizer<'a> {
    /// Create a text normalizer using the default input-text plugin.
    pub fn new(grammar: &Grammar) -> SudachiResult<Self> {
        Ok(Self {
            source: TextNormalizerSource::Default(set_up_default_plugin(grammar)?),
            input: InputBuffer::new(),
        })
    }

    /// Create a text normalizer using the default input-text plugin and an empty grammar.
    pub fn default() -> SudachiResult<Self> {
        let grammar = empty_grammar()?;
        Self::new(&grammar)
    }

    /// Create a text normalizer using the input-text plugins from a dictionary.
    pub fn from_dictionary<D>(dictionary: &'a D) -> Self
    where
        D: DictionaryAccess + Sync + 'a,
    {
        Self {
            source: TextNormalizerSource::BorrowedDictionary(dictionary),
            input: InputBuffer::new(),
        }
    }

    /// Create a text normalizer using the input-text plugins from a shared dictionary handle.
    pub fn from_shared_dictionary<D>(dictionary: Arc<D>) -> TextNormalizer<'static>
    where
        D: DictionaryAccess + Sync + Send + 'static,
    {
        let dictionary: Arc<dyn DictionaryAccess + Sync + Send> = dictionary;
        TextNormalizer {
            source: TextNormalizerSource::SharedDictionary(dictionary),
            input: InputBuffer::new(),
        }
    }

    pub fn normalize(&mut self, text: &str) -> SudachiResult<String> {
        self.input.reset().push_str(text);
        self.input.start_build()?;
        match &self.source {
            TextNormalizerSource::Default(plugin) => plugin.rewrite(&mut self.input)?,
            TextNormalizerSource::BorrowedDictionary(dictionary) => {
                rewrite_with_dictionary(*dictionary, &mut self.input)?;
            }
            TextNormalizerSource::SharedDictionary(dictionary) => {
                rewrite_with_dictionary(dictionary.as_ref(), &mut self.input)?;
            }
        }
        Ok(self.input.current().to_owned())
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

fn empty_grammar() -> SudachiResult<Grammar<'static>> {
    let connection = ConnectionMatrix::from_bytes(ZERO_CONNECTION_BYTES)?;
    Ok(Grammar::from_parts(PosList::default(), connection))
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
                grammar: empty_grammar().unwrap(),
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
    fn default_normalizer_reuses_buffer() {
        let mut normalizer = TextNormalizer::default().unwrap();

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
