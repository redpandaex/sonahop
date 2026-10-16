import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { assetNames, releaseVersion, repoRoot, requireEnv } from "./metadata.ts";

if (
  process.env.GITHUB_ACTIONS !== "true" ||
  process.env.GITHUB_EVENT_NAME !== "push" ||
  process.env.GITHUB_REF_TYPE !== "tag"
)
  throw new Error("Publishing is restricted to a GitHub Actions tag push.");
const repo = requireEnv("GITHUB_REPOSITORY");
if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repo)) throw new Error("Invalid repository.");
const tag = requireEnv("RELEASE_TAG");
const version = releaseVersion(tag);
const commit = requireEnv("EXPECTED_RELEASE_COMMIT");
if (!/^[a-f0-9]{40,64}$/.test(commit)) throw new Error("Invalid approved commit.");
const prerelease = (version.split("+")[0] ?? "").includes("-");
const gh = (args: string[]) =>
  execFileSync("gh", args, {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
function gitObject(response: unknown): { type: "commit" | "tag"; sha: string } {
  if (!response || typeof response !== "object" || !("object" in response))
    throw new Error("GitHub returned an invalid Git reference.");
  const object = response.object;
  if (
    !object ||
    typeof object !== "object" ||
    !("type" in object) ||
    (object.type !== "commit" && object.type !== "tag") ||
    !("sha" in object) ||
    typeof object.sha !== "string" ||
    !/^[a-f0-9]{40,64}$/.test(object.sha)
  )
    throw new Error("GitHub returned an invalid Git object.");
  return { type: object.type, sha: object.sha };
}

function assertRemoteSource() {
  let target = gitObject(
    JSON.parse(gh(["api", `repos/${repo}/git/ref/tags/${encodeURIComponent(tag)}`])),
  );
  for (let depth = 0; target.type === "tag" && depth < 16; depth++) {
    target = gitObject(JSON.parse(gh(["api", `repos/${repo}/git/tags/${target.sha}`])));
  }
  if (target.type !== "commit" || target.sha !== commit)
    throw new Error("Remote tag no longer points to the approved commit.");
  const main = gitObject(JSON.parse(gh(["api", `repos/${repo}/git/ref/heads/main`])));
  if (main.type !== "commit") throw new Error("main does not point to a commit.");
  const comparison: unknown = JSON.parse(
    gh(["api", `repos/${repo}/compare/${commit}...${main.sha}`]),
  );
  if (
    !comparison ||
    typeof comparison !== "object" ||
    !("status" in comparison) ||
    (comparison.status !== "ahead" && comparison.status !== "identical")
  )
    throw new Error("Approved commit is no longer on main.");
}

assertRemoteSource();
const folder = join(repoRoot, "artifacts/release-assets");
const parsed: unknown = JSON.parse(readFileSync(join(folder, "release-manifest.json"), "utf8"));
if (
  !parsed ||
  typeof parsed !== "object" ||
  !("tag" in parsed) ||
  !("commit" in parsed) ||
  !("version" in parsed) ||
  parsed.tag !== tag ||
  parsed.commit !== commit ||
  parsed.version !== version ||
  !("artifacts" in parsed) ||
  !Array.isArray(parsed.artifacts)
)
  throw new Error("Artifact manifest is not for the approved tag/commit.");
const names = assetNames(version);
if (parsed.artifacts.length !== names.length) throw new Error("Expected three release binaries.");
const rows = parsed.artifacts as unknown[];
const checksums = `${names
  .map((name) => {
    const matches = rows.filter(
      (row) => row && typeof row === "object" && "name" in row && row.name === name,
    );
    if (matches.length !== 1) throw new Error(`Missing or duplicate artifact: ${name}`);
    const row = matches[0];
    if (!row || typeof row !== "object" || !("sha256" in row) || !("bytes" in row))
      throw new Error("Invalid artifact metadata.");
    const bytes = readFileSync(join(folder, name));
    const hash = createHash("sha256").update(bytes).digest("hex");
    if (hash !== row.sha256 || bytes.length !== row.bytes)
      throw new Error(`Artifact checksum mismatch: ${name}`);
    return `${hash}  ${name}`;
  })
  .join("\n")}\n`;
if (readFileSync(join(folder, "SHA256SUMS.txt"), "utf8") !== checksums)
  throw new Error("Checksum file differs from verified artifact hashes.");
const view = spawnSync("gh", ["release", "view", tag, "--repo", repo, "--json", "isDraft"], {
  cwd: repoRoot,
  encoding: "utf8",
});
if (view.status === 0) {
  const existing: unknown = JSON.parse(view.stdout);
  if (
    !existing ||
    typeof existing !== "object" ||
    !("isDraft" in existing) ||
    existing.isDraft !== true
  )
    throw new Error("This release is already published; published files will not be overwritten.");
} else if (/release not found|HTTP 404/i.test(view.stderr)) {
  gh([
    "release",
    "create",
    tag,
    "--repo",
    repo,
    "--verify-tag",
    "--target",
    commit,
    "--draft",
    "--title",
    `SonaHop ${version}`,
    "--generate-notes",
    ...(prerelease ? ["--prerelease"] : []),
  ]);
} else {
  throw new Error(`Cannot inspect the existing release: ${view.stderr.trim()}`);
}
const uploads = [...names, "SHA256SUMS.txt", "release-manifest.json"].map((name) =>
  join(folder, name),
);
// Only unpublished drafts can be retried. Recheck refs after the potentially long upload.
gh(["release", "upload", tag, "--repo", repo, "--clobber", ...uploads]);
assertRemoteSource();
gh([
  "release",
  "edit",
  tag,
  "--repo",
  repo,
  "--verify-tag",
  "--draft=false",
  `--prerelease=${String(prerelease)}`,
]);
console.log(`Published SonaHop ${version} with all verified assets.`);
