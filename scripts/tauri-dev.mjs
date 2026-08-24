import { spawn, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "..");
const desktopDirectory = path.join(repositoryRoot, "apps", "desktop");
const environment = { ...process.env };

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    env: environment,
    stdio: "inherit",
  });
  if (result.error || result.status !== 0) {
    throw result.error ?? new Error(`${command} exited with status ${result.status}`);
  }
}

function findPython() {
  for (const command of process.platform === "win32" ? ["python", "py"] : ["python3", "python"]) {
    const result = spawnSync(command, ["--version"], { stdio: "ignore" });
    if (result.status === 0) return command;
  }
  throw new Error("Python is required to bootstrap CMake and libclang for the Windows Whisper build.");
}

if (process.platform === "win32") {
  const toolsDirectory = path.join(repositoryRoot, "target", "tools");
  const cmakeDirectory = path.join(toolsDirectory, "cmake");
  const cmakeExecutable = path.join(cmakeDirectory, "bin", "cmake.exe");
  const libclangDirectory = path.join(toolsDirectory, "libclang");
  const libclangNativeDirectory = path.join(libclangDirectory, "clang", "native");
  const libclangLibrary = path.join(libclangNativeDirectory, "libclang.dll");

  if (!existsSync(cmakeExecutable) || !existsSync(libclangLibrary)) {
    const python = findPython();
    if (!existsSync(cmakeExecutable)) {
      console.log("Installing an isolated CMake toolchain for whisper.cpp…");
      run(python, ["-m", "pip", "install", "--disable-pip-version-check", "--target", cmakeDirectory, "cmake"]);
    }
    if (!existsSync(libclangLibrary)) {
      console.log("Installing isolated libclang bindings for whisper.cpp…");
      run(python, ["-m", "pip", "install", "--disable-pip-version-check", "--target", libclangDirectory, "libclang"]);
    }
  }

  environment.CMAKE = cmakeExecutable;
  environment.LIBCLANG_PATH = libclangNativeDirectory;
  environment.PYTHONPATH = [cmakeDirectory, environment.PYTHONPATH].filter(Boolean).join(path.delimiter);
}

const command = process.platform === "win32" ? environment.ComSpec ?? "cmd.exe" : "npm";
const args = process.platform === "win32" ? ["/d", "/s", "/c", "npm exec tauri dev"] : ["exec", "tauri", "dev"];
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
