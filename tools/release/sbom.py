#!/usr/bin/env python3
"""A CycloneDX 1.5 SBOM (JSON) of what a RapidR release ships, from the real
dependency graph — the same list THIRD_PARTY_NOTICES.md is made from
(tools/third_party_notices.py: the workspace's normal dependencies), plus the
non-Cargo pieces LICENSES.md credits that a release ships (the Liberation
fonts). The legacy web IDE's Monaco editor isn't shipped: the web bundle is
RapidR Studio's (docs/security-audit.md SEC-17).

    python3 tools/release/sbom.py --out dist/<ver>/rapidr-<ver>.cdx.json

No tool beyond Python and cargo: nothing to install, nothing to license.
"""

import argparse
import datetime
import importlib.util
import json
import os
import re
import tomllib
import uuid

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def notices():
    spec = importlib.util.spec_from_file_location("third_party_notices", os.path.join(ROOT, "tools", "third_party_notices.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def vscode_extension_packages():
    """The JavaScript packages bundled into the VS Code extension (rapidr-<ver>.vsix):
    its package-lock.json's production entries (utilities/vscodeext/rapidr)."""
    lock_path = os.path.join(ROOT, "utilities", "vscodeext", "rapidr", "package-lock.json")
    with open(lock_path, encoding="utf-8") as f:
        lock = json.load(f)
    out = []
    for key, meta in lock.get("packages", {}).items():
        if not key.startswith("node_modules/") or meta.get("dev") or meta.get("devOptional"):
            continue
        name = key.rsplit("node_modules/", 1)[1]
        out.append((name, meta["version"], meta.get("license") or "NOASSERTION"))
    return sorted(set(out))


def licenses(expr):
    """CycloneDX wants a single SPDX id as `license.id`, else an expression."""
    if re.fullmatch(r"[A-Za-z0-9.+-]+", expr):
        return [{"license": {"id": expr}}]
    return [{"expression": expr}]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    tpn = notices()
    clarified = tpn.clarifications()
    version = tomllib.load(open(os.path.join(ROOT, "Cargo.toml"), "rb"))["workspace"]["package"]["version"]
    components = []
    for p in sorted(tpn.shipped_packages(), key=lambda p: (p["name"], p["version"])):
        lic = tpn.normalize(p.get("license") or clarified.get(p["name"]) or "NOASSERTION")
        c = {
            "type": "library",
            "bom-ref": f"pkg:cargo/{p['name']}@{p['version']}",
            "name": p["name"],
            "version": p["version"],
            "purl": f"pkg:cargo/{p['name']}@{p['version']}",
            "licenses": licenses(lic),
        }
        url = p.get("repository") or p.get("homepage")
        if url:
            c["externalReferences"] = [{"type": "vcs" if p.get("repository") else "website", "url": url}]
        components.append(c)
    # (credited by hand in LICENSES.md: not Cargo packages)
    components += [
        {
            "type": "data",
            "bom-ref": "liberation-fonts@2.1.5",
            "name": "Liberation fonts",
            "version": "2.1.5",
            "licenses": [{"license": {"id": "OFL-1.1"}}],
            "description": "built into the runtimes (crates/rapidr-value/fonts)",
            "externalReferences": [{"type": "vcs", "url": "https://github.com/liberationfonts/liberation-fonts"}],
        },
    ]
    # (bundled into the VS Code extension's dist/extension.js; listed in its THIRD_PARTY_NOTICES.md)
    for name, ver, lic in vscode_extension_packages():
        components.append({
            "type": "library",
            "bom-ref": f"pkg:npm/{name}@{ver}",
            "name": name,
            "version": ver,
            "purl": f"pkg:npm/{name.replace('@', '%40')}@{ver}",
            "licenses": licenses(lic),
            "description": f"in the VS Code extension (rapidr-{version}.vsix)",
        })
    bom = {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "serialNumber": f"urn:uuid:{uuid.uuid4()}",
        "version": 1,
        "metadata": {
            "timestamp": datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0).isoformat(),
            "tools": {"components": [{"type": "application", "name": "tools/release/sbom.py", "version": version}]},
            "component": {
                "type": "application",
                "bom-ref": f"rapidr@{version}",
                "name": "RapidR",
                "version": version,
                "licenses": [{"license": {"id": "MIT"}}],
                "externalReferences": [{"type": "vcs", "url": "https://github.com/iBobX/RapidR"}],
            },
        },
        "components": components,
    }
    with open(args.out, "w") as f:
        json.dump(bom, f, indent=1)
        f.write("\n")
    print(f"wrote {args.out} ({len(components)} components)")


if __name__ == "__main__":
    main()
