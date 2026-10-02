#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { accessSync, constants, realpathSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const usage = `Install or update the SozoCraft agent CLI.

Usage: node scripts/install-cli.mjs [--root DIRECTORY]
       pnpm cli:install [--root DIRECTORY]

Builds the release binary with Cargo and verifies the installed command.
Default install root: CARGO_INSTALL_ROOT, CARGO_HOME, or ~/.cargo.
Requires Rust/Cargo and the app's native Tauri build prerequisites.
`;

function fail(message) {
  console.error(message);
  process.exit(1);
}

const args = process.argv.slice(2);
if (args.length === 1 && ["--help", "-h"].includes(args[0])) {
  console.log(usage);
  process.exit(0);
}
if (
  args.length !== 0 &&
  (args.length !== 2 || args[0] !== "--root" || !args[1] || args[1].startsWith("--"))
) {
  fail("Expected no arguments or --root DIRECTORY. Use --help.");
}

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const installRoot = resolve(
  args[1] || process.env.CARGO_INSTALL_ROOT || process.env.CARGO_HOME || join(homedir(), ".cargo"),
);
const install = spawnSync(
  "cargo",
  ["install", "--path", join(repoRoot, "src-tauri"), "--bin", "sozocraft-cli", "--locked", "--force", "--root", installRoot],
  { cwd: repoRoot, stdio: "inherit" },
);
if (install.error?.code === "ENOENT") {
  fail("Cargo was not found on PATH. Install Rust/Cargo before installing the CLI.");
}
if (install.error || install.status !== 0) {
  fail("CLI installation failed. Check the Cargo output above and the app's build prerequisites.");
}

const binaryName = process.platform === "win32" ? "sozocraft-cli.exe" : "sozocraft-cli";
const binDirectory = join(installRoot, "bin");
const binary = join(binDirectory, binaryName);
const verification = spawnSync(binary, ["--help"], { encoding: "utf8" });
if (verification.error || verification.status !== 0 || !verification.stdout?.includes("SozoCraft agent CLI")) {
  fail("Installation finished, but the installed CLI failed its --help check.");
}
console.log(`Installed and verified: ${binary}`);

let pathBinary;
for (const directory of (process.env.PATH || "").split(delimiter)) {
  const candidate = resolve(directory || ".", binaryName);
  try {
    accessSync(candidate, process.platform === "win32" ? constants.F_OK : constants.X_OK);
    pathBinary = realpathSync(candidate);
    break;
  } catch {
    // Continue searching PATH when this directory has no executable CLI.
  }
}
if (!pathBinary) {
  console.log(`Add this directory to PATH to use sozocraft-cli: ${binDirectory}`);
} else if (pathBinary !== realpathSync(binary)) {
  console.log(`PATH currently selects another CLI: ${pathBinary}`);
  console.log(`Place this directory first on PATH: ${binDirectory}`);
} else {
  console.log("Ready: sozocraft-cli --help");
}
