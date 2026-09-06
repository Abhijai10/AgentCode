// AgentCode — reference catalog collector (P01-WP01)
// Scans the reference library and records per-repo identity: HEAD SHA, branch,
// dirty state, remote URL, license files. Writes docs/reference/agentcode_reference_catalog.json.
import { execSync } from "node:child_process";
import { readdirSync, existsSync, writeFileSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const REF = "/Volumes/T7 Shield/GitHub-Repos-dependency";
const OUT = "/Volumes/T7 Shield/PROJECTS/AgentCode/docs/reference/agentcode_reference_catalog.json";

const licenseNames = ["LICENSE", "LICENSE.md", "LICENSE.txt", "LICENSE.rst", "LICENCE", "LICENCE.md", "LICENCE.txt", "COPYING", "COPYING.md", "COPYING.txt", "LICENSE-APACHE", "LICENSE-MIT"];

function sh(cwd, cmd) {
  try {
    return execSync(cmd, { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"], timeout: 15000 }).trim();
  } catch {
    return null;
  }
}

const entries = [];
for (const dir of readdirSync(REF)) {
  const path = join(REF, dir);
  if (!statSync(path).isDirectory()) continue;
  const gitDir = join(path, ".git");
  const hasGit = existsSync(gitDir);
  const entry = {
    catalog_id: null,
    local_path: `reference://${dir}`,
    directory: dir,
    remote_url: null,
    branch: null,
    head_sha: null,
    dirty: null,
    license_files: [],
    has_git: hasGit,
    category: null,
    priority: null,
    primary_role: null,
    possible_agentcode_use: null,
    extraction_status: "NOT_EXTRACTED",
    extracted_at_commit: null,
  };
  if (hasGit) {
    entry.remote_url = sh(path, "git remote get-url origin");
    entry.branch = sh(path, "git rev-parse --abbrev-ref HEAD");
    entry.head_sha = sh(path, "git rev-parse HEAD");
    const dirty = sh(path, "git status --porcelain");
    entry.dirty = dirty && dirty.length > 0 ? dirty.split("\n").length : 0;
  }
  for (const l of licenseNames) {
    if (existsSync(join(path, l))) entry.license_files.push(l);
  }
  // peek at declared license for common files
  const pkg = join(path, "package.json");
  if (existsSync(pkg)) {
    try {
      const p = JSON.parse(readFileSync(pkg, "utf8"));
      entry.package_json_license = p.license ?? null;
    } catch { /* ignore */ }
  }
  entries.push(entry);
}

entries.sort((a, b) => a.directory.localeCompare(b.directory));

const catalog = {
  schema_version: 1,
  kind: "agentcode_reference_catalog",
  reference_root: "/Volumes/T7 Shield/GitHub-Repos-dependency",
  notice: "Research input only. Never a runtime/build dependency. No committed path references this root.",
  generated_at: new Date().toISOString(),
  repositories: entries,
};

writeFileSync(OUT, JSON.stringify(catalog, null, 2) + "\n");
console.log(`catalog written: ${entries.length} repositories`);
const missing = entries.filter((e) => !e.head_sha);
for (const m of missing) console.warn(`  no git metadata: ${m.directory}`);