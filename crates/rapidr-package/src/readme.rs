//! `README.txt` in every archive: how to open the app the first time on
//! that system, in the app's language (English or Spanish for now).
//!
//! DRAFT TEXTS: the lawyer agent reviews their wording before release
//! (docs/build-for-every-platform-plan.md §5).

use crate::info::AppInfo;
use crate::Target;

/// The README's languages.
pub const LANGUAGES: [&str; 2] = ["en", "es"];

/// The text for `target`, in `language` (`en`, `es`; anything else:
/// English). `app` true: a windowed app (an `.app`, an AppDir), false: a
/// console program.
pub fn readme(info: &AppInfo, target: Target, language: &str, app: bool) -> String {
    let es = language.trim().to_ascii_lowercase().starts_with("es");
    let t = |en: &str, es_text: &str| if es { es_text.to_string() } else { en.to_string() };
    let exe = &info.exe;
    let name = info.name.trim();
    let mut out = String::new();
    out.push_str(&format!("{name} {}\n", info.version.trim()));
    out.push_str(&format!("{}\n", t(&format!("for {}", target.label()), &format!("para {}", label_es(target)))));
    if !info.description.trim().is_empty() {
        out.push_str(&format!("{}\n", info.description.trim()));
    }
    let copyright = info.copyright_line();
    if !copyright.is_empty() {
        out.push_str(&format!("{copyright}\n"));
    }
    if !info.website.trim().is_empty() {
        out.push_str(&format!("{}\n", info.website.trim()));
    }
    out.push('\n');
    out.push_str(&t("HOW TO OPEN IT\n", "CÓMO ABRIRLA\n"));
    out.push_str("--------------\n");
    let body = match target.system() {
        Some(crate::System::Windows) => windows(exe, target, app, es),
        Some(crate::System::MacOs) => macos(name, exe, target, app, es),
        Some(crate::System::Linux) => linux(exe, app, es),
        None => web(es),
    };
    out.push_str(&body);
    out.push('\n');
    out.push_str(&t(
        "THIRD-PARTY-NOTICES.txt lists the open-source software inside this app and\ntheir licences; keep it with the app when you share it.\n\nMade with RapidR.\n",
        "THIRD-PARTY-NOTICES.txt lista el software de código abierto que esta app\nlleva dentro y sus licencias; consérvalo junto a la app al compartirla.\n\nHecha con RapidR.\n",
    ));
    out.replace('\n', if target.system() == Some(crate::System::Windows) { "\r\n" } else { "\n" })
}

fn label_es(target: Target) -> &'static str {
    match target {
        Target::WindowsX64 => "Windows (64 bits, x64)",
        Target::WindowsArm64 => "Windows (ARM64)",
        Target::WindowsX86 => "Windows (32 bits, x86)",
        Target::MacosUniversal => "macOS (Apple silicon e Intel)",
        Target::MacosArm64 => "macOS (Apple silicon)",
        Target::MacosX64 => "macOS (Intel)",
        Target::LinuxX64 => "Linux (x64)",
        Target::LinuxArm64 => "Linux (ARM64)",
        Target::Web => "la web",
    }
}

fn windows(exe: &str, target: Target, app: bool, es: bool) -> String {
    let runs_on = match (target, es) {
        (Target::WindowsX86, false) => "It runs on Windows 7, 8, 8.1, 10 and 11, 32-bit and 64-bit.\n\n",
        (Target::WindowsX86, true) => "Funciona en Windows 7, 8, 8.1, 10 y 11, de 32 y de 64 bits.\n\n",
        (Target::WindowsArm64, false) => "It is for Windows PCs with an ARM processor (Snapdragon and others).\n\n",
        (Target::WindowsArm64, true) => "Es para PC con Windows y procesador ARM (Snapdragon y otros).\n\n",
        (_, false) => "It runs on 64-bit Windows 10 and 11 (most PCs).\n\n",
        (_, true) => "Funciona en Windows 10 y 11 de 64 bits (la mayoría de los PC).\n\n",
    };
    let mut s = runs_on.to_string();
    if es {
        s.push_str("1. Descomprime este archivo: clic derecho sobre el .zip, \"Extraer todo...\",\n   \"Extraer\".\n");
        if app {
            s.push_str(&format!("2. Abre la carpeta y haz doble clic en {exe}.exe.\n"));
        } else {
            s.push_str(&format!(
                "2. Es un programa de consola: en la carpeta, escribe cmd en la barra de\n   direcciones y pulsa Intro; en la ventana que se abre, escribe {exe} y\n   pulsa Intro.\n"
            ));
        }
        s.push_str(
            "\nLa primera vez Windows puede mostrar \"Windows protegió su PC\" (SmartScreen),\nporque la app es nueva y no está firmada con un certificado. Si confías en\nquien te la dio, haz clic en \"Más información\" y luego en \"Ejecutar de todas\nformas\". Windows lo recuerda.\n\nMantén juntos los archivos de esta carpeta: la app lee los suyos de al lado\nde su .exe.\n",
        );
    } else {
        s.push_str("1. Unzip this file: right-click the .zip, \"Extract All...\", \"Extract\".\n");
        if app {
            s.push_str(&format!("2. Open the folder and double-click {exe}.exe.\n"));
        } else {
            s.push_str(&format!(
                "2. It is a console program: in the folder, type cmd in the address bar and\n   press Enter; in the window that opens, type {exe} and press Enter.\n"
            ));
        }
        s.push_str(
            "\nThe first time, Windows may show \"Windows protected your PC\" (SmartScreen),\nbecause the app is new and not signed with a certificate. If you trust\nwhoever gave it to you, click \"More info\", then \"Run anyway\". Windows\nremembers it.\n\nKeep the files of this folder together: the app reads its own from beside\nits .exe.\n",
        );
    }
    s
}

