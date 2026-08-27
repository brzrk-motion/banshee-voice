import { existsSync, mkdtempSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

export function requestedTarget(args) {
  const index = args.indexOf("--target");
  if (index >= 0) return args[index + 1];
  return args.find((argument) => argument.startsWith("--target="))?.slice("--target=".length);
}

export function usesVulkanTarget(target) {
  return target.includes("linux") || target.includes("windows");
}

export function mergeCargoFeature(args, feature) {
  const output = [];
  const features = [];
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "--features") {
      features.push(...(args[index + 1] ?? "").split(","));
      index += 1;
    } else if (argument.startsWith("--features=")) {
      features.push(...argument.slice("--features=".length).split(","));
    } else {
      output.push(argument);
    }
  }
  features.push(feature);
  const merged = [...new Set(features.map((value) => value.trim()).filter(Boolean))];
  return ["--features", merged.join(","), ...output];
}

function commandWorks(command, args, environment, runner) {
  const result = runner(command, args, { env: environment, stdio: "ignore" });
  return !result.error && result.status === 0;
}

function cmakePackageWorks(cmake, environment, runner) {
  const workingDirectory = mkdtempSync(path.join(os.tmpdir(), "banshee-cmake-probe-"));
  try {
    const result = runner(
      cmake,
      [
        "--find-package",
        "-DNAME=SPIRV-Headers",
        "-DCOMPILER_ID=GNU",
        "-DLANGUAGE=CXX",
        "-DMODE=EXIST",
      ],
      { cwd: workingDirectory, env: environment, stdio: "ignore" },
    );
    return !result.error && result.status === 0;
  } finally {
    rmSync(workingDirectory, { recursive: true, force: true });
  }
}

export function vulkanPrerequisiteError(
  target,
  environment,
  platform = process.platform,
  runner = spawnSync,
  fileExists = existsSync,
) {
  if (!usesVulkanTarget(target)) return undefined;

  if (target.includes("windows")) {
    const sdk = environment.VULKAN_SDK;
    if (
      platform === "win32" &&
      sdk &&
      fileExists(path.join(sdk, "Bin", "glslc.exe")) &&
      fileExists(path.join(sdk, "Include", "vulkan", "vulkan.h"))
    ) {
      return undefined;
    }
    return "Vulkan GPU builds for Windows require the LunarG Vulkan SDK and VULKAN_SDK environment variable. Install it from https://vulkan.lunarg.com/sdk/home#windows, then restart the terminal.";
  }

  const cmake = environment.CMAKE ?? "cmake";
  const checks = [
    ["glslc", commandWorks("glslc", ["--version"], environment, runner)],
    ["Vulkan headers and loader", commandWorks("pkg-config", ["--exists", "vulkan"], environment, runner)],
    [
      "SPIRV-Headers CMake package",
      cmakePackageWorks(cmake, environment, runner),
    ],
  ];
  const missing = checks.filter(([, available]) => !available).map(([name]) => name);
  if (missing.length === 0) return undefined;
  return [
    `Vulkan GPU build prerequisites are missing: ${missing.join(", ")}.`,
    "Install the system packages and retry:",
    "  Arch: sudo pacman -S shaderc vulkan-headers vulkan-icd-loader spirv-headers",
    "  Ubuntu: sudo apt install glslc libvulkan-dev spirv-headers",
    "  Fedora: sudo dnf install glslc vulkan-headers vulkan-loader-devel spirv-headers",
  ].join("\n");
}

export function assertVulkanPrerequisites(target, environment) {
  const error = vulkanPrerequisiteError(target, environment);
  if (error) throw new Error(error);
}
