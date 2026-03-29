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
        methods: ['cls', 'pset', 'line', 'circle', 'fillrect', 'rectangle', 'textout', 'drawtext',
                  'paint', 'repaint', 'refresh'],
        events: ['onclick', 'ondblclick', 'onmousedown', 'onmouseup', 'onmousemove', 'onpaint']
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
        props: ['cols', 'rows', 'fixedcols', 'fixedrows', 'colcount', 'rowcount', 'colwidth',
                'rowheight', 'gridlinewidth', 'defaultcolwidth', 'defaultrowheight', 'width',
                'height', 'top', 'left', 'visible', 'enabled', 'color', 'font', 'fontsize',
                'fontcolor', 'scrollbars', 'options', 'col', 'row', 'selectedrow', 'hint', 'showhint', 'cursor',
                'tag', 'parent', 'editorenabled', 'borderstyle', 'flat', 'cell', 'cells', 'rowsel', 'colsel'],
        methods: ['addrow', 'deleterow', 'insertrow', 'clear', 'setcell', 'getcell',
                  'setcolwidth', 'repaint', 'refresh', 'setfocus', 'setsuggestions'],
        events: ['onclick', 'ondblclick', 'onselectcell', 'ondrawcell', 'onchange', 'onrowselect']
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
        props: ['color', 'tag'],
        methods: ['execute'],
        events: []
    },
    'RFONTDIALOG': {
        props: ['fontname', 'fontsize', 'fontcolor', 'fontstyle', 'tag'],
        methods: ['execute'],
        events: []
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
        props: ['count', 'width', 'height', 'tag'],
        methods: ['addimage', 'addimages', 'clear'],
        events: []
    },
    'RMYSQL': {
        props: ['host', 'user', 'password', 'database', 'port', 'connected', 'rowcount',
                'colcount', 'fieldcount', 'fieldname', 'row', 'dbcount', 'db', 'tablecount',
                'table', 'escapestring', 'tag'],
        methods: ['connect', 'open', 'close', 'query', 'fetchrow', 'fetchfield', 'use',
                  'selectdb', 'rowseek', 'fieldseek', 'createdb', 'dropdb'],
        events: ['onconnect', 'ondisconnect', 'onerror', 'onquerydone']
    },
    'RSQLITE': {
        props: ['database', 'db', 'connected', 'rowcount', 'colcount', 'fieldcount',
                'fieldname', 'row', 'tablecount', 'table', 'dbcount', 'tag'],
        methods: ['connect', 'close', 'query', 'fetchrow', 'fetchfield', 'rowseek', 'fieldseek'],
        events: ['onconnect', 'ondisconnect', 'onerror', 'onquerydone']
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
        props: ['position', 'size', 'tag'],
        methods: ['write', 'read', 'readbyte', 'writebyte', 'savetofile', 'loadfromfile', 'clear', 'copyto'],
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
    'RNUMPY': {
        props: ['data', 'shape', 'size', 'dtype', 'tag'],
        methods: ['zeros', 'ones', 'arange', 'linspace', 'reshape', 'sum', 'mean', 'std',
                  'min', 'max', 'dot', 'transpose', 'tolist', 'sort', 'savetofile', 'loadfromfile'],
        events: []
    },
    'RMATPLOTLIB': {
        props: ['title', 'xlabel', 'ylabel', 'width', 'height', 'grid', 'tag'],
        methods: ['plot', 'scatter', 'bar', 'hist', 'pie', 'legend', 'clear',
                  'savetofile', 'show', 'saveto_buffer'],
        events: []
    },
    'RPANDAS': {
        props: ['data', 'rowcount', 'colcount', 'columns', 'tag'],
        methods: ['loadfromcsv', 'savetocsv', 'loadfromjson', 'savetojson', 'head', 'tail',
                  'describe', 'sort', 'filter', 'groupby', 'addcolumn', 'deletecolumn',
                  'cell', 'setcell', 'query', 'tostring', 'tolist'],
        events: []
    }
};

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
    { name: 'INSERT', description: 'Inserts string at position', signature: 'INSERT(str, pos, text)', snippet: 'INSERT(${1:str}, ${2:pos}, ${3:text})' },
    { name: 'DELETE', description: 'Deletes characters from string', signature: 'DELETE(str, pos, count)', snippet: 'DELETE(${1:str}, ${2:pos}, ${3:count})' },
    { name: 'REVERSE', description: 'Reverses a string', signature: 'REVERSE(str)', snippet: 'REVERSE(${1:str})' },
    { name: 'FIELD', description: 'Returns nth field from delimited string', signature: 'FIELD(str, delimiter, n)', snippet: 'FIELD(${1:str}, ${2:delim}, ${3:n})' },
    { name: 'TALLY', description: 'Counts occurrences of substring', signature: 'TALLY(str, search)', snippet: 'TALLY(${1:str}, ${2:search})' },
    { name: 'STRF', description: 'Formatted string conversion', signature: 'STRF(number, format)', snippet: 'STRF(${1:number}, ${2:format})' },

    // Math functions
    { name: 'ABS', description: 'Returns absolute value', signature: 'ABS(number)', snippet: 'ABS(${1:number})' },
    { name: 'ATN', description: 'Returns arctangent (radians)', signature: 'ATN(number)', snippet: 'ATN(${1:number})' },
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
    { name: 'FORMAT$', description: 'Formats number/date', signature: 'FORMAT$(value, formatStr)', snippet: 'FORMAT\\$(${1:value}, ${2:format})' },

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
    { name: 'SLEEP', description: 'Pauses execution (milliseconds)', signature: 'SLEEP(ms)', snippet: 'SLEEP(${1:ms})' },
    { name: 'TIMER', description: 'Returns seconds since midnight', signature: 'TIMER', snippet: 'TIMER' },
    { name: 'DATE$', description: 'Returns current date string', signature: 'DATE$', snippet: 'DATE\\$' },
    { name: 'TIME$', description: 'Returns current time string', signature: 'TIME$', snippet: 'TIME\\$' },
    { name: 'COMMAND$', description: 'Returns command line arguments', signature: 'COMMAND$', snippet: 'COMMAND\\$' },
    { name: 'ENVIRON$', description: 'Returns environment variable', signature: 'ENVIRON$(name)', snippet: 'ENVIRON\\$(${1:name})' },
    { name: 'DOEVENTS', description: 'Processes pending GUI events', signature: 'DOEVENTS', snippet: 'DOEVENTS' },
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
    { name: 'VARPTR', description: 'Returns reference to variable', signature: 'VARPTR(variable)', snippet: 'VARPTR(${1:variable})' },
    { name: 'VARPTR$', description: 'Returns string representation of reference', signature: 'VARPTR$(variable)', snippet: 'VARPTR\\$(${1:variable})' },
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
    { name: 'APPTYPE', description: 'Set application type: GUI, CONSOLE, or CGI', snippet: 'APPTYPE ${1|GUI,CONSOLE,CGI|}' },
    { name: 'INCLUDE', description: 'Include an external source file', snippet: 'INCLUDE "${1:filename.rp}"' },
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

module.exports = { COMPONENT_REGISTRY, BUILTIN_FUNCTIONS, KEYWORDS, TYPE_KEYWORDS, DIRECTIVES };
