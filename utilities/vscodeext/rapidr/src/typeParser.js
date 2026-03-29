/**
 * Parses user-defined TYPE blocks from a RapidR document and returns
 * a registry of type names → { fields, methods, functions, extends }.
 *
 * Handles:
 *   - Fields:       Name AS Type,  Name(size) AS Type
 *   - SUBs:         SUB Name(params) ... END SUB
 *   - FUNCTIONs:    FUNCTION Name(params) AS ReturnType ... END FUNCTION
 *   - Inheritance:  TYPE Child EXTENDS Parent
 *   - CONSTRUCTOR:  CONSTRUCTOR(params) ... END CONSTRUCTOR
 *   - PROPERTY:     PROPERTY Name AS Type = value
 */

/**
 * Parse all TYPE...END TYPE blocks in the given document text.
 * @param {string} text - Full document text
 * @returns {Object.<string, {fields: Array, methods: Array, functions: Array, extends: string|null}>}
 *   Keys are uppercase type names. Each entry has:
 *     fields:    [{ name, type, isArray }]
 *     methods:   [{ name, params, kind }]  — kind is 'SUB', 'FUNCTION', or 'CONSTRUCTOR'
 *     functions: [{ name, params, returnType }]
 *     extends:   parent type name (uppercase) or null
 */
function parseUserTypes(text) {
    const types = {};
    const lines = text.split('\n');
    let i = 0;

    while (i < lines.length) {
        const line = lines[i].trim();
        // Match: TYPE Name  or  TYPE Name EXTENDS Parent
        const typeMatch = line.match(/^TYPE\s+([A-Za-z_]\w*)(?:\s+EXTENDS\s+([A-Za-z_]\w*))?\s*$/i);
        if (typeMatch && !line.toUpperCase().startsWith('TYPECHECK')) {
            const typeName = typeMatch[1].toUpperCase();
            const extendsName = typeMatch[2] ? typeMatch[2].toUpperCase() : null;
            const entry = { fields: [], methods: [], functions: [], extends: extendsName };

            i++;
            while (i < lines.length) {
                const inner = lines[i].trim();
                const upperInner = inner.toUpperCase();

                // End of TYPE block
                if (/^END\s+TYPE\b/i.test(inner)) {
                    break;
                }

                // SUB member
                const subMatch = inner.match(/^SUB\s+([A-Za-z_]\w*)\s*(?:\(([^)]*)\))?\s*$/i);
                if (subMatch) {
                    entry.methods.push({
                        name: subMatch[1],
                        params: subMatch[2] ? subMatch[2].trim() : '',
                        kind: 'SUB'
                    });
                    // Skip to END SUB
                    i++;
                    while (i < lines.length && !/^\s*END\s+SUB\b/i.test(lines[i])) i++;
                    i++;
                    continue;
                }

                // FUNCTION member
                const funcMatch = inner.match(/^FUNCTION\s+([A-Za-z_]\w*)\s*(?:\(([^)]*)\))?\s*(?:AS\s+(\w+))?\s*$/i);
                if (funcMatch) {
                    entry.functions.push({
                        name: funcMatch[1],
                        params: funcMatch[2] ? funcMatch[2].trim() : '',
                        returnType: funcMatch[3] || 'VARIANT'
                    });
                    // Also add to methods so it shows in dot-completion
                    entry.methods.push({
                        name: funcMatch[1],
                        params: funcMatch[2] ? funcMatch[2].trim() : '',
                        kind: 'FUNCTION'
                    });
                    // Skip to END FUNCTION
                    i++;
                    while (i < lines.length && !/^\s*END\s+FUNCTION\b/i.test(lines[i])) i++;
                    i++;
                    continue;
                }

                // CONSTRUCTOR
                const ctorMatch = inner.match(/^CONSTRUCTOR\s*(?:\(([^)]*)\))?\s*$/i);
                if (ctorMatch) {
                    entry.methods.push({
                        name: 'CONSTRUCTOR',
                        params: ctorMatch[1] ? ctorMatch[1].trim() : '',
                        kind: 'CONSTRUCTOR'
                    });
                    // Skip to END CONSTRUCTOR
                    i++;
                    while (i < lines.length && !/^\s*END\s+CONSTRUCTOR\b/i.test(lines[i])) i++;
                    i++;
                    continue;
                }

                // PROPERTY Name AS Type [= default]
                const propMatch = inner.match(/^PROPERTY\s+([A-Za-z_]\w*)\s+AS\s+(\w+)/i);
                if (propMatch) {
                    entry.fields.push({
                        name: propMatch[1],
                        type: propMatch[2].toUpperCase(),
                        isArray: false
                    });
                    i++;
                    continue;
                }

                // Field: Name AS Type  or  Name(size) AS Type
                const fieldMatch = inner.match(/^([A-Za-z_]\w*)(\([^)]*\))?\s+AS\s+(\w+)/i);
                if (fieldMatch) {
                    entry.fields.push({
                        name: fieldMatch[1],
                        type: fieldMatch[3].toUpperCase(),
                        isArray: !!fieldMatch[2]
                    });
                    i++;
                    continue;
                }

                i++;
            }

            types[typeName] = entry;
        }
        i++;
    }

    // Resolve inheritance: merge parent fields/methods into children
    for (const [name, entry] of Object.entries(types)) {
        if (entry.extends && types[entry.extends]) {
            const parent = types[entry.extends];
            // Prepend parent fields that aren't overridden
            const childFieldNames = new Set(entry.fields.map(f => f.name.toUpperCase()));
            for (const pf of parent.fields) {
                if (!childFieldNames.has(pf.name.toUpperCase())) {
                    entry.fields.unshift(pf);
                }
            }
            // Prepend parent methods that aren't overridden
            const childMethodNames = new Set(entry.methods.map(m => m.name.toUpperCase()));
            for (const pm of parent.methods) {
                if (!childMethodNames.has(pm.name.toUpperCase())) {
                    entry.methods.unshift(pm);
                }
            }
            const childFuncNames = new Set(entry.functions.map(f => f.name.toUpperCase()));
            for (const pf of parent.functions) {
                if (!childFuncNames.has(pf.name.toUpperCase())) {
                    entry.functions.unshift(pf);
                }
            }
        }
    }

    return types;
}

/**
 * Resolve the type name for a variable from DIM/CREATE statements.
 * Returns the type name (uppercase) or null.
 * @param {string} text - Full document text
 * @param {string} varName - Variable name to look up
 * @returns {string|null}
 */
function resolveVariableType(text, varName) {
    const escaped = varName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    // CREATE varName AS Type
    const createRe = new RegExp(`\\bCREATE\\s+${escaped}\\s+AS\\s+(\\w+)`, 'i');
    const m1 = text.match(createRe);
    if (m1) return m1[1].toUpperCase();
    // DIM varName AS Type
    const dimRe = new RegExp(`\\bDIM\\s+${escaped}\\s+AS\\s+(\\w+)`, 'i');
    const m2 = text.match(dimRe);
    if (m2) return m2[1].toUpperCase();
    return null;
}

module.exports = { parseUserTypes, resolveVariableType };
