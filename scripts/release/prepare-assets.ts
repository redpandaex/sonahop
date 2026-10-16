import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  assertVersions,
  assetNames,
  identity,
  isOnMain,
  repoRoot,
  requireEnv,
} from "./metadata.ts";

const value = identity(requireEnv("RELEASE_TAG"));
if (!isOnMain(value.commit)) throw new Error("Tag is outside main.");
assertVersions(value.version);
const input = join(repoRoot, "artifacts/release-build");
const output = join(repoRoot, "artifacts/release-assets");
mkdirSync(output, { recursive: true });
const sources = [
  "sonahop-windows.exe",
  "sonahop-windows-setup.exe",
  "sonahop-android-arm64-release.apk",
];
const names = assetNames(value.version);
const files = names.map((name, index) => {
  const source = sources[index];
  if (!source) throw new Error("Missing asset source.");
  const target = join(output, name);
  copyFileSync(join(input, source), target);
  return {
    name,
    bytes: statSync(target).size,
    sha256: createHash("sha256").update(readFileSync(target)).digest("hex"),
  };
});
const signature: unknown = JSON.parse(readFileSync(join(input, "android-signature.json"), "utf8"));
if (
  !signature ||
  typeof signature !== "object" ||
  !("verified" in signature) ||
  signature.verified !== true
)
  throw new Error("No successful APK signature verification record.");
const manifest = { ...value, artifacts: files, androidSignature: signature };
writeFileSync(join(output, "release-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
writeFileSync(
  join(output, "SHA256SUMS.txt"),
  `${files.map((file) => `${file.sha256}  ${file.name}`).join("\n")}\n`,
);
console.log("Prepared three binaries with checksums and source commit metadata.");
