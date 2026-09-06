// AgentCode — enrich reference catalog with categories, priorities, roles, and
// license classification from actual license files (P01-WP01/P01-WP02).
// Reads agentcode_reference_catalog.json, enriches, rewrites it, and emits
// docs/reference/licenses_scan.json for the license matrix.
import { readFileSync, writeFileSync, existsSync, readdirSync } from "node:fs";
import { join } from "node:path";

const REF = "/Volumes/T7 Shield/GitHub-Repos-dependency";
const CATALOG_PATH = "/Volumes/T7 Shield/PROJECTS/AgentCode/docs/reference/agentcode_reference_catalog.json";

// Category and priority per Doc 07 sections 14-19.
const META = {
  "OmniRoute":            { category: "MODEL / PROVIDER ROUTING", priority: "P0", role: "Provider fabric reference: routing, fallback, model capability mapping", use: "Phase 4 Model Broker / OmniRoute-compatible provider gateway (ADR-0006)" },
  "codex":                { category: "CODING AGENTS", priority: "P0", role: "OpenAI Codex CLI — Rust core, agent loop, changesets, sandbox, worktrees", use: "Runtime, tool loop, editing, git, sandbox, context patterns" },
  "gemini-cli":           { category: "CODING AGENTS", priority: "P0", role: "Google Gemini CLI — TS agent with tools, sessions, approvals", use: "Runtime/tool-loop and session patterns" },
  "OpenHands":            { category: "CODING AGENTS", priority: "P0", role: "OpenHands — Python agent runtime, tools, sandbox (docker), git service", use: "Runtime, tools, git, sandbox patterns" },
  "software-agent-sdk":   { category: "CODING AGENTS", priority: "P0", role: "OpenHands software-agent-sdk", use: "Runtime/service-boundary patterns" },
  "aider":                { category: "CODING AGENTS", priority: "P0", role: "Aider — Python pair-programming agent: repo map, edit formats, git", use: "Code intelligence (repo map), edit, git, context patterns" },
  "mini-swe-agent":       { category: "CODING AGENTS", priority: "P0", role: "Small SWE agent — minimal loop, tools, simple harness", use: "Simplest full agent-loop reference" },
  "SWE-ReX":              { category: "CODING AGENTS", priority: "P0", role: "SWE-ReX — ReX runtime, REST API, tool execution server", use: "Tool-execution/runtime boundary patterns" },
  "tree-sitter":          { category: "CODE INTELLIGENCE", priority: "P0", role: "Incremental parsing framework", use: "Code intelligence syntax layer (Phase 8)" },
  "ast-grep":             { category: "CODE INTELLIGENCE", priority: "P0", role: "AST-based structural search/rewrite", use: "Code intelligence search patterns (Phase 8)" },
  "ripgrep":              { category: "CODE INTELLIGENCE", priority: "P0", role: "Fast regex search", use: "Exact search primitive (external tool policy)" },
  "letta-code":           { category: "MEMORY / CONTEXT", priority: "P0", role: "Letta Code — coding agent with memory blocks", use: "Context/memory patterns (Phase 9)" },
  "rtk":                  { category: "TOKEN EFFICIENCY", priority: "P0", role: "RTK — token compression library", use: "Output compression/context compaction patterns" },
  "playwright":           { category: "BROWSER / QA", priority: "P0", role: "Browser automation", use: "Browser/QA phases" },
  "openhack":             { category: "ORCHESTRATION", priority: "P0", role: "Hackathon-style multi-agent orchestration app", use: "Orchestration UX patterns" },
  "opencode":             { category: "CODING AGENTS", priority: "P1", role: "TypeScript terminal coding agent — tools, edit, sessions, permissions", use: "Tool/edit/permission/session patterns" },
  "cline":                { category: "CODING AGENTS", priority: "P1", role: "VS Code extension agent — permissions, plans, tools", use: "Permission/approval and edit patterns (dirty tree noted)" },
  "goose":                { category: "CODING AGENTS", priority: "P1", role: "Block-based agent — extensions, MCP, sandboxed commands", use: "Extension/tool patterns" },
  "langgraph":            { category: "ORCHESTRATION", priority: "P1", role: "LangGraph — stateful graph orchestration", use: "Orchestration patterns (evaluation)" },
  "agent-framework":      { category: "ORCHESTRATION", priority: "P1", role: "Microsoft Agent Framework", use: "Multi-agent patterns" },
  "scip":                 { category: "CODE INTELLIGENCE", priority: "P1", role: "SCIP index format", use: "Symbol indexing format (optional)" },
  "zoekt":                { category: "CODE INTELLIGENCE", priority: "P1", role: "Code search index", use: "Optional search backend" },
  "graphiti":             { category: "MEMORY / CONTEXT", priority: "P1", role: "Temporal knowledge graph memory", use: "Memory/retrieval patterns" },
  "superpowers":          { category: "SKILLS / WORKFLOWS", priority: "P1", role: "Claude skills/workflow library", use: "Skill packaging patterns" },
  "compound-engineering-plugin": { category: "SKILLS / WORKFLOWS", priority: "P1", role: "Compound engineering skills", use: "Skill patterns" },
  "onlook":               { category: "DESIGN / APP BUILDERS", priority: "P1", role: "Visual editor / DOM mapping", use: "Design Studio patterns" },
  "dyad":                 { category: "DESIGN / APP BUILDERS", priority: "P1", role: "Web app builder", use: "Design/App builder patterns" },
  "bolt.diy":             { category: "DESIGN / APP BUILDERS", priority: "P1", role: "Bolt-style app builder fork", use: "App builder patterns" },
  "trailofbits-skills":    { category: "SKILLS / WORKFLOWS", priority: "P1", role: "Trail of Bits security skills", use: "Security skill patterns" },
  "prowler":              { category: "CLOUD SECURITY", priority: "P1", role: "AWS security assessments", use: "Cloud security phase" },
  "semgrep":              { category: "APPLICATION SECURITY", priority: "P2", role: "Static analysis", use: "Security phases (external tool)" },
  "codeql":               { category: "APPLICATION SECURITY", priority: "P2", role: "CodeQL analysis", use: "Security phases (external tool)" },
  "gitleaks":             { category: "APPLICATION SECURITY", priority: "P2", role: "Secret scanning", use: "Secret scan tooling (external)" },
  "osv-scanner":          { category: "APPLICATION SECURITY", priority: "P2", role: "Vulnerability scanner", use: "Dependency scanning (external)" },
  "trivy":                { category: "APPLICATION SECURITY", priority: "P2", role: "Container/filesystem scanner", use: "Security phases" },
  "checkov":              { category: "APPLICATION SECURITY", priority: "P2", role: "IaC scanning", use: "Security phases" },
  "zaproxy":              { category: "APPLICATION SECURITY", priority: "P2", role: "Web app scanner/proxy", use: "Browser/security phases" },
  "nuclei":               { category: "APPLICATION SECURITY", priority: "P2", role: "Vulnerability templates scanner", use: "Security phases" },
  "nuclei-templates":     { category: "APPLICATION SECURITY", priority: "P3", role: "Nuclei template library", use: "Reference templates only" },
  "ScoutSuite":           { category: "CLOUD SECURITY", priority: "P2", role: "Multi-cloud audit", use: "Cloud security phase" },
  "cloudsploit":          { category: "CLOUD SECURITY", priority: "P2", role: "Cloud audit", use: "Cloud security phase" },
  "stratus-red-team":     { category: "RED TEAM", priority: "P2", role: "Cloud attack simulation", use: "Cloud security/red team" },
  "pacu":                 { category: "RED TEAM", priority: "P2", role: "AWS exploitation framework", use: "Cloud security phase" },
  "cloudgoat":            { category: "RED TEAM", priority: "P2", role: "AWS vulnerable labs", use: "Cloud security fixtures" },
  "promptfoo":            { category: "EVALUATION", priority: "P2", role: "LLM eval framework", use: "Model evaluation" },
  "garak":                { category: "AI SECURITY", priority: "P2", role: "LLM vulnerability scanner", use: "AI security phase" },
  "PyRIT":                { category: "AI SECURITY", priority: "P2", role: "AI risk tooling", use: "AI security phase" },
  "SWE-bench":            { category: "EVALUATION", priority: "P2", role: "SWE benchmark", use: "Evaluation fixtures" },
  "continue":             { category: "CODING AGENTS", priority: "P2", role: "IDE assistant", use: "UX/context patterns" },
  "Roo-Code":             { category: "CODING AGENTS", priority: "P2", role: "Roo Code extension", use: "Mode/role patterns" },
  "daytona":              { category: "EXECUTION / SANDBOX", priority: "P2", role: "Dev environment manager", use: "Sandbox/execution patterns" },
  "letta":                { category: "MEMORY / CONTEXT", priority: "P3", role: "Letta agent framework (memory blocks)", use: "Memory patterns (evaluation)" },
  "letta-skills":         { category: "SKILLS / WORKFLOWS", priority: "P3", role: "Letta skills", use: "Skill patterns" },
  "caveman":              { category: "CODING AGENTS", priority: "P3", role: "Caveman agent", use: "Reference only" },
  "browser-use":          { category: "BROWSER / QA", priority: "P3", role: "Browser use agent", use: "Browser phase patterns" },
  "browser-harness":      { category: "BROWSER / QA", priority: "P3", role: "Browser harness", use: "Browser phase patterns" },
  "servers":              { category: "SKILLS / WORKFLOWS", priority: "P3", role: "MCP servers collection", use: "MCP reference" },
  "smolagents":           { category: "CODING AGENTS", priority: "P3", role: "HF smolagents", use: "Reference only" },
  "ctags":                { category: "CODE INTELLIGENCE", priority: "P3", role: "Universal Ctags", use: "Optional symbol source" },
  "stack-graphs":         { category: "CODE INTELLIGENCE", priority: "P3", role: "Stack graphs", use: "Optional symbol resolution" },
  "scorecard":            { category: "EVALUATION", priority: "P3", role: "OpenSSF scorecard", use: "Supply-chain evaluation" },
  "munder-difflin":       { category: "SKILLS / WORKFLOWS", priority: "P3", role: "Munder Difflin", use: "REJECTED by Doc 07 §25-26 (recorded)" },
};

