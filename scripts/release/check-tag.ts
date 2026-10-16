import { appendFileSync } from "node:fs";
import { assertVersions, identity, isOnMain, requireEnv } from "./metadata.ts";

const value = identity(requireEnv("RELEASE_TAG"));
const approved = isOnMain(value.commit);
if (approved) assertVersions(value.version);
const output = `${[
  `approved=${String(approved)}`,
  `tag=${value.tag}`,
  `version=${value.version}`,
  `commit=${value.commit}`,
  `prerelease=${String(value.prerelease)}`,
].join("\n")}\n`;
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, output);
console.log(
  approved
    ? "Release tag belongs to main and all app versions match."
    : "Tag is outside main; build and release jobs will be skipped.",
);
