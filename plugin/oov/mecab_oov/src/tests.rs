use super::*;

use sudachi::dic::category_type::CategoryTypes;

#[test]
fn provide_oov000() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: false,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov100() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: true,
            is_group: false,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov010() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    println!("{:?}", text);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
    let wi = nodes[0].word_info.clone().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov110() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: true,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
    let wi = nodes[0].word_info.clone().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert_eq!(1, nodes.len());
}

#[test]
fn provide_oov002() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: false,
            length: 2,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(2, nodes.len());

    let wi = nodes[0].word_info.clone().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.clone().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov012() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: true,
            length: 2,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    let wi = nodes[0].word_info.clone().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.clone().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[2].word_info.clone().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov112() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: true,
            is_group: true,
            length: 2,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    let wi = nodes[0].word_info.clone().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.clone().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[2].word_info.clone().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());
}

#[test]
fn provide_oov006() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJI,
        CategoryInfo {
            category_type: CategoryType::KANJI,
            is_invoke: false,
            is_group: false,
            length: 6,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(3, nodes.len());

    let wi = nodes[0].word_info.clone().unwrap();
    assert_eq!("あ", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.clone().unwrap();
    assert_eq!("あい", wi.surface);
    assert_eq!(6, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[2].word_info.clone().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let nodes = plugin
        .provide_oov(&text, 0, true)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov_multi_oov() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::KANJINUMERIC,
        CategoryInfo {
            category_type: CategoryType::KANJINUMERIC,
            is_invoke: false,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJINUMERIC);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert_eq!(2, nodes.len());

    let wi = nodes[0].word_info.clone().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(1, wi.pos_id);

    let wi = nodes[1].word_info.clone().unwrap();
    assert_eq!("あいう", wi.surface);
    assert_eq!(9, wi.head_word_length);
    assert_eq!(2, wi.pos_id);
}

#[test]
fn provide_oov_without_cinfo() {
    let plugin = build_plugin();
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::KANJI);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

#[test]
fn provide_oov_without_oov_list() {
    let mut plugin = build_plugin();
    plugin.categories.insert(
        CategoryType::HIRAGANA,
        CategoryInfo {
            category_type: CategoryType::HIRAGANA,
            is_invoke: false,
            is_group: true,
            length: 0,
        },
    );
    let text = "あいうえお";
    let text = build_input_text(text, 0, 3, CategoryType::HIRAGANA);

    let nodes = plugin
        .provide_oov(&text, 0, false)
        .expect("Failed to generate oovs");
    assert!(nodes.is_empty());
}

fn build_plugin() -> MeCabOovPlugin {
    let mut plugin = MeCabOovPlugin::default();
    let oov1 = OOV {
        right_id: -1,
        left_id: -1,
        cost: -1,
        pos_id: 1,
    };
    let oov2 = OOV {
        right_id: -1,
        left_id: -1,
        cost: -1,
        pos_id: 2,
    };
    plugin
        .oov_list
        .insert(CategoryType::KANJI, vec![oov1.clone()]);
    plugin
        .oov_list
        .insert(CategoryType::KANJINUMERIC, vec![oov1, oov2]);
    plugin
}
fn build_input_text<'a>(
    text: &'a str,
    begin: usize,
    end: usize,
    ctype: CategoryType,
) -> Utf8InputText<'a> {
    // assume text = "あいうえお"
    let offsets = vec![0, 0, 0, 3, 3, 3, 6, 6, 6, 9, 9, 9, 12, 12, 12, 15];
    let byte_indexes = vec![0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5];
    let mut char_category_types = vec![CategoryTypes::default(); 5];
    for i in begin..end {
        char_category_types[i].insert(ctype);
    }
    let can_bow_list = vec![true; 5];
    let mut char_category_continuities = vec![0; 15];
    for i in begin * 3..end * 3 {
        char_category_continuities[i] = (end - begin) * 3 - i;
    }

    let text = Utf8InputText::new(
        text,
        text,
        offsets,
        byte_indexes,
        char_category_types,
        can_bow_list,
        char_category_continuities,
    );
    text
}