const SPDX_PATTERNS = [
  [/dual-licensed under the Unlicense and MIT|Unlicense OR MIT/i, "Unlicense OR MIT"],
  [/MIT License|Permission is hereby granted, free of charge/i, "MIT"],
  [/Apache License[\s\n]*Version 2\.0/i, "Apache-2.0"],
  [/Attribution-ShareAlike [\d.]+ International/i, "CC-BY-SA-4.0"],
  [/GNU GENERAL PUBLIC LICENSE[\s\n]*Version 2/i, (text) => /any later version/i.test(text) ? "GPL-2.0-or-later" : "GPL-2.0"],
  [/GNU GENERAL PUBLIC LICENSE[\s\n]*Version 3/i, (text) => /any later version/i.test(text) ? "GPL-3.0-or-later" : "GPL-3.0"],
  [/GNU AFFERO GENERAL PUBLIC LICENSE/i, "AGPL-3.0"],
  [/GNU LESSER GENERAL PUBLIC LICENSE/i, (text) => /any later version/i.test(text) ? "LGPL-2.1-or-later" : "LGPL-2.1-only"],
  [/BSD 2-Clause|Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met/i, "BSD-2-Clause"],
  [/BSD 3-Clause|Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:\s*\n\s*\n\s*1\. Redistributions of source code/i, "BSD-3-Clause"],
  [/Mozilla Public License Version 2\.0/i, "MPL-2.0"],
  [/ISC License|Permission to use, copy, modify, and\/or distribute this software/i, "ISC"],
  [/Zlib license|This software is provided 'as-is', without any express or implied warranty/i, "Zlib"],
  [/CC0 1\.0 Universal|Creative Commons Zero|No Rights Reserved/i, "CC0-1.0"],
  [/Business Source License|BSL/i, "BSL-1.1"],
  [/Server Side Public License/i, "SSPL-1.0"],
  [/Elastic License/i, "Elastic-2.0"],
  [/Unlicense/i, "Unlicense"],
];

