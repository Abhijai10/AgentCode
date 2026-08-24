use ac_agent::{DogfoodHarness, DogfoodMissionInput, DogfoodMissionKind};
use ac_changeset::ChangeSetState;
use ac_common::StableId;
use ac_runtime::{ChaosFaultKind, ChaosHarness, ChaosRunConfig};

#[test]
fn chaos_and_dogfood_flows_record_recovery_and_verified_self_improvement() {
    let mut chaos = ChaosHarness::new();
    let scenario = ChaosHarness::catalog()
        .into_iter()
        .find(|scenario| scenario.fault_kind == ChaosFaultKind::WorkerDeath)
        .unwrap();
    let experiment = chaos
        .run_scenario(
            &scenario,
            ChaosRunConfig {
                mission_id: StableId::new("mission"),
                seed: 25,
                repeats: 3,
            },
        )
        .unwrap();
    assert!(experiment.state_equivalent);
    assert_eq!(experiment.passes, 3);
    assert!(experiment.timeline.iter().any(|event| {
        event.observed_behavior.contains("lease expired")
            && event.recovery_action.contains("automatic")
    }));
    assert_eq!(
        chaos.reliability_report("integration").recovery_percent,
        100
    );

    let mut dogfood = DogfoodHarness::new();
    let mission = dogfood
        .run_self_mission(DogfoodMissionInput {
            repository_id: StableId::new("repo"),
            repository_path: "/repo/agentcode".to_string(),
            commit_ref: "integration-commit".to_string(),
            kind: DogfoodMissionKind::MultiFileFeature,
            objective: "add integrated chaos evidence plumbing".to_string(),
        })
        .unwrap();
    assert_eq!(mission.status, "verified");
    assert_eq!(mission.changeset.state, ChangeSetState::Accepted);
    assert_eq!(mission.proposals[0].decision, "accepted");
    assert!(!mission.privileged_bypass_used);
    assert!(mission
        .verification_report_id
        .as_str()
        .starts_with("verify-"));
}
