//! Shared CSS for all RapidR web pages.
//!
//! Single source of truth for compiled-mode (`rapidr-cli`'s
//! `generate_html_shell`), interpreted-mode (`rapidr-webbundle`'s
//! `render_index_html`), and the runtime-web crate.

/// Base stylesheet for `.rr-form` / `.rr-widget` elements.
pub const RR_BASE_CSS: &str = r#"
* { box-sizing: border-box; }
body { margin: 0; padding: 0; background: #e8e8e8; font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; font-size: 13px; overflow: auto; }
.rr-form { position: absolute; background: #f0f0f0; border: 1px solid #888; border-radius: 6px; box-shadow: 0 4px 16px rgba(0,0,0,0.18); overflow: hidden; }
.rr-form-titlebar { background: linear-gradient(135deg, #4a90d9, #357abd); color: white; padding: 7px 12px; font-weight: 600; font-size: 13px; user-select: none; cursor: default; letter-spacing: 0.3px; }
.rr-widget { position: absolute; box-sizing: border-box; }
button.rr-widget { background: linear-gradient(to bottom, #4a90d9, #3a7bc8); color: white; border: 1px solid #2d6db5; border-radius: 4px; padding: 4px 14px; font-size: 13px; cursor: pointer; font-family: inherit; transition: background 0.15s; }
button.rr-widget:hover { background: linear-gradient(to bottom, #5a9ee9, #4a8bd8); }
button.rr-widget:active { background: linear-gradient(to bottom, #2d6db5, #3a7bc8); }
input[type="text"].rr-widget, input[type="password"].rr-widget { border: 1px solid #aaa; border-radius: 3px; padding: 4px 8px; font-size: 13px; font-family: inherit; outline: none; background: white; }
input[type="text"].rr-widget:focus, input[type="password"].rr-widget:focus { border-color: #4a90d9; box-shadow: 0 0 3px rgba(74,144,217,0.4); }
textarea.rr-widget { border: 1px solid #aaa; border-radius: 3px; padding: 6px 8px; font-size: 13px; font-family: inherit; outline: none; resize: none; background: white; }
textarea.rr-widget:focus { border-color: #4a90d9; box-shadow: 0 0 3px rgba(74,144,217,0.4); }
select.rr-widget { border: 1px solid #aaa; border-radius: 3px; padding: 4px 8px; font-size: 13px; font-family: inherit; background: white; cursor: pointer; }
progress.rr-widget { border: none; border-radius: 3px; height: 22px; appearance: none; -webkit-appearance: none; }
progress.rr-widget::-webkit-progress-bar { background: #ddd; border-radius: 3px; }
progress.rr-widget::-webkit-progress-value { background: linear-gradient(to right, #4caf50, #45a049); border-radius: 3px; }
table.rr-grid { border-collapse: collapse; width: 100%; font-size: 13px; }
table.rr-grid th, table.rr-grid td { border: 1px solid #ccc; padding: 5px 10px; text-align: left; }
table.rr-grid th { background: #e0e0e0; font-weight: 600; position: sticky; top: 0; }
table.rr-grid tr:nth-child(even) { background: #f8f8f8; }
table.rr-grid tr:hover { background: #e8f0fe; }
.rr-tab-btn { padding: 6px 16px; cursor: pointer; border: none; background: transparent; font-size: 13px; font-family: inherit; border-bottom: 2px solid transparent; color: #555; transition: all 0.15s; }
.rr-tab-btn:hover { color: #333; background: #e8e8e8; }
.rr-tab-btn.active { color: #4a90d9; border-bottom-color: #4a90d9; font-weight: 600; }
canvas.rr-widget { border: 1px solid #aaa; background: white; cursor: crosshair; }
label.rr-widget { font-size: 13px; display: flex; align-items: center; gap: 4px; cursor: pointer; }
fieldset.rr-widget { border: 1px solid #aaa; border-radius: 4px; padding: 8px; }
fieldset.rr-widget legend { font-size: 13px; padding: 0 4px; }
.rr-plot-container { border: 1px solid #aaa; border-radius: 3px; background: white; overflow: hidden; }
#rr-console { position: fixed; bottom: 0; left: 0; width: 100%; max-height: 200px; overflow-y: auto; background: #1e1e1e; color: #d4d4d4; font-family: 'Consolas', 'Monaco', monospace; font-size: 13px; padding: 8px; display: none; z-index: 10000; border-top: 2px solid #333; }
"#;
