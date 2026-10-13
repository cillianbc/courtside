// Smoke test for the WebAssembly build (run tools/wasm/build.sh first).
//   node tools/wasm/smoke.mjs                 checks a made-up fixture
//   node tools/wasm/smoke.mjs <file.dvw>...   also checks that the WASM
//       output equals the CLI's (`validate --json`, `stats --json` with
//       both symbol sets, `convert` to .json) for each file
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { readFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname, basename } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "../..");
const require = createRequire(import.meta.url);
const wasm = require(join(root, "target/wasm/nodejs/courtside_ffi.js"));

const cargoVersion = /\nversion = "([^"]+)"/.exec(readFileSync(join(root, "Cargo.toml"), "utf8"))[1];
assert.equal(wasm.version(), cargoVersion);

// Made up; inline because *.dvw is gitignored to keep match files out.
const fixture = new TextEncoder().encode(
  [
    "[3DATAVOLLEYSCOUT]",
    "GENERATOR-PRG: Data Volley",
    "[3MATCH]",
    "04/23/2026;;;;;;;;1252;1;Z;",
    "[3TEAMS]",
    "T1;Home;0;;;;",
    "T2;Away;0;;;;",
    "[3PLAYERS-H]",
    "0;1;1;;;;;;H1;One;A;;;2;;",
    "[3PLAYERS-V]",
    "1;2;1;;;;;;V2;Two;B;;;2;;",
    "[3SCOUT]",
    "*01SH#;;;;;;;;;;;;;;;;;;;;;;;;;;",
    "",
  ].join("\n"),
);
assert.deepEqual(JSON.parse(wasm.validate("fixture.dvw", fixture)), []);
const stats = JSON.parse(wasm.stats(fixture, "datavolley"));
assert.ok(stats && typeof stats === "object");
const plays = JSON.parse(wasm.plays(fixture));
assert.equal(plays.format, "courtside-plays");
assert.equal(plays.plays.length, 1);
assert.throws(() => wasm.stats(fixture, "other"), /symbols must be/);
assert.throws(() => wasm.plays(new TextEncoder().encode("[Match]\n")), Error);
const unreadable = JSON.parse(wasm.validate("x.txt", new Uint8Array()));
assert.equal(unreadable[0].severity, "error");
console.log("fixture ok");

const files = process.argv.slice(2);
if (files.length > 0) {
  execFileSync("cargo", ["build", "-q", "-p", "courtside-cli"], { cwd: root, stdio: "inherit" });
  const cli = join(root, "target/debug/courtside");
  const run = (args) => {
    try {
      return execFileSync(cli, args, { encoding: "utf8", maxBuffer: 1 << 28 });
    } catch (e) {
      // validate exits 1 when a file has errors; its output is still valid.
      if (e.status === 1) return e.stdout;
      throw e;
    }
  };
  const tmp = mkdtempSync(join(tmpdir(), "courtside-wasm-"));
  try {
    for (const file of files) {
      const bytes = readFileSync(file);
      assert.deepEqual(
        JSON.parse(wasm.validate(basename(file), bytes)),
        JSON.parse(run(["validate", "--json", file])),
        `validate ${file}`,
      );
      for (const symbols of ["datavolley", "volleystation"]) {
        assert.deepEqual(
          JSON.parse(wasm.stats(bytes, symbols)),
          JSON.parse(run(["stats", "--json", "--symbols", symbols, file])),
          `stats ${symbols} ${file}`,
        );
      }
      const out = join(tmp, "plays.json");
      run(["convert", file, out]);
      assert.deepEqual(
        JSON.parse(wasm.plays(bytes)),
        JSON.parse(readFileSync(out, "utf8")),
        `plays ${file}`,
      );
    }
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
  console.log(`${files.length} file(s): WASM output equals the CLI's`);
}
