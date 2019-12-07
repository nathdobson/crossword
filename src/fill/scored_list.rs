use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, stdout, Write};
use std::io;

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

use crate::core::word::Word;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ScoredWord {
    pub word: Word,
    pub score: u8,
}

impl ScoredWord {
    fn read(main: &mut dyn Read) -> io::Result<Vec<Self>> {
        assert_eq!(main.read_u64::<BigEndian>()?, 0xb260_fc0d_0000_0000);
        let dict_end = main.read_u32::<LittleEndian>()? as usize;
        let mut dictionary_buffer = vec![0u8; dict_end - 12];
        main.read_exact(&mut dictionary_buffer)?;
        let mut dictionary: &[u8] = &mut dictionary_buffer;
        let mut words = vec![];
        let mut buffer = vec![0];
        while dictionary.len() > 0 {
            let header = dictionary.read_u8()? as usize;
            let has_length = header >> 7 == 1;
            let prefix: usize = ((header & 0b0111_1111) - 1) as usize;
            let mut score = dictionary.read_u8()?;
            if score == 80 {
                score = 0;
            }
            if has_length {
                let length = dictionary.read_u8()? as usize;
                buffer.resize(length, 0u8);
                dictionary.read_exact(&mut buffer[prefix as usize..])?;
            } else if prefix == 0 {
                buffer.clear();
                while dictionary[0] >= 32 && dictionary[0] < 127 {
                    buffer.push(dictionary.read_u8()?);
                }
            } else {
                dictionary.read_exact(&mut buffer[prefix as usize..])?;
            }
            let string: String = buffer.iter().map(|&c| c as char).collect();
//            if score > 50 {
//                println!("{:?} -> {:?} {:#X}", string, score, dict_end - dictionary.len());
//            }
            if let Some(word) = Word::from_str(&string) {
                words.push(ScoredWord { word, score });
            }
        }
        words.sort_by_key(|word| -(word.score as i32));
        Ok(words)
    }
    pub fn default() -> io::Result<Vec<Self>> {
        let mut f = File::open("dictionaries/Default.lst")?;
        Self::read(&mut f)
    }
}


#[test]
fn test() {
    //ScoredWord::default().unwrap();
    //println!("{:?}", ScoredWord::default());
    for word in ScoredWord::default().unwrap() {
        if word.word.len() <= 3 {
            println!("{:?}", word);
        }
    }
}