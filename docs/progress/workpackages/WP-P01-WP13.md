# WP-P01-WP13 — AppSec Extraction Campaign

- **Phase:** P01
- **Status:** ACCEPTED
- **Risk:** HIGH | **Release scope:** REQUIRED_V1
- **Base commit:** 60d7cc1526643a6af7932825fb7455295813e55e
- **Accepted commit:** d2c64fe9a77bc8be119d7e019fd40ffc407f3d6b
- **Owner modules:** docs/extraction/, docs/extraction/packets/, docs/extraction/decisions/
- **Architecture refs:** DOC-07 security campaigns; DOC-10 P1-G4; DOC-11 P01-WP13
- **Acceptance gates:** P1-G4 extraction evidence

## Objective

Create implementation-grade AppSec extraction artifacts covering scanner contracts, finding identity, secret scanning, dependency/IaC/app scans, active scan policy, and security evidence normalization.

## Outputs

- `docs/extraction/13-appsec/EXTRACTION_13_APPSEC.md`
- `docs/extraction/decisions/ADOPTION_13_APPSEC.md`
- `docs/extraction/packets/P01_WP13_APPSEC_IMPLEMENTATION_PACKET.md`

## Evidence

Artifacts cite donor repositories, pinned commits, exact source paths, mechanisms, control flows, failure behavior, limitations, and AgentCode adoption decisions. No production implementation code was added.
