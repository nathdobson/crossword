use crate::core::word::Word;
use std::collections::HashMap;
use std::fs::File;
use csv::{ReaderBuilder, WriterBuilder};
use std::io;

pub struct EditedDictionary {
    source: Vec<Word>,
    update_file: String,
    updates: HashMap<Word, bool>,
}

impl EditedDictionary {
    pub fn new(source: Vec<Word>, update_file: &str) -> Self {
        EditedDictionary {
            source,
            update_file: update_file.to_string(),
            updates: ReaderBuilder::new()
                .has_headers(false)
                .from_reader(File::open(update_file).unwrap())
                .records()
                .map(|record| {
                    let record = record?;
                    Ok((Word::from_str(&record[0].to_string()).unwrap(), &record[1] == "true"))
                })
                .collect::<io::Result<HashMap<Word, bool>>>().
                unwrap(),
        }
    }
    pub fn status(&self, word: Word) -> Option<bool> {
        self.updates.get(&word).cloned()
    }
    pub fn set_status(&mut self, word: Word, status: Option<bool>) {
        match status {
            None => { self.updates.remove(&word); }
            Some(value) => { self.updates.insert(word, value); }
        }
        let mut writer =
            WriterBuilder::new()
                .has_headers(false)
                .from_writer(File::create(&self.update_file).unwrap());
        for (word, value) in self.updates.iter() {
            writer.write_record([word.to_unicode(), format!("{}", value)].iter()).unwrap();
        }
    }

    pub fn build(&self) -> Vec<Word> {
        self.source.iter().filter(|word| self.updates.get(word) != Some(&false)).cloned().collect()
    }
}
