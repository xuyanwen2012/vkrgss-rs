//! The link between the two sides: a guest endpoint and a host endpoint over channels.
//!
//! [`link`] makes a connected pair. The guest (script thread) holds the [`GuestEnd`], the host
//! holds the [`HostEnd`]. Messages arrive in the order they were sent. The host does not block
//! on the link: it is told through the wake hook that something may be waiting and then drains
//! it with [`HostEnd::try_recv`]. Dropping either end disconnects the other, which reports
//! [`Disconnected`] instead of blocking forever.
//!
//! This is the in-process transport. A later process split replaces the module as a whole.

use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use thiserror::Error;

use crate::{GuestToHost, HostToGuest};

/// The other end of the link has been dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the other end of the guest/host link has gone")]
pub struct Disconnected;

/// Creates a connected pair of endpoints.
///
/// `wake` is called on the guest's thread after every message the guest sends, and once more
/// when the guest end is dropped, so a host that sleeps between events can be woken to read the
/// link. It must not block.
pub fn link(wake: impl Fn() + Send + Sync + 'static) -> (GuestEnd, HostEnd) {
    let (to_host, from_guest) = channel();
    let (to_guest, from_host) = channel();
    (
        GuestEnd {
            to_host,
            from_host,
            wake: Wake(Box::new(wake)),
        },
        HostEnd {
            from_guest,
            to_guest,
        },
    )
}

/// The wake hook. Dropping it wakes the host one last time.
struct Wake(Box<dyn Fn() + Send + Sync>);

impl Drop for Wake {
    fn drop(&mut self) {
        (self.0)();
    }
}

/// The guest's end of the link.
pub struct GuestEnd {
    // Declared before `wake`: fields drop in declaration order, so by the time the final wake
    // runs the host already sees the disconnection.
    to_host: Sender<GuestToHost>,
    from_host: Receiver<HostToGuest>,
    wake: Wake,
}

impl GuestEnd {
    /// Sends a message to the host and wakes it.
    pub fn send(&self, message: GuestToHost) -> Result<(), Disconnected> {
        self.to_host.send(message).map_err(|_| Disconnected)?;
        (self.wake.0)();
        Ok(())
    }

    /// Waits for the next message from the host.
    pub fn recv(&self) -> Result<HostToGuest, Disconnected> {
        self.from_host.recv().map_err(|_| Disconnected)
    }

    /// The next message from the host if one is waiting, without blocking.
    pub fn try_recv(&self) -> Result<Option<HostToGuest>, Disconnected> {
        try_recv(&self.from_host)
    }
}

impl std::fmt::Debug for GuestEnd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuestEnd").finish_non_exhaustive()
    }
}

/// The host's end of the link.
#[derive(Debug)]
pub struct HostEnd {
    from_guest: Receiver<GuestToHost>,
    to_guest: Sender<HostToGuest>,
}

impl HostEnd {
    /// The next message from the guest if one is waiting, without blocking. Messages sent
    /// before the guest end was dropped are still delivered; the disconnection is reported
    /// after the last of them.
    pub fn try_recv(&self) -> Result<Option<GuestToHost>, Disconnected> {
        try_recv(&self.from_guest)
    }

    /// Sends a message to the guest.
    pub fn send(&self, message: HostToGuest) -> Result<(), Disconnected> {
        self.to_guest.send(message).map_err(|_| Disconnected)
    }
}

