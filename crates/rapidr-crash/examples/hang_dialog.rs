//! The hang watchdog's question as the RapidR program it runs, for a look
//! (captures in each theme and language):
//!
//!   cargo run -p rapidr-crash --example hang_dialog [es] > hang.rr
//!   RAPIDR_CAPTURE=shot RAPIDR_CAPTURE_DELAY=2 RAPIDR_THEME=rapidr-dark rapidr __dialog hang.rr

use rapidr_crash::watchdog::{dialog_source, Texts};

fn main() {
    let es = std::env::args().nth(1).as_deref() == Some("es");
    let t = if es {
        // (ide/lang/es.po's words for the question)
        Texts {
            title: "RapidR Studio no responde".into(),
            body: "Su ventana no responde desde hace unos segundos. Esperar le da más tiempo. Guardar copias y salir lo cierra: las copias de tu trabajo sin guardar que Studio guarda mientras escribes (cada 20 segundos) se ofrecen cuando vuelva a abrirse. Se escribió un informe en esta computadora.".into(),
            wait: "&Esperar".into(),
            quit: "&Guardar copias y salir".into(),
            ..Texts::default()
        }
    } else {
        Texts::default()
    };
    print!("{}", dialog_source(&t));
}
