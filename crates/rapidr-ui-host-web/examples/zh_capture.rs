//! A form with Chinese text, painted by the kernel and rasterized by the
//! desktop host's CPU path, saved as a PNG (the evidence for the CJK font
//! check, docs/legal/i18n-zh-font.md): the very pixels this computer's
//! system fonts give Chinese in RapidR's controls — labels, buttons, an
//! edit, a check box, a memo and a list — at 1× and 2×.
//!
//! `cargo run -p rapidr-ui-host-web --example zh_capture --release -- <dir>`
//! writes `<dir>/zh@1x.png` and `<dir>/zh@2x.png`.

use rapidr_ui_kernel::{FormUi, MemStore, TextSystem};
use rapidr_value::{v_int, v_str};

fn place(s: &mut MemStore, id: &str, (l, t, w, h): (i64, i64, i64, i64)) {
    s.set(id, "left", v_int(l)).set(id, "top", v_int(t));
    if w > 0 {
        s.set(id, "width", v_int(w));
    }
    if h > 0 {
        s.set(id, "height", v_int(h));
    }
}

fn build() -> MemStore {
    let mut s = MemStore::new();
    s.add("f", "RFORM", None).set("f", "caption", v_str("RapidR Studio 中文"));
    s.set("f", "clientwidth", v_int(480)).set("f", "clientheight", v_int(330));
    let label = |s: &mut MemStore, id: &str, text: &str, at: (i64, i64, i64, i64)| {
        s.add(id, "RLABEL", Some("f")).set(id, "caption", v_str(text));
        place(s, id, at);
    };
    label(&mut s, "l1", "文件  编辑  视图  项目  运行  调试  格式  工具  窗口  帮助", (10, 8, 460, 0));
    label(&mut s, "l2", "直 骨 角 刃 门 户 说 化 海 关：简体字形，不是日文字形", (10, 30, 460, 0));
    label(&mut s, "l3", "名称:", (10, 58, 40, 0));
    s.add("e", "REDIT", Some("f")).set("e", "text", v_str("你好，世界！Hello 123"));
    place(&mut s, "e", (56, 54, 200, 0));
    s.add("ok", "RBUTTON", Some("f")).set("ok", "caption", v_str("确定"));
    place(&mut s, "ok", (270, 54, 0, 0));
    s.add("cancel", "RBUTTON", Some("f")).set("cancel", "caption", v_str("取消(&C)"));
    place(&mut s, "cancel", (350, 54, 0, 0));
    s.add("c", "RCHECKBOX", Some("f")).set("c", "caption", v_str("启用 AI 助手(&A)"));
    place(&mut s, "c", (10, 88, 200, 0));
    s.add("m", "RMEMO", Some("f"));
    place(&mut s, "m", (10, 116, 300, 110));
    s.call(
        "m",
        "addstrings",
        &[v_str("第一行：保存、另存为、全部保存"), v_str("第二行：调用堆栈和断点"), v_str("第三行：RAI 想在你正在运行的程序中点击和输入。"), v_str("第四行：Studio 出了点问题。你的工作是安全的。")]
            .to_vec(),
    );
    s.add("list", "RLISTBOX", Some("f"));
    place(&mut s, "list", (322, 116, 148, 110));
    s.call("list", "additems", &["项目", "属性", "工具箱", "输出", "问题", "即时", "监视", "断点"].map(v_str).to_vec());
    label(&mut s, "l4", "Studio 在每次保存此文件时保留的副本，不会进入版本。这一行很长，用来检查自动换行时，一行可以在两个汉字之间结束。", (10, 240, 460, 0));
    s.set("l4", "autosize", v_int(0)).set("l4", "wordwrap", v_int(-1)).set("l4", "height", v_int(40));
    s
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
    }
    !c
}

/// A PNG of `width` × `height` pixels (`pixels` as &HBBGGRR): stored (uncompressed) deflate blocks.
fn png(width: usize, height: usize, pixels: &[u32]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width * 3 + 1) * height);
    for row in pixels.chunks(width) {
        raw.push(0);
        for p in row {
            raw.extend_from_slice(&[(*p & 0xFF) as u8, (*p >> 8 & 0xFF) as u8, (*p >> 16 & 0xFF) as u8]);
        }
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(u8::from(last));
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "target/zh-capture".into());
    std::fs::create_dir_all(&dir).expect("output directory");
    // (a Chinese UI: the system's own Simplified Chinese fonts come first)
    rapidr_value::i18n::set_language(&std::env::var("ZH_CAPTURE_LANG").unwrap_or_else(|_| "zh-Hans".into()));
    let mut ts = TextSystem::new();
    for scale in [1.0, 2.0] {
        rapidr_value::objects::bitmap::set_display_scale(scale);
        let store = build();
        let mut ui = FormUi::build(&store, "f", true);
        ui.blinks = false;
        let list = ui.paint(&store, &mut ts, scale);
        let px = rapidr_ui_render::cpu::capture(&list, &mut ts, &ui);
        let file = format!("{dir}/zh@{scale}x.png");
        std::fs::write(&file, png(px.width as usize, px.height as usize, &px.pixels)).expect("write the capture");
        println!("{file}: {}x{}", px.width, px.height);
    }
}
