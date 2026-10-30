// The Help page's Markdown as HTML (src/markdown.js).
'use strict';

const assert = require('assert');
const { renderMarkdown } = require('../../src/markdown');

describe('renderMarkdown', () => {
    it('renders the page of a component: heading, text, a code block, link lists', () => {
        const html = renderMarkdown([
            '## RButton',
            '',
            'Component · desktop and web',
            '',
            'Its default event is [OnClick](help:OnClick@RButton).',
            '',
            '```basic',
            'CREATE Button1 AS RButton',
            '',
            'END CREATE',
            '```',
            '',
            '[Insert into the code](help-insert:)',
            '',
            '### Properties (3)',
            '',
            '[Caption](help:Caption@RButton) · [Color](help:Color@RButton)',
        ].join('\n'));
        assert.match(html, /<h2>RButton<\/h2>/);
        assert.match(html, /<h3>Properties \(3\)<\/h3>/);
        assert.match(html, /<a href="#" data-link="help:OnClick@RButton">OnClick<\/a>/);
        assert.match(html, /<a href="#" data-link="help-insert:">Insert into the code<\/a>/);
        assert.match(html, /<pre><code>CREATE Button1 AS RButton\n\nEND CREATE<\/code><\/pre>/);
        assert.match(html, /<a [^>]*>Caption<\/a> · <a [^>]*>Color<\/a>/);
    });

    it('renders lists, bold, inline code and the escapes `plain` writes', () => {
        const html = renderMarkdown('- [RForm](help:RForm): a window\n- [PRINT](help:PRINT): show text\n\nPut the caret on a word and press **F1**; `DIM x AS INTEGER` \\<b\\> \\* \\_');
        assert.match(html, /<ul><li><a [^>]*>RForm<\/a>: a window<\/li><li>/);
        assert.match(html, /<strong>F1<\/strong>/);
        assert.match(html, /<code>DIM x AS INTEGER<\/code>/);
        assert.match(html, /&lt;b&gt; \* _/, 'escaped characters are shown as written');
    });

    it('escapes everything: a page can\'t inject markup, and only help: and https: are links', () => {
        const html = renderMarkdown('<script>alert(1)</script> [x](javascript:alert(1)) [y](file:///etc/passwd) [ok](https://example.com) [h](help:Word%20Two)\n\n```\n<img src=x onerror=1>\n```');
        assert.ok(!/<script|<img|javascript:|file:/.test(html), html);
        assert.match(html, /&lt;script&gt;/);
        assert.match(html, /data-link="https:\/\/example.com"/);
        assert.match(html, /data-link="help:Word%20Two"/);
        // a link with a scheme a page may not use is its text only
        assert.match(html, / x\)? y /);
    });

    it('keeps an unclosed backtick, bracket and asterisks as text', () => {
        assert.strictEqual(renderMarkdown('a ` b [ c ** d'), '<p>a ` b [ c ** d</p>');
    });
});
