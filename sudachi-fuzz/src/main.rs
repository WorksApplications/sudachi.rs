use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use criterion::black_box;
#[cfg(fuzzing)]
use honggfuzz::fuzz;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::prelude::*;

#[allow(unused)]
fn consume_mlist<'a, 'b: 'a>(
    mlist: &'a MorphemeList<&'b JapaneseDictionary>,
    mlist2: &'a mut MorphemeList<&'b JapaneseDictionary>,
) {
    if mlist.is_empty() {
        return;
    }

    //mlist.get_internal_cost() as isize;
    // use black_box function to forbit optimizing accesses to API functions
    // this is important for fuzzing, we want to trigger any possible panics that can happen
    for i in 0..mlist.len() {
        let m = mlist.get(i);
        let surf = m.surface();
        black_box(surf.deref());
        black_box(m.begin());
        black_box(m.begin_c());
        black_box(m.end());
        black_box(m.end_c());
        black_box(m.word_id().word());
        black_box(m.word_id().dic());
        black_box(m.part_of_speech_id());
        black_box(m.part_of_speech());
        black_box(m.get_word_info().a_unit_split());
        black_box(m.get_word_info().b_unit_split());
        black_box(m.get_word_info().synonym_group_ids());
        black_box(m.get_word_info().dictionary_form());
        black_box(m.get_word_info().dictionary_form_word_id());
        black_box(m.get_word_info().reading_form());
        black_box(m.get_word_info().surface());
        black_box(m.get_word_info().normalized_form());

        mlist2.clear();
        if m.split_into(Mode::A, mlist2).is_err() {
            return;
        }
        for j in 0..mlist2.len() {
            let m = mlist.get(j);
            let surf = m.surface();
            black_box(surf.deref());
            black_box(m.begin());
            black_box(m.begin_c());
            black_box(m.end());
            black_box(m.end_c());
            black_box(m.word_id().word());
            black_box(m.word_id().dic());
            black_box(m.part_of_speech_id());
            black_box(m.part_of_speech());
            black_box(m.get_word_info().a_unit_split());
            black_box(m.get_word_info().b_unit_split());
            black_box(m.get_word_info().synonym_group_ids());
            black_box(m.get_word_info().dictionary_form());
            black_box(m.get_word_info().dictionary_form_word_id());
            black_box(m.get_word_info().reading_form());
            black_box(m.get_word_info().surface());
            black_box(m.get_word_info().normalized_form());
        }
    }
}

#[allow(unused)]
fn main() {
    let cfg = Config::new(
        Some(
            PathBuf::from_str("/home/arseny/work/sudachi/sudachi.rs/resources/sudachi.json")
                .unwrap(),
        ),
        Some(PathBuf::from_str("/home/arseny/work/sudachi/sudachi.rs/resources").unwrap()),
        None,
    )
    .unwrap();
    let ana = JapaneseDictionary::from_cfg(&cfg).unwrap();

    let mut st = StatefulTokenizer::create(&ana, false, Mode::A);
    let mut mlist = MorphemeList::empty(&ana);
    let mut mlist2 = MorphemeList::empty(&ana);

    if cfg!(not(fuzzing)) {
        st.reset().push_str("㍿=============");
        st.do_tokenize().unwrap();
        mlist.collect_results(&mut st).unwrap();
        consume_mlist(&mlist, &mut mlist2);
    }

    #[cfg(fuzzing)]
    loop {
        fuzz!(|data: &str| {
            st.reset().push_str(data);

            if st.do_tokenize().is_err() {
                return;
            }

            if mlist.collect_results(&mut st).is_err() {
                return;
            };

            consume_mlist(&mlist, &mut mlist2);
        });
    }
}
