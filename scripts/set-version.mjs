// Usage: npm run version <x.y.z>
// Sets the same version in package.json, Cargo.toml (workspace) and
// src-tauri/tauri.conf.json so a release only needs one command.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error('Usage: npm run version <x.y.z>   (for example: npm run version 1.0.0)');
  process.exit(1);
}

const root = new URL('../', import.meta.url);
const path = (p) => fileURLToPath(new URL(p, root));

/** Rewrites a file; fails loudly if the pattern was not found exactly once. */
function update(file, transform) {
  const before = readFileSync(path(file), 'utf8');
  const after = transform(before);
  if (after === before && !before.includes(`"${version}"`)) {
    console.error(`Could not find a version to update in ${file}`);
    process.exit(1);
  }
  writeFileSync(path(file), after);
  console.log(`updated ${file}`);
}

// JSON files: replace only the top-level "version" so formatting is preserved.
const jsonVersion = /^(\s{2}"version":\s*)"[^"]*"/m;
update('package.json', (s) => s.replace(jsonVersion, `$1"${version}"`));
update('src-tauri/tauri.conf.json', (s) => s.replace(jsonVersion, `$1"${version}"`));

// The Rust crates inherit `version.workspace = true` from the root manifest.
update('Cargo.toml', (s) =>
  s.replace(/(\[workspace\.package\][^[]*?\nversion\s*=\s*)"[^"]*"/, `$1"${version}"`)
);

console.log(`\nVersion set to ${version}. Run "cargo check" to refresh Cargo.lock.`);
