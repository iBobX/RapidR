// Runs test/suite/*.test.js inside the VS Code that test/runTest.js starts.
'use strict';

const fs = require('fs');
const path = require('path');
const Mocha = require('mocha');

exports.run = function run() {
    const mocha = new Mocha({ ui: 'bdd', color: true, timeout: 60000 });
    if (process.env.RAPIDR_TEST_GREP) mocha.grep(process.env.RAPIDR_TEST_GREP);
    for (const f of fs.readdirSync(__dirname).filter((n) => n.endsWith('.test.js')).sort()) {
        mocha.addFile(path.join(__dirname, f));
    }
    return new Promise((resolve, reject) => {
        mocha.run((failures) => (failures > 0 ? reject(new Error(`${failures} test(s) failed`)) : resolve()));
    });
};
