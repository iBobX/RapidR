// Complete RapidR language data — extracted from compiler/codegen.py COMPONENT_REGISTRY and builtins.

const COMPONENT_REGISTRY = {
    'RFORM': {
        props: ['caption', 'width', 'height', 'top', 'left', 'visible', 'color', 'borderstyle',
                'windowstate', 'formstyle', 'center', 'autosize', 'font', 'fontsize', 'fontcolor',
                'icon', 'alphablend', 'alphablendvalue', 'parent', 'tag', 'cursor', 'enabled',
                'hint', 'showhint', 'popupmenu', 'helpfile', 'helpcontext'],
        methods: ['show', 'showmodal', 'close', 'center', 'repaint', 'refresh', 'hide',
                  'setfocus', 'bringtofront', 'sendtoback', 'update'],
        events: ['onclick', 'onclose', 'onresize', 'onshow', 'onhide', 'onactivate',
                 'ondeactivate', 'onpaint', 'onmousemove', 'onmousedown', 'onmouseup',
                 'onkeydown', 'onkeyup', 'onkeypress', 'ondblclick', 'ontimer']
    },
    'RMEMO': {
        props: ['text', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'readonly',
                'font', 'fontsize', 'fontcolor', 'color', 'wordwrap', 'scrollbars', 'hint', 'showhint', 'cursor',
                'tag', 'parent', 'taborder', 'borderstyle', 'alignment'],
        methods: ['clear', 'setfocus', 'selectall', 'copytoclipboard', 'cuttoclipboard',
                  'pastefromclipboard', 'undo', 'repaint', 'refresh'],
        events: ['onchange', 'onclick', 'ondblclick', 'onkeydown', 'onkeyup', 'onkeypress']
    },
    'RUPDOWN': {
        props: ['min', 'max', 'position', 'width', 'height', 'top', 'left', 'visible', 'enabled',
                'hint', 'showhint', 'cursor', 'tag', 'parent', 'taborder'],
        methods: ['setfocus', 'repaint', 'refresh'],
        events: ['onclick', 'onchange']
    },
    'RDATETIMEPICKER': {
        props: ['date', 'time', 'width', 'height', 'top', 'left', 'visible', 'enabled',
                'font', 'fontsize', 'fontcolor', 'color', 'hint', 'showhint', 'cursor', 'tag', 'parent', 'taborder'],
        methods: ['repaint', 'refresh', 'setfocus'],
        events: ['onclick', 'onchange']
    },
    'RTOOLBAR': {
        props: ['width', 'height', 'top', 'left', 'visible', 'enabled', 'color',
                'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['repaint', 'refresh'],
        events: ['onclick']
    },
    'RBUTTON': {
        props: ['caption', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'font',
                'fontsize', 'fontcolor', 'color', 'hint', 'showhint', 'cursor', 'tag', 'parent', 'taborder'],
        methods: ['setfocus', 'repaint', 'refresh', 'bringtofront', 'sendtoback'],
        events: ['onclick', 'onmousedown', 'onmouseup', 'onmousemove']
    },
    'RLABEL': {
        props: ['caption', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'font',
                'fontsize', 'fontcolor', 'color', 'alignment', 'autosize', 'wordwrap', 'transparent',
                'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['repaint', 'refresh'],
        events: ['onclick', 'ondblclick', 'onmousedown', 'onmouseup', 'onmousemove']
    },
    'REDIT': {
        props: ['text', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'font',
                'fontsize', 'fontcolor', 'color', 'maxlength', 'readonly', 'passwordchar',
                'alignment', 'borderstyle', 'hint', 'showhint', 'cursor', 'tag', 'parent', 'taborder',
                'selstart', 'sellength', 'seltext'],
        methods: ['setfocus', 'clear', 'selectall', 'copytoclipboard', 'cuttoclipboard',
                  'pastefromclipboard', 'undo', 'repaint', 'refresh'],
        events: ['onchange', 'onclick', 'ondblclick', 'onkeydown', 'onkeyup', 'onkeypress']
    },
    'RCANVAS': {
        props: ['width', 'height', 'top', 'left', 'visible', 'color', 'pencolor', 'penwidth',
                'brushcolor', 'font', 'fontsize', 'fontcolor', 'hint', 'showhint', 'cursor',
                'tag', 'parent'],
        methods: ['cls', 'pset', 'line', 'circle', 'fillcircle', 'rect', 'fillrect', 'rectangle', 'textout', 'drawtext',
                  'setfont', 'setpixel', 'paint', 'repaint', 'refresh'],
        events: ['onclick', 'ondblclick', 'onmousedown', 'onmouseup', 'onmousemove', 'onpaint'],
        methodSignatures: {
            'drawtext': { sig: 'DrawText(x, y, text [, color])', desc: 'Draws text at (x, y). Optional color argument (RGB integer).' },
            'fillrect': { sig: 'FillRect(x1, y1, x2, y2 [, color])', desc: 'Fills a rectangle from (x1,y1) to (x2,y2). Optional fill color.' },
            'setfont': { sig: 'SetFont(family [, size])', desc: 'Sets the font used by subsequent DrawText calls. On the web runtime this maps to ctx.font = "<size>px <family>". Default size 12.' },
            'cls':      { sig: 'Cls()',                  desc: 'Clears the canvas using the current Color.' },
            'line':     { sig: 'Line(x1, y1, x2, y2 [, color])', desc: 'Draws a line.' },
            'circle':   { sig: 'Circle(x, y, radius [, color])', desc: 'Draws a circle outline.' },
            'fillcircle': { sig: 'FillCircle(x, y, radius [, color])', desc: 'Draws a filled circle.' }
        }
    },
    'RPANEL': {
        props: ['caption', 'width', 'height', 'top', 'left', 'visible', 'color', 'alignment',
                'bevelinner', 'bevelouter', 'borderstyle', 'font', 'fontsize', 'fontcolor',
                'hint', 'showhint', 'cursor', 'tag', 'parent', 'enabled'],
        methods: ['repaint', 'refresh', 'bringtofront', 'sendtoback'],
        events: ['onclick', 'ondblclick', 'onmousedown', 'onmouseup', 'onmousemove', 'onresize']
    },
    'RCHECKBOX': {
        props: ['caption', 'checked', 'width', 'height', 'top', 'left', 'visible', 'enabled',
                'font', 'fontsize', 'fontcolor', 'color', 'state', 'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['setfocus', 'repaint', 'refresh'],
        events: ['onclick']
    },
    'RRADIOBUTTON': {
        props: ['caption', 'checked', 'width', 'height', 'top', 'left', 'visible', 'enabled',
                'font', 'fontsize', 'fontcolor', 'color', 'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['setfocus', 'repaint', 'refresh'],
        events: ['onclick']
    },
    'RCOMBOBOX': {
        props: ['text', 'itemindex', 'itemcount', 'width', 'height', 'top', 'left', 'visible',
                'enabled', 'font', 'fontsize', 'fontcolor', 'color', 'sorted', 'style', 'hint',
                'showhint', 'cursor', 'tag', 'parent', 'taborder'],
        methods: ['additem', 'additems', 'clear', 'deleteitem', 'setfocus', 'repaint', 'refresh'],
        events: ['onchange', 'onclick', 'ondblclick']
    },
    'RLISTBOX': {
        props: ['itemindex', 'itemcount', 'width', 'height', 'top', 'left', 'visible', 'enabled',
                'font', 'fontsize', 'fontcolor', 'color', 'sorted', 'multiselect', 'hint',
                'showhint', 'cursor', 'tag', 'parent', 'taborder'],
        methods: ['additem', 'additems', 'clear', 'deleteitem', 'setfocus', 'repaint', 'refresh', 'item'],
        events: ['onclick', 'ondblclick', 'onchange']
    },
    'RGROUPBOX': {
        props: ['caption', 'width', 'height', 'top', 'left', 'visible', 'color', 'font',
                'fontsize', 'fontcolor', 'hint', 'showhint', 'cursor', 'tag', 'parent', 'enabled'],
        methods: ['repaint', 'refresh'],
        events: ['onclick']
    },
    'RRICHEDIT': {
        props: ['text', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'readonly',
                'font', 'fontsize', 'fontcolor', 'color', 'wordwrap', 'scrollbars', 'line',
                'linecount', 'selstart', 'sellength', 'seltext', 'hint', 'showhint', 'cursor',
                'tag', 'parent', 'taborder', 'borderstyle', 'alignment'],
        methods: ['clear', 'setfocus', 'selectall', 'copytoclipboard', 'cuttoclipboard',
                  'pastefromclipboard', 'undo', 'loadfromfile', 'savetofile', 'addstrings', 'repaint', 'refresh'],
        events: ['onchange', 'onclick', 'ondblclick', 'onkeydown', 'onkeyup', 'onkeypress']
    },
    'RTIMER': {
        props: ['interval', 'enabled', 'tag'],
        methods: [],
        events: ['ontimer']
    },
    'RPROGRESSBAR': {
        props: ['min', 'max', 'position', 'width', 'height', 'top', 'left', 'visible',
                'color', 'hint', 'showhint', 'tag', 'parent'],
        methods: ['stepit', 'stepby', 'repaint', 'refresh'],
        events: []
    },
    'RSTRINGGRID': {
        props: ['colcount', 'rowcount', 'fixedcols', 'fixedrows', 'defaultcolwidth', 'defaultrowheight',
                'colwidths', 'rowheights', 'col', 'row', 'toprow', 'leftcol', 'separator', 'columnstyle',
                'columnlist', 'gridwidth', 'gridheight', 'editormode', 'cell', 'cells',
                'cols', 'rows', 'colwidth', 'selectedrow', 'selectedcol',
                'width', 'height', 'top', 'left', 'visible', 'enabled', 'color', 'font', 'fontsize',
                'fontcolor', 'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['addoptions', 'deloptions', 'insertrow', 'deleterow', 'insertcol', 'deletecol',
                  'swaprows', 'swapcols', 'savetofile', 'loadfromfile', 'savetostream', 'loadfromstream',
                  'addrow', 'setcell', 'getcell', 'clear', 'setrowcount', 'setcolcount',
                  'repaint', 'refresh', 'setfocus'],
        events: ['onclick', 'ondblclick', 'onselectcell', 'onsetedittext', 'onellipsisclick', 'onchange']
    },
    'RTABCONTROL': {
        props: ['tabindex', 'tabcount', 'width', 'height', 'top', 'left', 'visible', 'enabled',
                'font', 'fontsize', 'fontcolor', 'color', 'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['addtab', 'addtabs', 'deletetab', 'repaint', 'refresh', 'tab'],
        events: ['onchange', 'onclick']
    },
    'RMAINMENU': {
        props: ['tag'],
        methods: [],
        events: []
    },
    'RMENUITEM': {
        props: ['caption', 'checked', 'enabled', 'visible', 'shortcut', 'tag'],
        methods: ['clear'],
        events: ['onclick']
    },
    'RSCROLLBAR': {
        props: ['min', 'max', 'position', 'smallchange', 'largechange', 'kind', 'width', 'height',
                'top', 'left', 'visible', 'enabled', 'tag', 'parent'],
        methods: ['repaint', 'refresh'],
        events: ['onchange']
    },
    'RCODEEDITOR': {
        props: ['text', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'font',
                'fontsize', 'fontcolor', 'color', 'readonly', 'linenumbers', 'wordwrap',
                'selstart', 'sellength', 'seltext', 'line', 'linecount', 'caretx', 'carety',
                'hint', 'showhint', 'cursor', 'tag', 'parent', 'highlighttypes', 'autocompletelist',
                'borderstyle'],
        methods: ['clear', 'setfocus', 'selectall', 'copytoclipboard', 'cuttoclipboard',
                  'pastefromclipboard', 'undo', 'redo', 'loadfromfile', 'savetofile',
                  'addstrings', 'gotosub', 'gotoline', 'getsublist', 'repaint', 'refresh'],
        events: ['onchange', 'onclick', 'ondblclick', 'onkeydown', 'onkeyup', 'onkeypress']
    },
    'RIMAGE': {
        props: ['width', 'height', 'top', 'left', 'visible', 'autosize', 'stretch', 'image',
                'bmpwidth', 'bmpheight', 'tag', 'parent'],
        methods: ['loadfromfile', 'savetofile', 'loadfromplot', 'cls', 'pset', 'line', 'circle',
                  'textout', 'fillrect', 'repaint', 'refresh'],
        events: ['onclick', 'ondblclick', 'onmousedown', 'onmouseup', 'onmousemove']
    },
    'RLISTVIEW': {
        props: ['width', 'height', 'top', 'left', 'visible', 'enabled', 'viewstyle', 'multiselect',
                'gridlines', 'checkboxes', 'rowselect', 'sorttype', 'sortcolumn', 'itemcount',
                'itemindex', 'font', 'fontsize', 'fontcolor', 'color', 'hint', 'showhint',
                'cursor', 'tag', 'parent', 'smallimages', 'largeimages', 'columns'],
        methods: ['addcolumn', 'additem', 'deleteitem', 'clear', 'setfocus', 'repaint', 'refresh',
                  'itemcheck', 'subitem'],
        events: ['onclick', 'ondblclick', 'oncolumnclick', 'onchange', 'onitemcheck']
    },
    'RFILESTREAM': {
        props: ['position', 'size', 'tag'],
        methods: ['open', 'close', 'read', 'write', 'readline', 'writeline', 'readnum',
                  'writenum', 'eof', 'seek'],
        events: []
    },
    'ROPENDIALOG': {
        props: ['filename', 'filter', 'initialdir', 'title', 'filterindex', 'defaultext', 'tag'],
        methods: ['execute'],
        events: []
    },
    'RSAVEDIALOG': {
        props: ['filename', 'filter', 'initialdir', 'title', 'filterindex', 'defaultext', 'tag'],
        methods: ['execute'],
        events: []
    },
    'RFILEDIALOG': {
        props: ['filename', 'filter', 'initialdir', 'title', 'filterindex', 'defaultext', 'tag'],
        methods: ['execute'],
        events: []
    },
    'RCOLORDIALOG': {
        description: 'Native colour-picker dialog. On the web runtime renders an HTML5 <input type="color">; on desktop uses the system colour dialog. After Execute, read the .Color property as an RGB integer.',
        props: ['color', 'tag'],
        methods: ['execute'],
        events: [],
        methodSignatures: {
            'execute': { sig: 'Execute()', desc: 'Opens the colour picker. Returns 1 if a colour was selected, 0 if cancelled. Selected value is in .Color.' }
        }
    },
    'RFONTDIALOG': {
        description: 'Font selection dialog. After Execute, read .FontName / .FontSize / .FontColor / .FontStyle to retrieve the user choice.',
        props: ['fontname', 'fontsize', 'fontcolor', 'fontstyle', 'tag'],
        methods: ['execute'],
        events: [],
        methodSignatures: {
            'execute': { sig: 'Execute()', desc: 'Opens the font dialog. Returns 1 if a font was chosen, 0 if cancelled. Read .FontName / .FontSize / .FontColor / .FontStyle for the result.' }
        }
    },
    'RSTATUSBAR': {
        props: ['caption', 'simpletext', 'simplepanel', 'panels', 'panelcount', 'font', 'fontsize', 'fontcolor',
                'visible', 'tag', 'parent'],
        methods: ['addpanel', 'repaint', 'refresh'],
        events: ['onclick']
    },
    'RLINE': {
        props: ['x1', 'y1', 'x2', 'y2', 'color', 'width', 'visible', 'tag', 'parent'],
        methods: [],
        events: []
    },
    'RICON': {
        props: ['filename', 'handle', 'tag'],
        methods: ['loadfromfile'],
        events: []
    },
    'RIMAGELIST': {
        props: ['count', 'width', 'height', 'masked', 'bkcolor', 'tag'],
        methods: ['addbmpfile', 'addbmphandle', 'insertbmpfile', 'insertbmphandle', 'getbmp',
                  'draw', 'delete', 'clear'],
        events: []
    },
    'RBITMAP': {
        props: ['bmp', 'width', 'height', 'pixel', 'empty', 'transparent', 'transparentcolor', 'tag'],
        methods: ['pset', 'line', 'rectangle', 'fillrect', 'circle', 'roundrect', 'paint', 'draw',
                  'copyrect', 'stretchdraw', 'loadfromfile', 'savetofile', 'loadfromstream',
                  'savetostream'],
        events: []
    },
    'RFONT': {
        props: ['name', 'size', 'color', 'bold', 'italic', 'underline', 'strikeout', 'fontcount', 'tag'],
        methods: ['addstyles', 'delstyles', 'fontname'],
        events: []
    },
    'RMYSQL': {
        props: ['host', 'user', 'password', 'database', 'port', 'connected', 'rowcount',
                'colcount', 'fieldcount', 'fieldname', 'row', 'dbcount', 'db', 'tablecount',
                'table', 'escapestring', 'tag'],
        methods: ['connect', 'open', 'close', 'query', 'fetchrow', 'fetchfield', 'use',
                  'selectdb', 'rowseek', 'fieldseek', 'createdb', 'dropdb', 'addparam', 'clearparams'],
        events: ['onconnect', 'ondisconnect', 'onerror', 'onquerydone'],
        methodSignatures: {
            'query': { sig: 'Query(sql [, value, …])', desc: 'Runs the SQL. Values after it are bound to its ? placeholders (sent apart from the SQL: no SQL injection); an array gives its elements. Returns 1, or 0 on an error (OnError gets the message).' },
            'addparam': { sig: 'AddParam value [, value …]', desc: 'Queues a value for the next Query\'s ? placeholders (before the values given to Query itself).' },
            'clearparams': { sig: 'ClearParams', desc: 'Drops the values queued with AddParam.' }
        }
    },
    'RSQLITE': {
        props: ['database', 'db', 'connected', 'rowcount', 'colcount', 'fieldcount',
                'fieldname', 'row', 'tablecount', 'table', 'dbcount', 'tag'],
        methods: ['connect', 'close', 'query', 'exec', 'queryscalar', 'fetchrow', 'fetchfield', 'rowseek', 'fieldseek',
                  'escapestring', 'addparam', 'clearparams'],
        events: ['onconnect', 'ondisconnect', 'onerror', 'onquerydone'],
        methodSignatures: {
            'connect': { sig: 'Connect(file)', desc: 'Opens (or creates) the database file; ":memory:" for one in memory. Returns 1 if the database was there already, 0 if new. On the web a database lasts for the page\'s session (a project\'s .db file is read in first).' },
            'query': { sig: 'Query(sql [, value, …])', desc: 'Runs the SQL\'s statements; one with result columns (SELECT, WITH, PRAGMA, … RETURNING) gives the rows FetchRow walks. Values after the SQL are bound to its ? placeholders (never spliced into it: no SQL injection); an array gives its elements. Returns 1, or 0 on an error (OnError gets the message).' },
            'queryscalar': { sig: 'QueryScalar(sql [, value, …])', desc: 'The first column of the first row the query gives ("" if none); the rows of the last Query stay.' },
            'addparam': { sig: 'AddParam value [, value …]', desc: 'Queues a value for the next query\'s ? placeholders (before the values given to Query itself).' },
            'clearparams': { sig: 'ClearParams', desc: 'Drops the values queued with AddParam.' },
            'fetchrow': { sig: 'FetchRow', desc: 'Moves to the next row of the last query\'s rows: 1, or 0 past the last.' },
            'rowseek': { sig: 'RowSeek(row)', desc: 'The next FetchRow fetches row `row` (from 0).' },
            'escapestring': { sig: 'EscapeString(text)', desc: 'The text with its single quotes doubled, to put between quotes in SQL (binding with ? is safer).' }
        }
    },
    'RSOCKET': {
        props: ['host', 'port', 'connected', 'timeout', 'tag'],
        methods: ['connect', 'close', 'write', 'writeline', 'read', 'readline', 'bind', 'listen', 'accept'],
        events: ['onconnect', 'ondisconnect', 'ondataready', 'onerror']
    },
    'RSERVERSOCKET': {
        props: ['host', 'port', 'clientcount', 'tag'],
        methods: ['start', 'stop', 'broadcast'],
        events: ['onclientconnect', 'onclientdisconnect', 'ondatareceived', 'onerror']
    },
    'RHTTP': {
        props: ['host', 'port', 'url', 'statuscode', 'responsetext', 'responseheaders', 'timeout', 'usessl', 'tag'],
        methods: ['get', 'post'],
        events: []
    },
    'RDESIGNSURFACE': {
        props: ['width', 'height', 'left', 'top', 'compcount', 'visible', 'formcaption', 'tag', 'parent'],
        methods: ['addcomponent', 'removecomponent', 'clearall', 'selectcomp', 'getname', 'setname',
                  'gettype', 'getprop', 'setprop', 'getevent', 'setevent', 'setcompbounds', 'getcompx', 'getcompy',
                  'getcompw', 'getcomph', 'show', 'hide', 'repaint', 'refresh'],
        events: ['onselect', 'ondblclick', 'onmove', 'onbgclick']
    },
    'RTREEVIEW': {
        props: ['width', 'height', 'top', 'left', 'visible', 'enabled', 'font', 'fontsize',
                'fontcolor', 'color', 'itemcount', 'tag', 'parent'],
        methods: ['additem', 'addchild', 'clear', 'expandall', 'collapseall', 'repaint', 'refresh'],
        events: ['onclick', 'ondblclick', 'onexpanded', 'oncollapsed', 'onchange']
    },
    'RSPLITTER': {
        props: ['width', 'height', 'top', 'left', 'visible', 'orientation', 'control1', 'control2',
                'tag', 'parent'],
        methods: [],
        events: []
    },
    'RTRACKBAR': {
        props: ['width', 'height', 'top', 'left', 'visible', 'enabled', 'min', 'max', 'position',
                'orientation', 'tickfrequency', 'tag', 'parent'],
        methods: ['setfocus', 'repaint', 'refresh'],
        events: ['onchange', 'onclick']
    },
    'RSCROLLBOX': {
        props: ['width', 'height', 'top', 'left', 'visible', 'color', 'tag', 'parent'],
        methods: ['repaint', 'refresh'],
        events: []
    },
    'RPOPUPMENU': {
        props: ['tag'],
        methods: ['additem', 'additems', 'popup', 'clear'],
        events: []
    },
    'RINI': {
        props: ['filename', 'section', 'tag'],
        methods: ['readstring', 'writestring', 'readinteger', 'writeinteger', 'deletesection', 'deletekey'],
        events: []
    },
    'RMEMORYSTREAM': {
        props: ['position', 'size', 'linecount', 'tag'],
        methods: ['write', 'writestr', 'writebinstr', 'writeline', 'writenum', 'readstr', 'readbinstr',
                  'readline', 'readnum', 'seek', 'copyfrom', 'close'],
        events: []
    },
    'RSTRINGLIST': {
        props: ['count', 'text', 'tag'],
        methods: ['add', 'delete', 'clear', 'sort', 'indexof', 'insert', 'exchange',
                  'loadfromfile', 'savetofile', 'item', 'setitem'],
        events: []
    },
    'RPRINTER': {
        props: ['tag'],
        methods: ['begindoc', 'enddoc', 'newpage', 'textout', 'printline'],
        events: []
    },
    'RCOOLBTN': {
        description: 'Flat/toggle toolbar button with optional multi-state BMP image. Supports GroupIndex for radio-button-like groups.',
        props: ['caption', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'font',
                'fontsize', 'fontcolor', 'color', 'flat', 'groupindex', 'down', 'allowallup',
                'bmp', 'numbmps', 'layout', 'spacing', 'align',
                'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['setfocus', 'repaint', 'refresh'],
        events: ['onclick']
    },
    'ROVALBTN': {
        description: 'Oval/round button with customizable colors and highlight/shadow effects.',
        props: ['caption', 'width', 'height', 'top', 'left', 'visible', 'enabled', 'font',
                'fontsize', 'fontcolor', 'color', 'colorhighlight', 'colorshadow', 'transparent',
                'flat', 'groupindex', 'down', 'allowallup',
                'hint', 'showhint', 'cursor', 'tag', 'parent'],
        methods: ['setfocus', 'repaint', 'refresh'],
        events: ['onclick']
    },

    // ── Web-exclusive components (WASM target) ──────────────────────────
    'RWEBVIEW': {
        description: 'Embedded HTML content viewer (iframe). Web-only component.',
        props: ['left', 'top', 'width', 'height', 'visible', 'enabled', 'html', 'url', 'sandbox',
                'color', 'font', 'fontsize', 'fontcolor', 'tag', 'parent'],
        methods: ['sethtml', 'navigate', 'show', 'hide', 'setfocus'],
        events: ['onclick', 'ondblclick', 'onload']
    },
    'RDOM': {
        description: 'Direct DOM element. Create arbitrary HTML elements. Web-only component.',
        props: ['innerhtml', 'innertext', 'cssclass', 'cssstyle', 'tagname',
                'left', 'top', 'width', 'height', 'visible', 'enabled', 'color',
                'font', 'fontsize', 'fontcolor', 'tag', 'parent'],
        methods: ['create', 'appendto', 'setattribute', 'getattribute', 'addclass',
                  'removeclass', 'toggleclass', 'remove', 'queryselector'],
        events: ['onclick', 'ondblclick', 'onchange', 'onmousedown', 'onmouseup',
                 'onmousemove', 'onfocus', 'onblur', 'onscroll']
    },
    'RJAVASCRIPT': {
        description: 'Execute arbitrary JavaScript from RapidR. Web-only component.',
        props: ['tag'],
        methods: ['eval', 'call'],
        events: []
    },
    'RWEBSTORAGE': {
        description: 'Browser localStorage / sessionStorage access. Web-only component.',
        props: ['storagetype', 'tag'],
        methods: ['set', 'get', 'remove', 'clear', 'keys', 'haskey'],
        events: []
    },
    'RWEBAUDIO': {
        description: 'HTML5 audio player. Web-only component.',
        props: ['src', 'volume', 'loop', 'autoplay', 'controls', 'currenttime', 'duration',
                'playing', 'paused', 'tag', 'parent'],
        methods: ['play', 'pause', 'stop', 'seek'],
        events: ['onplay', 'onpause', 'onended', 'ontimeupdate']
    },
    'RWEBVIDEO': {
        description: 'HTML5 video player. Web-only component.',
        props: ['src', 'volume', 'loop', 'autoplay', 'controls', 'poster', 'currenttime',
                'duration', 'playing', 'paused', 'left', 'top', 'width', 'height',
                'visible', 'tag', 'parent'],
        methods: ['play', 'pause', 'stop', 'seek', 'fullscreen'],
        events: ['onplay', 'onpause', 'onended', 'ontimeupdate', 'onclick']
    },
    'RWEBNOTIFICATION': {
        description: 'Browser push notification. Web-only component.',
        props: ['title', 'body', 'tag'],
        methods: ['requestpermission', 'show'],
        events: []
    },
    'RWEBGEOLOCATION': {
        description: 'Browser geolocation API. Web-only component.',
        props: ['latitude', 'longitude', 'accuracy', 'tag'],
        methods: ['getposition'],
        events: []
    },
    'RROUTER': {
        description: 'Single-page app hash-based router. Web-only component.',
        props: ['tag'],
        methods: ['navigate', 'back', 'forward'],
        events: ['onroutechange']
    },
    // RNUM, RDATAFRAME, RPLOT: one implementation on every runtime
    // (rapidr_value::datascience; docs/manual/data-science.md, and every
    // name in docs/manual/reference/data-science.md).
    'RNUM': {
        description: 'NumPy-style array of numbers, the same on every runtime: creation, aggregates, element-wise math and arithmetic (in place), ordering, cumulative ops, dot products, random numbers. Another array is named by its component name.',
        props: ['data', 'shape', 'size', 'length', 'len', 'count', 'ndim', 'dtype', 'sum', 'mean', 'min', 'max', 'std', 'tag'],
        methods: [
            // Creation and indexing
            'create', 'arange', 'linspace', 'zeros', 'ones', 'full', 'fromlist', 'set', 'get', 'push',
            // Aggregation
            'sum', 'mean', 'min', 'max', 'std', 'var', 'median', 'argmin', 'argmax', 'count', 'ptp',
            // Element-wise math
            'sin', 'cos', 'tan', 'asin', 'acos', 'atan', 'sqrt', 'abs', 'exp', 'log', 'log2', 'log10',
            'floor', 'ceil', 'round', 'sign', 'reciprocal', 'square', 'negative',
            // Arithmetic
            'add', 'subtract', 'multiply', 'divide', 'power', 'mod', 'clip',
            // Ordering
            'sort', 'reverse', 'unique', 'shuffle', 'append', 'slice',
            // Cumulative
            'cumsum', 'cumprod', 'diff',
            // Linear algebra
            'dot', 'norm', 'normalize',
            // Boolean / search
            'any', 'all', 'where', 'nonzero', 'searchsorted',
            // Random
            'rand', 'randn', 'uniform', 'randint', 'choice',
            // Output
            'tolist', 'tostring', 'print', 'show', 'reshape', 'clear'
        ],
        events: [],
        methodSignatures: {
            'create': { sig: 'create([n])', desc: 'Makes the array n zeros (empty without n)' },
            'arange': { sig: 'arange(start, stop [, step])', desc: 'Evenly spaced values in [start, stop) with the given step (default 1; may be negative)' },
            'linspace': { sig: 'linspace(start, stop [, num])', desc: 'num evenly spaced values from start to stop, both included (default 50)' },
            'zeros': { sig: 'zeros(n)', desc: 'n zeros' },
            'ones': { sig: 'ones(n)', desc: 'n ones' },
            'full': { sig: 'full(n, fillValue)', desc: 'n elements of fillValue' },
            'fromlist': { sig: 'fromlist("v1,v2,v3,...")', desc: 'The numbers of a comma-separated list' },
            'set': { sig: 'set(index, value)', desc: 'Sets element index (from 0); past the end the array grows with zeros' },
            'get': { sig: 'get(index)', desc: 'Element index (from 0); 0 past the end' },
            'push': { sig: 'push(value)', desc: 'Adds value at the end' },
            'sum': { sig: 'sum()', desc: 'The sum of the elements' },
            'mean': { sig: 'mean()', desc: 'The arithmetic mean' },
            'std': { sig: 'std()', desc: 'The (population) standard deviation' },
            'var': { sig: 'var()', desc: 'The (population) variance' },
            'median': { sig: 'median()', desc: 'The median' },
            'argmin': { sig: 'argmin()', desc: 'The index of the smallest element' },
            'argmax': { sig: 'argmax()', desc: 'The index of the largest element' },
            'count': { sig: 'count()', desc: 'The number of elements' },
            'ptp': { sig: 'ptp()', desc: 'Peak to peak: max - min' },
            'clip': { sig: 'clip(low, high)', desc: 'Limits every element to [low, high]' },
            'add': { sig: 'add(value | "array")', desc: 'Adds a number, or another array element by element (arrays of the same length)' },
            'subtract': { sig: 'subtract(value | "array")', desc: 'Subtracts a number or another array, element by element' },
            'multiply': { sig: 'multiply(value | "array")', desc: 'Multiplies by a number or another array, element by element' },
            'divide': { sig: 'divide(value | "array")', desc: 'Divides by a number or another array, element by element (by 0: NAN)' },
            'power': { sig: 'power([exp | "array"])', desc: 'Raises every element to a power (default 2)' },
            'mod': { sig: 'mod(divisor | "array")', desc: 'The remainder of each element' },
            'slice': { sig: 'slice(start [, end])', desc: 'Keeps the elements from start to end (not included)' },
            'append': { sig: 'append("array" | "v1,v2,..." | value)', desc: 'Adds another array, a list or a number at the end' },
            'dot': { sig: 'dot("array")', desc: 'The dot product with another array' },
            'searchsorted': { sig: 'searchsorted(value)', desc: 'Where value would go in the sorted array' },
            'rand': { sig: 'rand([n])', desc: 'n random values in [0, 1) (default 1)' },
            'randn': { sig: 'randn([n [, mean [, std]]])', desc: 'n normally distributed values' },
            'uniform': { sig: 'uniform(low, high [, n])', desc: 'n random values in [low, high)' },
            'randint': { sig: 'randint(low, high [, n])', desc: 'n random whole numbers from low to high, both included' },
            'choice': { sig: 'choice([n])', desc: 'One element at random (returned), or the array becomes n of them' },
            'round': { sig: 'round([decimals])', desc: 'Rounds every element (default 0 decimals)' },
            'tolist': { sig: 'tolist()', desc: 'The elements as "1,2,3"' },
            'print': { sig: 'print()', desc: 'Prints the array as [1, 2, 3]' },
        }
    },
    'RPLOT': {
        description: 'Matplotlib-style chart: lines, bars, scatter, steps, areas, histograms, pies, reference lines and notes. The same model on every runtime; saved as a PNG or shown in a QIMAGE on the desktop, drawn on the page on the web.',
        props: ['title', 'xlabel', 'ylabel', 'width', 'height', 'grid', 'dpi', 'tag'],
        methods: [
            'clear', 'plot', 'bar', 'barh', 'scatter', 'step', 'area', 'fill_between',
            'hist', 'pie', 'addseries', 'hline', 'axhline', 'vline', 'axvline',
            'annotate', 'legend', 'settitle', 'setxlabel', 'setylabel', 'grid',
            'savefig', 'save', 'show', 'render', 'figsize', 'xlim', 'ylim', 'xscale', 'yscale'
        ],
        events: [],
        methodSignatures: {
            'plot': { sig: 'plot(x, y [, label [, color [, style]]])', desc: 'A line: x and y are RNum arrays (or "1,2,3"); style "-" (default), "--" dashed, "o" points.' },
            'bar': { sig: 'bar(x, y [, label [, color]])', desc: 'Vertical bars.' },
            'barh': { sig: 'barh(x, y [, label [, color]])', desc: 'Horizontal bars.' },
            'scatter': { sig: 'scatter(x, y [, label [, color]])', desc: 'Points.' },
            'step': { sig: 'step(x, y [, label [, color]])', desc: 'A step line.' },
            'area': { sig: 'area(x, y [, label [, color]])', desc: 'A filled area under the line.' },
            'hist': { sig: 'hist(data [, bins [, label [, color]]])', desc: 'A histogram (default 10 bins).' },
            'pie': { sig: 'pie(values [, "label1,label2,..." [, "color1,color2,..."]])', desc: 'A pie chart.' },
            'addseries': { sig: 'addseries(label, "y1,y2,..." [, "x1,x2,..." [, color]])', desc: 'A line from numbers written in place, drawn at once where the chart shows.' },
            'hline': { sig: 'hline(y [, color])', desc: 'A horizontal reference line at y.' },
            'vline': { sig: 'vline(x [, color])', desc: 'A vertical reference line at x.' },
            'annotate': { sig: 'annotate(text, x, y [, color])', desc: 'Text at (x, y).' },
            'legend': { sig: 'legend()', desc: 'Shows the series labels.' },
            'savefig': { sig: 'savefig(filename)', desc: 'Draws the chart into a PNG file (on the web: on the page).' },
            'show': { sig: 'show()', desc: 'Draws the chart where it is shown (the page, on the web).' },
            'figsize': { sig: 'figsize(width, height)', desc: 'The size in inches at the chart\'s DPI.' },
            'xlim': { sig: 'xlim(min, max)', desc: 'The X axis range.' },
            'ylim': { sig: 'ylim(min, max)', desc: 'The Y axis range.' },
            'xscale': { sig: 'xscale(type)', desc: '"linear" or "log" (kept with the chart; drawn linear).' },
            'yscale': { sig: 'yscale(type)', desc: '"linear" or "log" (kept with the chart; drawn linear).' },
            'clear': { sig: 'clear()', desc: 'Removes every series and setting.' },
        }
    },
    'RDATAFRAME': {
        description: 'Pandas-style table, the same on every runtime: CSV / JSON in and out, selection, filtering, grouping, column operations, statistics, sampling, joins, transforms, printing as a table, filling a QSTRINGGRID.',
        props: ['rowcount', 'height', 'nrows', 'colcount', 'width', 'ncols', 'columns', 'shape', 'empty', 'dtypes', 'tag'],
        methods: [
            // Creation and I/O
            'create', 'addrow', 'loadfromcsv', 'readcsv', 'savetocsv', 'loadfromjson', 'savetojson',
            // Selection / indexing
            'head', 'tail', 'cell', 'cellbyname', 'at', 'setcell', 'iloc', 'select',
            // Sorting / filtering
            'sort', 'sort_values', 'filter', 'query',
            // Grouping
            'groupby',
            // Column operations
            'drop', 'rename', 'addcolumn',
            // Missing data
            'fillna', 'dropna',
            // Statistics
            'describe', 'value_counts', 'nunique', 'corr',
            // Sampling
            'sample', 'nlargest', 'nsmallest',
            // Info
            'info', 'dtypes', 'shape',
            // Merge / join
            'merge', 'join', 'concat',
            // Transform
            'transpose', 'apply', 'replace',
            // Display
            'clear', 'columns', 'rows', 'tostring', 'show', 'print', 'togrid'
        ],
        events: [],
        methodSignatures: {
            'create': { sig: 'create()', desc: 'Makes the frame empty.' },
            'addrow': { sig: 'addrow(v1, v2, ... | "v1,v2,...")', desc: 'Adds a row (columns column_1, ... if the frame has fewer).' },
            'loadfromcsv': { sig: 'loadfromcsv(file | csvText)', desc: 'Reads CSV (the first line the header) from a file, or the CSV text itself.' },
            'readcsv': { sig: 'readcsv(file | csvText)', desc: 'The same as loadfromcsv.' },
            'savetocsv': { sig: 'savetocsv([file])', desc: 'Writes the frame as CSV (default output.csv).' },
            'loadfromjson': { sig: 'loadfromjson(file | jsonText)', desc: 'Reads JSON records ([{...}, ...]), JSON Lines or columns ({"a": [...]}).' },
            'savetojson': { sig: 'savetojson([file])', desc: 'Writes the frame as an array of JSON records (default output.json).' },
            'head': { sig: 'head([n])', desc: 'Keeps the first n rows (default 5).' },
            'tail': { sig: 'tail([n])', desc: 'Keeps the last n rows (default 5).' },
            'cell': { sig: 'cell(row, col)', desc: 'The text of a cell (col: an index from 0, or a column name); "" for a missing value.' },
            'cellbyname': { sig: 'cellbyname(row, columnName)', desc: 'The text of a cell by its column name.' },
            'setcell': { sig: 'setcell(row, col, value)', desc: 'Sets a cell (col: an index or a name); past the end the frame grows.' },
            'iloc': { sig: 'iloc(start [, end])', desc: 'Keeps rows start to end (not included).' },
            'select': { sig: 'select("col1,col2,...")', desc: 'Keeps those columns, in that order.' },
            'sort': { sig: 'sort("col1[,col2...]" [, ascending])', desc: 'Sorts by columns: 1 ascending (default), 0 descending; numbers as numbers, missing values last.' },
            'sort_values': { sig: 'sort_values("col1[,col2...]" [, ascending])', desc: 'The same as sort.' },
            'filter': { sig: 'filter(column, operator, value)', desc: 'Keeps the rows that pass: =, ==, <>, !=, >, <, >=, <=, contains, startswith, endswith.' },
            'query': { sig: 'query("column op value")', desc: 'Filters with one condition: query "age > 30".' },
            'groupby': { sig: 'groupby(column [, function])', desc: 'A row a group: the column, then mean (default), sum, count, min, max, first, last, median or std of every other column.' },
            'drop': { sig: 'drop(column)', desc: 'Removes a column.' },
            'rename': { sig: 'rename(oldName, newName)', desc: 'Renames a column.' },
            'addcolumn': { sig: 'addcolumn(name [, "v1,v2,..."])', desc: 'Adds (or replaces) a column; a frame without rows gets one a value.' },
            'fillna': { sig: 'fillna([value])', desc: 'Missing values become value (default "0").' },
            'dropna': { sig: 'dropna()', desc: 'Removes the rows with a missing value.' },
            'describe': { sig: 'describe()', desc: 'The frame becomes the summary of its numeric columns: count, mean, std, min, 25%, 50%, 75%, max.' },
            'value_counts': { sig: 'value_counts(column)', desc: 'The frame becomes each value of the column and its count, most first.' },
            'nunique': { sig: 'nunique(column)', desc: 'How many different values the column has.' },
            'corr': { sig: 'corr(col1, col2)', desc: 'Pearson\'s correlation of two columns.' },
            'sample': { sig: 'sample([n])', desc: 'Keeps n rows at random (default 5).' },
            'nlargest': { sig: 'nlargest(column [, n])', desc: 'Keeps the n rows with the largest values (default 5).' },
            'nsmallest': { sig: 'nsmallest(column [, n])', desc: 'Keeps the n rows with the smallest values (default 5).' },
            'info': { sig: 'info()', desc: 'Prints the size and each column\'s type and non-null count.' },
            'dtypes': { sig: 'dtypes()', desc: 'The columns\' types: "name: str,age: i64".' },
            'merge': { sig: 'merge(otherFrame, onColumn [, how])', desc: 'Joins another frame on a column: inner (default), left, right, outer, cross.' },
            'concat': { sig: 'concat(otherFrame)', desc: 'Adds another frame\'s rows (columns matched by name).' },
            'transpose': { sig: 'transpose()', desc: 'Columns become rows.' },
            'apply': { sig: 'apply(column, op [, decimals])', desc: 'Transforms a column: upper, lower, trim, abs, round, sqrt, log, exp.' },
            'replace': { sig: 'replace(column, oldValue, newValue)', desc: 'Cells that are oldValue become newValue.' },
            'tostring': { sig: 'tostring()', desc: 'The frame as a text table.' },
            'print': { sig: 'print()', desc: 'Prints the frame as a text table.' },
            'togrid': { sig: 'togrid(gridName)', desc: 'Fills a QSTRINGGRID: the header row, then the cells.' },
        }
    },
    'RJSON': {
        description: 'JSON parsing and generation component. Supports parse, stringify, prettify, dot-path get/set, file I/O, and key enumeration.',
        props: ['text', 'filename', 'count'],
        methods: [
            'parse', 'stringify', 'prettify',
            'get', 'set', 'has', 'remove',
            'count', 'keys',
            'loadfile', 'savefile', 'clear'
        ],
        events: [],
        methodSignatures: {
            'parse': { sig: 'parse(jsonString)', desc: 'Parses a JSON string into the internal store. Returns 1 on success, 0 on error.' },
            'stringify': { sig: 'stringify()', desc: 'Returns the stored JSON as a compact string.' },
            'prettify': { sig: 'prettify()', desc: 'Returns the stored JSON as a pretty-printed string with indentation.' },
            'get': { sig: 'get(path)', desc: 'Returns the value at the dot-path (e.g. "user.name" or "items.0"). Returns "" if not found.' },
            'set': { sig: 'set(path, value)', desc: 'Sets the value at the dot-path, creating intermediate objects as needed.' },
            'has': { sig: 'has(path)', desc: 'Returns 1 if the path exists, 0 otherwise.' },
            'remove': { sig: 'remove(path)', desc: 'Removes the key at the given path.' },
            'count': { sig: 'count()', desc: 'Returns the number of top-level keys (object) or elements (array).' },
            'keys': { sig: 'keys()', desc: 'Returns a comma-separated list of top-level keys.' },
            'loadfile': { sig: 'loadfile(filename)', desc: 'Loads and parses a JSON file. Returns 1 on success, 0 on error.' },
            'savefile': { sig: 'savefile(filename)', desc: 'Saves the stored JSON to a file (pretty-printed). Returns 1 on success, 0 on error.' },
            'clear': { sig: 'clear()', desc: 'Clears the stored JSON data.' }
        }
    }
};
// RapidR's layout extensions (rapidr_value::layout), on every visual
// component: Anchors (akLeft 1 + akTop 2 + akRight 4 + akBottom 8) and
// Constraints (MinWidth … MaxHeight; Constraints.MinWidth is MinWidth).
for (const c of Object.values(COMPONENT_REGISTRY)) {
    if (c.props.includes('left')) c.props.push('anchors', 'minwidth', 'minheight', 'maxwidth', 'maxheight');
}

const BUILTIN_FUNCTIONS = [
    // String functions
    { name: 'CHR$', description: 'Returns character for ASCII code', signature: 'CHR$(code AS INTEGER)', snippet: 'CHR\\$(${1:code})' },
    { name: 'ASC', description: 'Returns ASCII code of character', signature: 'ASC(char AS STRING)', snippet: 'ASC(${1:char})' },
    { name: 'LEFT$', description: 'Returns leftmost n characters', signature: 'LEFT$(str, n)', snippet: 'LEFT\\$(${1:str}, ${2:n})' },
    { name: 'RIGHT$', description: 'Returns rightmost n characters', signature: 'RIGHT$(str, n)', snippet: 'RIGHT\\$(${1:str}, ${2:n})' },
    { name: 'MID$', description: 'Returns substring from position', signature: 'MID$(str, start [, length])', snippet: 'MID\\$(${1:str}, ${2:start}, ${3:length})' },
    { name: 'LEN', description: 'Returns length of string', signature: 'LEN(str)', snippet: 'LEN(${1:str})' },
    { name: 'INSTR', description: 'Finds substring in string, returns position', signature: 'INSTR([start,] str, search)', snippet: 'INSTR(${1:str}, ${2:search})' },
    { name: 'RINSTR', description: 'Finds last occurrence of substring', signature: 'RINSTR(str, search)', snippet: 'RINSTR(${1:str}, ${2:search})' },
    { name: 'UCASE$', description: 'Converts string to uppercase', signature: 'UCASE$(str)', snippet: 'UCASE\\$(${1:str})' },
    { name: 'LCASE$', description: 'Converts string to lowercase', signature: 'LCASE$(str)', snippet: 'LCASE\\$(${1:str})' },
    { name: 'LTRIM$', description: 'Removes leading whitespace', signature: 'LTRIM$(str)', snippet: 'LTRIM\\$(${1:str})' },
    { name: 'RTRIM$', description: 'Removes trailing whitespace', signature: 'RTRIM$(str)', snippet: 'RTRIM\\$(${1:str})' },
    { name: 'TRIM$', description: 'Removes leading and trailing whitespace', signature: 'TRIM$(str)', snippet: 'TRIM\\$(${1:str})' },
    { name: 'SPACE$', description: 'Returns string of n spaces', signature: 'SPACE$(n)', snippet: 'SPACE\\$(${1:n})' },
    { name: 'STRING$', description: 'Returns string of n repeated characters', signature: 'STRING$(n, char)', snippet: 'STRING\\$(${1:n}, ${2:char})' },
    { name: 'STR$', description: 'Converts number to string', signature: 'STR$(number)', snippet: 'STR\\$(${1:number})' },
    { name: 'REPLACE', description: 'Replaces occurrences in string', signature: 'REPLACE(str, old, new)', snippet: 'REPLACE(${1:str}, ${2:old}, ${3:new})' },
    { name: 'REPLACESUBSTR', description: 'Replaces substring', signature: 'REPLACESUBSTR(str, old, new)', snippet: 'REPLACESUBSTR(${1:str}, ${2:old}, ${3:new})' },
    { name: 'INSERT', description: 'Inserts a string into another before a 1-based position (INSERT$("hi", "Hello", 3) = "Hehillo")', signature: 'INSERT$(insert, source, index)', snippet: 'INSERT\\$(${1:insert}, ${2:source}, ${3:index})' },
    { name: 'DELETE', description: 'Deletes characters from string', signature: 'DELETE(str, pos, count)', snippet: 'DELETE(${1:str}, ${2:pos}, ${3:count})' },
    { name: 'REVERSE', description: 'Reverses a string', signature: 'REVERSE(str)', snippet: 'REVERSE(${1:str})' },
    { name: 'FIELD', description: 'Returns nth field from delimited string', signature: 'FIELD(str, delimiter, n)', snippet: 'FIELD(${1:str}, ${2:delim}, ${3:n})' },
    { name: 'TALLY', description: 'Counts occurrences of substring', signature: 'TALLY(str, search)', snippet: 'TALLY(${1:str}, ${2:search})' },
    { name: 'STRF', description: 'Number as text (FloatToStrF): format 0 ffGeneral, 1 ffExponent, 2 ffFixed, 3 ffNumber', signature: 'STRF$(number, format, precision, digits)', snippet: 'STRF\\$(${1:number}, ${2:ffFixed}, ${3:15}, ${4:2})' },

    // Math functions
    { name: 'ABS', description: 'Returns absolute value', signature: 'ABS(number)', snippet: 'ABS(${1:number})' },
    { name: 'ATN', description: 'Returns arctangent (radians)', signature: 'ATN(number)', snippet: 'ATN(${1:number})' },
    { name: 'LPRINT', description: 'As PRINT, to the printer (printed at LFLUSH or when the program ends)', signature: 'LPRINT [expressions][;|,]', snippet: 'LPRINT ${1:"text"}' },
    { name: 'LFLUSH', description: 'Prints what LPRINT wrote so far', signature: 'LFLUSH', snippet: 'LFLUSH' },
    { name: 'ATAN', description: 'Returns arctangent (radians); the same as ATN', signature: 'ATAN(number)', snippet: 'ATAN(${1:number})' },
    { name: 'TAB', description: 'In PRINT: moves to column n (to the next line when already past it)', signature: 'TAB(n)', snippet: 'TAB(${1:n})' },
    { name: 'GET$', description: 'Reads up to n bytes from standard input (a CGI request body)', signature: 'GET$(n)', snippet: 'GET$(${1:n})' },
    { name: 'SETCONSOLETITLE', description: 'Sets the console window title (the page title on the web)', signature: 'SETCONSOLETITLE title', snippet: 'SETCONSOLETITLE ${1:"title"}' },
    { name: 'CHDRIVE', description: 'Changes the current drive (Windows)', signature: 'CHDRIVE drive', snippet: 'CHDRIVE ${1:"d:"}' },
    { name: 'COS', description: 'Returns cosine', signature: 'COS(angle)', snippet: 'COS(${1:angle})' },
    { name: 'SIN', description: 'Returns sine', signature: 'SIN(angle)', snippet: 'SIN(${1:angle})' },
    { name: 'TAN', description: 'Returns tangent', signature: 'TAN(angle)', snippet: 'TAN(${1:angle})' },
    { name: 'EXP', description: 'Returns e raised to power', signature: 'EXP(number)', snippet: 'EXP(${1:number})' },
    { name: 'LOG', description: 'Returns natural logarithm', signature: 'LOG(number)', snippet: 'LOG(${1:number})' },
    { name: 'SQR', description: 'Returns square root', signature: 'SQR(number)', snippet: 'SQR(${1:number})' },
    { name: 'RND', description: 'Returns random number (0 to 1)', signature: 'RND[(n)]', snippet: 'RND(${1:1})' },
    { name: 'CEIL', description: 'Returns ceiling (round up)', signature: 'CEIL(number)', snippet: 'CEIL(${1:number})' },
    { name: 'FLOOR', description: 'Returns floor (round down)', signature: 'FLOOR(number)', snippet: 'FLOOR(${1:number})' },
    { name: 'ACOS', description: 'Returns arc cosine', signature: 'ACOS(number)', snippet: 'ACOS(${1:number})' },
    { name: 'ASIN', description: 'Returns arc sine', signature: 'ASIN(number)', snippet: 'ASIN(${1:number})' },
    { name: 'FIX', description: 'Truncates decimal portion', signature: 'FIX(number)', snippet: 'FIX(${1:number})' },
    { name: 'FRAC', description: 'Returns fractional portion', signature: 'FRAC(number)', snippet: 'FRAC(${1:number})' },
    { name: 'ROUND', description: 'Rounds to n decimal places', signature: 'ROUND(number [, places])', snippet: 'ROUND(${1:number}, ${2:places})' },
    { name: 'SGN', description: 'Returns sign (-1, 0, or 1)', signature: 'SGN(number)', snippet: 'SGN(${1:number})' },
    { name: 'CBOOL', description: 'True (-1) for a non-zero number or numeric string, or any other non-empty string', signature: 'CBOOL(value)', snippet: 'CBOOL(${1:value})' },
    { name: 'CINT', description: 'Converts to integer', signature: 'CINT(number)', snippet: 'CINT(${1:number})' },
    { name: 'CLNG', description: 'Converts to long integer', signature: 'CLNG(number)', snippet: 'CLNG(${1:number})' },
    { name: 'INT', description: 'Returns integer portion', signature: 'INT(number)', snippet: 'INT(${1:number})' },
    { name: 'VAL', description: 'Converts string to number', signature: 'VAL(str)', snippet: 'VAL(${1:str})' },
    { name: 'RANDOMIZE', description: 'Seeds random number generator', signature: 'RANDOMIZE [seed]', snippet: 'RANDOMIZE ${1:seed}' },
    { name: 'IIF', description: 'Inline if: returns trueVal or falseVal', signature: 'IIF(condition, trueVal, falseVal)', snippet: 'IIF(${1:condition}, ${2:trueVal}, ${3:falseVal})' },

    // Conversion functions
    { name: 'HEX$', description: 'Converts number to hex string', signature: 'HEX$(number)', snippet: 'HEX\\$(${1:number})' },
    { name: 'BIN$', description: 'Converts number to binary string', signature: 'BIN$(number)', snippet: 'BIN\\$(${1:number})' },
    { name: 'OCT$', description: 'Converts number to octal string', signature: 'OCT$(number)', snippet: 'OCT\\$(${1:number})' },
    { name: 'HEXTODEC', description: 'Converts hex string to decimal', signature: 'HEXTODEC(hexStr)', snippet: 'HEXTODEC(${1:hexStr})' },
    { name: 'CONVBASE', description: 'Converts between number bases', signature: 'CONVBASE(value, fromBase, toBase)', snippet: 'CONVBASE(${1:value}, ${2:fromBase}, ${3:toBase})' },
    { name: 'FORMAT$', description: 'Pascal-style Format: %d %.5d %-8s %.2f %n %m %e %g %x, %0:d reuses an argument', signature: 'FORMAT$(format, arg1, arg2, ...)', snippet: 'FORMAT\\$("${1:%d}", ${2:value})' },

    // I/O functions
    { name: 'DIR$', description: 'Returns directory listing', signature: 'DIR$([path])', snippet: 'DIR\\$(${1:path})' },
    { name: 'CURDIR$', description: 'Returns current directory', signature: 'CURDIR$', snippet: 'CURDIR\\$' },
    { name: 'DIREXISTS', description: 'Checks if directory exists', signature: 'DIREXISTS(path)', snippet: 'DIREXISTS(${1:path})' },
    { name: 'FILEEXISTS', description: 'Checks if file exists', signature: 'FILEEXISTS(path)', snippet: 'FILEEXISTS(${1:path})' },
    { name: 'CHDIR', description: 'Changes current directory', signature: 'CHDIR(path)', snippet: 'CHDIR(${1:path})' },
    { name: 'MKDIR', description: 'Creates a directory', signature: 'MKDIR(path)', snippet: 'MKDIR(${1:path})' },
    { name: 'RMDIR', description: 'Removes a directory', signature: 'RMDIR(path)', snippet: 'RMDIR(${1:path})' },
    { name: 'KILL', description: 'Deletes a file', signature: 'KILL(path)', snippet: 'KILL(${1:path})' },
    { name: 'RENAME', description: 'Renames a file', signature: 'RENAME(oldName, newName)', snippet: 'RENAME(${1:oldName}, ${2:newName})' },

    // System functions
    { name: 'SHELL', description: 'Executes a shell command', signature: 'SHELL(command)', snippet: 'SHELL(${1:command})' },
    { name: 'SHELLWAIT', description: 'Executes shell command and waits', signature: 'SHELLWAIT(command)', snippet: 'SHELLWAIT(${1:command})' },
    { name: 'RUN', description: 'Runs an external program', signature: 'RUN(program)', snippet: 'RUN(${1:program})' },
    { name: 'SLEEP', description: 'Pauses execution for a number of seconds (SLEEP 1.5: one and a half)', signature: 'SLEEP seconds', snippet: 'SLEEP ${1:seconds}' },
    { name: 'TIMER', description: 'Returns seconds since midnight', signature: 'TIMER', snippet: 'TIMER' },
    { name: 'DATE$', description: 'Returns current date string', signature: 'DATE$', snippet: 'DATE\\$' },
    { name: 'TIME$', description: 'Returns current time string', signature: 'TIME$', snippet: 'TIME\\$' },
    { name: 'COMMAND$', description: 'Returns command line arguments', signature: 'COMMAND$', snippet: 'COMMAND\\$' },
    { name: 'ENVIRON$', description: 'Returns environment variable', signature: 'ENVIRON$(name)', snippet: 'ENVIRON\\$(${1:name})' },
    { name: 'DOEVENTS', description: 'Processes pending GUI events', signature: 'DOEVENTS', snippet: 'DOEVENTS' },
    { name: 'INKEY$', description: 'The next key pressed, or "" (does not wait); arrows and function keys are CHR$(27) + their scan code ($OPTION INKEY$ TRAPALL: Shift, Ctrl, Alt and the lock keys too)', signature: 'INKEY$', snippet: 'INKEY$' },
    { name: 'INPUT$', description: 'Waits for n keys (not echoed) and returns them as soon as the last one is pressed', signature: 'INPUT$(n)', snippet: 'INPUT$(${1:1})' },
    { name: 'END', description: 'Terminates the program', signature: 'END', snippet: 'END' },

    // GUI functions
    { name: 'SHOWMESSAGE', description: 'Displays a message dialog', signature: 'SHOWMESSAGE(message)', snippet: 'SHOWMESSAGE(${1:message})' },
    { name: 'MESSAGEBOX', description: 'Shows message box with buttons', signature: 'MESSAGEBOX(text, title, type)', snippet: 'MESSAGEBOX(${1:text}, ${2:title}, ${3:0})' },
    { name: 'MESSAGEDLG', description: 'Shows message dialog', signature: 'MESSAGEDLG(text, type, buttons)', snippet: 'MESSAGEDLG(${1:text}, ${2:type}, ${3:buttons})' },
    { name: 'RGB', description: 'Returns RGB color value', signature: 'RGB(red, green, blue)', snippet: 'RGB(${1:red}, ${2:green}, ${3:blue})' },
    { name: 'CALLBACK', description: 'Creates a callback reference', signature: 'CALLBACK(subroutine)', snippet: 'CALLBACK(${1:sub})' },
    { name: 'CALLFUNC', description: 'Calls a function by name', signature: 'CALLFUNC(name, args...)', snippet: 'CALLFUNC(${1:name})' },
    { name: 'SOUND', description: 'Plays a system sound', signature: 'SOUND(frequency, duration)', snippet: 'SOUND(${1:frequency}, ${2:duration})' },
    { name: 'PLAYWAV', description: 'Plays a WAV file', signature: 'PLAYWAV(filename)', snippet: 'PLAYWAV(${1:filename})' },

    // Array functions
    { name: 'LBOUND', description: 'Returns lower bound of array', signature: 'LBOUND(array)', snippet: 'LBOUND(${1:array})' },
    { name: 'UBOUND', description: 'Returns upper bound of array', signature: 'UBOUND(array)', snippet: 'UBOUND(${1:array})' },
    { name: 'QUICKSORT', description: 'Sorts an array in place', signature: 'QUICKSORT(array)', snippet: 'QUICKSORT(${1:array})' },
    { name: 'INITARRAY', description: 'Initializes array elements', signature: 'INITARRAY(array, value)', snippet: 'INITARRAY(${1:array}, ${2:value})' },
    { name: 'SWAP', description: 'Swaps two variables', signature: 'SWAP(a, b)', snippet: 'SWAP(${1:a}, ${2:b})' },
    { name: 'REDIM', description: 'Resizes an array', signature: 'REDIM(array, newSize)', snippet: 'REDIM(${1:array}, ${2:newSize})' },

    // VARPTR
    { name: 'VARPTR', description: 'The address of a variable or array element (for MEMCPY, MEMSET, VARPTR$, DLL calls); memory-safe', signature: 'VARPTR(variable)', snippet: 'VARPTR(${1:variable})' },
    { name: 'VARPTR$', description: 'The text at an address, up to a NUL character', signature: 'VARPTR$(address)', snippet: 'VARPTR\\$(${1:address})' },
    { name: 'UDTPTR', description: 'The address of a TYPE variable', signature: 'UDTPTR(udt)', snippet: 'UDTPTR(${1:udt})' },
    { name: 'MEMCPY', description: 'Copies n bytes from one address to another (a TYPE variable is its own address)', signature: 'MEMCPY(destination, source, n)', snippet: 'MEMCPY(${1:dest}, ${2:source}, ${3:n})' },
    { name: 'MEMSET', description: 'Fills n bytes at an address with a byte value', signature: 'MEMSET(address, byte, n)', snippet: 'MEMSET(${1:address}, ${2:0}, ${3:n})' },
    { name: 'MEMCMP', description: 'Compares n bytes at two addresses: non-zero when they are the same', signature: 'MEMCMP(address1, address2, n)', snippet: 'MEMCMP(${1:a}, ${2:b}, ${3:n})' },
    { name: 'SIZEOF', description: 'The size in bytes of a type (INTEGER, a TYPE) or a variable (a STRING: its length)', signature: 'SIZEOF(type | variable)', snippet: 'SIZEOF(${1:INTEGER})' },
    { name: 'RTLMOVEMEMORY', description: 'Copies n bytes from one variable to another (by reference)', signature: 'RTLMOVEMEMORY(dest, source, n)', snippet: 'RTLMOVEMEMORY(${1:dest}, ${2:source}, ${3:n})' },
    { name: 'VARTYPE', description: 'Returns type of variable', signature: 'VARTYPE(variable)', snippet: 'VARTYPE(${1:variable})' },
];

const KEYWORDS = [
    'DIM', 'AS', 'IF', 'THEN', 'ELSE', 'ELSEIF', 'END IF', 'FOR', 'TO', 'STEP', 'NEXT',
    'WHILE', 'WEND', 'DO', 'LOOP', 'UNTIL', 'SELECT CASE', 'CASE', 'CASE ELSE', 'END SELECT',
    'SUB', 'END SUB', 'FUNCTION', 'END FUNCTION', 'CALL', 'RETURN', 'EXIT FOR', 'EXIT WHILE',
    'EXIT DO', 'EXIT SUB', 'EXIT FUNCTION', 'PRINT', 'INPUT', 'GOTO', 'GOSUB',
    'IMPORT', 'CREATE', 'END CREATE', 'CONST', 'TYPE', 'END TYPE', 'DECLARE', 'LIB', 'ALIAS',
    'WITH', 'END WITH', 'EXTENDS', 'PROPERTY', 'SET', 'BYVAL', 'BYREF', 'BIND', 'CONSTRUCTOR',
    'END CONSTRUCTOR', 'PRIVATE', 'PUBLIC', 'AND', 'OR', 'NOT', 'XOR', 'MOD',
    'TRUE', 'FALSE', 'NOTHING', 'REM'
];

const TYPE_KEYWORDS = [
    { name: 'INTEGER', description: 'Integer number (int)' },
    { name: 'LONG', description: 'Long integer (int)' },
    { name: 'INT64', description: '64-bit integer (int)' },
    { name: 'BYTE', description: '8-bit unsigned integer' },
    { name: 'WORD', description: '16-bit unsigned integer' },
    { name: 'DWORD', description: '32-bit unsigned integer' },
    { name: 'SINGLE', description: 'Single-precision float' },
    { name: 'DOUBLE', description: 'Double-precision float' },
    { name: 'CURRENCY', description: 'Currency (float)' },
    { name: 'STRING', description: 'Text string (str)' },
    { name: 'VARIANT', description: 'Any type (None)' },
    { name: 'POBJECT', description: 'Object reference (None)' }
];

const DIRECTIVES = [
    { name: 'APPTYPE', description: 'Set application type: GUI, CONSOLE, CGI, or WEB', snippet: 'APPTYPE ${1|GUI,CONSOLE,CGI,WEB|}' },
    { name: 'INCLUDE', description: 'Include an external source file', snippet: 'INCLUDE "${1:filename.rr}"' },
    { name: 'DEFINE', description: 'Define a text substitution macro', snippet: 'DEFINE ${1:SYMBOL} ${2:value}' },
    { name: 'UNDEF', description: 'Remove a defined symbol', snippet: 'UNDEF ${1:SYMBOL}' },
    { name: 'IFDEF', description: 'Conditional: compile if symbol is defined', snippet: 'IFDEF ${1:SYMBOL}' },
    { name: 'IFNDEF', description: 'Conditional: compile if symbol is NOT defined', snippet: 'IFNDEF ${1:SYMBOL}' },
    { name: 'ELSE', description: 'Else branch of conditional compilation', snippet: 'ELSE' },
    { name: 'ENDIF', description: 'End conditional compilation block', snippet: 'ENDIF' },
    { name: 'MACRO', description: 'Define a parameterized macro', snippet: 'MACRO ${1:NAME}(${2:params}) = ${3:body}' },
    { name: 'TYPECHECK', description: 'Enable/disable strict type checking', snippet: 'TYPECHECK ${1|ON,OFF|}' },
    { name: 'OPTION', description: 'Set compiler option', snippet: 'OPTION ${1|EXPLICIT,DIM|} ${2}' },
    { name: 'OPTIMIZE', description: 'Optimization hint (pass-through)', snippet: 'OPTIMIZE ${1|ON,OFF|}' },
    { name: 'ESCAPECHARS', description: 'Escape character mode (pass-through)', snippet: 'ESCAPECHARS ${1|ON,OFF|}' }
];

export { COMPONENT_REGISTRY, BUILTIN_FUNCTIONS, KEYWORDS, TYPE_KEYWORDS, DIRECTIVES };

// Pretty display name for an upper-case component key (RBUTTON -> RButton).
const _NAME_MAP = {
  RFORM:"RForm", RBUTTON:"RButton", RLABEL:"RLabel", REDIT:"REdit",
  RMEMO:"RMemo", RUPDOWN:"RUpDown", RDATETIMEPICKER:"RDateTimePicker", RTOOLBAR:"RToolBar",
  RCANVAS:"RCanvas", RPANEL:"RPanel", RTIMER:"RTimer",
  RMAINMENU:"RMainMenu", RMENUITEM:"RMenuItem", RCOMBOBOX:"RComboBox",
  RLISTBOX:"RListBox", RCHECKBOX:"RCheckBox", RRADIOBUTTON:"RRadioButton",
  RRICHEDIT:"RRichEdit", RSTRINGGRID:"RStringGrid", RIMAGE:"RImage",
  RSCROLLBAR:"RScrollBar", RTABCONTROL:"RTabControl",
  RGROUPBOX:"RGroupBox", RMYSQL:"RMySQL", RSQLITE:"RSQLite",
  RPROGRESSBAR:"RProgressBar", RLISTVIEW:"RListView",
  ROPENDIALOG:"ROpenDialog", RSAVEDIALOG:"RSaveDialog",
  RFILESTREAM:"RFileStream", RFILEDIALOG:"RFileDialog",
  RCODEEDITOR:"RCodeEditor", RLINE:"RLine", RICON:"RIcon",
  RIMAGELIST:"RImageList", RBITMAP:"RBitmap", RFONT:"RFont", RSOCKET:"RSocket",
  RSERVERSOCKET:"RServerSocket", RHTTP:"RHttp",
  RSTATUSBAR:"RStatusBar", RCOLORDIALOG:"RColorDialog",
  RFONTDIALOG:"RFontDialog", RDESIGNSURFACE:"RDesignSurface",
  RTREEVIEW:"RTreeView", RSPLITTER:"RSplitter", RTRACKBAR:"RTrackBar",
  RSCROLLBOX:"RScrollBox", RPOPUPMENU:"RPopupMenu",
  RINI:"RIni", RMEMORYSTREAM:"RMemoryStream", RSTRINGLIST:"RStringList",
  RPRINTER:"RPrinter", RNUM:"RNum", RPLOT:"RPlot",
  RDATAFRAME:"RDataFrame", RJSON:"RJson",
  RWEBVIEW:"RWebView", RDOM:"RDOM", RJAVASCRIPT:"RJavaScript",
  RWEBSTORAGE:"RWebStorage", RWEBAUDIO:"RWebAudio", RWEBVIDEO:"RWebVideo",
  RWEBNOTIFICATION:"RWebNotification", RWEBGEOLOCATION:"RWebGeolocation",
  RROUTER:"RRouter", RCOOLBTN:"RCoolBtn", ROVALBTN:"ROvalBtn",
};
export function prettyComponentName(upper) {
  if (!upper) return upper;
  const u = String(upper).toUpperCase();
  return _NAME_MAP[u] || (u[0] + u.slice(1).toLowerCase());
}

// Heuristic variable→type resolver. Scans DIM/CREATE/AS lines.
export function resolveVariableType(text, varName) {
  if (!text || !varName) return null;
  const v = varName.replace(/[.\[\]()$#%&!]+$/, "");
  // CREATE Foo AS RButton
  let re = new RegExp("\\bCREATE\\s+" + v + "\\s+AS\\s+(\\w+)", "i");
  let m = re.exec(text);
  if (m) return m[1].toUpperCase();
  // DIM Foo AS RButton
  re = new RegExp("\\bDIM\\s+" + v + "\\s+AS\\s+(\\w+)", "i");
  m = re.exec(text);
  if (m) return m[1].toUpperCase();
  // GLOBAL Foo AS RButton
  re = new RegExp("\\bGLOBAL\\s+" + v + "\\s+AS\\s+(\\w+)", "i");
  m = re.exec(text);
  if (m) return m[1].toUpperCase();
  return null;
}
