"""Habits from other languages, injected into a valid RapidR program. The compiler
(RapidR's own check_code) says what each one does; a mutation that the compiler
accepts is dropped. The fix is the original line(s).

What small models write instead of RapidR (tools/ai_eval report 2026-10-29):
SET, +=, %, for ... in, braces, ELSE IF, a missing END IF, print(), !=, ===.
"""

from __future__ import annotations

import random
import re
from dataclasses import dataclass


@dataclass
class Mutation:
    name: str
    text: str  # the broken program
    old: str  # the broken line(s), as they are in `text` (whole lines, joined by \n)
    new: str  # the original line(s) that fix them
    context: str  # for an edit: `old` plus the line before/after when `old` has to be unique


# the one-line reason, per language: what RapidR wants instead
WHY = {
    "set_keyword": ("RapidR assigns without SET: write `x = 1`.", "En RapidR se asigna sin SET: escribe `x = 1`.", "No RapidR a atribuição não usa SET: escreva `x = 1`."),
    "plus_equals": ("RapidR has no `+=`: write `x = x + 1`.", "RapidR no tiene `+=`: escribe `x = x + 1`.", "O RapidR não tem `+=`: escreva `x = x + 1`."),
    "percent_mod": ("The remainder is `MOD`, not `%`.", "El resto se escribe `MOD`, no `%`.", "O resto se escreve `MOD`, não `%`."),
    "neq_bang": ("Not-equal is `<>`, not `!=`.", "Distinto se escribe `<>`, no `!=`.", "Diferente se escreve `<>`, não `!=`."),
    "double_equals": ("Comparison uses a single `=`: `IF a = b THEN`.", "La comparación usa un solo `=`: `IF a = b THEN`.", "A comparação usa um único `=`: `IF a = b THEN`."),
    "and_symbol": ("Combine conditions with `AND` and `OR`, not `&&` and `||`.", "Las condiciones se combinan con `AND` y `OR`, no con `&&` y `||`.", "As condições se combinam com `AND` e `OR`, não com `&&` e `||`."),
    "else_if": ("It is one word: `ELSEIF`.", "Se escribe en una palabra: `ELSEIF`.", "Escreve-se em uma palavra: `ELSEIF`."),
    "missing_end_if": ("Every block `IF … THEN` ends with `END IF`.", "Cada bloque `IF … THEN` termina con `END IF`.", "Cada bloco `IF … THEN` termina com `END IF`."),
    "for_in_range": ("A loop is `FOR i = 1 TO n … NEXT i`, not `for … in range`.", "Un bucle es `FOR i = 1 TO n … NEXT i`, no `for … in range`.", "Um laço é `FOR i = 1 TO n … NEXT i`, não `for … in range`."),
    "brace_if": ("A block `IF` ends with `THEN` and `END IF`, not braces.", "Un bloque `IF` termina con `THEN` y `END IF`, no con llaves.", "Um bloco `IF` termina com `THEN` e `END IF`, não com chaves."),
    "print_parens": ("`PRINT` is a statement: `PRINT x`, not `print(x)`.", "`PRINT` es una instrucción: `PRINT x`, no `print(x)`.", "`PRINT` é uma instrução: `PRINT x`, não `print(x)`."),
    "console_log": ("Output is `PRINT x`, not `console.log(x)`.", "La salida es `PRINT x`, no `console.log(x)`.", "A saída é `PRINT x`, não `console.log(x)`."),
    "increment": ("There is no `++`: write `i = i + 1` (or `INC i`).", "No existe `++`: escribe `i = i + 1` (o `INC i`).", "Não existe `++`: escreva `i = i + 1` (ou `INC i`)."),
    "comment_slash": ("A comment starts with an apostrophe `'`, not `//`.", "Un comentario empieza con un apóstrofo `'`, no con `//`.", "Um comentário começa com um apóstrofo `'`, não com `//`."),
    "var_decl": ("A variable is declared `DIM n AS INTEGER`, not `int n;`.", "Una variable se declara `DIM n AS INTEGER`, no `int n;`.", "Uma variável se declara `DIM n AS INTEGER`, não `int n;`."),
    "not_bang": ("Negation is `NOT x`, not `!x`.", "La negación es `NOT x`, no `!x`.", "A negação é `NOT x`, não `!x`."),
    "then_missing": ("An `IF` needs `THEN`.", "Un `IF` necesita `THEN`.", "Um `IF` precisa de `THEN`."),
    "semicolon_end": ("Statements end at the end of the line: no `;`.", "Las instrucciones terminan con el fin de línea: sin `;`.", "As instruções terminam no fim da linha: sem `;`."),
    "array_brackets": ("An array item is `v(i)`, not `v[i]`.", "Un elemento de un arreglo es `v(i)`, no `v[i]`.", "Um item de um array é `v(i)`, não `v[i]`."),
    "next_missing": ("Every `FOR` ends with `NEXT`.", "Cada `FOR` termina con `NEXT`.", "Cada `FOR` termina com `NEXT`."),
    "return_value": ("A FUNCTION returns by assigning to its own name: `Name = value`.", "Una FUNCTION devuelve asignando a su propio nombre: `Nombre = valor`.", "Uma FUNCTION retorna atribuindo ao próprio nome: `Nome = valor`."),
}
# what to look up when the same error comes twice (the manual's words)
LOOKUP = {
    "set_keyword": "assign a value to a variable",
    "plus_equals": "add to a variable in a loop",
    "percent_mod": "remainder MOD operator",
    "neq_bang": "compare two values not equal",
    "double_equals": "IF condition compare equal",
    "and_symbol": "AND OR conditions",
    "else_if": "IF ELSEIF ELSE block",
    "missing_end_if": "IF THEN END IF block",
    "for_in_range": "FOR NEXT loop",
    "brace_if": "IF THEN END IF block",
    "print_parens": "PRINT statement output",
    "console_log": "PRINT statement output",
    "increment": "add one to a variable INC",
    "comment_slash": "comments in a program",
    "var_decl": "DIM declare a variable AS INTEGER",
    "not_bang": "NOT operator",
    "then_missing": "IF THEN block",
    "semicolon_end": "statements one per line",
    "array_brackets": "arrays DIM item index",
    "next_missing": "FOR NEXT loop",
    "return_value": "FUNCTION return a value",
}