fn macos(name: &str, exe: &str, target: Target, app: bool, es: bool) -> String {
    let runs_on = match (target, es) {
        (Target::MacosArm64, false) => "It runs on Macs with Apple silicon (macOS 11 or later).\n\n",
        (Target::MacosArm64, true) => "Funciona en Mac con Apple silicon (macOS 11 o posterior).\n\n",
        (Target::MacosX64, false) => "It runs on Intel Macs (macOS 10.13 or later).\n\n",
        (Target::MacosX64, true) => "Funciona en Mac con Intel (macOS 10.13 o posterior).\n\n",
        (_, false) => "It runs on every Mac: Apple silicon (macOS 11 or later) and Intel\n(macOS 10.13 or later).\n\n",
        (_, true) => "Funciona en todos los Mac: Apple silicon (macOS 11 o posterior) e Intel\n(macOS 10.13 o posterior).\n\n",
    };
    let mut s = runs_on.to_string();
    if es {
        s.push_str("1. Haz doble clic en el .zip para descomprimirlo (puede que tu Mac ya lo\n   haya hecho).\n");
        if app {
            s.push_str(&format!("2. Arrastra {name}.app a tu carpeta Aplicaciones (o déjala donde quieras).\n3. Haz doble clic en ella.\n"));
            s.push_str(&format!(
                "\nLa primera vez, macOS dice que no abrió {name}: Apple no pudo verificar\nque no tenga software malicioso, porque la app no está firmada con un\nDeveloper ID de Apple. Si confías en quien te la dio:\n  1. Haz clic en \"Listo\" (no en \"Mover a la papelera\").\n  2. Abre Configuración del Sistema > Privacidad y seguridad, baja hasta el\n     aviso sobre {name} y haz clic en \"Abrir de todos modos\"; confirma con\n     tu contraseña o Touch ID, y haz clic en \"Abrir\". macOS lo recuerda.\n  (En macOS 14 y anteriores también sirve: clic derecho sobre la app,\n  \"Abrir\", y otra vez \"Abrir\".)\n"
            ));
        } else {
            s.push_str(&format!(
                "2. Es un programa de consola: abre Terminal en la carpeta (clic derecho\n   sobre ella, Servicios > Nuevo Terminal en la carpeta) y escribe ./{exe}\n\nSi macOS no lo deja abrir la primera vez, ve a Configuración del Sistema >\nPrivacidad y seguridad y haz clic en \"Abrir de todos modos\".\n"
            ));
        }
    } else {
        s.push_str("1. Double-click the .zip to unzip it (your Mac may have done it already).\n");
        if app {
            s.push_str(&format!("2. Drag {name}.app to your Applications folder (or keep it anywhere).\n3. Double-click it.\n"));
            s.push_str(&format!(
                "\nThe first time, macOS says \"{name}\" Not Opened: \"Apple could not verify\n{name} is free of malware...\", because the app is not signed with an Apple\nDeveloper ID. If you trust whoever gave it to you:\n  1. Click \"Done\" (not \"Move to Trash\").\n  2. Open System Settings > Privacy & Security, scroll down to the message\n     about {name} and click \"Open Anyway\"; confirm with your password or\n     Touch ID, then click \"Open\". macOS remembers it.\n  (On macOS 14 and earlier this works too: right-click the app, choose\n  \"Open\", then \"Open\" again.)\n"
            ));
        } else {
            s.push_str(&format!(
                "2. It is a console program: open Terminal in the folder (right-click it,\n   Services > New Terminal at Folder) and type ./{exe}\n\nIf macOS won't open it the first time, go to System Settings > Privacy &\nSecurity and click \"Open Anyway\".\n"
            ));
        }
    }
    s
}

