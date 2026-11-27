use std::path::Path;

use tokenizers::Tokenizer;

use crate::content::is_binary;
use crate::error::Error;

pub struct Counter<'a> {
    tokenizer: &'a Tokenizer,
}

impl<'a> Counter<'a> {
    pub fn new(tokenizer: &'a Tokenizer) -> Self {
        Self { tokenizer }
    }

    pub fn count(&self, path: impl AsRef<Path>) -> Result<usize, Error> {
        let path = path.as_ref();
        let bytes =
            std::fs::read(path).map_err(|source| Error::read_file(path.to_path_buf(), source))?;

        if is_binary(&bytes) {
            return Err(Error::not_text(path.to_path_buf()));
        }

        let text = String::from_utf8(bytes).map_err(|_| Error::not_text(path.to_path_buf()))?;

        self.tokenizer
            .encode_fast(text.as_str(), false)
            .map(|encoding| encoding.len())
            .map_err(|source| Error::encode(path.to_path_buf(), source))
    }
}
