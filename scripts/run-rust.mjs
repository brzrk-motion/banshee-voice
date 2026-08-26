import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import { prepareNativeEnvironment } from "./native-toolchain.mjs";
import { assertWindowsDevelopmentAllowed } from "./windows-application-control.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");
const [command, ...args] = process.argv.slice(2);

if (!command) {
  console.error("Usage: node scripts/run-rust.mjs <command> [...args]");
  process.exit(1);
}

const environment = prepareNativeEnvironment(repositoryRoot);
try {
  assertWindowsDevelopmentAllowed(environment);
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}

const child = spawn(command, args, {
  cwd: repositoryRoot,
  env: environment,
  stdio: "inherit",
});

child.on("error", (error) => {
  console.error(error.message);
  process.exitCode = 1;
});
child.on("exit", (code) => {
  process.exitCode = code ?? 1;
});
