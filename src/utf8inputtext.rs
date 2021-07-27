#[derive(Debug)]
pub struct Utf8InputText {
    pub original: String,
    pub modified: String,
}

impl Utf8InputText {
    pub fn new(original: String) -> Utf8InputText {
        let modified = original.clone();

        Utf8InputText { original, modified }
    }

    pub fn can_bow(&self, idx: usize) -> bool {
        // todo: construct can_bow_list
        (self.modified.as_bytes()[idx] & 0xC0) != 0x80
    }
}
