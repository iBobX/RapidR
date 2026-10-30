// (moved here from rapidr-value's i18n with the parser)
use super::po::*;
use super::*;

    #[test]
    fn plural_rules() {
        let en = Plural::default();
        assert_eq!((en.index(0), en.index(1), en.index(2)), (1, 0, 1));
        let es = Plural::parse(" nplurals=2; plural=(n != 1);").unwrap();
        assert_eq!((es.index(1), es.index(5)), (0, 1));
        let zh = Plural::parse("nplurals=1; plural=0;").unwrap();
        assert_eq!((zh.index(1), zh.index(7)), (0, 0));
        // Polish: 1, 2-4 (not 12-14), the rest
        let pl = Plural::parse("nplurals=3; plural=(n==1 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);").unwrap();
        let forms: Vec<usize> = [1, 2, 4, 5, 12, 14, 22, 25, 101, 102].iter().map(|&n| pl.index(n)).collect();
        assert_eq!(forms, [0, 1, 1, 2, 2, 2, 1, 2, 2, 1]);
        // Arabic: six forms, nested conditions
        let ar = Plural::parse("nplurals=6; plural=n==0 ? 0 : n==1 ? 1 : n==2 ? 2 : n%100>=3 && n%100<=10 ? 3 : n%100>=11 ? 4 : 5;").unwrap();
        let forms: Vec<usize> = [0, 1, 2, 3, 11, 100].iter().map(|&n| ar.index(n)).collect();
        assert_eq!(forms, [0, 1, 2, 3, 4, 5]);
        // French: 0 and 1 singular
        let fr = Plural::parse("nplurals=2; plural=(n > 1);").unwrap();
        assert_eq!((fr.index(0), fr.index(1), fr.index(2)), (0, 0, 1));
        // `!`, arithmetic, division by zero, an index past the last form
        let odd = Plural::parse("nplurals=2; plural=!(n - n) * 7 + n / 0;").unwrap();
        assert_eq!(odd.index(3), 1);
        // not a rule
        for bad in ["nplurals=2;", "plural=n!=1;", "nplurals=0; plural=0;", "nplurals=2; plural=n !=;", "nplurals=2; plural=(n;", "nplurals=2; plural=x;"] {
            assert_eq!(Plural::parse(bad), None, "{bad}");
        }
        let deep = format!("nplurals=2; plural={}n{};", "(".repeat(500), ")".repeat(500));
        assert_eq!(Plural::parse(&deep), None);
    }

    const ES: &str = r#"# Spanish
msgid ""
msgstr ""
"Content-Type: text/plain; charset=UTF-8\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

#: main.bas:3
msgid "Save file"
msgstr "Guardar archivo"

msgctxt "menu"
msgid "Open"
msgstr "Abrir"

msgid "Open"
msgstr "Abierto"

msgid "{n} file"
msgid_plural "{n} files"
msgstr[0] "{n} archivo"
msgstr[1] "{n} archivos"

msgid ""
"Two "
"lines"
msgstr "Dos\n"
"líneas \"citadas\"\t\\"

#, fuzzy
msgid "Draft"
msgstr "Borrador"

msgid "Untranslated"
msgstr ""

