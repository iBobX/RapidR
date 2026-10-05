//! RapidQ's LPRINT / LFLUSH: "a statement just like PRINT except all
//! output is directed to the default printer"; LFLUSH starts the print job,
//! and what's left is printed when the program ends (RapidQ manual). The
//! text goes on PRINTER pages (rapidr_value::objects::printer: A4, 10 pt,
//! half-inch margins) and to the runtime's print hook, as `Printer.EndDoc`.

use std::cell::RefCell;

use crate::Value;

thread_local! {
    static TEXT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Most text kept before LFLUSH (beyond it, LPRINT flushes by itself).
const MAX_TEXT: usize = 4 << 20;
const ZONE: usize = 14;

/// `LPRINT a; b, c` (the parser's `__lprint(newline, item, zone, …)`):
/// numbers as PRINT shows them (`crate::format::print_text`); the parser
/// gives `,` no print zone, as PRINT (a zone flag still pads to 14 columns).
pub fn lprint(args: &[Value]) -> Value {
    let newline = args.first().is_some_and(|v| v.to_i64() != 0);
    let full = TEXT.with(|t| {
        let mut t = t.borrow_mut();
        for pair in args.get(1..).unwrap_or(&[]).chunks(2) {
            t.push_str(&crate::format::print_text(&pair[0]));
            if pair.get(1).is_some_and(|z| z.to_i64() != 0) {
                let col = t.len() - t.rfind('\n').map_or(0, |i| i + 1);
                t.push_str(&" ".repeat(ZONE - col % ZONE));
            }
        }
        if newline {
            t.push('\n');
        }
        t.len() > MAX_TEXT
    });
    if full {
        let _ = flush();
    }
    Value::Null
}

/// `LFLUSH`: the text LPRINTed so far, printed (nothing when there is none).
pub fn flush() -> Result<(), String> {
    let text = TEXT.with(|t| std::mem::take(&mut *t.borrow_mut()));
    if text.is_empty() {
        return Ok(());
    }
    let mut p = crate::objects::printer::Printer::default();
    p.font.size = 10;
    p.title = "LPRINT".to_string();
    let (_, page_h) = p.page_size();
    let margin = (crate::objects::printer::DPI / 2.0) as i64;
    let line = p.text_height();
    p.call("begindoc", &[]);
    let mut y = margin;
    for l in text.trim_end_matches('\n').split('\n') {
        if y + line > page_h - margin {
            p.call("newpage", &[]);
            y = margin;
        }
        let feed = l.contains('\x0c');
        let l = l.replace('\t', "        ").replace('\x0c', "");
        if !l.trim().is_empty() {
            p.call("textout", &[Value::Integer(margin), Value::Integer(y), Value::String(l)]);
        }
        if feed {
            // (a form feed: the next page)
            p.call("newpage", &[]);
            y = margin;
            continue;
        }
        y += line;
    }
    match p.end_doc() {
        Some(job) => crate::objects::print_job(&job),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_int, v_str};

    thread_local! {
        static JOBS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn keep(job: &crate::objects::printer::PrintJob) -> Result<(), String> {
        JOBS.with(|j| j.borrow_mut().push(String::from_utf8_lossy(&job.pdf).into_owned()));
        Ok(())
    }

    #[test]
    fn lprint_lines_go_to_the_printer_at_lflush() {
        crate::objects::set_print_hook(keep);
        lprint(&[v_int(1), v_str("Hello"), v_int(0), v_int(42), v_int(0)]);
        lprint(&[v_int(1), v_str("a"), v_int(1), v_str("b"), v_int(0)]);
        lprint(&[v_int(0), v_str("same "), v_int(0)]);
        lprint(&[v_int(1), v_str("line"), v_int(0)]);
        assert!(JOBS.with(|j| j.borrow().is_empty()), "nothing printed before LFLUSH");
        flush().unwrap();
        let pdf = JOBS.with(|j| j.borrow()[0].clone());
        assert!(pdf.contains("(Hello42) Tj"), "{pdf}");
        assert!(pdf.contains(&format!("(a{}b) Tj", " ".repeat(13))), "{pdf}");
        assert!(pdf.contains("(same line) Tj"), "{pdf}");
        flush().unwrap();
        assert_eq!(JOBS.with(|j| j.borrow().len()), 1, "an empty LFLUSH prints nothing");
    }
}
