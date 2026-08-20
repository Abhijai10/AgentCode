# WP13 AppSec Extraction

## Donors Inspected

| Donor | Commit | Files inspected |
|---|---:|---|
| Semgrep | `808ee51cda4f` | `cli/src/semgrep/run_scan.py`; `cli/src/semgrep/rule_match.py` |
| Gitleaks | `b58d3f102cf3` | `cmd/detect.go` |
| Trivy | `dcbadb7b1507` | `pkg/commands/artifact/run.go` |
| ZAP | `e3793d73d04f` | `docker/zap-full-scan.py` |

## Mechanisms Discovered

### Semgrep scan orchestration

Source: `cli/src/semgrep/run_scan.py`.

Important symbols: `run_scan`, `TargetManager`, `ConfigLoader`, `Rule`, `RuleMatchMap`, `OutputHandler`, `BaselineHandler`, `target_mode_conf`, `sanity_check_resolved_config`.

Control flow: loads and validates configs, resolves target mode as historical/diff/whole scan, builds target manager and rules, runs core scanner, filters ignored/excluded matches, handles dependency-aware rules, formats outputs.

Failure behavior: invalid configs raise `SemgrepError`; missing config exits with configured code; baseline and product modes alter scope.

### Semgrep finding identity

Source: `cli/src/semgrep/rule_match.py`.

Important symbols: `RuleMatch`, `rule_id`, `path`, `start`, `end`, `git_blob`, `git_commit`, `syntactic_id`, `match_based_id`, `code_hash`, `pattern_hash`.

Control flow: wraps core match, attaches rule severity/message/metadata/fix, derives lines/context and multiple stable ids/hashes.

Failure behavior: historical findings can read source from git blob; otherwise from file path.

### Gitleaks secret scan modes

Source: `cmd/detect.go`.

Important symbols: `detectCmd`, `runDetect`, `Detector`, `DetectSource`, `sources.Files`, `sources.File`, `sources.Git`, `FollowSymlinks`, `findingSummaryAndExit`.

Control flow: initializes config/diagnostics/detector, chooses directory/stdin/git-history scan, sets symlink and archive limits, scans, logs errors, writes summary/exit code.

Failure behavior: directory/git errors are logged and report still generated where possible; stdin fatal exits because report cannot continue.

### Trivy artifact scanner

Source: `pkg/commands/artifact/run.go`.

Important symbols: `TargetKind`, `Runner`, `NewRunner`, `ScanFilesystem`, `ScanRepository`, `ScanSBOM`, `Filter`, `Report`, `Close`, `ScannerConfig`, `SkipScan`.

Control flow: initializes DB/VEX/WASM modules, chooses scan service by target kind/input/server, scans artifact, filters report, writes report, closes DB/modules.

Failure behavior: init errors wrap DB/VEX/WASM context; deferred close runs after failed initialization.

### ZAP app security scan

Source: `docker/zap-full-scan.py`.

Important symbols: `main`, `config_dict`, `ignore_scan_rules`, `in_progress_issues`, `report_json`, exit codes.

Control flow: parses target and policy, spiders, active scans, applies warn/fail/ignore classifications, writes reports.

Failure behavior: invalid config/options exit 3; warnings/failures are policy-mapped.

## Limitations and Rejections

Security scanners produce findings, not fixes or task truth. Auto-fix, active scanning, symlink following, remote DB/module downloads, and custom rules require policy gates and provenance.
