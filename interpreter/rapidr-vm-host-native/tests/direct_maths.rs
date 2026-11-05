//! The pure maths builtins the VM calls itself (docs/vm-fast-design.md:
//! `ABS`, `SIN`, `INT` …) are the very ones this host answers: for every
//! name, in any case, and every kind of value, the host's own call gives
//! the same kind and bits as the VM's direct function — so a program gives
//! the same answers whether the VM or the host runs them.

use rapidr_value::Value;
use rapidr_vm::Host;
use rapidr_vm_host_native::NativeHost;

fn exact(v: &Value) -> String {
    match v {
        Value::Integer(n) => format!("I{n}"),
        Value::Double(f) => format!("D{:016x}", f.to_bits()),
        Value::Boolean(b) => format!("B{b}"),
        Value::String(s) => format!("S{:?}", s.as_str()),
        Value::Null => "N".into(),
        other => format!("{other:?}"),
    }
}

#[test]
fn the_host_answers_every_direct_builtin_as_the_vm_does() {
    assert!(<NativeHost as Host>::DIRECT_MATHS, "this host lets the VM run them");
    let mut values = vec![Value::Null, Value::Boolean(true), Value::Boolean(false), Value::String("2.5".into()), Value::String("abc".into()), Value::String("1,2".into())];
    for n in [0i64, 1, -1, 7, -7, i32::MAX as i64, i32::MIN as i64, 1 << 40, -(1 << 53) - 1] {
        values.push(Value::Integer(n));
    }
    for f in [0.0, -0.0, 0.5, -0.5, 2.5, -2.5, 2.7, -2.7, 1e10, -1e10, 1e300, f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 0.1] {
        values.push(Value::Double(f));
    }
    let mut host = NativeHost::default();
    let names: Vec<&str> = rapidr_vm::direct_names().collect();
    assert_eq!(names, rapidr_bytecode::builtins::PURE_MATHS);
    for name in names {
        for spelled in [name.to_string(), name.to_ascii_uppercase()] {
            if let Some(direct) = rapidr_vm::direct_builtin2(name) {
                // (SHL and SHR: two arguments)
                for a in &values {
                    for b in &values {
                        let by_host = host.call_builtin(&spelled, &[a.clone(), b.clone()]).unwrap_or_else(|e| panic!("{spelled}({a:?}, {b:?}): {e}"));
                        assert_eq!(exact(&by_host), exact(&direct(a, b)), "{spelled}({a:?}, {b:?})");
                    }
                }
                continue;
            }
            let direct = rapidr_vm::direct_builtin(name).unwrap();
            for v in &values {
                let by_host = host.call_builtin(&spelled, std::slice::from_ref(v)).unwrap_or_else(|e| panic!("{spelled}({v:?}): {e}"));
                assert_eq!(exact(&by_host), exact(&direct(v)), "{spelled}({v:?})");
            }
        }
    }
}
