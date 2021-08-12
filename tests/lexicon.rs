#[macro_use]
extern crate lazy_static;

mod common;
use common::LEXICON;

#[test]
fn lookup() {
    let res = LEXICON
        .lookup("東京都".as_bytes(), 0)
        .expect("failed to lookup");
    assert_eq!(3, res.len());
    assert_eq!((4, 3), res[0]); // 東
    assert_eq!((5, 6), res[1]); // 東京
    assert_eq!((6, 9), res[2]); // 東京都

    let res = LEXICON
        .lookup("東京都に".as_bytes(), 9)
        .expect("failed to lookup");
    assert_eq!(2, res.len());
    assert_eq!((1, 12), res[0]); // に(接続助詞)
    assert_eq!((2, 12), res[1]); // に(格助詞)

    let res = LEXICON
        .lookup("あれ".as_bytes(), 0)
        .expect("failed to lookup");
    assert_eq!(0, res.len());
}

#[test]
fn parameters() {
    // た
    assert_eq!(
        (1, 1, 8729),
        LEXICON.get_word_param(0).expect("failed to get word param")
    );

    // 東京都
    assert_eq!(
        (6, 8, 5320),
        LEXICON.get_word_param(6).expect("failed to get word param")
    );

    // 都
    assert_eq!(
        (8, 8, 2914),
        LEXICON.get_word_param(9).expect("failed to get word param")
    );
}

#[test]
fn word_info() {
    // た
    let wi = LEXICON.get_word_info(0).expect("failed to get word_info");
    assert_eq!("た", wi.surface);
    assert_eq!(3, wi.head_word_length);
    assert_eq!(0, wi.pos_id);
    assert_eq!("た", wi.normalized_form);
    assert_eq!(-1, wi.dictionary_form_word_id);
    assert_eq!("た", wi.dictionary_form);
    assert_eq!("タ", wi.reading_form);
    assert!(wi.a_unit_split.is_empty());
    assert!(wi.b_unit_split.is_empty());
    assert!(wi.word_structure.is_empty());

    // 東京都
    let wi = LEXICON.get_word_info(6).expect("failed to get word_info");
    assert_eq!("東京都", wi.surface);
    assert_eq!([5, 9], &wi.a_unit_split[..]);
    assert!(&wi.b_unit_split.is_empty());
    assert_eq!([5, 9], &wi.word_structure[..]);
    // todo: after read synonym group ids
    // assert_eq!([], wi.synonym_group_ids);

    // 行っ
    let wi = LEXICON.get_word_info(8).expect("failed to get word_info");
    assert_eq!("行っ", wi.surface);
    assert_eq!("行く", wi.normalized_form);
    assert_eq!(7, wi.dictionary_form_word_id);
    assert_eq!("行く", wi.dictionary_form);
}

#[test]
fn word_info_with_longword() {
    // todo: impl after lexicon update
    // 0123456789 * 30
    // let wi = LEXICON.get_word_info(36).expect("failed to get word_info");
    // assert_eq!(300, wi.surface.chars().count());
    // assert_eq!(300, wi.head_word_length);
    // assert_eq!(300, wi.normalized_form.chars().count());
    // assert_eq!(-1, wi.dictionary_form_word_id);
    // assert_eq!(300, wi.dictionary_form.chars().count());
    // assert_eq!(570, wi.reading_form.chars().count());
}

#[test]
fn size() {
    assert_eq!(39, LEXICON.size())
}
