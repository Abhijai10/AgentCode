use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ac_agent::{AutonomousState, Goal};
use ac_changeset::ChangeSetState;
use ac_kernel::{AllowAllPolicy, Kernel};
use ac_security::{Capability, CapabilityPolicy};

fn run_git<const N: usize>(cwd: &Path, args: [&str; N]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:{}\nstderr:{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "agentcode-integration-{name}-{}",
        ac_common::StableId::new("tmp")
    ))
}

#[test]
fn autonomous_flow_reaches_completion_only_after_verification_evidence() {
    let source = temp_path("source");
    let worktree = temp_path("worktree");
    let _ = fs::remove_dir_all(&source);
    let _ = fs::remove_dir_all(&worktree);
    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(
        source.join("src/lib.rs"),
        "pub fn fixture_answer() -> u32 {\n    41\n}\n",
    )
    .unwrap();
    fs::create_dir_all(source.join("tests")).unwrap();
    fs::write(
        source.join("Cargo.toml"),
        "[package]\nname = \"agentcode-demo-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(
        source.join("tests/fixture.rs"),
        "use agentcode_demo_fixture::fixture_answer;\n\n#[test]\nfn fixture_answer_is_correct() {\n    assert_eq!(fixture_answer(), 42);\n}\n",
    )
    .unwrap();
    run_git(&source, ["init"]);
    run_git(&source, ["add", "."]);
    run_git(
        &source,
        [
            "-c",
            "user.name=AgentCode Test",
            "-c",
            "user.email=agentcode@example.test",
            "commit",
            "-m",
            "initial",
        ],
    );

    std::env::set_var("AGENTCODE_PROVIDER_MODE", "mock");
    let mut agent = ac_agent::isolated_workspace_agent_for_process_restricted_test(
        source.clone(),
        worktree.clone(),
        Kernel::new(AllowAllPolicy),
        CapabilityPolicy::new()
            .allow(Capability::FilesystemRead("*".to_string()))
            .allow(Capability::FilesystemWrite("*".to_string()))
            .allow(Capability::ProcessExec("*".to_string())),
    )
    .unwrap();

    let report = agent
        .run_goal(Goal::new("Fix the bug in src/lib.rs").unwrap())
        .unwrap();

    assert_eq!(report.state, AutonomousState::Completed);
    assert!(report.validation.as_ref().unwrap().passed);
    assert_eq!(
        report.changeset.as_ref().unwrap().state,
        ChangeSetState::Approved
    );
    let completion = report.completion_request.as_ref().unwrap();
    assert!(completion.verification_passed);
    assert!(completion.evidence_refs.len() >= 3);

    let _ = fs::remove_dir_all(source);
    let _ = fs::remove_dir_all(worktree);
}
