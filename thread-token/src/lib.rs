
use std::sync::{Mutex, Condvar};

/// A `TokenBucket` is a bucket containing a fixed number of "tokens". Threads can take request to
/// take out a token. If no tokens are available, the thread is blocked (while consuming no CPU
/// time) until a token is released and returned to the bucket. When this happens, one of the
/// threads waiting for a token is woken up and takes the token. There is no guarantee as to which
/// thread in the queue gets woken up when a token is available.
///
/// A `TokenBucket` is useful for limiting the number of active threads used by a function/program.
pub struct TokenBucket {
    max_threads : usize,
    running_threads : Mutex<usize>,
    condvar : Condvar
}

impl TokenBucket {
    /// Constructs a new `TokenBucket` with a fixed number of tokens.
    pub fn new(max_threads : usize) -> Self {
        Self {
            max_threads,
            running_threads : Mutex::new(0),
            condvar : Condvar::new()
        }
    }

    /// Attempts to take a token from the bucket. This will block until a token is available.
    pub fn take_token<'a>(&'a self) -> Token<'a> {
        let mut running_threads = self.running_threads.lock().unwrap();

        loop {
            if *running_threads < self.max_threads {
                *running_threads += 1;
                return Token { bucket : &self };
            } else {
                running_threads = self.condvar.wait(running_threads).unwrap();
            }
        }
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
    bucket : &'a TokenBucket
}

impl<'a> Drop for Token<'a> {
    fn drop(&mut self) {
        self.bucket.release_token();
    }
}

impl<'a> Token<'a> {
    /// This function does not do anything. It simply takes ownership and ends, which triggers
    /// `drop`.
    pub fn release(self) { }
}
