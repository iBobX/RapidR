use super::*;

/// Every component type the compilers know has an icon, under its RapidR
/// name and under its RapidQ name (QBUTTON, QGAUGE, QOUTLINE, COMPORT).
/// (When rapidr-lang lands, this walks its registry instead.)
#[test]
fn every_component_type_has_an_icon() {
    let mut missing = Vec::new();
    for t in rapidr_ast::COMPONENT_TYPES {
        if component(t).is_none() {
            missing.push(t.to_string());
        }
        let q = format!("Q{}", &t[1..]);
        if rapidr_ast::canonical_type_name(&q) == *t && component(&q).is_none() {
            missing.push(q);
        }
    }
    for q in ["QGAUGE", "QOUTLINE", "COMPORT", "qbutton"] {
        if component(q).is_none() {
            missing.push(q.into());
        }
        assert_eq!(canonical_component(q), rapidr_ast::canonical_type_name(q).to_ascii_uppercase(), "{q}");
    }
    assert!(missing.is_empty(), "component types without an icon: {missing:?} (design/icons/inventory.toml)");
    // (and the table names nothing the compilers don't know, except the planned)
    for (t, _, planned) in COMPONENTS {
        assert!(*planned || rapidr_ast::COMPONENT_TYPES.contains(t), "{t} isn't a component type: mark it planned");
        assert!(!*planned || !rapidr_ast::COMPONENT_TYPES.contains(t), "{t} exists now: move it out of [planned-components]");
    }
}

/// Every command, marker, file kind, project kind, symbol and toolbox group
/// names an icon that exists.
#[test]
fn every_command_and_kind_has_an_icon() {
    let mut missing = Vec::new();
    for (table, what) in [(COMMANDS, "command"), (MARKERS, "marker"), (FILES, "file"), (PROJECT_KINDS, "project kind"), (SYMBOLS, "symbol")] {
        for (k, id) in table {
            if get(id).is_none() {
                missing.push(format!("{what} {k} -> {id}"));
            }
        }
    }
    for g in TOOLBOX_GROUPS {
        if get(g.icon).is_none() {
            missing.push(format!("toolbox group {}", g.id));
        }
    }
    assert!(missing.is_empty(), "no icon for: {missing:?}");
    for c in ["file.save", "run.start", "debug.stepOver", "designer.alignLeft", "view.dock"] {
        assert!(command(c).is_some(), "{c}");
    }
}

/// Every component is in exactly one toolbox group.
#[test]
fn toolbox_groups_hold_every_component_once() {
    for (t, _, _) in COMPONENTS {
        let n = TOOLBOX_GROUPS.iter().filter(|g| g.members.contains(t)).count();
        assert_eq!(n, 1, "{t} is in {n} toolbox groups");
    }
}

#[test]
fn lookups() {
    assert_eq!(get("run").unwrap().id, "actions/run");
    assert_eq!(get("actions/RUN").unwrap().id, "actions/run");
    assert_eq!(get("QBUTTON").unwrap().id, "components/button");
    assert_eq!(get("rbutton").unwrap().id, "components/button");
    assert_eq!(get("button").unwrap().id, "components/button");
    assert_eq!(get("edit").unwrap().id, "actions/edit");
    assert_eq!(get("REDIT").unwrap().id, "components/edit");
    assert_eq!(get("QGAUGE").unwrap().id, "components/progressbar");
    assert_eq!(get("file.save").unwrap().id, "actions/save");
    assert_eq!(file("src/Main.RR").id, "files/rr");
    assert_eq!(file("C:\\x\\prog.bas").id, "files/bas");
    assert_eq!(file("README").id, "files/file");
    assert_eq!(file("a.unknown").id, "files/file");
    assert!(get("no-such-icon").is_none());
    assert!(all().windows(2).all(|w| w[0].id < w[1].id), "sorted by id");
}

/// Each source is one standalone SVG per size that resvg reads, with the
/// size's own view box; monochrome icons carry no token colour.
#[test]
fn sources_are_sound() {
    for i in all() {
        for (n, &s) in SIZES.iter().enumerate() {
            let svg = i.svg[n];
            assert!(svg.starts_with(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{s}\" height=\"{s}\" viewBox=\"0 0 {s} {s}\"")), "{} @{s}", i.id);
            assert!(resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).is_ok(), "{} @{s} doesn't parse", i.id);
            if i.mono {
                for key in generated::KEYS {
                    assert!(!svg.contains(key), "{} is monochrome but uses {key}", i.id);
                }
            }
        }
    }
    // (token values are swapped by text: each must spell differently in
    // lower case, so a swapped value can't be swapped again)
    for key in generated::KEYS {
        assert!(key.chars().any(|c| c.is_ascii_uppercase()), "{key}");
    }
}

