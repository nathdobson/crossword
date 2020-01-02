#![allow(unused)]
#![feature(wait_until, libc, step_trait, range_is_empty, try_from, non_ascii_idents, never_type)]

extern crate libc;
extern crate xi_unicode;
extern crate unicode_segmentation;
extern crate encoding;
extern crate byteorder;
extern crate core;

use byteorder::ReadBytesExt;
use std::convert::TryFrom;
use std::env;
use std::io;
use std::mem;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::ops::Range;
use std::io::BufWriter;
use std::thread::sleep;
use std::fs;
use std::fs::File;
use std::convert::TryInto;
use std::io::BufReader;
use std::io::BufRead;
use crate::play::play::Play;
use crate::play::interface::{TerminalInput, start_rendering, TerminalOutput, stop_rendering};
use crate::play::puzzle::{Puzzle, View, Mode};
use crate::play::dirty::DirtyLoop;
use crate::util::bag::Bag;
use crate::core::puzzle::Direction;


struct Game {
    puzzle: Puzzle,
    listeners: Bag<Arc<DirtyLoop>>,
}

struct EventLoop {
    input: TcpStream,
    game: Arc<Mutex<Game>>,
    view: Arc<Mutex<View>>,
    render_loop: Arc<DirtyLoop>,
}

impl EventLoop {
    fn run(&mut self) -> io::Result<()> {
        while let Some(action) = (TerminalInput { input: &mut self.input }.read_event()?) {
            let mut view = self.view.lock().unwrap();
            let mut game = self.game.lock().unwrap();
            let mut play = Play::new(&mut *view, &mut game.puzzle, None);
            play.do_action(action);
            if play.puzzle_changed() {
                for listener in game.listeners.into_iter() {
                    listener.mark_dirty();
                }
            } else if play.view_changed() {
                self.render_loop.mark_dirty();
            }
        }
        Ok(())
    }
}

fn handle(mut stream: TcpStream, game1: Arc<Mutex<Game>>) -> io::Result<()> {
    let mut input = stream.try_clone()?;
    let mut output = BufWriter::new(stream);
    let view = Arc::new(Mutex::new(View {
        position: (0, 0),
        direction: Direction::Across,
        mode: Mode::Solving,
        pencil: false
    }));
    let view2 = view.clone();
    let game2 = game1.clone();
    start_rendering(&mut output);
    let render_loop = Arc::new(DirtyLoop::new(Box::new(move || {
        let view_clone = view2.lock().unwrap().clone();
        let puzzle_clone = game2.lock().unwrap().puzzle.clone();
        TerminalOutput { output: &mut output, view: &view_clone, puzzle: &puzzle_clone }.render();
    })));
    let render_token = game1.lock().unwrap().listeners.insert(render_loop.clone());
    render_loop.mark_dirty();
    let mut event_loop = EventLoop {
        input: input,
        game: game1.clone(),
        view: view,
        render_loop: render_loop,
    };
    event_loop.run();
    game1.lock().unwrap().listeners.remove(render_token);
    stop_rendering(&mut event_loop.input);
    Ok(())
}

fn fix_listener(listener: &TcpListener) {
    use std::os::unix::io::AsRawFd;
    unsafe {
        let optval: libc::c_int = 1;
        let ret = libc::setsockopt(listener.as_raw_fd(),
                                   libc::SOL_SOCKET,
                                   libc::SO_REUSEPORT,
                                   &optval as *const _ as *const libc::c_void,
                                   mem::size_of_val(&optval) as libc::socklen_t);
        if ret != 0 {
            let err: Result<(), _> = Err(io::Error::last_os_error());
            err.expect("setsockopt failed");
        }
    }
}

fn run_server(filename: &str, address: &str) -> io::Result<()> {
    let file = Path::new(filename).to_path_buf();
    let puzzle = Puzzle::read_from(&mut BufReader::new(File::open(&file)?))?;
    let game = Arc::new(Mutex::new(Game { puzzle: puzzle, listeners: Bag::new() }));
    {
        let game2 = game.clone();
        game.lock().unwrap().listeners.insert(Arc::new(DirtyLoop::new(Box::new(move || {
            game2.lock().unwrap().puzzle.clone().write_to(&mut (File::create(&file).unwrap()));
        }))));
    }
    let listener = TcpListener::bind(address)?;
    fix_listener(&listener);
    for stream_result in listener.incoming() {
        let stream = stream_result?;
        let puzzle2 = game.clone();
        thread::spawn(move || {
            handle(stream, puzzle2);
        });
    }
    thread::sleep(Duration::from_secs(10));
    Ok(())
}
