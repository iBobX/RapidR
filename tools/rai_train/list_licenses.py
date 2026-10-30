#!/usr/bin/env python3
"""The licences of every package installed in a Python environment (from the packages' own
metadata), for the lawyer check before a trained model is shared.

    $RAI_ROOT/venv/bin/python tools/rai_train/list_licenses.py
    $RAI_ROOT/venv-convert/bin/python tools/rai_train/list_licenses.py
"""

import importlib.metadata as md


def licence(d: md.Distribution) -> str:
    m = d.metadata
    expr = m.get("License-Expression")
    if expr:
        return expr
    classes = [c.split("::")[-1].strip() for c in m.get_all("Classifier", []) if c.startswith("License ::")]
    if classes:
        return "; ".join(classes)
    text = (m.get("License") or "").strip().splitlines()
    return (text[0][:70] if text else "unknown")


rows = sorted((d.metadata["Name"], d.version, licence(d)) for d in md.distributions())
width = max(len(n) for n, _, _ in rows)
for name, ver, lic in rows:
    print(f"{name:<{width}} {ver:<12} {lic}")
