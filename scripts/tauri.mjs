import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import { prepareNativeEnvironment } from "./native-toolchain.mjs";
import { buildPromptWorker } from "./build-sidecar.mjs";
import { mergeCargoFeature, requestedTarget, usesVulkanTarget } from "./gpu-build.mjs";
import { assertWindowsDevelopmentAllowed } from "./windows-application-control.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");
const desktopDirectory = path.join(repositoryRoot, "apps", "desktop");
const subcommand = process.argv[2];
const tauriArgs = process.argv.slice(3);

function hostTarget(environment) {
  const result = spawnSync("rustc", ["-vV"], { env: environment, encoding: "utf8" });
  const target = result.stdout?.match(/^host:\s+(.+)$/m)?.[1]?.trim();
  if (result.status !== 0 || !target) throw new Error("Could not determine the Rust host target");
  return target;
}

if (!new Set(["dev", "build"]).has(subcommand)) {
  console.error("Usage: node scripts/tauri.mjs <dev|build>");
  process.exit(1);
}

const environment = prepareNativeEnvironment(repositoryRoot);
try {
  assertWindowsDevelopmentAllowed(environment);
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
buildPromptWorker(repositoryRoot, environment, subcommand === "build", tauriArgs);
environment.TAURI_CONFIG = JSON.stringify({
  bundle: { externalBin: ["binaries/banshee-prompt-worker"] },
});
const command = process.platform === "win32" ? environment.ComSpec ?? "cmd.exe" : "npm";
const target = requestedTarget(tauriArgs) ?? hostTarget(environment);
const desktopArgs = usesVulkanTarget(target)
  ? mergeCargoFeature(tauriArgs, "gpu-vulkan")
  : tauriArgs;
const args =
  process.platform === "win32"
    ? ["/d", "/s", "/c", ["npm", "exec", "tauri", subcommand, ...desktopArgs].join(" ")]
    : ["exec", "tauri", subcommand, ...desktopArgs];
const child = spawn(command, args, {
  cwd: desktopDirectory,
  env: environment,
  stdio: "inherit",
});

for (const signal of ["SIGINT", "SIGTERM"]) {
  process.on(signal, () => {
    if (process.platform === "win32" && child.pid) {
      spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"], { stdio: "ignore" });
    } else {
      child.kill(signal);
    }
    process.exit(0);
  });
}

child.on("error", (error) => {
  console.error(error.message);
  process.exitCode = 1;
});
child.on("exit", (code) => {
  process.exitCode = code ?? 1;
});