function classifyLicense(text, fileName) {
  if (!text) return { spdx: null, source: fileName };
  for (const [re, spdx] of SPDX_PATTERNS) {
    if (re.test(text)) {
      const value = typeof spdx === "function" ? spdx(text) : spdx;
      return { spdx: value, source: fileName };
    }
  }
  return { spdx: "UNKNOWN", source: fileName };
}

const catalog = JSON.parse(readFileSync(CATALOG_PATH, "utf8"));
const scans = [];
for (const entry of catalog.repositories) {
  const meta = META[entry.directory] ?? { category: "UNCATEGORIZED", priority: "P3", role: null, use: null };
  entry.catalog_id = `REF-${String(parseInt(entry.catalog_id?.replace("REF-", "") ?? scans.length + 1)).padStart(3, "0")}`;
  entry.category = meta.category;
  entry.priority = meta.priority;
  entry.primary_role = meta.role;
  entry.possible_agentcode_use = meta.use;

  // license classification from actual files
  let license = null;
  for (const lf of entry.license_files ?? []) {
    const p = join(REF, entry.directory, lf);
    if (existsSync(p)) {
      const text = readFileSync(p, "utf8").slice(0, 60000);
      const r = classifyLicense(text, lf);
      if (r.spdx) { license = r; break; }
      license = r; // keep last UNKNOWN
    }
  }
  entry.license_spdx = license?.spdx ?? (entry.package_json_license ?? "UNKNOWN");
  entry.license_file_used = license?.source ?? null;
  scans.push({
    catalog_id: entry.catalog_id,
    directory: entry.directory,
    head_sha: entry.head_sha,
    license_spdx: entry.license_spdx,
    license_file: entry.license_file_used,
    priority: entry.priority,
  });
}

writeFileSync(CATALOG_PATH, JSON.stringify(catalog, null, 2) + "\n");
writeFileSync("/Volumes/T7 Shield/PROJECTS/AgentCode/docs/reference/licenses_scan.json", JSON.stringify({ schema_version: 1, generated_at: new Date().toISOString(), repos: scans }, null, 2) + "\n");
console.log("enriched. unknown licenses:", scans.filter((s) => s.license_spdx === "UNKNOWN" || !s.license_spdx).map((s) => `${s.directory}(${s.license_file ?? "no file"})`).join(", ") || "none");