#!/usr/bin/env python3
"""No private key, certificate, signing key or token in a tracked file.

tools/check_key_shapes.py looks for API keys' shapes. This one looks for the
rest of what must never be committed (Preview 4 security, X2):

  * key files by name: .pem .key .p12 .pfx .jks .keystore .p8 .gpg .kdbx,
    id_rsa / id_ed25519, .env files, a minisign secret key (.key, .sec);
  * PEM blocks: a "BEGIN ... PRIVATE KEY" line (RSA, EC, DSA, OPENSSH, PGP,
    ENCRYPTED, plain) followed by base64 text, and "BEGIN CERTIFICATE"
    likewise. The words alone (a scanner's own pattern, a test's string, a
    document explaining them) pass: only a header with its body counts;
  * a minisign secret key (its comment line, or its base64 start);
  * tokens by shape: JSON Web Tokens, Stripe live keys, npm / Hugging Face /
    SendGrid / Twilio tokens, Slack webhooks, a service account's
    private_key, a password in a URL (user:password@host, not a placeholder).

Every tracked file is read (text only). A finding prints its file, line and
KIND only, never the value; the exit code is 1.

    python3 tools/check_secrets.py
"""

import re
import subprocess
import sys

NAME_RE = re.compile(
    r"(?:\.(?:pem|key|p12|pfx|jks|keystore|p8|gpg|kdbx|mobileprovision|provisionprofile)"
    r"|(?:^|/)id_(?:rsa|dsa|ecdsa|ed25519)"
    r"|(?:^|/)\.env(?:\..+)?"
    r"|\.minisign\.sec|minisign.*\.sec)$",
    re.IGNORECASE,
)
# (an example or template env file carries no value)
NAME_OK = re.compile(r"\.env\.(?:example|sample|template|dist)$|(?:^|/)\.env\.example$", re.IGNORECASE)

PEM_HEADER = re.compile(r"^\s*(?:[#/*;>-]+\s*)?-----BEGIN ((?:[A-Z0-9]+ )*(?:PRIVATE KEY(?: BLOCK)?|CERTIFICATE))-----\s*$")
B64_BODY = re.compile(r"^[A-Za-z0-9+/=]{40,}$")

LINE_SHAPES = [
    ("minisign secret key", re.compile(r"minisign encrypted secret key|\bRWRTY0Iy[A-Za-z0-9+/=]{40,}")),
    ("JSON Web Token", re.compile(r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}")),
    ("Stripe live key", re.compile(r"\b[sr]k_live_[A-Za-z0-9]{16,}")),
    ("npm token", re.compile(r"\bnpm_[A-Za-z0-9]{36}\b")),
    ("Hugging Face token", re.compile(r"\bhf_[A-Za-z0-9]{34,}\b")),
    ("SendGrid key", re.compile(r"\bSG\.[A-Za-z0-9_-]{22}\.[A-Za-z0-9_-]{43}")),
    ("Twilio key", re.compile(r"\bSK[0-9a-f]{32}\b")),
    ("Slack webhook", re.compile(r"hooks\.slack\.com/services/T[A-Z0-9]{8,}/B[A-Z0-9]{8,}/[A-Za-z0-9]{20,}")),
    ("service account private key", re.compile(r"\"private_key\"\s*:\s*\"-----BEGIN")),
    ("Authorization bearer token", re.compile(r"(?i)authorization\s*[:=]\s*[\"']?bearer\s+[A-Za-z0-9._~+/=-]{30,}")),
]
URL_PASSWORD = re.compile(r"\b[a-z][a-z0-9+.-]*://([^/\s:@\"'<>]+):([^/\s@\"'<>]+)@[A-Za-z0-9.-]+")
PLACEHOLDER = re.compile(r"(?i)^(?:pass(?:word)?|pwd|secret|token|xxx+|\*+|<[^>]*>|\{[^}]*\}|\$\{?[A-Za-z_]+\}?|%s|%[sd]|\.\.\.|…|redacted|changeme|example|user|test|\d{0,3})$")


def main() -> int:
    files = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout.split(b"\0")
    found = 0
    scanned = 0
    for name in filter(None, files):
        path = name.decode("utf-8", "replace")
        if NAME_RE.search(path) and not NAME_OK.search(path):
            print(f"{path}: a key or secret file by its name")
            found += 1
        if path == "tools/check_secrets.py":
            continue
        try:
            text = open(path, encoding="utf-8").read()
        except (UnicodeDecodeError, IsADirectoryError, FileNotFoundError):
            continue
        scanned += 1
        lines = text.splitlines()
        for n, line in enumerate(lines, 1):
            m = PEM_HEADER.match(line)
            if m:
                body = next((l.strip() for l in lines[n : n + 4] if l.strip()), "")
                if B64_BODY.match(body) or body.startswith(("Proc-Type:", "DEK-Info:")):
                    kind = "certificate (public)" if m.group(1).endswith("CERTIFICATE") else "PRIVATE KEY block"
                    print(f"{path}:{n}: a {kind}")
                    found += 1
                continue
            for kind, rx in LINE_SHAPES:
                if rx.search(line):
                    print(f"{path}:{n}: a {kind}")
                    found += 1
            for um in URL_PASSWORD.finditer(line):
                if not PLACEHOLDER.match(um.group(2)):
                    print(f"{path}:{n}: a password in a URL")
                    found += 1
    if found:
        print(f"secrets: {found} found")
        return 1
    print(f"secrets: none ({scanned} text files read)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
