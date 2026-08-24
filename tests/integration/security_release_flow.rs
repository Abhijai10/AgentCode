use ac_daemon::{ReleaseChecklist, ReleaseEngineer, ReleaseVersion, UpdateDecision};
use ac_security::{SecretReviewInput, SecurityHardeningReview};

#[test]
fn security_audit_release_validation_and_artifact_approval_flow() {
    let mut security = SecurityHardeningReview::new();
    security
        .record_dependency(
            "agentcode-workspace",
            "0.1.0",
            "MIT",
            "workspace",
            "sha256:workspace",
        )
        .unwrap();
    security
        .record_supply_chain("ac-daemon", "workspace", "0.1.0", "sha256:daemon")
        .unwrap();
    security
        .validate_secret_review(SecretReviewInput {
            canary_secret: "AC_SECRET_CANARY".to_string(),
            evidence: "evidence [REDACTED]".to_string(),
            report: "report [REDACTED]".to_string(),
            logs: "logs [REDACTED]".to_string(),
        })
        .unwrap();
    security
        .run_boundary_campaign(true, true, true, true, true)
        .unwrap();
    security.generate_sbom().unwrap();
    let report = security.report();
    assert!(!report.release_blocked);

    let mut release = ReleaseEngineer::new();
    release
        .capture_build("1.0.0", "commit-a", "release", "macos-arm64")
        .unwrap();
    let version = ReleaseVersion {
        version: "1.0.0".to_string(),
        source_commit: "commit-a".to_string(),
    };
    let bytes = b"release artifact";
    let artifact = release
        .create_artifact(&version, "macos-arm64", "app-bundle", bytes)
        .unwrap();
    let checklist = ReleaseChecklist {
        security_checks: !report.release_blocked,
        tests: true,
        artifact_verification: release.verify_artifact(&artifact, bytes),
        migration_validation: true,
    };
    assert!(checklist.approved());
    let update = release.plan_update("0.9.0", &artifact, bytes);
    assert_eq!(update.decision, UpdateDecision::Install);
    assert!(update.verified);
}
