# WP14 Cloud Security Extraction

## Donors Inspected

| Donor | Commit | Files inspected |
|---|---:|---|
| Checkov | `5458c889320a` | `checkov/main.py` |
| Prowler | `0b9791ffdc6e` | `prowler/__main__.py` |
| Trivy | `dcbadb7b1507` | `pkg/commands/artifact/run.go` |

## Mechanisms Discovered

### Checkov multi-runner IaC scan

Source: `checkov/main.py`.

Important symbols: `Checkov`, `DEFAULT_RUNNERS`, `parse_config`, `normalize_config`, `RunnerRegistry`, `RunnerFilter`, `Baseline`, `scan_reports`, `graphs`.

Control flow: parses CLI/config/env, normalizes frameworks and skips, validates platform API requirements, creates runners for Terraform/CloudFormation/Kubernetes/Dockerfile/secrets/SCA/SAST/etc., loads custom policies and metadata, runs selected frameworks, stores reports.

Failure behavior: missing platform API fields are parser errors; missing config files are ignored only under validation logic; signature verification errors are imported as explicit errors.

### Prowler provider/check/compliance workflow

Source: `prowler/__main__.py`.

Important symbols: `ProwlerArgumentParser`, `Provider.init_global_provider`, `load_checks_to_execute`, `Compliance.get_bulk`, `CheckMetadata.get_bulk`, `execute_checks`, `report`, output classes `CSV`, `SARIF`, `OCSF`, `HTML`, `ASFF`.

Control flow: parses provider/check/service/category/severity/compliance filters, loads provider metadata, initializes provider credentials, loads checks and compliance frameworks, executes checks, writes multiple report formats.

Failure behavior: list-only modes exit before scan; external tool providers skip compliance framework loading; credentials are printed through provider abstraction.

### Trivy cloud/IaC artifact modes

Source: `pkg/commands/artifact/run.go`.

Important symbols: `TargetK8s`, `TargetFilesystem`, `TargetRepository`, `TargetSBOM`, `Filter`, `Report`.

Control flow: target kind determines scanner service; results are filtered and reported after DB/VEX/module setup.

Failure behavior: setup failures are contextualized and close cleans resources.

## Limitations and Rejections

Cloud scanners require credentials and may call cloud APIs. AgentCode must never let donor credential discovery, broad account enumeration, remediation, or compliance selection bypass Kernel policy.
