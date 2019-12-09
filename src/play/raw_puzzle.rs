use std::{io, fs, iter};
use std::io::BufRead;
use byteorder::LittleEndian;
use encoding::{Encoding, DecoderTrap};
use encoding::all::ISO_8859_1;
use std::io::Read;
use std::io::BufReader;
use encoding::EncoderTrap;
use std::io::Write;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;
use std::io::ErrorKind;
use std::collections::HashMap;
use std::collections::BTreeMap;
use crate::util::grid::Grid;
use std::ffi::OsStr;
use csv;
use std::ops::Range;
use encoding::types::DecoderTrap::Call;

pub static MAGIC: [u8; 12] = *b"ACROSS&DOWN\x00";

pub struct Checksums {
    pub file_checksum: u16,
    pub cib_checksum: u16,
    pub magic_checksum: [u8; 8],
    pub magic: [u8; 12],
    pub scrambled_checksum: u16,
    pub scrambled: u16,
    pub bitmask: u16,
}

#[derive(Clone, Debug)]
pub struct RawHeader {
    pub preamble: Vec<u8>,
    pub version: [u8; 4],
    pub reserved1: [u8; 2],
    pub reserved2: [u8; 12],
    pub width: u8,
    pub height: u8,
    pub clues: u16,
}

#[derive(Debug)]
pub struct PlayData {
    time: usize,
    running: bool,
}

#[derive(Debug)]
pub struct RawPuzzle {
    pub header: RawHeader,
    pub solution: Grid<u8>,
    pub answer: Grid<u8>,
    pub title: String,
    pub author: String,
    pub copyright: String,
    pub clues: Vec<String>,
    pub note: String,
    pub rebus_index: Option<Grid<u8>>,
    pub style: Option<Grid<u8>>,
    pub rebus_data: Option<BTreeMap<u8, String>>,
    pub play_data: Option<PlayData>,
    pub rebus_user: Option<Grid<String>>,
}

struct PuzzleReader<'a> {
    input: &'a [u8],
}

