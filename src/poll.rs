use std::time::Duration;
use std::fmt;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::sys::{self, io};
use crate::{Token, Ready, PollOpt, Events, Evented};

pub struct Poll(pub(crate) sys::Epoll);

impl Poll {
    pub fn new() -> io::Result<Poll> {
        is_send::<Poll>();
        is_sync::<Poll>();

        Ok(Poll(sys::Epoll::new()?))
    }

    pub fn wait(&self, events: &mut Events, timeout: Option<Duration>) -> io::Result<usize> {
        self.0.wait(&mut events.inner, timeout)?;
        Ok(events.len())
    }

    pub fn register<E: ?Sized>(&self, handle: &E, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()>
        where E: Evented
    {
        validate_args(token, interest)?;

        handle.register(self, token, interest, opts)?;

        Ok(())
    }

    pub fn reregister<E: ?Sized>(&self, handle: &E, token: Token, interest: Ready, opts: PollOpt) -> io::Result<()>
        where E: Evented
    {
        validate_args(token, interest)?;

        handle.reregister(self, token, interest, opts)?;

        Ok(())
    }

    pub fn deregister<E: ?Sized>(&self, handle: &E) -> io::Result<()>
        where E: Evented
    {
        handle.deregister(self)?;

        Ok(())
    }
}

impl AsRawFd for Poll {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

impl fmt::Debug for Poll {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Poll")
    }
}

fn validate_args(_token: Token, interest: Ready) -> io::Result<()> {
    if !interest.is_readable() && !interest.is_writable() {
        return Err(io::Error::new(io::ErrorKind::Other, "interest must include readable or writable"));
    }

    Ok(())
}

fn is_send<T: Send>() {}
fn is_sync<T: Sync>() {}

#[derive(Debug)]
pub struct SelectorId {
    id: AtomicUsize,
}

impl SelectorId {
    pub fn new() -> SelectorId {
        SelectorId {
            id: AtomicUsize::new(0),
        }
    }

    pub fn associate_selector(&self, poll: &Poll) -> io::Result<()> {
        let selector_id = self.id.load(Ordering::SeqCst);

        if selector_id != 0 && selector_id != poll.0.id() {
            Err(io::Error::new(io::ErrorKind::Other, "socket already registered"))
        } else {
            self.id.store(poll.0.id(), Ordering::SeqCst);
            Ok(())
        }
    }
}

impl Clone for SelectorId {
    fn clone(&self) -> SelectorId {
        SelectorId {
            id: AtomicUsize::new(self.id.load(Ordering::SeqCst)),
        }
    }
}
