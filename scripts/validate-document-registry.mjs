// AgentCode — document registry validator (P00-WP01)
// Validates docs/registry/documents.json against its declared validation rules.
// Usage: node scripts/validate-document-registry.mjs [--fix-status]
import { readFileSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const registryPath = join(root, "docs/registry/documents.json");
const registry = JSON.parse(readFileSync(registryPath, "utf8"));
const docs = registry.documents;
const errors = [];
const warnings = [];

const ids = new Set();
for (const d of docs) {
  if (ids.has(d.canonical_id)) {
    errors.push(`duplicate canonical_id: ${d.canonical_id}`);
  }
  ids.add(d.canonical_id);

  const filePath = join(root, "docs/coredocs", d.canonical_filename);
  if (!existsSync(filePath)) {
    errors.push(`missing canonical file for ${d.canonical_id}: ${d.canonical_filename}`);
    if (d.verified) warnings.push(`document ${d.canonical_id} marked verified but file missing`);
  } else {
    const sha = createHash("sha256").update(readFileSync(filePath)).digest("hex");
    if (d.sha256 && d.sha256 !== sha) {
      errors.push(`sha256 mismatch for ${d.canonical_id} (file changed since registration)`);
    }
    if (!d.sha256) warnings.push(`document ${d.canonical_id} has no recorded sha256`);
  }

  if (!d.title || !d.status || !d.domain || !d.authority) {
    errors.push(`incomplete record for ${d.canonical_id}`);
  }
}

// superseded alias check: no active document may share an authority domain
const domains = new Map();
for (const d of docs) {
  if (domains.has(d.domain) && d.status !== "SUPERSEDED") {
    errors.push(`two active documents share domain '${d.domain}'`);
  }
  domains.set(d.domain, d);
}

if (errors.length) {
  console.error("DOCUMENT REGISTRY VALIDATION FAILED");
  for (const e of errors) console.error(`  ERROR: ${e}`);
  for (const w of warnings) console.warn(`  WARN: ${w}`);
  process.exit(1);
}
console.log(`DOCUMENT REGISTRY VALIDATION PASSED (${docs.length} documents)`);
for (const w of warnings) console.warn(`  WARN: ${w}`);