impl<'a> PuzzleReader<'a> {
    fn read_fixed<B: AsMut<[u8]>>(&mut self, mut buf: B) -> io::Result<B> {
        self.input.read_exact(buf.as_mut())?;
        Ok(buf)
    }
    fn read_raw_header(&mut self) -> io::Result<(RawHeader, Checksums)> {
        let magic_position = self.input.fill_buf()?.windows(MAGIC.len()).position(|window| window == MAGIC).unwrap();
        let mut preamble = vec![0; magic_position - 2];
        self.input.read_exact(&mut preamble)?;
        let file_checksum = self.input.read_u16::<LittleEndian>()?;
        let magic = self.read_fixed([0u8; 12])?;
        let cib_checksum = self.input.read_u16::<LittleEndian>()?;
        let magic_checksum = self.read_fixed([0u8; 8])?;
        let version = self.read_fixed([0u8; 4])?;
        let reserved1 = self.read_fixed([0u8; 2])?;
        let scrambled_checksum = self.input.read_u16::<LittleEndian>()?;
        let reserved2 = self.read_fixed([0u8; 12])?;
        let width = self.input.read_u8()?;
        let height = self.input.read_u8()?;
        let clues = self.input.read_u16::<LittleEndian>()?;
        let bitmask = self.input.read_u16::<LittleEndian>()?;
        let scrambled = self.input.read_u16::<LittleEndian>()?;
        Ok((RawHeader {
            preamble,
            version,
            reserved1,
            reserved2,
            width,
            height,
            clues,
        }, Checksums {
            file_checksum,
            cib_checksum,
            magic_checksum,
            magic,
            scrambled,
            scrambled_checksum,
            bitmask,
        }))
    }
    fn read_grid(&mut self, size: (usize, usize)) -> io::Result<Grid<u8>> {
        let mut grid = Grid::new(size, |x, y| 0);
        for y in 0..size.1 {
            for x in 0..size.0 {
                grid[(x, y)] = self.input.read_u8()?;
            }
        }
        Ok(grid)
    }
    fn read_string(&mut self) -> io::Result<String> {
        let mut vec = vec![];
        self.input.read_until(0, &mut vec)?;
        vec.truncate(vec.len() - 1);
        match ISO_8859_1.decode(&vec, DecoderTrap::Strict) {
            Ok(x) => Ok(x),
            Err(e) => Err(io::Error::new(io::ErrorKind::InvalidData, e))
        }
    }
    fn read_raw_puzzle(&mut self) -> io::Result<RawPuzzle> {
        let (header, checksums) = self.read_raw_header()?;
        let solution = self.read_grid((header.width as usize, header.height as usize))?;
        let answer = self.read_grid((header.width as usize, header.height as usize))?;
        let title = self.read_string()?;
        let author = self.read_string()?;
        let copyright = self.read_string()?;
        let mut clues = vec![];
        for i in 0..header.clues {
            clues.push(self.read_string()?);
        }
        let note = self.read_string()?;
        let mut result = RawPuzzle {
            header: header.clone(),
            solution: solution,
            answer: answer,
            title: title,
            author: author,
            copyright: copyright,
            note: note,
            clues: clues,
            rebus_index: None,
            rebus_data: None,
            style: None,
            rebus_user: None,
            play_data: None,
        };
        loop {
            let name = match self.read_fixed([0u8; 4]) {
                Ok(name) => name,
                Err(e) => {
                    if e.kind() == ErrorKind::UnexpectedEof {
                        break;
                    } else { return Err(e); }
                }
            };
            let length = self.input.read_u16::<LittleEndian>()?;
            let checksum = self.input.read_u16::<LittleEndian>()?;
            let mut content = vec![0u8; length as usize];
            self.input.read_exact(&mut content)?;
            assert_eq!(0, self.input.read_u8()?);
            match &name {
                b"GRBS" => {
                    result.rebus_index =
                        Some(
                            PuzzleReader { input: &mut (&content as &[u8]) }
                                .read_grid((header.width as usize, header.height as usize))?)
                }
                b"RTBL" => {
                    let mut rebus_data = BTreeMap::<u8, String>::new();
                    for pair in content.split(|&c| c == b';') {
                        if pair != b"" {
                            let mut elements = pair.split(|&c| c == b':');
                            let mut key = elements.next().unwrap();
                            while key[0] == b' ' {
                                key = &key[1..];
                            }
                            let value = elements.next().unwrap();
                            assert!(elements.next().is_none());
                            rebus_data.insert(std::str::from_utf8(key).unwrap().parse::<u8>().unwrap(),
                                              ISO_8859_1.decode(value, DecoderTrap::Strict).unwrap());
                        }
                    }
                    result.rebus_data = Some(rebus_data);
                }
                b"GEXT" => {
                    result.style =
                        Some(
                            PuzzleReader { input: &mut (&content as &[u8]) }
                                .read_grid((header.width as usize, header.height as usize))?)
                }
                b"LTIM" => {
                    let mut split = std::str::from_utf8(&content).unwrap().split(",");
                    result.play_data = Some(PlayData {
                        time: split.next().unwrap().parse().unwrap(),
                        running: split.next().unwrap().parse::<u8>().unwrap() == 1,
                    });
                    assert!(split.next().is_none());
                }
                b"RUSR" => {
                    let mut reader = PuzzleReader { input: &mut (&content as &[u8]) };
                    let mut cells =
                        iter::repeat_with(|| reader.read_string())
                            .take(result.solution.size().0 * result.solution.size().1)
                            .collect::<io::Result<Vec<_>>>()?.into_iter();
                    result.rebus_user = Some(
                        Grid::new(result.solution.size(),
                                  |x, y| cells.next().unwrap())
                    );
                    let mut unused = vec![];
                    reader.input.read_to_end(&mut unused)?;
                    assert!(unused.len() == 0);
                }
                x => panic!("Can't handle {:?}", String::from_utf8(name.to_vec()))
            }
        }
        let mut unparsed = vec![];
        self.input.read_to_end(&mut unparsed)?;
        assert_eq!(Vec::<u8>::new(), unparsed);
        assert_eq!(result.compute_cib_checksum(), checksums.cib_checksum);
        assert_eq!(result.compute_file_checksum(), checksums.file_checksum);
        assert_eq!(result.compute_magic_checksum(), checksums.magic_checksum);
        assert_eq!(checksums.magic, MAGIC);
        assert_eq!(0, checksums.scrambled);
        assert_eq!(0, checksums.scrambled_checksum);
        assert_eq!(1, checksums.bitmask);
        Ok(result)
    }
}

struct Checksum(u16);

