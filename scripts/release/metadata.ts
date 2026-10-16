import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

export const repoRoot = fileURLToPath(new URL("../../", import.meta.url));
export interface ReleaseIdentity {
  tag: string;
  version: string;
  commit: string;
  prerelease: boolean;
}
export function requireEnv(name: string): string {
  const value = process.env[name];
  if (!value?.trim()) throw new Error(`Missing environment variable: ${name}`);
  return value;
}
export function git(args: string[]): string {
  return execFileSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}
export function releaseVersion(tag: string): string {
  const version = tag.startsWith("v") ? tag.slice(1) : "";
  const semver =
    /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;
  if (!semver.test(version)) throw new Error("Use a semantic version tag such as v0.5.0.");
  return version;
}
export function assertVersions(version: string): void {
  const jsonVersion = (file: string) => {
    const data: unknown = JSON.parse(
      readFileSync(new URL(`../../${file}`, import.meta.url), "utf8"),
    );
    if (!data || typeof data !== "object" || !("version" in data))
      throw new Error(`Version missing: ${file}`);
    return data.version;
  };
  const cargo = readFileSync(
    new URL("../../apps/sonahop-ui/src-tauri/Cargo.toml", import.meta.url),
    "utf8",
  );
  const section = cargo.match(/^\[package\][\s\S]*?(?=^\[|(?![\s\S]))/m)?.[0];
  const cargoVersion = section?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  for (const [file, actual] of [
    ["apps/sonahop-ui/package.json", jsonVersion("apps/sonahop-ui/package.json")],
    [
      "apps/sonahop-ui/src-tauri/tauri.conf.json",
      jsonVersion("apps/sonahop-ui/src-tauri/tauri.conf.json"),
    ],
    ["apps/sonahop-ui/src-tauri/Cargo.toml", cargoVersion],
  ]) {
    if (actual !== version)
      throw new Error(`Tag version ${version} differs from ${file}: ${String(actual)}`);
  }
}
export function identity(tag: string): ReleaseIdentity {
  const version = releaseVersion(tag);
  const commit = git(["rev-parse", "HEAD"]);
  const tagCommit = git(["rev-parse", `refs/tags/${tag}^{commit}`]);
  if (commit !== tagCommit) throw new Error("Checkout does not match the tag commit.");
  const expected = process.env.EXPECTED_RELEASE_COMMIT;
  if (expected && commit !== expected)
    throw new Error("Tag moved after validation; refusing release.");
  return { tag, version, commit, prerelease: (version.split("+")[0] ?? "").includes("-") };
}
export function isOnMain(commit: string): boolean {
  const result = spawnSync(
    "git",
    ["merge-base", "--is-ancestor", commit, "refs/remotes/origin/main"],
    {
      cwd: repoRoot,
      stdio: "ignore",
    },
  );
  if (result.status !== 0 && result.status !== 1)
    throw new Error("Cannot check main ancestry; fetch full history.");
  return result.status === 0;
}
export function assetNames(version: string): string[] {
  return [
    `SonaHop-${version}-windows-x64.exe`,
    `SonaHop-${version}-windows-x64-setup.exe`,
    `SonaHop-${version}-android-arm64.apk`,
  ];
}
