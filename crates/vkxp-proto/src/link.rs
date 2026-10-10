//! The link between the two sides: a guest endpoint and a host endpoint.
//!
//! [`link`] makes a connected pair over channels. The guest (script thread) holds the
//! [`GuestEnd`], the host holds the [`HostEnd`]. Messages arrive in the order they were sent.
//! The host does not block on the link: it is told through the wake hook that something may be
//! waiting and then drains it with [`HostEnd::try_recv`]. Dropping either end disconnects the
//! other, which reports [`Disconnected`] instead of blocking forever.
//!
//! That is the in-process transport. A guest in a process of its own holds a [`GuestEnd`]
//! made by [`GuestEnd::over_stream`] instead: the same operations on a connected Unix stream
//! socket, with the messages framed as [`crate::framing`] says. It reads and writes the socket
//! on the thread that calls it and has no thread of its own. Whatever is at the other side of
//! the socket hands the messages on; the host keeps its [`HostEnd`] of an in-process link.

use std::cell::RefCell;
use std::io::{ErrorKind, Read};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

use thiserror::Error;

use crate::framing::{self, LENGTH_LEN};
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
            sender: GuestSender(Sending::Channel {
                to_host,
                wake: Wake(Box::new(wake)),
            }),
            receiver: GuestReceiver(Receiving::Channel(from_host)),
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
    sender: GuestSender,
    receiver: GuestReceiver,
}

impl GuestEnd {
    /// A guest end over a connected Unix stream socket, whose other side reads
    /// [`GuestToHost`] and writes [`HostToGuest`] messages with [`crate::framing`]. The socket
    /// is used in blocking mode, except for the time of a [`GuestEnd::try_recv`].
    pub fn over_stream(stream: UnixStream) -> GuestEnd {
        let stream = Arc::new(stream);
        GuestEnd {
            sender: GuestSender(Sending::Stream(Arc::clone(&stream))),
            receiver: GuestReceiver(Receiving::Stream {
                stream,
                buffered: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Sends a message to the host and wakes it.
    pub fn send(&self, message: GuestToHost) -> Result<(), Disconnected> {
        self.sender.send(message)
    }

    /// Waits for the next message from the host.
    pub fn recv(&self) -> Result<HostToGuest, Disconnected> {
        self.receiver.recv()
    }

    /// The next message from the host if one is waiting, without blocking.
    pub fn try_recv(&self) -> Result<Option<HostToGuest>, Disconnected> {
        self.receiver.try_recv()
    }

    /// The two directions of this end, for two threads that each serve one. The host sees
    /// the guest as gone once the sender is dropped.
    pub fn split(self) -> (GuestSender, GuestReceiver) {
        (self.sender, self.receiver)
    }
}

impl std::fmt::Debug for GuestEnd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuestEnd").finish_non_exhaustive()
    }
}

/// The sending direction of a [`GuestEnd`].
pub struct GuestSender(Sending);

enum Sending {
    Channel {
        // Declared before `wake`: fields drop in declaration order, so by the time the final
        // wake runs the host already sees the disconnection.
        to_host: Sender<GuestToHost>,
        wake: Wake,
    },
    Stream(Arc<UnixStream>),
}

impl GuestSender {
    /// Sends a message to the host and wakes it.
    pub fn send(&self, message: GuestToHost) -> Result<(), Disconnected> {
        match &self.0 {
            Sending::Channel { to_host, wake } => {
                to_host.send(message).map_err(|_| Disconnected)?;
                (wake.0)();
                Ok(())
            }
            Sending::Stream(stream) => {
                // An error of the framing ends the connection like one of the socket.
                framing::write_message(&mut &**stream, &message).map_err(|_| Disconnected)
            }
        }
    }
}

impl std::fmt::Debug for GuestSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuestSender").finish_non_exhaustive()
    }
}

/// The receiving direction of a [`GuestEnd`].
pub struct GuestReceiver(Receiving);

enum Receiving {
    Channel(Receiver<HostToGuest>),
    Stream {
        stream: Arc<UnixStream>,
        /// Bytes read from the socket that no message was made of yet.
        buffered: RefCell<Vec<u8>>,
    },
}

impl GuestReceiver {
    /// Waits for the next message from the host.
    pub fn recv(&self) -> Result<HostToGuest, Disconnected> {
        match &self.0 {
            Receiving::Channel(from_host) => from_host.recv().map_err(|_| Disconnected),
            Receiving::Stream { stream, buffered } => {
                let mut buffered = buffered.borrow_mut();
                loop {
                    if let Some(message) = take_message(&mut buffered)? {
                        return Ok(message);
                    }
                    // In blocking mode nothing read means the wait was interrupted.
                    read_more(stream, &mut buffered)?;
                }
            }
        }
    }

    /// The next message from the host if one is waiting, without blocking.
    pub fn try_recv(&self) -> Result<Option<HostToGuest>, Disconnected> {
        match &self.0 {
            Receiving::Channel(from_host) => try_recv(from_host),
            Receiving::Stream { stream, buffered } => {
                let mut buffered = buffered.borrow_mut();
                if let Some(message) = take_message(&mut buffered)? {
                    return Ok(Some(message));
                }
                stream.set_nonblocking(true).map_err(|_| Disconnected)?;
                let read = loop {
                    match read_more(stream, &mut buffered) {
                        Ok(true) => {}
                        other => break other,
                    }
                };
                stream.set_nonblocking(false).map_err(|_| Disconnected)?;
                // What arrived before the other side closed is still delivered.
                match take_message(&mut buffered)? {
                    Some(message) => Ok(Some(message)),
                    None => read.map(|_| None),
                }
            }
        }
    }
}

