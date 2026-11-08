// Checks that every version number in the repo is valid semver and that they all match.
// Files that don't exist yet are skipped. Run from the repo root: node .github/scripts/check-versions.mjs

import { existsSync, readFileSync } from "node:fs";

// semver.org's recommended pattern.
const SEMVER =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

const found = []; // [where, version]
const problems = [];

function read(path) {
  return existsSync(path) ? readFileSync(path, "utf8") : null;
}

function json(path, pick) {
  const text = read(path);
  if (text === null) return;
  let data;
  try {
    data = JSON.parse(text);
  } catch (e) {
    problems.push(`${path}: not valid JSON (${e.message})`);
    return;
  }
  for (const [label, value] of pick(data)) {
    if (value === undefined) problems.push(`${path}: no ${label}`);
    else found.push([`${path} ${label}`, value]);
  }
}

// Cargo workspace: [workspace.package] version.
const cargo = read("Cargo.toml");
if (cargo === null) {
  problems.push("Cargo.toml: missing");
} else {
  const section = cargo.match(/^\[workspace\.package\]\s*$([\s\S]*?)(?=^\[|(?![\s\S]))/m);
  const version = section?.[1].match(/^version\s*=\s*"([^"]*)"/m)?.[1];
  if (version === undefined) problems.push("Cargo.toml: no version in [workspace.package]");
  else found.push(["Cargo.toml [workspace.package] version", version]);
}

// Each crate must inherit the workspace version rather than set its own.
for (const path of ["crates/core/Cargo.toml", "crates/cli/Cargo.toml", "app/src-tauri/Cargo.toml"]) {
  const text = read(path);
  if (text !== null && !/^version\.workspace\s*=\s*true/m.test(text)) {
    problems.push(`${path}: should use version.workspace = true`);
  }
  // Internal crates are path dependencies; a version requirement on one breaks every release bump.
  const pinned = text?.match(/^(offloader-[\w-]+)\s*=\s*\{[^}\n]*\bversion\s*=/m);
  if (pinned) {
    problems.push(`${path}: depend on ${pinned[1]} by path only (no version), or release bumps can't resolve it`);
  }
}

// Cargo.lock: the workspace crates' own entries.
const lock = read("Cargo.lock");
if (lock !== null) {
  const crates = ["offloader-core", "offloader-cli", "media-offloader-app"];
  for (const block of lock.split("[[package]]")) {
    const name = block.match(/^name = "([^"]*)"/m)?.[1];
    const version = block.match(/^version = "([^"]*)"/m)?.[1];
    if (crates.includes(name) && version !== undefined) found.push([`Cargo.lock ${name}`, version]);
  }
}

// `tauri build` refuses to run if a Tauri crate and its npm package differ in major.minor.
// Only release builds run it, so check here to catch that on the pull request instead.
const npmLock = read("app/package-lock.json");
if (lock !== null && npmLock !== null) {
  const crates = new Map();
  for (const block of lock.split("[[package]]")) {
    const name = block.match(/^name = "([^"]*)"/m)?.[1];
    const version = block.match(/^version = "([^"]*)"/m)?.[1];
    if (name && version) crates.set(name, version);
  }
  const packages = JSON.parse(npmLock).packages ?? {};
  for (const [path, info] of Object.entries(packages)) {
    const pkg = path.match(/^node_modules\/@tauri-apps\/(api|plugin-[\w-]+)$/)?.[1];
    if (!pkg) continue;
    const crate = pkg === "api" ? "tauri" : `tauri-${pkg}`;
    const version = crates.get(crate);
    if (version === undefined) continue;
    const majorMinor = (v) => v.split(".").slice(0, 2).join(".");
    if (majorMinor(version) !== majorMinor(info.version)) {
      problems.push(`Tauri versions differ: ${crate} ${version} (Cargo.lock), @tauri-apps/${pkg} ${info.version} (app/package-lock.json). tauri build needs the same major.minor`);
    }
  }
}

json(".release-please-manifest.json", (d) => [['"."', d["."]]]);
json("app/package.json", (d) => [["version", d.version]]);
json("app/package-lock.json", (d) => [
  ["version", d.version],
  ['packages[""].version', d.packages?.[""]?.version],
]);
json("app/src-tauri/tauri.conf.json", (d) => [["version", d.version]]);

for (const [where, version] of found) {
  if (!SEMVER.test(version)) problems.push(`${where}: "${version}" is not valid semver`);
}
const distinct = [...new Set(found.map(([, v]) => v))];
if (distinct.length > 1) {
  problems.push("versions don't match:\n" + found.map(([w, v]) => `    ${v.padEnd(12)} ${w}`).join("\n"));
}

for (const [where, version] of found) console.log(`${version.padEnd(12)} ${where}`);
if (problems.length) {
  console.error("\n" + problems.map((p) => `error: ${p}`).join("\n"));
  process.exit(1);
}
console.log(`\nAll ${found.length} versions are ${distinct[0]}.`);