impl Write for Checksum {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for x in buf {
            self.0 = self.0.rotate_right(1).overflowing_add(*x as u16).0;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode_string(string: &str) -> io::Result<Vec<u8>> {
    match ISO_8859_1.encode(string, EncoderTrap::Strict) {
        Ok(encoded) => {
            Ok(encoded)
        }
        Err(e) => {
            Err(io::Error::new(io::ErrorKind::InvalidData, e))?
        }
    }
}


trait PuzzleWriter: Write {
    fn write_header(&mut self, puzzle: &RawPuzzle) -> io::Result<()> {
        let header = &puzzle.header;
        self.write_all(&header.preamble)?;
        self.write_u16::<LittleEndian>(puzzle.compute_file_checksum())?;
        self.write_all(&MAGIC)?;
        self.write_u16::<LittleEndian>(puzzle.compute_cib_checksum())?;
        self.write_all(&puzzle.compute_magic_checksum())?;
        self.write_all(&header.version)?;
        self.write_all(&header.reserved1)?;
        self.write_u16::<LittleEndian>(0)?;
        self.write_all(&header.reserved2)?;
        self.write_u8(header.width)?;
        self.write_u8(header.height)?;
        self.write_u16::<LittleEndian>(header.clues)?;
        self.write_u16::<LittleEndian>(1)?;
        self.write_u16::<LittleEndian>(0)?;
        Ok(())
    }

    fn write_grid(&mut self, grid: &Grid<u8>) -> io::Result<()> {
        for y in 0..grid.size().1 {
            for x in 0..grid.size().0 {
                self.write_u8(grid[(x, y)])?;
            }
        }
        Ok(())
    }


    fn write_string(&mut self, string: &str) -> io::Result<()> {
        self.write_all(&encode_string(&string)?)?;
        self.write_u8(0)?;
        Ok(())
    }

    fn write_raw_puzzle(&mut self, puzzle: &RawPuzzle) -> io::Result<()> {
        self.write_header(&puzzle)?;
        self.write_grid(&puzzle.solution)?;
        self.write_grid(&puzzle.answer)?;
        self.write_string(&puzzle.title)?;
        self.write_string(&puzzle.author)?;
        self.write_string(&puzzle.copyright)?;
        for clue in puzzle.clues.iter() {
            self.write_string(clue)?;
        }
        self.write_string(&puzzle.note)?;
        let mut extras = Vec::<([u8; 4], Vec<u8>)>::new();
        if let Some(ref rebus_index) = puzzle.rebus_index {
            let mut data = vec![];
            data.write_grid(&rebus_index)?;
            extras.push((*b"GRBS", data));
        }
        if let Some(ref style) = puzzle.style {
            let mut data = vec![];
            data.write_grid(&style)?;
            extras.push((*b"GEXT", data));
        }
        if let Some(ref rebus) = puzzle.rebus_data {
            let mut data = vec![];
            for (&rebus_index, rebus) in rebus {
                if rebus_index < 10 {
                    write!(&mut data, " {}", rebus_index)?;
                } else {
                    write!(&mut data, "{}", rebus_index)?;
                }
                write!(&mut data, ":")?;
                data.extend_from_slice(&ISO_8859_1.encode(rebus, EncoderTrap::Strict).unwrap());
                write!(&mut data, ";")?;
            }
            extras.push((*b"RTBL", data));
        }
        if let Some(ref play_data) = puzzle.play_data {
            extras.push((*b"LTIM", format!("{},{}", play_data.time, play_data.running as u8).into_bytes()));
        }
        if let Some(ref rebus_user) = puzzle.rebus_user {
            let mut data = vec![];
            for rebus in rebus_user.iter() {
                data.extend_from_slice(&ISO_8859_1.encode(rebus, EncoderTrap::Strict).unwrap());
                data.extend_from_slice(&[0]);
            }
            extras.push((*b"RUSR", data));
        }
        for (name, data) in extras {
            self.write_all(&name)?;
            self.write_u16::<LittleEndian>(data.len() as u16)?;
            let mut checksum = Checksum(0);
            checksum.write_all(&data)?;
            self.write_u16::<LittleEndian>(checksum.0)?;
            self.write_all(&data)?;
            self.write_u8(0)?;
        }
        fn u16_to_u8x2(x: u16) -> [u8; 2] {
            let mut result: Vec<u8> = vec![];
            result.write_u16::<LittleEndian>(x).unwrap();
            [result[0], result[1]]
        }
        Ok(())
    }
}

impl<W> PuzzleWriter for W where W: Write {}

impl RawPuzzle {
    fn text_checksum(&self, checksum: &mut Checksum) {
        if !self.title.is_empty() {
            checksum.write_string(&self.title).unwrap();
        }
        if !self.author.is_empty() {
            checksum.write_string(&self.author).unwrap();
        }
        if !self.copyright.is_empty() {
            checksum.write_string(&self.copyright).unwrap();
        }
        for clue in self.clues.iter() {
            if !clue.is_empty() {
                checksum.write_all(&(encode_string(&clue).unwrap())).unwrap();
            }
        }
        if !self.note.is_empty() {
            checksum.write_string(&self.note).unwrap();
        }
    }