impl std::fmt::Debug for GuestReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GuestReceiver").finish_non_exhaustive()
    }
}

/// Reads what the socket has into `buffered`. `Ok(false)` is a socket that has nothing now;
/// one that the other side closed is the disconnection.
fn read_more(stream: &UnixStream, buffered: &mut Vec<u8>) -> Result<bool, Disconnected> {
    let mut chunk = [0; 4096];
    match (&mut &*stream).read(&mut chunk) {
        Ok(0) => Err(Disconnected),
        Ok(count) => {
            buffered.extend_from_slice(&chunk[..count]);
            Ok(true)
        }
        Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {
            Ok(false)
        }
        Err(_) => Err(Disconnected),
    }
}

/// Takes the first message out of `buffered` if all of it is there. A length above the bound
/// and bytes that do not decode end the connection.
fn take_message(buffered: &mut Vec<u8>) -> Result<Option<HostToGuest>, Disconnected> {
    let Some((length, rest)) = buffered.split_first_chunk::<LENGTH_LEN>() else {
        return Ok(None);
    };
    let len = framing::message_len(*length).map_err(|_| Disconnected)?;
    let Some(bytes) = rest.get(..len) else {
        return Ok(None);
    };
    let message = framing::decode_message(bytes).map_err(|_| Disconnected)?;
    buffered.drain(..LENGTH_LEN + len);
    Ok(Some(message))
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
    use std::io::Write;
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

    /// A guest end over one socket of a pair, and the other socket.
    fn over_socket() -> (GuestEnd, UnixStream) {
        let (guest, other) = UnixStream::pair().unwrap();
        (GuestEnd::over_stream(guest), other)
    }

    /// Scenario "Messages in order".
    #[test]
    fn a_guest_end_over_a_socket_receives_in_the_order_written() {
        let (guest, mut other) = over_socket();
        framing::write_message(&mut other, &HostToGuest::Input(InputSnapshot::EMPTY)).unwrap();
        framing::write_message(&mut other, &HostToGuest::Tick).unwrap();
        assert_eq!(guest.recv(), Ok(HostToGuest::Input(InputSnapshot::EMPTY)));
        assert_eq!(guest.recv(), Ok(HostToGuest::Tick));
    }

    /// Scenario "Nothing waiting".
    #[test]
    fn a_guest_end_over_a_socket_does_not_wait_when_asked_not_to() {
        let (guest, mut other) = over_socket();
        assert_eq!(guest.try_recv(), Ok(None));
        framing::write_message(&mut other, &HostToGuest::Quit).unwrap();
        assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Quit)));
        assert_eq!(guest.try_recv(), Ok(None));

        // A message that has arrived in part is not one yet.
        let mut bytes = Vec::new();
        framing::write_message(&mut bytes, &HostToGuest::Tick).unwrap();
        let (first, rest) = bytes.split_at(5);
        other.write_all(first).unwrap();
        assert_eq!(guest.try_recv(), Ok(None));
        other.write_all(rest).unwrap();
        assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
    }

    #[test]
    fn a_guest_end_over_a_socket_sends_framed_messages() {
        let (guest, mut other) = over_socket();
        guest.send(title("one")).unwrap();
        guest.send(GuestToHost::Shutdown).unwrap();
        drop(guest);
        assert_eq!(
            framing::read_message(&mut other).unwrap(),
            Some(title("one"))
        );
        assert_eq!(
            framing::read_message(&mut other).unwrap(),
            Some(GuestToHost::Shutdown)
        );
        assert_eq!(
            framing::read_message::<GuestToHost>(&mut other).unwrap(),
            None
        );
    }

    /// Scenario "Other side gone".
    #[test]
    fn a_guest_end_over_a_socket_learns_that_the_other_side_has_gone() {
        let (guest, other) = over_socket();
        drop(other);
        assert_eq!(guest.recv(), Err(Disconnected));
        assert_eq!(guest.try_recv(), Err(Disconnected));
        assert_eq!(guest.send(GuestToHost::Freeze), Err(Disconnected));

        // What was written before the other side closed is still delivered.
        let (guest, mut other) = over_socket();
        framing::write_message(&mut other, &HostToGuest::Tick).unwrap();
        drop(other);
        assert_eq!(guest.try_recv(), Ok(Some(HostToGuest::Tick)));
        assert_eq!(guest.try_recv(), Err(Disconnected));
    }

    #[test]
    fn bytes_that_are_no_message_end_the_connection() {
        let (guest, mut other) = over_socket();
        other
            .write_all(&(framing::MAX_MESSAGE_LEN + 1).to_le_bytes())
            .unwrap();
        assert_eq!(guest.recv(), Err(Disconnected));

        let (guest, mut other) = over_socket();
        framing::write_message(&mut other, &GuestToHost::Freeze).unwrap();
        assert_eq!(guest.try_recv(), Err(Disconnected));
    }

    #[test]
    fn the_halves_of_a_split_end_work_on_two_threads() {
        let (guest, host, wakes) = counted();
        let (sender, receiver) = guest.split();
        let waiter = thread::spawn(move || receiver.recv());
        sender.send(GuestToHost::Freeze).unwrap();
        host.send(HostToGuest::Tick).unwrap();
        assert_eq!(waiter.join().unwrap(), Ok(HostToGuest::Tick));
        assert_eq!(host.try_recv(), Ok(Some(GuestToHost::Freeze)));

        // The host sees the guest as gone once the sender is dropped.
        drop(sender);
        assert_eq!(wakes.load(Ordering::SeqCst), 2, "one send, one drop");
        assert_eq!(host.try_recv(), Err(Disconnected));
    }
}
