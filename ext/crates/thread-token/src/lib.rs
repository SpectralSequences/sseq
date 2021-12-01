#![allow(clippy::mutex_atomic)]
use core::num::NonZeroUsize;
use crossbeam_channel::{unbounded, Receiver, TryRecvError};
use std::sync::{Condvar, Mutex};

/// A `TokenBucket` is a bucket containing a fixed number of "tokens". Threads can take request to
/// take out a token. If no tokens are available, the thread is blocked (while consuming no CPU
/// time) until a token is released and returned to the bucket. When this happens, one of the
/// threads waiting for a token is woken up and takes the token. There is no guarantee as to which
/// thread in the queue gets woken up when a token is available.
///
/// A `TokenBucket` is useful for limiting the number of active threads used by a function/program.
pub struct TokenBucket {
    pub max_threads: NonZeroUsize,
    running_threads: Mutex<usize>,
    condvar: Condvar,
}

impl Default for TokenBucket {
    /// The default value of TokenBucket has two threads.
    fn default() -> Self {
        Self::new(NonZeroUsize::new(2).unwrap())
    }
}

impl TokenBucket {
    /// Constructs a new `TokenBucket` with a fixed number of tokens.
    pub fn new(max_threads: NonZeroUsize) -> Self {
        Self {
            max_threads,
            running_threads: Mutex::new(0),
            condvar: Condvar::new(),
        }
    }

    /// Attempts to take a token from the bucket. This will block until a token is available.
    pub fn take_token(&'_ self) -> Token {
        let mut running_threads = self.running_threads.lock().unwrap();

        loop {
            if *running_threads < self.max_threads.get() {
                *running_threads += 1;
                return Token { bucket: self };
            } else {
                running_threads = self.condvar.wait(running_threads).unwrap();
            }
        }
    }

    /// This function attempts to read a message from `receiver` (if available). If a message is
    /// received, it returns the same token. If not, it releases the existing token. It then waits
    /// for a message, and then queues for a new token. The new token is then returned.
    pub fn recv_or_release<'a>(
        &'a self,
        mut token: Token<'a>,
        receiver: &Option<Receiver<()>>,
    ) -> Token<'a> {
        if let Some(recv) = &receiver {
            match recv.try_recv() {
                Ok(_) => (),
                Err(TryRecvError::Empty) => {
                    token.release();
                    recv.recv().unwrap();
                    token = self.take_token();
                }
                Err(TryRecvError::Disconnected) => panic!("Sender disconnected"),
            }
        }
        token
    }

    /// This function attempts to read a message from `receiver` (if available). If a message is
    /// received, it returns the same token. If not, it releases the existing token. It then waits
    /// for a message, and then queues for a new token. The new token is then returned.
    pub fn recv2_or_release<'a>(
        &'a self,
        mut token: Token<'a>,
        receiver1: &Option<Receiver<()>>,
        receiver2: &Option<Receiver<()>>,
    ) -> Token<'a> {
        if receiver1.is_none() {
            return self.recv_or_release(token, receiver2);
        } else if receiver2.is_none() {
            return self.recv_or_release(token, receiver1);
        }

        let recv1 = receiver1.as_ref().unwrap();

        match recv1.try_recv() {
            Ok(_) => {
                token = self.recv_or_release(token, receiver2);
            }
            Err(TryRecvError::Empty) => {
                token.release();

                let recv2 = receiver2.as_ref().unwrap();

                match recv2.try_recv() {
                    Ok(_) => {
                        recv1.recv().unwrap();
                        token = self.take_token();
                    }
                    // We are waiting for both recv1 and recv2
                    Err(TryRecvError::Empty) => {
                        recv1.recv().unwrap();
                        recv2.recv().unwrap();
                        token = self.take_token();
                    }
                    Err(TryRecvError::Disconnected) => panic!("Sender disconnected"),
                }
            }
            Err(TryRecvError::Disconnected) => panic!("Sender disconnected"),
        }
        token
    }

    fn release_token(&self) {
        let mut running_threads = self.running_threads.lock().unwrap();
        *running_threads -= 1;
        self.condvar.notify_one();
    }

    /// Apply the function `f` to all `(s, t)` pairs where `s_range.contains(s)` and `min_t <= t <
    /// max_t(s)`, with the condition that we wait for `(s - 1, t - 1)` and `(s, t - 1)` to be
    /// completed before running `(s, t)`.
    ///
    /// The variable `&mut init` is passed as the last argument to `f`, which should be used as
    /// some scratch data, e.g. a scratch [`FpVector`]. It will be cloned for each thread and
    /// reused within the same thread.
    ///
    /// This spawns one thread for each s, and the thread is named after s.
    pub fn iter_s_t<T: Clone + Send>(
        &self,
        s_range: std::ops::Range<u32>,
        min_t: i32,
        max_t: impl Fn(u32) -> i32,
        init: T,
        f: impl Fn(u32, i32, &mut T) + Send + Sync + Clone,
    ) {
        crossbeam_utils::thread::scope(|scope| {
            let mut last_receiver: Option<Receiver<()>> = None;

            for s in s_range {
                let (sender, receiver) = unbounded();
                sender.send(()).unwrap();

                let mut init = init.clone();
                let f = f.clone();
                let max_t = max_t(s);

                scope
                    .builder()
                    .name(format!("s = {s}"))
                    .spawn(move |_| {
                        let mut token = self.take_token();
                        for t in min_t..max_t {
                            token = self.recv_or_release(token, &last_receiver);

                            f(s, t, &mut init);
                            // The last receiver will be dropped so the send will fail
                            sender.send(()).ok();
                        }
                    })
                    .unwrap();
                last_receiver = Some(receiver);
            }
        })
        .unwrap();
    }
}

/// A `Token` is what `TokenBucket::take_token` returns. The token is automatically released when
/// the `Token` is dropped. One can also explicitly release the token via `Token::release`, but
/// this function does nothing but drop the token.
pub struct Token<'a> {
    bucket: &'a TokenBucket,
}

impl<'a> Drop for Token<'a> {
    fn drop(&mut self) {
        self.bucket.release_token();
    }
}

impl<'a> Token<'a> {
    /// This function does not do anything. It simply takes ownership and ends, which triggers
    /// `drop`.
    pub fn release(self) {}
}
