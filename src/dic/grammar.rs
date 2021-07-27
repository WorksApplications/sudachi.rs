use nom::{le_i16, le_u16};

use crate::dic::utf16_string;
use crate::prelude::*;

pub struct Grammar<'a> {
    bytes: &'a [u8],
    pub pos_list: Vec<Vec<String>>,
    connect_table_offset: usize,
    left_id_size: i16,
    _right_id_size: i16,

    pub storage_size: usize,
}

impl<'a> Grammar<'a> {
    const POS_DEPTH: usize = 6;

    pub const BOS_PARAMETER: (i16, i16, i16) = (0, 0, 0); // left_id, right_id, cost
    pub const EOS_PARAMETER: (i16, i16, i16) = (0, 0, 0); // left_id, right_id, cost

    pub fn new(buf: &[u8], offset: usize) -> SudachiResult<Grammar> {
        let (rest, (pos_list, left_id_size, right_id_size)) =
            grammar_parser(buf, offset).map_err(|_| SudachiError::InvalidDictionaryGrammar)?;

        let connect_table_offset = buf.len() - rest.len();
        let storage_size =
            (connect_table_offset - offset) + 2 * left_id_size as usize * right_id_size as usize;

        Ok(Grammar {
            bytes: buf,
            pos_list,
            connect_table_offset,
            left_id_size,
            _right_id_size: right_id_size,
            storage_size,
        })
    }

    pub fn get_connect_cost(&self, left_id: i16, right_id: i16) -> SudachiResult<i16> {
        let (_rest, connect_cost) = connect_cost_parser(
            self.bytes,
            self.connect_table_offset,
            left_id as usize,
            self.left_id_size as usize,
            right_id as usize,
        )?;

        Ok(connect_cost)
    }
}

named_args!(
    grammar_parser(offset: usize)<&[u8], (Vec<Vec<String>>, i16, i16)>,
    do_parse!(
        _seek: take!(offset) >>
        pos_size: le_u16 >>
        pos_list: count!(count!(utf16_string, Grammar::POS_DEPTH), pos_size as usize) >>
        left_id_size: le_i16 >>
        right_id_size: le_i16 >>

        ( pos_list, left_id_size, right_id_size )
  )
);

named_args!(
    connect_cost_parser(offset: usize,
                        left_id: usize,
                        left_id_size: usize,
                        right_id: usize)<&[u8], i16>,
    do_parse!(
        _seek: take!(offset + (left_id * 2) + (2 * left_id_size * right_id)) >>
        connect_cost: le_i16 >>

        (connect_cost)
    )
);
