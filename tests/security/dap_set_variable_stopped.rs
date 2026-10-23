//! Security regression SEC-29 (docs/security/audit-preview4.md): the debug
//! adapter's setVariable changes a value only while the program is stopped,
//! by an explicit check (as setExpression), not only because a running
//! program's variable references are gone. Compiled into rapidr-dap's
//! adapter (`cargo test -p rapidr-dap security_`).

use super::*;

fn answers(out: &[u8]) -> Vec<Value> {
    let mut reader = std::io::BufReader::new(out);
    std::iter::from_fn(|| dap::read_message(&mut reader).ok().flatten()).collect()
}

#[test]
fn sec29_set_variable_only_while_stopped() {
    let (tx, _rx) = mpsc::channel();
    let mut a = Adapter::new(Vec::new(), tx);
    // (a reference still known — a refactor that keeps them across a
    // resume — while the program runs)
    a.refs.insert(7, (Some(0), None));
    a.stopped = false;
    a.client(json!({"seq": 1, "type": "request", "command": "setVariable", "arguments": {"variablesReference": 7, "name": "total", "value": "100"}}));
    let r = answers(&a.out);
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0]["success"], false);
    assert_eq!(r[0]["message"], "the program isn't stopped", "{r:?}");
    assert!(a.waiting.is_empty(), "nothing was sent to the program");
    // (stopped: past the check — to the program, which isn't there here)
    a.stopped = true;
    a.out.clear();
    a.client(json!({"seq": 2, "type": "request", "command": "setVariable", "arguments": {"variablesReference": 7, "name": "total", "value": "100"}}));
    let r = answers(&a.out);
    assert!(r.iter().all(|m| m["message"] != "the program isn't stopped"), "{r:?}");
}
