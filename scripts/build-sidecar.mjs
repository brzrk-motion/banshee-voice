import { copyFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import {
  assertVulkanPrerequisites,
  requestedTarget,
  usesVulkanTarget,
} from "./gpu-build.mjs";

function run(command, args, options) {
  const result = spawnSync(command, args, { ...options, stdio: "inherit" });
  if (result.error || result.status !== 0) {
    throw result.error ?? new Error(`${command} exited with status ${result.status}`);
  }
}

function rustHost(environment) {
  const result = spawnSync("rustc", ["-vV"], {
    env: environment,
    encoding: "utf8",
  });
  if (result.error || result.status !== 0) {
    throw result.error ?? new Error("rustc -vV failed");
  }
  const host = result.stdout.match(/^host:\s+(.+)$/m)?.[1]?.trim();
  if (!host) throw new Error("Could not determine the Rust host target");
  return host;
}

export function buildPromptWorker(repositoryRoot, environment, release, tauriArgs = []) {
  const target = requestedTarget(tauriArgs) ?? rustHost(environment);
  if (target === "universal-apple-darwin") {
    throw new Error("Universal macOS builds require a universal prompt worker binary");
  }

  assertVulkanPrerequisites(target, environment);
  const features = usesVulkanTarget(target) ? "worker,gpu-vulkan" : "worker";
  const cargoArgs = ["build", "-p", "banshee-prompt-enhancer", "--features", features, "--bin", "banshee-prompt-worker"];
  if (release) cargoArgs.push("--release");
  if (requestedTarget(tauriArgs)) cargoArgs.push("--target", target);
  run("cargo", cargoArgs, { cwd: repositoryRoot, env: environment });

  const executableName = process.platform === "win32" ? "banshee-prompt-worker.exe" : "banshee-prompt-worker";
  const profile = release ? "release" : "debug";
  const artifactDirectory = requestedTarget(tauriArgs)
    ? path.join(repositoryRoot, "target", target, profile)
    : path.join(repositoryRoot, "target", profile);
  const source = path.join(artifactDirectory, executableName);
  const binaries = path.join(repositoryRoot, "apps", "desktop", "src-tauri", "binaries");
  mkdirSync(binaries, { recursive: true });
  const extension = process.platform === "win32" ? ".exe" : "";
  const destination = path.join(binaries, `banshee-prompt-worker-${target}${extension}`);
  copyFileSync(source, destination);
}
