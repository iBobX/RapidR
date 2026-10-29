// The Markdown of RapidR's Help pages as HTML for a webview — no VS Code API
// here (unit-tested in test/unit/markdown.test.js). It reads the subset the
// pages use (rapidr_langsvc::help): headings, paragraphs, bulleted lists,
// fenced code, and inline code, **bold** and links, with backslash escapes.
// Everything is escaped; a link keeps only the schemes a page may use
// (`help:` and its `help-insert:` / `help-goto:` / `help-refs:`, `https:`),
// as data the page's script reads: no navigation, nothing runs.
'use strict';

function escapeHtml(text) {
    return String(text).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

const LINK_SCHEMES = /^(help|help-insert|help-goto|help-refs|https?):/i;

/** Inline Markdown: `code`, **bold**, [text](target), \escapes. */
function inline(text) {
    let out = '';
    let i = 0;
    while (i < text.length) {
        const c = text[i];
        if (c === '\\' && i + 1 < text.length) {
            out += escapeHtml(text[i + 1]);
            i += 2;
        } else if (c === '`') {
            const end = text.indexOf('`', i + 1);
            if (end < 0) {
                out += '`';
                i++;
            } else {
                out += `<code>${escapeHtml(text.slice(i + 1, end))}</code>`;
                i = end + 1;
            }
        } else if (c === '*' && text[i + 1] === '*') {
            const end = text.indexOf('**', i + 2);
            if (end < 0) {
                out += '**';
                i += 2;
            } else {
                out += `<strong>${inline(text.slice(i + 2, end))}</strong>`;
                i = end + 2;
            }
        } else if (c === '[') {
            const m = /^\[((?:[^\]\\]|\\.)*)\]\(([^)\s]*)\)/.exec(text.slice(i));
            if (m && LINK_SCHEMES.test(m[2])) {
                out += `<a href="#" data-link="${escapeHtml(m[2])}">${inline(m[1])}</a>`;
                i += m[0].length;
            } else if (m) {
                // (a scheme a page may not use: its text, not a link)
                out += inline(m[1]);
                i += m[0].length;
            } else {
                out += '[';
                i++;
            }
        } else {
            out += escapeHtml(c);
            i++;
        }
    }
    return out;
}

/** Markdown -> HTML (the body of the page). */
function renderMarkdown(markdown) {
    const lines = String(markdown).replace(/\r\n?/g, '\n').split('\n');
    const out = [];
    let paragraph = [];
    let list = null;
    const flushParagraph = () => {
        if (paragraph.length) out.push(`<p>${inline(paragraph.join(' '))}</p>`);
        paragraph = [];
    };
    const flushList = () => {
        if (list) out.push(`<ul>${list.map((item) => `<li>${inline(item)}</li>`).join('')}</ul>`);
        list = null;
    };
    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const fence = /^```(\w*)\s*$/.exec(line);
        if (fence) {
            flushParagraph();
            flushList();
            const body = [];
            for (i++; i < lines.length && !/^```\s*$/.test(lines[i]); i++) body.push(lines[i]);
            out.push(`<pre><code>${escapeHtml(body.join('\n'))}</code></pre>`);
            continue;
        }
        const heading = /^(#{1,6})\s+(.*?)\s*#*\s*$/.exec(line);
        if (heading) {
            flushParagraph();
            flushList();
            const level = heading[1].length;
            out.push(`<h${level}>${inline(heading[2])}</h${level}>`);
            continue;
        }
        const item = /^\s*[-*]\s+(.*)$/.exec(line);
        if (item) {
            flushParagraph();
            (list = list || []).push(item[1]);
            continue;
        }
        if (line.trim() === '') {
            flushParagraph();
            flushList();
            continue;
        }
        flushList();
        paragraph.push(line.trim());
    }
    flushParagraph();
    flushList();
    return out.join('\n');
}

module.exports = { renderMarkdown, escapeHtml };