    fn compute_cib_checksum(&self) -> u16 {
        let mut checksum = Checksum(0);
        checksum.write_u8(self.header.width).unwrap();
        checksum.write_u8(self.header.height).unwrap();
        checksum.write_u16::<LittleEndian>(self.header.clues).unwrap();
        checksum.write_u16::<LittleEndian>(1).unwrap();
        checksum.write_u16::<LittleEndian>(0).unwrap();
        checksum.0
    }


    fn compute_file_checksum(&self) -> u16 {
        let mut checksum = Checksum(self.compute_cib_checksum());
        checksum.write_grid(&self.solution).unwrap();
        checksum.write_grid(&self.answer).unwrap();
        self.text_checksum(&mut checksum);
        checksum.0
    }

    fn compute_magic_checksum(&self) -> [u8; 8] {
        let c_cib = self.compute_cib_checksum();
        let c_sol = {
            let mut checksum = Checksum(0);
            checksum.write_grid(&self.solution).unwrap();
            checksum.0
        };
        let c_grid = {
            let mut checksum = Checksum(0);
            checksum.write_grid(&self.answer).unwrap();
            checksum.0
        };
        let c_part = {
            let mut checksum = Checksum(0);
            self.text_checksum(&mut checksum);
            checksum.0
        };

        [0x49 ^ (c_cib & 0xFF) as u8,
            0x43 ^ (c_sol & 0xFF) as u8,
            0x48 ^ (c_grid & 0xFF) as u8,
            0x45 ^ (c_part & 0xFF) as u8,
            0x41 ^ ((c_cib & 0xFF00) >> 8) as u8,
            0x54 ^ ((c_sol & 0xFF00) >> 8) as u8,
            0x45 ^ ((c_grid & 0xFF00) >> 8) as u8,
            0x44 ^ ((c_part & 0xFF00) >> 8) as u8]
    }


    pub fn read_from(read: &mut dyn BufRead) -> io::Result<RawPuzzle> {
        let mut buffer = vec![];
        read.read_to_end(&mut buffer)?;
        PuzzleReader {
            input: &buffer,
        }.read_raw_puzzle()
    }

    pub fn write_to(&self, mut write: &mut dyn Write) -> io::Result<()> {
        write.write_raw_puzzle(self)?;
        Ok(())
    }
}

//#[test]
//fn test_read() {
//    for filename_result in fs::read_dir("puzzles").unwrap() {
//        let filename = filename_result.unwrap().path();
//        if filename.extension() == Some(OsStr::new("puz")) {
//            println!("Reading {:?}", filename);
//            let data = fs::read(&filename).unwrap();
//            let mut data_reader: &[u8] = &data;
//            let puzzle = PuzzleReader { input: &mut data_reader }.read_raw_puzzle().unwrap();
//        }
//    }
//}

#[test]
fn test_round_trip_read_write() {
    for filename_result in fs::read_dir("puzzles").unwrap() {
        let mut filename = filename_result.unwrap().path();
        if filename.extension() == Some(OsStr::new("puz")) {
            let data = fs::read(&filename).unwrap();
            let mut data_reader: &[u8] = &data;
            let puzzle = RawPuzzle::read_from(&mut data_reader).unwrap();
            let mut new_data: Vec<u8> = vec![];
            puzzle.write_to(&mut new_data).unwrap();
            filename.set_extension("puz.test");
            fs::write(&filename, &new_data).unwrap();
            assert!(data == new_data);
        }
    }
}