// Bundles src/ and its production dependencies (vscode-languageclient) into
// dist/extension.js with esbuild: the .vsix carries one file, no node_modules.
//
//   node scripts/build.js [--production] [--watch]
'use strict';

const esbuild = require('esbuild');
const path = require('path');

const production = process.argv.includes('--production');
const watch = process.argv.includes('--watch');
const root = path.join(__dirname, '..');

const options = {
    absWorkingDir: root,
    entryPoints: ['src/extension.js'],
    outfile: 'dist/extension.js',
    bundle: true,
    platform: 'node',
    format: 'cjs',
    target: 'node18',
    external: ['vscode'],
    minify: production,
    sourcemap: !production,
    // (the bundled packages' licences are in THIRD_PARTY_NOTICES.md: scripts/notices.js)
    legalComments: 'none',
    logLevel: 'info',
};

(async () => {
    if (watch) {
        const ctx = await esbuild.context(options);
        await ctx.watch();
    } else {
        await esbuild.build(options);
        // (no stale source map from a development build beside the production bundle)
        if (production) require('fs').rmSync(path.join(root, 'dist', 'extension.js.map'), { force: true });
    }
})().catch((err) => {
    console.error(err);
    process.exit(1);
});