fn try_recv<T>(receiver: &Receiver<T>) -> Result<Option<T>, Disconnected> {
    match receiver.try_recv() {
        Ok(message) => Ok(Some(message)),
        Err(TryRecvError::Empty) => Ok(None),
        Err(TryRecvError::Disconnected) => Err(Disconnected),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::thread;

    use super::*;
    use crate::InputSnapshot;

    fn title(text: &str) -> GuestToHost {
        GuestToHost::SetTitle {
            title: text.to_owned(),
        }
    }

    /// A link whose wake hook counts its calls.
    fn counted() -> (GuestEnd, HostEnd, Arc<AtomicUsize>) {
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        let (guest, host) = link(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        (guest, host, wakes)
    }

    #[test]
    fn host_receives_guest_messages_in_the_order_sent() {
        let (guest, host, _) = counted();
        let sender = thread::spawn(move || {
            for text in ["one", "two", "three"] {
                guest.send(title(text)).unwrap();
            }
            guest
        });
        // Keeps the guest end alive, so the link is still connected while it is read.
        let _guest = sender.join().unwrap();

        for text in ["one", "two", "three"] {
            assert_eq!(host.try_recv(), Ok(Some(title(text))));
        }
        assert_eq!(host.try_recv(), Ok(None));
    }

    #[test]
    fn guest_receives_host_messages_in_the_order_sent() {
        let (guest, host, _) = counted();
        let receiver = thread::spawn(move || {
            let first = guest.recv().unwrap();
            let second = guest.recv().unwrap();
            let third = guest.recv().unwrap();
            (first, second, third, guest.try_recv())
        });
        host.send(HostToGuest::Input(InputSnapshot::EMPTY)).unwrap();
        host.send(HostToGuest::Tick).unwrap();
        host.send(HostToGuest::Quit).unwrap();

        // The host end is still alive here, so the guest's last poll finds the link empty.
        assert_eq!(
            receiver.join().unwrap(),
            (
                HostToGuest::Input(InputSnapshot::EMPTY),
                HostToGuest::Tick,
                HostToGuest::Quit,
                Ok(None)
            )
        );
    }

    #[test]
    fn wake_hook_has_run_by_the_time_send_returns() {
        let (guest, _host, wakes) = counted();
        assert_eq!(
            wakes.load(Ordering::SeqCst),
            0,
            "creating the link wakes nobody"
        );
        let observed = Arc::clone(&wakes);
        let sender = thread::spawn(move || {
            guest.send(GuestToHost::Freeze).unwrap();
            let after_first = observed.load(Ordering::SeqCst);
            guest.send(GuestToHost::Freeze).unwrap();
            (after_first, observed.load(Ordering::SeqCst), guest)
        });
        let (after_first, after_second, _guest) = sender.join().unwrap();
        assert_eq!((after_first, after_second), (1, 2));
    }

    #[test]
    fn guest_waiting_for_a_message_learns_that_the_host_has_gone() {
        let (guest, host, _) = counted();
        let (waiting, started) = mpsc::channel();
        let waiter = thread::spawn(move || {
            waiting.send(()).unwrap();
            guest.recv()
        });
        started.recv().unwrap();
        drop(host);
        assert_eq!(waiter.join().unwrap(), Err(Disconnected));
    }

    #[test]
    fn guest_cannot_send_or_poll_once_the_host_has_gone() {
        let (guest, host, wakes) = counted();
        drop(host);
        assert_eq!(guest.send(GuestToHost::Freeze), Err(Disconnected));
        assert_eq!(guest.try_recv(), Err(Disconnected));
        assert_eq!(guest.recv(), Err(Disconnected));
        assert_eq!(wakes.load(Ordering::SeqCst), 0, "nobody is left to wake");
    }

    #[test]
    fn host_drains_what_was_sent_then_learns_that_the_guest_has_gone() {
        let (guest, host, wakes) = counted();
        let sender = thread::spawn(move || {
            guest.send(GuestToHost::Shutdown).unwrap();
        });
        sender.join().unwrap();

        assert_eq!(wakes.load(Ordering::SeqCst), 2, "one send, one drop");
        assert_eq!(host.try_recv(), Ok(Some(GuestToHost::Shutdown)));
        assert_eq!(host.try_recv(), Err(Disconnected));
        assert_eq!(host.send(HostToGuest::Tick), Err(Disconnected));
    }
}
