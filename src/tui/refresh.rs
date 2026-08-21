//! Daemonless, coalesced background refresh transport.

use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};
use std::thread;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefreshRequest<R> {
    pub generation: u64,
    pub reason: R,
}

#[derive(Debug)]
struct RefreshResponse<T> {
    generation: u64,
    result: Result<T, String>,
}

#[derive(Debug)]
pub enum RefreshPoll<T> {
    Pending,
    Ready { generation: u64, value: T },
    Failed,
    Stale,
    WorkerUnavailable,
}

/// At most one refresh is active. While it runs, requests collapse into the
/// newest generation; results for superseded generations are ignored.
pub struct RefreshController<R, T> {
    sender: SyncSender<RefreshRequest<R>>,
    receiver: Receiver<RefreshResponse<T>>,
    next_generation: u64,
    active_generation: Option<u64>,
    pending: Option<RefreshRequest<R>>,
    worker_unavailable: bool,
}

impl<R, T> RefreshController<R, T>
where
    R: Clone + Send + 'static,
    T: Send + 'static,
{
    pub fn spawn(
        mut task: impl FnMut(RefreshRequest<R>) -> Result<T, String> + Send + 'static,
    ) -> Self {
        let (sender, request_receiver) = sync_channel(1);
        let (response_sender, receiver) = sync_channel(1);
        thread::spawn(move || worker_loop(&mut task, request_receiver, response_sender));
        Self {
            sender,
            receiver,
            next_generation: 0,
            active_generation: None,
            pending: None,
            worker_unavailable: false,
        }
    }

    pub fn request(&mut self, reason: R) -> u64 {
        self.next_generation += 1;
        let request = RefreshRequest {
            generation: self.next_generation,
            reason,
        };
        if self.active_generation.is_some() {
            self.pending = Some(request);
        } else {
            self.dispatch(request);
        }
        self.next_generation
    }

    pub fn poll(&mut self) -> RefreshPoll<T> {
        if self.worker_unavailable {
            self.worker_unavailable = false;
            return RefreshPoll::WorkerUnavailable;
        }
        match self.receiver.try_recv() {
            Ok(response) => self.accept_response(response),
            Err(TryRecvError::Empty) => RefreshPoll::Pending,
            Err(TryRecvError::Disconnected) => RefreshPoll::WorkerUnavailable,
        }
    }

    fn dispatch(&mut self, request: RefreshRequest<R>) {
        let generation = request.generation;
        match self.sender.try_send(request) {
            Ok(()) => self.active_generation = Some(generation),
            Err(TrySendError::Full(request)) => self.pending = Some(request),
            Err(TrySendError::Disconnected(_)) => self.worker_unavailable = true,
        }
    }

    fn accept_response(&mut self, response: RefreshResponse<T>) -> RefreshPoll<T> {
        if self.active_generation == Some(response.generation) {
            self.active_generation = None;
        }
        let stale = response.generation != self.next_generation;
        if let Some(request) = self.pending.take() {
            self.dispatch(request);
        }
        if stale {
            return RefreshPoll::Stale;
        }
        match response.result {
            Ok(value) => RefreshPoll::Ready {
                generation: response.generation,
                value,
            },
            Err(_) => RefreshPoll::Failed,
        }
    }
}

fn worker_loop<R, T>(
    task: &mut impl FnMut(RefreshRequest<R>) -> Result<T, String>,
    receiver: Receiver<RefreshRequest<R>>,
    sender: SyncSender<RefreshResponse<T>>,
) {
    while let Ok(request) = receiver.recv() {
        let generation = request.generation;
        let result = task(request);
        if sender.send(RefreshResponse { generation, result }).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_responses_do_not_replace_the_newest_generation() {
        let (sender, receiver) = sync_channel::<RefreshRequest<()>>(1);
        let (response_sender, response_receiver) = sync_channel::<RefreshResponse<&'static str>>(2);
        let mut controller: RefreshController<(), &'static str> = RefreshController {
            sender,
            receiver: response_receiver,
            next_generation: 2,
            active_generation: Some(1),
            pending: None,
            worker_unavailable: false,
        };
        let _unused_sender = response_sender;
        drop(receiver);
        assert!(matches!(
            controller.accept_response(RefreshResponse {
                generation: 1,
                result: Ok("old"),
            }),
            RefreshPoll::Stale
        ));
    }

    #[test]
    fn repeated_requests_coalesce_to_the_latest_generation() {
        let mut controller =
            RefreshController::spawn(|request: RefreshRequest<u8>| Ok(request.reason));
        let first = controller.request(1);
        let second = controller.request(2);
        assert_eq!(first, 1);
        assert_eq!(second, 2);
        assert_eq!(
            controller
                .pending
                .as_ref()
                .map(|request| request.generation),
            Some(2)
        );
    }
}
