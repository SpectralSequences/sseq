#![allow(clippy::mutex_atomic)]
use core::num::NonZeroUsize;
use crossbeam_channel::{Receiver, TryRecvError};
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
