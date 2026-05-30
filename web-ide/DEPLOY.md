# Deploying the RapidR Web IDE / Generated Apps

## Why "GET /runtime/rapidrintr.js 403 (Forbidden)"?

Two common causes on shared hosting (cPanel, Plesk, OVH, Hostinger,
GoDaddy, etc.):

1. **Missing files** — `web-ide/runtime/.gitignore` contains `*` so the
   wasm artifacts (`rapidrintr.js`, `rapidrintr_bg.wasm`) are *not*
   committed. If you uploaded via `git push` / `git archive` they will be
   missing and Apache returns `403` when directory listing is disabled.
2. **Server denies JS/WASM** — some hosts ship with `Require all denied`
   defaults or refuse to serve `.wasm` because the MIME type is unknown.

## Fix in 30 seconds

1. Build the artifacts locally (this populates `web-ide/runtime/`):

   ```bash
   bash tools/build_web_artifacts.sh
   ```

2. Upload the **entire** `web-ide/` folder to your hosting (FTP, SCP,
   cPanel File Manager, etc.). Do **not** use `git push` for this folder
   because `runtime/.gitignore` will skip the wasm files.

   Make sure these four files are present on the server:

   ```
   /your-site/runtime/rapidrintr.js
   /your-site/runtime/rapidrintr_bg.wasm
   /your-site/runtime/rapidrintr.d.ts        (optional)
   /your-site/runtime/rapidrintr_bg.wasm.d.ts (optional)
   ```

3. The included `web-ide/.htaccess` already configures:
   - `Require all granted` (Apache 2.4) / `Allow from all` (2.2)
   - `application/wasm` MIME for `.wasm`
   - `application/javascript` MIME for `.js` / `.mjs`
   - permissive CORS for static assets
   - directory listing disabled

   It is uploaded automatically when you copy the folder — verify your
   FTP client is **not hiding dotfiles**.

## Generated apps (Run → Build & Download)

Every bundle ZIP exported from the IDE now includes its own `.htaccess`,
so just unzip into any web folder. The same MIME / permissions config
applies and you should see `index.html` load `loader.js` →
`rapidrintr.js` → `rapidrintr_bg.wasm` → `<project>.rrbc` without 403s.

## Quick sanity check

After deploying, open the browser dev tools Network tab and hit refresh.
You should see all four files as `200 OK`:

| File                   | Content-Type                |
|------------------------|-----------------------------|
| `index.html`           | `text/html`                 |
| `loader.js` (bundle)   | `application/javascript`    |
| `rapidrintr.js`        | `application/javascript`    |
| `rapidrintr_bg.wasm`   | `application/wasm`          |
| `<project>.rrbc`       | `application/octet-stream`  |

If `rapidrintr_bg.wasm` shows up as `text/html` or `application/octet-stream`
the browser will refuse to start it — re-check that the `.htaccess` was
uploaded and that `mod_mime` is enabled on your server.

## Nginx variant

If your host runs Nginx instead of Apache, the `.htaccess` is ignored.
Add this to your server block:

```nginx
location / {
    types {
        application/wasm        wasm;
        application/javascript  js mjs;
        text/css                css;
        text/html               html;
        application/json        json;
        application/octet-stream rrbc;
    }
    add_header Access-Control-Allow-Origin "*";
    autoindex off;
    try_files $uri $uri/ =404;
}
```
