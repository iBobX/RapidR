const vscode = require('vscode');
const { BUILTIN_FUNCTIONS } = require('./languageData');

class RapidRSignatureHelpProvider {
    provideSignatureHelp(document, position) {
        const lineText = document.lineAt(position.line).text;
        const textBefore = lineText.substring(0, position.character);

        // Walk backwards to find the function name and count commas for active parameter
        let depth = 0;
        let commaCount = 0;
        let funcEnd = -1;

        for (let i = textBefore.length - 1; i >= 0; i--) {
            const ch = textBefore[i];
            if (ch === ')') {
                depth++;
            } else if (ch === '(') {
                if (depth === 0) {
                    funcEnd = i;
                    break;
                }
                depth--;
            } else if (ch === ',' && depth === 0) {
                commaCount++;
            }
        }

        if (funcEnd < 0) return null;

        // Extract function name (may include $)
        const before = textBefore.substring(0, funcEnd);
        const nameMatch = before.match(/([A-Za-z_]\w*\$?)\s*$/);
        if (!nameMatch) return null;

        const funcName = nameMatch[1].toUpperCase();

        // Find matching builtin
        const builtin = BUILTIN_FUNCTIONS.find(f => f.name === funcName);
        if (!builtin) return null;

        const sig = new vscode.SignatureInformation(builtin.signature, builtin.description);

        // Parse parameters from the signature
        const paramMatch = builtin.signature.match(/\(([^)]*)\)/);
        if (paramMatch) {
            const params = paramMatch[1].split(',').map(p => p.trim()).filter(p => p);
            for (const p of params) {
                sig.parameters.push(new vscode.ParameterInformation(p));
            }
        }

        const help = new vscode.SignatureHelp();
        help.signatures = [sig];
        help.activeSignature = 0;
        help.activeParameter = Math.min(commaCount, sig.parameters.length - 1);

        return help;
    }
}

module.exports = { RapidRSignatureHelpProvider };
