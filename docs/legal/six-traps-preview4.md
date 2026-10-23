# The six legal traps, Preview 4 (2026-10-23)

Robert's rule: before a release that adds accounts, personal data, analytics, email, payments or uploads, one reviewer per trap reads the actual code. Six read-only Sonnet reviews ran on development (42ec22bd + the security fixes); the gaps were closed in one pass (commit "Legal (X4, the six traps)").

| Trap | Result | What was found | What changed |
|---|---|---|---|
| 1. Children (COPPA) | Doesn't apply | No accounts, no telemetry, no crash reports; the only outbound calls are the AI providers the user picks, with the user's own key (CSP `connect-src` in `ide/web/index.html`). Chats and the audit log stay on the user's computer. | Wording: RAI's set-up wizard and the manual's "Privacy and safety" say cloud services are for adults; under 18, a parent or teacher makes the key. |
| 2. Third-party fonts, scripts, embeds (GDPR) | Doesn't apply | Fonts ship with RapidR (`runtime/fonts/`, same-origin); no CDN, no embeds, no analytics; Monaco is gone; generated program pages have a strict CSP. | The web manual: a program that loads other sites' content on purpose makes its author responsible for that content's privacy rules. |
| 3. Session replay and analytics (CIPA/GDPR) | Doesn't apply | No replay, analytics or telemetry. Test hooks need environment variables (desktop) or a page global no URL can set (web). Chats are redacted before writing. | The audit log now redacts every line (a provider's error text could carry a key), with a test; the manual says how to clear it. |
| 4. Marketing email (CAN-SPAM) | Doesn't apply | RapidR sends no email; no email component, no newsletter, no sign-up; releases are published by hand. | — (re-check if rapidr.dev adds a newsletter or an SMTP component arrives) |
| 5. Subscriptions and auto-renew (California) | Applied (support links) | Support goes through GitHub Sponsors and Buy Me a Coffee (their checkout, their cancel); RapidR's links didn't say "renews monthly, cancel in the platform's settings, nothing in return". | README (under the badges and in "Support RapidR"), ANNOUNCEMENT.md, the past release notes in the repo and the release-notes template now say it. The GitHub release pages of v2.117–v2.120 still show the old text (Robert's call whether to edit them). |
| 6. User uploads (DMCA) | Doesn't apply | Projects stay on the user's computer or in their own browser (OPFS); no share, gallery or publish feature; Versions' GitHub publishing isn't in Preview 4. | — (re-check before Versions' Publish or any share feature) |

Still open, not blocking: whoever hosts the web Studio logs visitors' IP addresses — name the host and add a privacy page when it's hosted (docs/legal/review-2026-10-22.md items 4g and 7d).