def _outside_create(lines: list[str]) -> list[bool]:
    """For each line: is it outside every CREATE … END CREATE block (so it is a statement)?"""
    depth = 0
    out = []
    for l in lines:
        s = l.strip().upper()
        if re.match(r"^CREATE\b", s):
            out.append(False)
            depth += 1
        elif s.startswith("END CREATE"):
            depth = max(0, depth - 1)
            out.append(False)
        else:
            out.append(depth == 0)
    return out


def _unique(text: str, old: str) -> bool:
    return text.count(old) == 1


def all_mutations(text: str, rng: random.Random) -> list[Mutation]:
    """Every way the rules below break `text` once (the caller keeps the ones the compiler refuses)."""
    lines = text.split("\n")
    outside = _outside_create(lines)
    out: list[Mutation] = []

    def replace_line(k: int, new_line: str, name: str) -> None:
        broken = list(lines)
        broken[k] = new_line
        out.append(Mutation(name, "\n".join(broken), new_line, lines[k], new_line))

    def code_line(k: int) -> bool:
        s = lines[k].strip()
        return bool(s) and not s.startswith("'") and not s.upper().startswith(("DECLARE", "CREATE", "END ", "$"))

    for k, l in enumerate(lines):
        s = l.strip()
        indent = l[: len(l) - len(l.lstrip())]
        if not code_line(k):
            if s.startswith("'") and k > 0 and len(s) > 12:
                replace_line(k, indent + "//" + s[1:], "comment_slash")
            continue
        m = re.match(r"^(\s*)([A-Za-z_]\w*(?:\.\w+)?)\s*=\s*(.+)$", l)
        if m and outside[k] and not re.match(r"^\s*(IF|ELSEIF|FOR|CASE|DIM|CONST|SELECT)\b", l, re.I):
            replace_line(k, f"{m.group(1)}SET {m.group(2)} = {m.group(3)}", "set_keyword")
        m = re.match(r"^(\s*)(\w+) = \2 \+ (.+)$", l)
        if m:
            replace_line(k, f"{m.group(1)}{m.group(2)} += {m.group(3)}", "plus_equals")
            if m.group(3).strip() == "1":
                replace_line(k, f"{m.group(1)}{m.group(2)}++", "increment")
        if " MOD " in l and not s.startswith("'"):
            replace_line(k, l.replace(" MOD ", " % ", 1), "percent_mod")
        if " <> " in l:
            replace_line(k, l.replace(" <> ", " != ", 1), "neq_bang")
        if re.match(r"^\s*(IF|ELSEIF)\b.* = .* THEN", l, re.I) and " <> " not in l and " >= " not in l and " <= " not in l:
            replace_line(k, re.sub(r"( [^=<>\s]+) = ", r"\1 == ", l, count=1), "double_equals")
        if re.match(r"^\s*(IF|ELSEIF)\b", l, re.I) and " AND " in l:
            replace_line(k, l.replace(" AND ", " && ", 1), "and_symbol")
        if re.match(r"^\s*(IF|ELSEIF)\b", l, re.I) and " OR " in l:
            replace_line(k, l.replace(" OR ", " || ", 1), "and_symbol")
        if re.match(r"^\s*ELSEIF\b", l, re.I):
            replace_line(k, re.sub(r"ELSEIF", "ELSE IF", l, count=1, flags=re.I), "else_if")
        m = re.match(r"^(\s*)FOR (\w+) = (\S+) TO (\S+)\s*$", l, re.I)
        if m:
            replace_line(k, f"{m.group(1)}for {m.group(2)} in range({m.group(3)}, {m.group(4)} + 1):", "for_in_range")
        m = re.match(r"^(\s*)(IF .+) THEN\s*$", l, re.I)
        if m:
            replace_line(k, f"{m.group(1)}{m.group(2)} {{", "brace_if")
            replace_line(k, f"{m.group(1)}{m.group(2)}", "then_missing")
        m = re.match(r"^(\s*)PRINT (.+?);?\s*$", l, re.I)
        if m and not s.upper().startswith("PRINT #") and ";" not in m.group(2):
            replace_line(k, f"{m.group(1)}print({m.group(2)})", "print_parens")
            replace_line(k, f"{m.group(1)}console.log({m.group(2)})", "console_log")
        m = re.match(r"^(\s*)DIM (\w+) AS INTEGER\s*$", l, re.I)
        if m:
            replace_line(k, f"{m.group(1)}int {m.group(2)};", "var_decl")
        if re.match(r"^\s*(IF|ELSEIF)\s+NOT\s+\w+", l, re.I):
            replace_line(k, re.sub(r"\bNOT\s+(\w+)", r"!\1", l, count=1, flags=re.I), "not_bang")
        m = re.match(r"^(\s*)(\w+(?:\(\w+\))? = [^'\"]+?)\s*$", l)
        if m and outside[k] and "(" not in m.group(1) and not re.match(r"^\s*(IF|FOR|CASE|DIM)\b", l, re.I) and "'" not in l:
            replace_line(k, f"{m.group(1)}{m.group(2)};", "semicolon_end")
        m = re.match(r"^(\s*)(\w+)\((\d+)\) = (.+)$", l)
        if m:
            replace_line(k, f"{m.group(1)}{m.group(2)}[{m.group(3)}] = {m.group(4)}", "array_brackets")

    # deleting a closing line: the fix puts it back, between its neighbours
    for k, l in enumerate(lines):
        s = l.strip().upper()
        name = "missing_end_if" if s == "END IF" else "next_missing" if re.match(r"^NEXT\b", s) else ""
        if name and 0 < k < len(lines) - 1:
            before, after = lines[k - 1], lines[k + 1]
            broken = lines[:k] + lines[k + 1 :]
            out.append(Mutation(name, "\n".join(broken), before + "\n" + after, before + "\n" + l + "\n" + after, before + "\n" + after))
    rng.shuffle(out)
    return out


def usable(m: Mutation, original: str) -> bool:
    """The edit must be expressible as a unique old_text."""
    return m.old != m.new and _unique(m.text, m.old)
