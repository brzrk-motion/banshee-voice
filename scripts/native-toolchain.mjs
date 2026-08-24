import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import process from "node:process";

function run(command, args, cwd, environment) {
  const result = spawnSync(command, args, {
    cwd,
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
  throw new Error("Python is required to bootstrap CMake and libclang for the Windows native build.");
}

export function prepareNativeEnvironment(repositoryRoot, baseEnvironment = process.env) {
  const environment = { ...baseEnvironment };

  if (process.platform !== "win32") return environment;

  const toolsDirectory = path.join(repositoryRoot, "target", "tools");
  const cmakeDirectory = path.join(toolsDirectory, "cmake");
  const cmakeExecutable = path.join(cmakeDirectory, "bin", "cmake.exe");
  const libclangDirectory = path.join(toolsDirectory, "libclang");
  const libclangNativeDirectory = path.join(libclangDirectory, "clang", "native");
  const libclangLibrary = path.join(libclangNativeDirectory, "libclang.dll");

  if (!existsSync(cmakeExecutable) || !existsSync(libclangLibrary)) {
    const python = findPython();
    if (!existsSync(cmakeExecutable)) {
      console.log("Installing an isolated CMake toolchain for the native build...");
      run(
        python,
        ["-m", "pip", "install", "--disable-pip-version-check", "--target", cmakeDirectory, "cmake"],
        repositoryRoot,
        environment,
      );
    }
    if (!existsSync(libclangLibrary)) {
      console.log("Installing isolated libclang bindings for the native build...");
      run(
        python,
        ["-m", "pip", "install", "--disable-pip-version-check", "--target", libclangDirectory, "libclang"],
        repositoryRoot,
        environment,
      );
    }
  }

  environment.CMAKE = cmakeExecutable;
  environment.LIBCLANG_PATH = libclangNativeDirectory;
  environment.PYTHONPATH = [cmakeDirectory, environment.PYTHONPATH].filter(Boolean).join(path.delimiter);
  return environment;
}