/// Every icon draws something at every size and scale, in every theme.
#[test]
fn renders_everywhere() {
    for p in palettes() {
        let style = Style { palette: p, color: None, disabled: false };
        for i in all() {
            for (logical, scale) in [(16, 1.0), (16, 1.5), (16, 2.0), (24, 1.0), (32, 1.0)] {
                let px = render(i, logical, scale, &style).unwrap_or_else(|| panic!("{} didn't render", i.id));
                let device = (logical as f32 * scale).round() as u32;
                assert_eq!((px.width, px.height), (device, device));
                assert!(px.data.chunks(4).any(|c| c[3] > 0), "{} @{logical}x{scale} {} is empty", i.id, p.theme);
            }
        }
    }
}

/// The variant picked lands on the pixel grid: the exact size, a whole
/// multiple, the nearest otherwise.
#[test]
fn variants() {
    assert_eq!(variant_for(16), (16, 1.0));
    assert_eq!(variant_for(24), (24, 1.0));
    assert_eq!(variant_for(32), (32, 1.0));
    assert_eq!(variant_for(48), (24, 2.0));
    assert_eq!(variant_for(64), (32, 2.0));
    assert_eq!(variant_for(12).0, 16);
    assert_eq!(variant_for(36).0, 32);
}

/// Hinting: at 16 px most inked pixels are fully opaque, and more of them
/// than in the 24 px
/// master shrunk.
#[test]
fn small_sizes_are_crisp() {
    let style = Style::new("modern");
    let solid = |px: &Rgba| {
        let inked: Vec<u8> = px.data.chunks(4).map(|c| c[3]).filter(|&a| a > 8).collect();
        inked.iter().filter(|&&a| a >= 250).count() as f64 / inked.len() as f64
    };
    for id in ["actions/save", "actions/new-file", "actions/align-left", "actions/split-vertical", "actions/stop", "components/button", "components/stringgrid"] {
        let i = get(id).unwrap();
        let hinted = solid(&render(i, 16, 1.0, &style).unwrap());
        let shrunk = solid(&render_svg(&themed_svg(i, 24, &style), 16, 16.0 / 24.0).unwrap());
        assert!(hinted >= 0.6 && hinted >= shrunk, "{id}: {:.0} % solid hinted, {:.0} % shrunk", hinted * 100.0, shrunk * 100.0);
    }
}

/// Theme colours: every hue's outline has 3:1 against the theme's
/// backgrounds, its paper and its own tint (WCAG 1.4.11, graphics).
#[test]
fn contrast_in_every_theme() {
    for p in palettes() {
        for hue in ["ink", "blue", "cyan", "teal", "amber", "red", "violet"] {
            let c = p.token(hue).unwrap();
            let mut against: Vec<u32> = p.backgrounds.to_vec();
            against.push(p.token("paper").unwrap());
            if let Some(t) = p.token(&format!("{hue}-tint")) {
                against.push(t);
            }
            for b in against {
                assert!(contrast(c, b) >= 3.0, "{} {hue} #{c:06X} on #{b:06X}: {:.2}", p.theme, contrast(c, b));
            }
        }
        for &b in p.backgrounds {
            assert!(contrast(p.fg, b) >= 4.5, "{}: currentColor on #{b:06X}", p.theme);
            assert!(contrast(p.disabled, b) >= 2.0 || p.theme == "highcontrast", "{}: disabled ink on #{b:06X}", p.theme);
        }
    }
}

#[test]
fn themed_svgs() {
    let run = get("run").unwrap();
    let dark = themed_svg(run, 24, &Style::new("dark"));
    assert!(dark.contains("#34cf98") && !dark.contains("#08805E"), "{dark}");
    let hc = themed_svg(run, 24, &Style::new("highcontrast"));
    assert!(hc.contains("#ffffff"));
    let save = get("save").unwrap();
    let blue = themed_svg(save, 16, &Style { color: Some(0x0000FF), ..Style::new("modern") });
    assert!(blue.contains("color=\"#0000FF\""));
    let off = themed_svg(run, 16, &Style { disabled: true, ..Style::new("modern") });
    assert!(off.contains("#8d93a0") && !off.contains("#08805E"));
}
