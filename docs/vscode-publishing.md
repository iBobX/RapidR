# Publishing the VS Code extension

How to put `rapidr-<version>.vsix` on the **Visual Studio Marketplace** (the store VS Code uses) and on **Open VSX** (the store VSCodium, Cursor, Gitpod, Eclipse Theia and other VS Code-compatible editors use).

This is never automated. No script in this repository publishes, and no remote CI runs. Both stores need **your own accounts and tokens**. Never commit a token, never put one in a file in the repository, and never paste one into an issue or chat.

The `.vsix` is built by the release (`tools/release/vscode.sh` writes `dist/<version>/out/rapidr-<version>.vsix`) or by `./build_vsc_extension.sh` (`utilities/vscodeext/rapidr/dist/`). Publish the same file that went on the GitHub release: its SHA-256 is in `SHA256SUMS`. Use `--packagePath` / the file argument for this, and don't let the tools rebuild it.

## The publisher name

`package.json` says `"publisher": "rapidr"`. That is a **placeholder** until you register a publisher. An extension's ID is `<publisher>.<name>`, here `rapidr.rapidr`, and it can't change after the first publish without becoming a different extension.

1. Pick a publisher ID that is free on **both** stores. On the Marketplace, `https://marketplace.visualstudio.com/publishers/<id>` should be 404. On Open VSX, `https://open-vsx.org/namespace/<id>` should be 404. `rapidr` is the natural choice. If it's taken, use something like `ibobx`.
2. If it isn't `rapidr`, change `"publisher"` in `utilities/vscodeext/rapidr/package.json`, and `EXTENSION_ID` in `test/suite/extension.test.js`. Then commit, and rebuild the `.vsix`. The ID is inside the `.vsix`, so the file must be built after the change.

## Visual Studio Marketplace

You need four things, all once:

1. **A Microsoft account**, at https://login.live.com.
2. **An Azure DevOps organization.** Go to https://dev.azure.com, sign in, and create an organization (any name). It exists only to issue the token. Publishing is free.
3. **A Personal Access Token.** In Azure DevOps, open **User settings** (the icon at the top right) and choose **Personal access tokens → New Token**:
   - Name: for example, `vsce RapidR`.
   - **Organization: "All accessible organizations"**. A token for one organization is refused by the Marketplace.
   - Expiration: as short as is practical. You can make a new token for each release.
   - Scopes: **Custom defined → Show all scopes → Marketplace → Manage**.

   Copy the token when it is shown. Azure DevOps shows it only once.
4. **The publisher.** Create it on the web at https://marketplace.visualstudio.com/manage/createpublisher, signed in with the same Microsoft account:
   - ID: the publisher ID above.
   - Display name: "RapidR".

   Fill in the description, website (https://github.com/iBobX/RapidR) and logo too. (`npx @vscode/vsce create-publisher` no longer creates publishers; use the web form.)

Then, from `utilities/vscodeext/rapidr` (vsce is a devDependency, so run `npm install` first):

```bash
npx @vscode/vsce login <publisher-id>          # asks for the token; stored in your OS keychain
npx @vscode/vsce verify-pat <publisher-id>     # optional: checks the token
npx @vscode/vsce publish --packagePath /path/to/dist/<version>/out/rapidr-<version>.vsix
```

`--packagePath` publishes the file as it is. Without it, vsce builds a new one from the working tree. The listing appears at `https://marketplace.visualstudio.com/items?itemName=<publisher-id>.rapidr` after a few minutes, once the Marketplace has scanned it.

When you're done, run `npx @vscode/vsce logout <publisher-id>`, or revoke the token in Azure DevOps.

## Open VSX

You need these, once:

1. **An Eclipse account**, at https://accounts.eclipse.org/user/register. Its GitHub username field must be your GitHub username.
2. **Sign in to https://open-vsx.org with GitHub** (the account linked above). Then go to your profile settings and **log in with your Eclipse account** to link the two.
3. **Sign the Eclipse Foundation Open VSX Publisher Agreement** from the open-vsx.org profile page (**Show Publisher Agreement → Agree**). You can't publish without it.
4. **An access token**: on open-vsx.org, go to **Settings → Access Tokens → Generate New Token**. Copy it when it is shown.
5. **The namespace** (the publisher ID, the same as on the Marketplace):

   ```bash
   npx ovsx create-namespace <publisher-id> -p <token>
   ```

   (`ovsx` is fetched by npx; it is MIT-licensed, from the Eclipse Foundation.) To get the "verified" mark, claim the namespace's ownership through the Open VSX GitHub issue tracker, as open-vsx.org's namespace page explains.

Then publish the same file:

```bash
npx ovsx publish /path/to/dist/<version>/out/rapidr-<version>.vsix -p <token>
```

The listing is at `https://open-vsx.org/extension/<publisher-id>/rapidr`. To keep the token out of your shell history, put it in an environment variable for that one command (`OVSX_PAT=… npx ovsx publish <vsix>`). `ovsx` reads `OVSX_PAT`, and vsce reads `VSCE_PAT` the same way.

## Before publishing: checklist

- [ ] The GitHub release for this version is published, and the `.vsix` is in it.
- [ ] `package.json`'s version is RapidR's (`npm run version:sync -- --check`, from `utilities/vscodeext/rapidr`).
- [ ] `publisher` is the registered ID, not the placeholder.
- [ ] The README's screenshots are real, not the placeholders (`images/completion.png`, `hover.png`, `diagnostics.png`, `debug.png`). Take them with the release's RapidR, in a default VS Code theme.
- [ ] `CHANGELOG.md` (the extension's) has an entry for this version.
- [ ] `npm test` passed against the release's `rapidr` (`RAPIDR_PATH=…/rapidr npm test`).
- [ ] The `.vsix` was installed in a clean VS Code profile (`code --profile Clean --install-extension rapidr-<version>.vsix`, or **Install from VSIX…**) and tried on a new machine's install of RapidR. Try a `.bas` file: completion, hover, an error, F5 with a breakpoint, and Run.
- [ ] `npx @vscode/vsce ls --packagePath <vsix>` lists no sources, tests or `node_modules`, and includes `THIRD_PARTY_NOTICES.md` and `LICENSE.txt`.
- [ ] The `.vsix`'s SHA-256 matches its line in `SHA256SUMS` (`shasum -a 256 rapidr-<version>.vsix`).

## Later releases

1. Build the release as usual. The `.vsix` comes with it, versioned as RapidR.
2. Make new tokens if the old ones have expired.
3. Run the two `publish` commands above with the new file.

A version can't be published twice to either store. If a published `.vsix` is broken, the fix ships in the next RapidR version.

To remove a version or the extension: on the Marketplace, use **Manage → ⋯ → Unpublish** (that hides it) or **Remove**. On Open VSX, ask through its issue tracker. Neither store allows re-using a version number.
