use std::thread::{spawn, JoinHandle};
use std::sync::{Arc, Mutex, Condvar};

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Copy, Clone)]
struct State {
    dirty: bool,
    canceled: bool,
}

pub struct DirtyLoop {
    thread: Option<JoinHandle<()>>,
    state: Arc<(Mutex<State>, Condvar)>,
}

impl DirtyLoop {
    pub fn new(mut action: Box<dyn FnMut() + Send + 'static>) -> Self {
        let state =
            Arc::new((Mutex::new(State { dirty: false, canceled: false }), Condvar::new()));
        let state2 = state.clone();
        DirtyLoop {
            thread: Some(spawn(move || {
                loop {
                    let old_state;
                    {
                        let mut lock =
                            state.1.wait_until(
                                state.0.lock().unwrap(),
                                |&mut value| value.dirty || value.canceled).unwrap();
                        old_state = *lock;
                        lock.dirty = false;
                    }
                    if old_state.dirty {
                        (*action)();
                    }
                    if old_state.canceled {
                        return;
                    }
                }
            })),
            state: state2,
        }
    }
    pub fn mark_dirty(&self) {
        self.state.0.lock().unwrap().dirty = true;
        self.state.1.notify_one();
    }
}

impl Drop for DirtyLoop {
    fn drop(&mut self) {
        self.state.0.lock().unwrap().canceled = true;
        self.state.1.notify_one();
        self.thread.take().unwrap().join().unwrap();
    }
}

#[test]
fn test_render_loop_once() {
    let s1 = Arc::new(Mutex::new(0));
    {
        let s2 = s1.clone();
        let render = DirtyLoop::new(Box::new(move || {
            let mut lock = s2.lock().unwrap();
            assert!(*lock == 0 || *lock == 1);
            *lock += 1;
        }));
        {
            let lock = s1.lock().unwrap();
            render.mark_dirty();
            render.mark_dirty();
            render.mark_dirty();
        }
    }
    let result = *s1.lock().unwrap();
    assert!(result == 1 || result == 2);
}

#[test]
fn test_render_loop_many() {
    for x in 0..100000 {
        test_render_loop_once();
    }
}