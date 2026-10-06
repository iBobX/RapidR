// Smoke test for the wasm-bindgen target=web build of rapidr-compiler-wasm.
// Loads target/web/rapidrintr.js, initializes the wasm by passing raw bytes
// (avoiding fetch()), and compiles examples/basics/hello.rr.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '..');
const wasmPath = path.join(repoRoot, 'target/web/rapidrintr_bg.wasm');
const jsPath = path.join(repoRoot, 'target/web/rapidrintr.js');
const srcPath = path.join(repoRoot, 'examples/basics/hello.rr');

const RRBC_MAGIC = Uint8Array.of(0x52, 0x52, 0x42, 0x43); // "RRBC"

function fail(msg, err) {
    console.error(`FAIL: ${msg}`);
    if (err) console.error(err);
    process.exit(1);
}

try {
    const mod = await import(jsPath);
    const wasmBytes = readFileSync(wasmPath);
    await mod.default(wasmBytes);

    const source = readFileSync(srcPath, 'utf8');
    const bytecode = mod.compile(source, 'hello');

    if (!(bytecode instanceof Uint8Array)) {
        fail(`compile() did not return a Uint8Array (got ${typeof bytecode})`);
    }
    if (bytecode.length < 4) {
        fail(`bytecode too short: ${bytecode.length}`);
    }
    for (let i = 0; i < 4; i++) {
        if (bytecode[i] !== RRBC_MAGIC[i]) {
            fail(`bad magic: got ${[...bytecode.slice(0, 4)].map(b => b.toString(16)).join(' ')}, want "RRBC"`);
        }
    }

    console.log(`OK compile() -> ${bytecode.length} bytes, magic="RRBC"`);
    process.stdout.write(`COMPILE_SIZE=${bytecode.length}\n`);
} catch (err) {
    fail('exception during smoke test', err);
}
