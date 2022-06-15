/*
 *  Copyright (c) 2021 Works Applications Co., Ltd.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use super::*;
use crate::test::zero_grammar;
use crate::util::testing::input_text;
use serde_json::json;

trait ProvideOovs {
    fn oovs(&self, data: impl AsRef<str>, offset: usize) -> Vec<Node> {
        self.oovs_other(data, offset, CreatedWords::empty())
    }

    fn oovs_other(
        &self,
        data: impl AsRef<str>,
        offset: usize,
        other_words: CreatedWords,
    ) -> Vec<Node>;
}

impl<T: OovProviderPlugin> ProvideOovs for T {
    fn oovs_other(
        &self,
        data: impl AsRef<str>,
        offset: usize,
        other_words: CreatedWords,
    ) -> Vec<Node> {
        let itext = input_text(data);
        let mut result = Vec::new();
        self.provide_oov(&itext, offset, other_words, &mut result)
            .expect("not failed");
        result
    }
}

fn plugin(regex: impl AsRef<str>) -> RegexOovProvider {
    let mut plugin = RegexOovProvider::default();
    let mut grammar = zero_grammar();
    let cfg = Config::minimal_at("");
    let jval = json!({
        "leftId": 0,
        "rightId": 0,
        "cost": 0,
        "regex": regex.as_ref(),
        "pos": ["a", "b", "c", "d", "e", "f"],
        "userPOS": "allow"
    });
    plugin.set_up(&jval, &cfg, &mut grammar).expect("failed");
    plugin
}

#[test]
fn works() {
    let p = plugin("test");
    let o1 = p.oovs("xtest", 0);
    assert_eq!(0, o1.len());
    let o2 = p.oovs("xtest", 1);
    assert_eq!(0, o2.len());
    let o3 = p.oovs("testf", 0);
    assert_eq!(1, o3.len());
}

#[test]
fn works_regex() {
    let p = plugin("[-0-9a-zA-Z]{4,}");
    let o1 = p.oovs("おらおら1512XF-2テスト", 4);
    assert_eq!(1, o1.len());
    let node = &o1[0];
    assert_eq!(4..12, node.char_range());

    let o2 = p.oovs_other("おらおら1512XF-2テスト", 4, CreatedWords::single(8));
    assert_eq!(0, o2.len());
}
