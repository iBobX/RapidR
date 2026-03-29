const vscode = require('vscode');

class RapidRDocumentSymbolProvider {
    provideDocumentSymbols(document) {
        const symbols = [];
        const lines = document.getText().split('\n');

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            const trimmed = line.trim().toUpperCase();

            // SUB Name(...)
            const subMatch = line.match(/^\s*SUB\s+([A-Za-z_]\w*)/i);
            if (subMatch) {
                const endLine = this._findBlockEnd(lines, i, /^\s*END\s+SUB\b/i);
                symbols.push(this._makeSymbol(
                    subMatch[1], vscode.SymbolKind.Function,
                    'SUB', document, i, endLine
                ));
                continue;
            }

            // FUNCTION Name(...)
            const funcMatch = line.match(/^\s*FUNCTION\s+([A-Za-z_]\w*)/i);
            if (funcMatch) {
                const endLine = this._findBlockEnd(lines, i, /^\s*END\s+FUNCTION\b/i);
                symbols.push(this._makeSymbol(
                    funcMatch[1], vscode.SymbolKind.Function,
                    'FUNCTION', document, i, endLine
                ));
                continue;
            }

            // TYPE Name
            const typeMatch = line.match(/^\s*TYPE\s+([A-Za-z_]\w*)/i);
            if (typeMatch && !trimmed.startsWith('TYPECHECK')) {
                const endLine = this._findBlockEnd(lines, i, /^\s*END\s+TYPE\b/i);
                symbols.push(this._makeSymbol(
                    typeMatch[1], vscode.SymbolKind.Struct,
                    'TYPE', document, i, endLine
                ));
                continue;
            }

            // CREATE name AS PType
            const createMatch = line.match(/^\s*CREATE\s+([A-Za-z_]\w*)\s+AS\s+(\w+)/i);
            if (createMatch) {
                const endLine = this._findBlockEnd(lines, i, /^\s*END\s+CREATE\b/i);
                symbols.push(this._makeSymbol(
                    createMatch[1], vscode.SymbolKind.Object,
                    createMatch[2], document, i, endLine
                ));
                continue;
            }

            // CONST name = value
            const constMatch = line.match(/^\s*CONST\s+([A-Za-z_]\w*)\s*=/i);
            if (constMatch) {
                symbols.push(this._makeSymbol(
                    constMatch[1], vscode.SymbolKind.Constant,
                    'CONST', document, i, i
                ));
                continue;
            }

            // DIM name AS type (top-level only — rough heuristic: not indented)
            const dimMatch = line.match(/^DIM\s+([A-Za-z_]\w*)\s+AS\s+(\w+)/i);
            if (dimMatch) {
                symbols.push(this._makeSymbol(
                    dimMatch[1], vscode.SymbolKind.Variable,
                    dimMatch[2], document, i, i
                ));
            }
        }

        return symbols;
    }

    _findBlockEnd(lines, startLine, endPattern) {
        for (let i = startLine + 1; i < lines.length; i++) {
            if (endPattern.test(lines[i])) return i;
        }
        return startLine;
    }

    _makeSymbol(name, kind, detail, document, startLine, endLine) {
        const range = new vscode.Range(startLine, 0, endLine, lines_len(document, endLine));
        const selRange = new vscode.Range(startLine, 0, startLine, lines_len(document, startLine));
        const sym = new vscode.DocumentSymbol(name, detail, kind, range, selRange);
        return sym;
    }
}

function lines_len(document, line) {
    if (line < document.lineCount) {
        return document.lineAt(line).text.length;
    }
    return 0;
}

module.exports = { RapidRDocumentSymbolProvider };
