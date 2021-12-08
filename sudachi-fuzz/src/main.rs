use std::ops::Deref;

use criterion::black_box;
#[cfg(fuzzing)]
use honggfuzz::fuzz;
use sudachi::analysis::stateful_tokenizer::StatefulTokenizer;
use sudachi::config::Config;
use sudachi::dic::dictionary::JapaneseDictionary;
use sudachi::dic::subset::InfoSubset;
use sudachi::prelude::*;

use arbitrary::{self, Arbitrary};

#[allow(unused)]
fn consume_mlist<'a, 'b: 'a>(
    mlist: &'a MorphemeList<&'b JapaneseDictionary>,
    mlist2: &'a mut MorphemeList<&'b JapaneseDictionary>,
    data: &'a mut String,
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
        data.push_str(surf.deref());
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
        let mut mlen = 0;
        for j in 0..mlist2.len() {
            let m1 = mlist2.get(j);
            let s1 = m1.surface();
            assert_eq!(&mlist.surface()[m1.begin()..m1.end()], s1.deref());
            mlen += (m1.end() - m1.begin());
            black_box(m1.begin());
            black_box(m1.begin_c());
            black_box(m1.end());
            black_box(m1.end_c());
            black_box(m1.word_id().word());
            black_box(m1.word_id().dic());
            black_box(m1.part_of_speech_id());
            black_box(m1.part_of_speech());
            black_box(m1.get_word_info().a_unit_split());
            black_box(m1.get_word_info().b_unit_split());
            black_box(m1.get_word_info().synonym_group_ids());
            black_box(m1.get_word_info().dictionary_form());
            black_box(m1.get_word_info().dictionary_form_word_id());
            black_box(m1.get_word_info().reading_form());
            black_box(m1.get_word_info().surface());
            black_box(m1.get_word_info().normalized_form());
        }
        if !mlist2.is_empty() {
            assert_eq!(surf.len(), mlen);
        }
    }
}

#[allow(unused)]
struct SudachiInput<'a> {
    subset: InfoSubset,
    input: &'a str,
}

impl<'a> Arbitrary<'a> for SudachiInput<'a> {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let input = SudachiInput {
            subset: InfoSubset::from_bits_truncate(u.arbitrary()?),
            input: u.arbitrary()?,
        };
        if u.is_empty() {
            Ok(input)
        } else {
            Err(arbitrary::Error::IncorrectFormat)
        }
    }
}

#[allow(unused)]
fn main() {
    let cfg = Config::new(None, None, None).unwrap();
    let ana = JapaneseDictionary::from_cfg(&cfg).unwrap();

    let mut st = StatefulTokenizer::create(&ana, false, Mode::C);
    let mut mlist = MorphemeList::empty(&ana);
    let mut mlist2 = MorphemeList::empty(&ana);
    let mut surf = String::new();

    if cfg!(not(fuzzing)) {
        st.reset().push_str("#\0M㍿");
        st.set_subset(InfoSubset::from_bits_truncate(599));
        st.do_tokenize().unwrap();
        mlist.collect_results(&mut st).unwrap();
        consume_mlist(&mlist, &mut mlist2, &mut surf);
    }

    #[cfg(fuzzing)]
    loop {
        fuzz!(|i: SudachiInput| {
            st.set_subset(i.subset);
            st.reset().push_str(i.input);

            if st.do_tokenize().is_err() {
                return;
            }

            if mlist.collect_results(&mut st).is_err() {
                return;
            };

            surf.clear();
            consume_mlist(&mlist, &mut mlist2, &mut surf);
            assert_eq!(surf, i.input);
        });
    }
}