fn linux(exe: &str, app: bool, es: bool) -> String {
    match (es, app) {
        (false, true) => format!(
            "1. Unpack it: right-click the .tar.gz, \"Extract Here\" (or: tar xzf <file>).\n2. Run it from its folder: ./AppRun\n   or put it in your applications menu, with its icon: ./install.sh\n   (no root needed; ./install.sh --uninstall removes it again).\n\nIf the files lost their \"executable\" mark (some tools do), run:\n   chmod +x AppRun install.sh usr/bin/{exe}\n"
        ),
        (false, false) => format!(
            "1. Unpack it: tar xzf <file>\n2. It is a console program: in a terminal in its folder, run ./{exe}\n\nIf it lost its \"executable\" mark (some tools do), run: chmod +x {exe}\n"
        ),
        (true, true) => format!(
            "1. Descomprímelo: clic derecho sobre el .tar.gz, \"Extraer aquí\" (o: tar xzf <archivo>).\n2. Ejecútala desde su carpeta: ./AppRun\n   o ponla en tu menú de aplicaciones, con su icono: ./install.sh\n   (no hace falta ser root; ./install.sh --uninstall la quita).\n\nSi los archivos perdieron la marca de \"ejecutable\" (algunas herramientas lo\nhacen), ejecuta:\n   chmod +x AppRun install.sh usr/bin/{exe}\n"
        ),
        (true, false) => format!(
            "1. Descomprímelo: tar xzf <archivo>\n2. Es un programa de consola: en una terminal en su carpeta, ejecuta ./{exe}\n\nSi perdió la marca de \"ejecutable\" (algunas herramientas lo hacen),\nejecuta: chmod +x {exe}\n"
        ),
    }
}

fn web(es: bool) -> String {
    if es {
        "Esta carpeta es una app web: una página que ejecuta la app en el navegador.\n\n1. Sube todos los archivos de esta carpeta, con sus carpetas (fonts/, icons/),\n   a cualquier alojamiento web de archivos estáticos.\n2. Abre su dirección en el navegador.\n\nAbrir index.html directamente desde el disco no funciona: los navegadores\nsolo la ejecutan servida por un servidor web. Para probarla en tu\nordenador, sírvela con un servidor web local.\nLos navegadores que instalan apps web muestran un botón \"Instalar\" en la\nbarra de direcciones.\n".into()
    } else {
        "This folder is a web app: a page that runs the app in the browser.\n\n1. Upload every file of this folder, with its folders (fonts/, icons/), to\n   any static web host.\n2. Open its address in a browser.\n\nOpening index.html straight from the disk doesn't work: browsers only run it\nwhen a web server serves it. To try it on your computer, serve the folder\nwith a local web server.\nBrowsers that install web apps show an \"Install\" button in the address bar.\n".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_target_has_its_first_launch_help() {
        let mut info = AppInfo::new("Tip Calculator", "Tip-Calculator");
        info.description = "Splits the bill".into();
        info.website = "https://example.com".into();
        info.company = "Example Ltd".into();
        for t in Target::ALL {
            for lang in LANGUAGES {
                for app in [true, false] {
                    let r = readme(&info, t, lang, app);
                    assert!(r.starts_with("Tip Calculator 1.0.0"), "{t} {lang}");
                    assert!(r.contains("https://example.com") && r.contains("© Example Ltd") && r.contains("THIRD-PARTY-NOTICES.txt"), "{t} {lang}");
                    let windows = t.system() == Some(crate::System::Windows);
                    assert_eq!(r.contains("\r\n"), windows, "{t}: Windows' line ends on Windows only");
                }
            }
        }
        assert!(readme(&info, Target::WindowsX64, "en", true).contains("Run anyway"));
        assert!(readme(&info, Target::WindowsX64, "es", true).contains("Ejecutar de todas"));
        let mac = readme(&info, Target::MacosUniversal, "en", true);
        assert!(mac.contains("\"Tip Calculator\" Not Opened") && mac.contains("Click \"Done\" (not \"Move to Trash\")") && mac.contains("Open Anyway"), "{mac}");
        assert!(readme(&info, Target::LinuxX64, "en", true).contains("./install.sh"));
        assert!(readme(&info, Target::WindowsX86, "en", true).contains("Windows 7"));
        assert!(readme(&info, Target::Web, "es", true).contains("alojamiento web"));
        assert!(readme(&info, Target::LinuxArm64, "en", false).contains("./Tip-Calculator"));
    }
}
