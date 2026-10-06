"""The component types, from the language registry
(crates/rapidr-lang/data/components/*.toml, read the way its build.rs reads
them): every `[[component]]` the compilers create (no `kind`, or
`kind = "component"`), as (RapidR name, RapidQ name or None), in the
registry's order."""

import os
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))


def components():
    out = []
    base = os.path.join(ROOT, "crates", "rapidr-lang", "data", "components")
    for dirpath, dirnames, filenames in sorted(os.walk(base)):
        dirnames.sort()
        for fname in sorted(filenames):
            if not fname.endswith(".toml"):
                continue
            with open(os.path.join(dirpath, fname), "rb") as fh:
                for c in tomllib.load(fh).get("component", []):
                    if c.get("kind", "component") == "component":
                        out.append((c["name"], c.get("rapidq")))
    return out
