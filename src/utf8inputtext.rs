pub struct Utf8InputTextBuilder<'a> {
    pub original: &'a str,
    pub modified: String,
}

impl Utf8InputTextBuilder<'_> {
    pub fn new(original: &str) -> Utf8InputTextBuilder {
        let modified = String::from(original);

        Utf8InputTextBuilder { original, modified }
    }

    pub fn build(&self) -> Utf8InputText {
        Utf8InputText {
            original: self.original,
            modified: self.modified.as_str(),
        }
    }
}

#[derive(Debug)]
pub struct Utf8InputText<'a> {
    pub original: &'a str,
    pub modified: &'a str,
}

impl Utf8InputText<'_> {
    pub fn can_bow(&self, byte_idx: usize) -> bool {
        (self.modified.as_bytes()[byte_idx] & 0xC0) != 0x80
    }
}
