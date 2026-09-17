import { copyFileSync, existsSync, mkdirSync, rmSync, statSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const root = resolve(import.meta.dirname, "..");
const buildDirectory = join(root, "target", "trainer-build");
const distDirectory = join(buildDirectory, "dist");
const isWindows = process.platform === "win32";
const executableName = isWindows ? "pubg-trainer.exe" : "pubg-trainer";

const rustc = spawnSync("rustc", ["-vV"], { encoding: "utf8" });
if (rustc.status !== 0) {
  throw new Error(rustc.stderr || "rustc -vV failed");
}
const tripleLine = rustc.stdout.split("\n").find((line) => line.startsWith("host: "));
if (!tripleLine) throw new Error("Rust target triple was not found");
const targetTriple = tripleLine.slice("host: ".length).trim();
const destinationDirectory = join(root, "src-tauri", "binaries");
const suffix = isWindows ? ".exe" : "";
const destination = join(destinationDirectory, `pubg-trainer-${targetTriple}${suffix}`);
const trainerSource = join(root, "trainer", "pubg_trainer.py");
const requirements = join(root, "trainer", "requirements.txt");

if (
  existsSync(destination) &&
  statSync(destination).mtimeMs >= Math.max(statSync(trainerSource).mtimeMs, statSync(requirements).mtimeMs)
) {
  console.log(`Sidecar is current: ${basename(destination)}`);
  process.exit(0);
}

rmSync(buildDirectory, { recursive: true, force: true });
mkdirSync(buildDirectory, { recursive: true });
const python = process.env.PYTHON || (isWindows ? "python" : "python3");
const result = spawnSync(
  python,
  [
    "-m",
    "PyInstaller",
    "--noconfirm",
    "--clean",
    "--onefile",
    "--name",
    "pubg-trainer",
    "--distpath",
    distDirectory,
    "--workpath",
    join(buildDirectory, "work"),
    "--specpath",
    buildDirectory,
    trainerSource,
  ],
  { stdio: "inherit", cwd: root },
);
if (result.status !== 0) process.exit(result.status ?? 1);

mkdirSync(destinationDirectory, { recursive: true });
copyFileSync(join(distDirectory, executableName), destination);
console.log(`Sidecar copied to ${basename(destination)}`);
