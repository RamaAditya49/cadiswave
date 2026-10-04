//! Keep only the latest packet for each meter key.
use super::MeterEvent;
use indexmap::IndexMap;
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
const CAPACITY: usize = 256;
struct State {
    pending: IndexMap<String, MeterEvent>,
    closed: bool,
}
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
    senders: AtomicUsize,
}
pub(super) struct MeterSender(Arc<Shared>);
pub struct MeterEvents(Arc<Shared>);
pub(super) fn channel() -> (MeterSender, MeterEvents) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State {
            pending: IndexMap::new(),
            closed: false,
        }),
        ready: Condvar::new(),
        senders: AtomicUsize::new(1),
    });
    (MeterSender(shared.clone()), MeterEvents(shared))
}
impl Clone for MeterSender {
    fn clone(&self) -> Self {
        self.0.senders.fetch_add(1, Ordering::Relaxed);
        Self(self.0.clone())
    }
}
impl Drop for MeterSender {
    fn drop(&mut self) {
        if self.0.senders.fetch_sub(1, Ordering::AcqRel) == 1
            && let Ok(mut state) = self.0.state.lock()
        {
            state.closed = true;
            self.0.ready.notify_all();
        }
    }
}
impl MeterSender {
    pub(super) fn send(
        &self,
        event: MeterEvent,
    ) -> std::result::Result<(), mpsc::SendError<MeterEvent>> {
        let Ok(mut state) = self.0.state.lock() else {
            return Err(mpsc::SendError(event));
        };
        if state.closed {
            return Err(mpsc::SendError(event));
        }
        if state
            .pending
            .get(&event.key)
            .is_some_and(|old| old.generation > event.generation)
        {
            return Ok(());
        }
        if state.pending.len() == CAPACITY && !state.pending.contains_key(&event.key) {
            state.pending.shift_remove_index(0);
        }
        state.pending.insert(event.key.clone(), event);
        self.0.ready.notify_one();
        Ok(())
    }
}
impl MeterEvents {
    pub fn try_recv(&self) -> std::result::Result<MeterEvent, mpsc::TryRecvError> {
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| mpsc::TryRecvError::Disconnected)?;
        if let Some((_, event)) = state.pending.shift_remove_index(0) {
            Ok(event)
        } else if state.closed {
            Err(mpsc::TryRecvError::Disconnected)
        } else {
            Err(mpsc::TryRecvError::Empty)
        }
    }
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> std::result::Result<MeterEvent, mpsc::RecvTimeoutError> {
        let deadline = Instant::now() + timeout;
        let mut state = self
            .0
            .state
            .lock()
            .map_err(|_| mpsc::RecvTimeoutError::Disconnected)?;
        loop {
            if let Some((_, event)) = state.pending.shift_remove_index(0) {
                return Ok(event);
            }
            if state.closed {
                return Err(mpsc::RecvTimeoutError::Disconnected);
            }
            if Instant::now() >= deadline {
                return Err(mpsc::RecvTimeoutError::Timeout);
            }
            state = self
                .0
                .ready
                .wait_timeout(state, deadline.saturating_duration_since(Instant::now()))
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)?
                .0;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn event(key: &str, generation: u64) -> MeterEvent {
        MeterEvent {
            key: key.into(),
            identity: cadiswave_core::model::NodeIdentity {
                server_cookie: 1,
                object_serial: "1".into(),
            },
            generation,
            peak: 0.5,
            channels: cadiswave_core::pcm::ChannelPeaks::Mono(0.5),
            available: true,
        }
    }
    #[test]
    fn slow_consumers_receive_bounded_latest_values() {
        let (sender, events) = channel();
        for generation in 0..1000 {
            sender.send(event("a", generation)).unwrap();
        }
        sender.send(event("a", 1)).unwrap();
        assert_eq!(events.try_recv().unwrap().generation, 999);
        assert!(events.try_recv().is_err());
        for key in 0..1000 {
            sender.send(event(&key.to_string(), 1000)).unwrap();
        }
        assert_eq!(events.0.state.lock().unwrap().pending.len(), CAPACITY);
        drop(sender);
        for _ in 0..CAPACITY {
            events.try_recv().unwrap();
        }
        assert_eq!(
            events.try_recv().unwrap_err(),
            mpsc::TryRecvError::Disconnected
        );
    }
}
