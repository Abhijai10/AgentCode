use ac_daemon::{
    FinalReleaseManifest, MigrationSafetyReport, ReleaseCandidateGate, ReleaseChecklist,
    ReleaseEngineer, ReleaseStateMachine, ReleaseStatus, ReleaseVersion,
};
use ac_security::{SecretReviewInput, SecurityHardeningReview};

#[test]
fn full_release_simulation_validates_approves_and_releases_exact_artifact() {
    let mut security = SecurityHardeningReview::new();
    security
        .record_dependency("agentcode", "1.0.0", "MIT", "workspace", "sha256:agentcode")
        .unwrap();
    security
        .validate_secret_review(SecretReviewInput {
            canary_secret: "AC_FINAL_RELEASE_SECRET".to_string(),
            evidence: "validation [REDACTED]".to_string(),
            report: "release report [REDACTED]".to_string(),
            logs: "diagnostics [REDACTED]".to_string(),
        })
        .unwrap();
    security
        .run_boundary_campaign(true, true, true, true, true)
        .unwrap();
    let security_report = security.report();
    assert!(!security_report.release_blocked);

    let mut release = ReleaseEngineer::new();
    let build = release
        .capture_build("1.0.0-rc.1", "commit-final", "release", "macos-arm64")
        .unwrap();
    let version = ReleaseVersion {
        version: "1.0.0".to_string(),
        source_commit: "commit-final".to_string(),
    };
    let artifact = release
        .create_artifact(&version, "macos-arm64", "app-bundle", b"agentcode-final")
        .unwrap();
    assert!(release.verify_artifact(&artifact, b"agentcode-final"));

    let mut candidate = release
        .create_candidate(
            "1.0.0-rc.1",
            "rc.1",
            &build,
            "macos-arm64",
            vec!["release/rc/scope-freeze.md".to_string()],
        )
        .unwrap();
    let migration = MigrationSafetyReport {
        fresh_install: true,
        upgrade: true,
        schema_version: 18,
        interrupted_recovery: true,
        evidence_ref: "release/reports/migration.md".to_string(),
    };
    let checklist = ReleaseChecklist {
        security_checks: !security_report.release_blocked,
        tests: true,
        artifact_verification: release.verify_artifact(&artifact, b"agentcode-final"),
        migration_validation: migration.passed(18),
    };
    let validation =
        release.run_production_validation(&candidate, &migration, &checklist, true, "release/rc");
    let gate = ReleaseCandidateGate {
        tests_pass: true,
        security_pass: true,
        migrations_pass: true,
        artifacts_valid: true,
        evidence_refs: validation.evidence_refs(),
    };
    release
        .approve_candidate(&mut candidate, &validation, &gate)
        .unwrap();

    let manifest: FinalReleaseManifest = release
        .final_manifest(
            "1.0.0",
            vec![
                "autonomous mission flow".to_string(),
                "provider fallback".to_string(),
                "sandboxed tools".to_string(),
                "evidence verification".to_string(),
            ],
            vec!["0001..0018".to_string()],
            std::slice::from_ref(&artifact),
            vec!["signed/notarized artifact is prerequisite-bound".to_string()],
        )
        .unwrap();
    let decision = release
        .approve_release(&manifest, &validation, "security passed")
        .unwrap();
    assert_eq!(decision.approved_version, "1.0.0");
    assert!(release
        .evidence_bundle(
            "1.0.0",
            "release/reports/audit.md",
            "release/reports/security.md",
            "release/reports/validation.md",
            "release/reports/artifact.md",
            "release/reports/migration.md",
        )
        .unwrap()
        .complete());

    let mut state = ReleaseStateMachine::new("1.0.0").unwrap();
    state
        .transition(ReleaseStatus::Candidate, "release/rc-accepted")
        .unwrap();
    state
        .transition(ReleaseStatus::Approved, "release/decision")
        .unwrap();
    state
        .transition(ReleaseStatus::Released, "release/published-hash")
        .unwrap();
    assert_eq!(state.status, ReleaseStatus::Released);
}