#~ msgid "Old"
#~ msgstr "Viejo"
"#;

    #[test]
    fn reads_a_po_file() {
        let c = parse_po(ES);
        assert_eq!(c.plural.nplurals, 2);
        assert_eq!(c.text("", "Save file"), Some("Guardar archivo"));
        assert_eq!(c.text("menu", "Open"), Some("Abrir"));
        assert_eq!(c.text("", "Open"), Some("Abierto"));
        assert_eq!(c.plural_text("", "{n} file", 1), Some("{n} archivo"));
        assert_eq!(c.plural_text("", "{n} file", 5), Some("{n} archivos"));
        assert_eq!(c.text("", "Two lines"), Some("Dos\nlíneas \"citadas\"\t\\"));
        assert_eq!(c.text("", "Draft"), None, "fuzzy entries are not used");
        assert_eq!(c.text("", "Untranslated"), None);
        assert_eq!(c.text("", "Old"), None);
        assert_eq!(c.len(), 5);
    }

    #[test]
    fn escapes() {
        assert_eq!(parse_string(r#""a\nb\tc\"d\\e\x41\101\q""#).as_deref(), Some("a\nb\tc\"d\\eAAq"));
        assert_eq!(parse_string(r#""ok" # note"#).as_deref(), Some("ok"));
        assert_eq!(parse_string(r#""open"#), None);
        assert_eq!(parse_string(r#""x" y"#), None);
        assert_eq!(parse_string(r#""\x""#), None);
    }

    #[test]
    fn broken_files_keep_their_good_entries() {
        let broken = "garbage line\nmsgid \"Good\"\nmsgstr \"Bueno\"\n%%%\n\"stray\"\nmsgid \"Bad\nmsgstr \"never\"\nmsgid \"Fine\"\nmsgstr \"Bien\"\nmsgstr[99] \"x\"\nmsgid \"Half\"\n\u{0}\u{ff}\nmsgid \"Also\"\nmsgstr \"Tambi\u{e9}n\"\n\"";
        let c = parse_po(broken);
        assert_eq!(c.text("", "Good"), Some("Bueno"));
        assert_eq!(c.text("", "Fine"), Some("Bien"));
        assert_eq!(c.text("", "Also"), Some("También"));
        assert_eq!(c.text("", "Half"), None);
        // nothing at all, only noise, a BOM, Windows line ends
        assert!(parse_po("").is_empty());
        assert!(parse_po("\"\"\"\nmsgstr[\nmsgid\n#,").is_empty());
        let c = parse_po("\u{FEFF}msgid \"A\"\r\nmsgstr \"B\"\r\n");
        assert_eq!(c.text("", "A"), Some("B"));
        // a header without plural forms: English's rule
        let c = parse_po("msgid \"\"\nmsgstr \"Language: es\\n\"\n");
        assert_eq!(c.plural, Plural::default());
    }

    #[test]
    fn plural_forms_past_the_rule_take_the_last() {
        let c = parse_po("msgid \"\"\nmsgstr \"Plural-Forms: nplurals=3; plural=n;\\n\"\nmsgid \"a\"\nmsgid_plural \"as\"\nmsgstr[0] \"zero\"\nmsgstr[1] \"one\"\n");
        assert_eq!(c.plural_text("", "a", 0), Some("zero"));
        assert_eq!(c.plural_text("", "a", 1), Some("one"));
        assert_eq!(c.plural_text("", "a", 9), Some("one"));
    }

    #[test]
    fn tags_and_fallback() {
        assert_eq!(normalize_tag("es_AR.UTF-8@euro"), "es-AR");
        assert_eq!(normalize_tag("ES-ar"), "es-AR");
        assert_eq!(normalize_tag("zh_hans_cn"), "zh-Hans-CN");
        assert_eq!(normalize_tag("es-419"), "es-419");
        assert_eq!(normalize_tag("C"), "");
        assert_eq!(normalize_tag("POSIX"), "");
        assert_eq!(normalize_tag("  "), "");
        assert_eq!(fallback_chain("es-AR"), ["es-AR", "es"]);
        assert_eq!(fallback_chain("zh-Hans-CN"), ["zh-Hans-CN", "zh-Hans", "zh"]);
        assert_eq!(fallback_chain("zh_CN"), ["zh-CN", "zh-Hans", "zh"]);
        assert_eq!(fallback_chain("zh-TW"), ["zh-TW", "zh-Hant", "zh"]);
        assert_eq!(fallback_chain("pt"), ["pt"]);
        assert!(fallback_chain("").is_empty());
    }

    #[test]
    fn placeholders() {
        let v = |n: &str| match n.to_ascii_lowercase().as_str() {
            "name" => Some("Ana".to_string()),
            "n" => Some("5".to_string()),
            _ => None,
        };
        assert_eq!(fill("Hola, {name}!", v), "Hola, Ana!");
        assert_eq!(fill("{n} de {n}", v), "5 de 5");
        assert_eq!(fill("{{name}} {Name} {other} {", v), "{name} Ana {other} {");
        assert_eq!(fill("a } b }} c {x{name}", v), "a } b } c {xAna");
        assert_eq!(fill("no braces", v), "no braces");
        assert_eq!(fill("{ name }", v), "Ana");
    }
