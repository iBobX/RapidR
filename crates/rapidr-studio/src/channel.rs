//! A session transport over messages (the web's: the program runs in a
//! sandboxed frame and speaks the protocol over a MessagePort). The host
//! gives the way to send a request and to end the frame; what the frame
//! sends comes in through [`incoming`] and [`ended`], one program at a time
//! (a page runs one program under development).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::time::Duration;

use rapidr_session::protocol::{Event, Incoming, Request};
use rapidr_session::Transport;

thread_local! {
    static QUEUE: RefCell<VecDeque<Incoming>> = const { RefCell::new(VecDeque::new()) };
    static ENDED: Cell<Option<i32>> = const { Cell::new(None) };
}

/// The frame sent `json` (a protocol event).
pub fn incoming(json: &str) {
    let item = match Event::from_json(json) {
        Ok(e) => Incoming::Event(e),
        Err(_) => Incoming::Text(json.to_string()),
    };
    QUEUE.with(|q| q.borrow_mut().push_back(item));
}

/// The frame ended without saying so (it failed to start, or was closed).
pub fn ended(code: i32) {
    ENDED.with(|e| e.set(Some(code)));
}

/// A transport whose requests go through `send` and which `stop` ends.
pub struct ChannelTransport {
    send: Box<dyn Fn(&str) -> Result<(), String>>,
    stop: Box<dyn Fn()>,
}

impl ChannelTransport {
    /// A new program's transport: what an earlier one left is dropped.
    pub fn new(send: impl Fn(&str) -> Result<(), String> + 'static, stop: impl Fn() + 'static) -> Self {
        QUEUE.with(|q| q.borrow_mut().clear());
        ENDED.with(|e| e.set(None));
        ChannelTransport { send: Box::new(send), stop: Box::new(stop) }
    }
}

impl Transport for ChannelTransport {
    fn send(&mut self, request: &Request) -> Result<(), String> {
        (self.send)(&request.to_json())
    }

    fn receive(&mut self, _timeout: Option<Duration>) -> Vec<Incoming> {
        QUEUE.with(|q| q.borrow_mut().drain(..).collect())
    }

    fn kill(&mut self) {
        (self.stop)();
        ENDED.with(|e| {
            if e.get().is_none() {
                e.set(Some(0));
            }
        });
    }

    fn ended(&mut self) -> Option<i32> {
        ENDED.with(Cell::get)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    #[test]
    fn messages_both_ways() {
        let sent = Rc::new(RefCell::new(Vec::new()));
        let s = sent.clone();
        let mut t = ChannelTransport::new(move |j| {
            s.borrow_mut().push(j.to_string());
            Ok(())
        }, || ());
        t.send(&Request::new(1, rapidr_session::Command::Pause)).unwrap();
        assert!(sent.borrow()[0].contains("pause"));
        incoming(r#"{"type":"output","stream":"stdout","text":"hi\n"}"#);
        let got = t.receive(None);
        assert!(matches!(&got[..], [Incoming::Event(_)]));
        assert_eq!(t.ended(), None);
        t.kill();
        assert_eq!(t.ended(), Some(0));
    }
}
