//! Two-participant named barriers for replayable concurrency tests.
//!
//! Each name in `steps` is a rendezvous. `arrive_and_wait` blocks until both
//! participants have arrived at the current name, then advances. Walking the
//! same names therefore releases steps in the same order. No `thread::sleep`.

use std::sync::{Condvar, Mutex};

/// Pairwise named-barrier seam. Fields stay private so loom can replace the
/// internals later without changing the type name or method signatures.
pub struct Interleaving {
    steps: Vec<&'static str>,
    state: Mutex<State>,
    cv: Condvar,
}

struct State {
    phase: usize,
    arrived: usize,
}

const PARTICIPANTS: usize = 2;

impl Interleaving {
    pub fn new(steps: &[&'static str]) -> Self {
        Self {
            steps: steps.to_vec(),
            state: Mutex::new(State {
                phase: 0,
                arrived: 0,
            }),
            cv: Condvar::new(),
        }
    }

    pub fn arrive_and_wait(&self, step: &'static str) {
        let mut state = self.state.lock().expect("interleaving mutex");
        loop {
            match self.steps.get(state.phase).copied() {
                Some(current) if current == step => break,
                Some(_) if self.steps[state.phase..].contains(&step) => {
                    state = self.cv.wait(state).expect("interleaving condvar");
                }
                Some(current) => {
                    panic!("interleaving step {step} is not current ({current}) or remaining")
                }
                None => panic!("interleaving finished, unexpected step {step}"),
            }
        }
        state.arrived += 1;
        if state.arrived == PARTICIPANTS {
            state.arrived = 0;
            state.phase += 1;
            self.cv.notify_all();
        } else {
            let waiting_phase = state.phase;
            while state.phase == waiting_phase {
                state = self.cv.wait(state).expect("interleaving condvar");
            }
        }
    }
}
