// One-off probe: load the built frontend in the real Chrome runtime and
// dump every console error that fires at app startup — this is how the
// "Something broke in the interface" renderer error is diagnosed.
use ac_common::StableId;
use ac_evidence::EvidenceStore;
use ac_security::Capability;
use ac_security::CapabilityPolicy;
use ac_verification::{BrowserAction, BrowserRuntime};

fn main() {
    let policy = CapabilityPolicy::new().allow(Capability::BrowserAutomation);
    let mut browser = BrowserRuntime::new(policy);
    let task = StableId::new("probe");
    let process = browser.launch(task.clone()).expect("chrome launch");
    let session = browser
        .create_session(task.clone(), process.id.clone())
        .expect("session");
    let noop = &mut EvidenceStore::new();
    for p in ["editorial", "bold", "technical"] {
        browser
            .act(
                &session.id,
                BrowserAction::Navigate {
                    url: format!("file:///tmp/agentcode-design-{p}.html"),
                },
                noop,
            )
            .expect("navigate");
        browser
            .act(&session.id, BrowserAction::Wait { millis: 700 }, noop)
            .expect("wait");
        let metrics = browser
            .act(
                &session.id,
                BrowserAction::Evaluate {
                    script: r#"(() => {
                      const h1 = document.querySelector('h1');
                      const h1s = h1 ? getComputedStyle(h1) : null;
                      const sections = document.querySelectorAll('section').length;
                      const cards = document.querySelectorAll('section div div').length;
                      const body = getComputedStyle(document.body);
                      return JSON.stringify({
                        h1_size: h1s ? h1s.fontSize : null,
                        h1_weight: h1s ? h1s.fontWeight : null,
                        font: body.fontFamily.slice(0, 40),
                        sections, cards,
                        body_h: document.body.scrollHeight,
                        overflow_wide: document.body.scrollWidth > window.innerWidth + 8,
                      });
                    })()"#
                        .to_string(),
                },
                noop,
            )
            .expect("metrics");
        println!(
            "{p}: {}",
            metrics.value.clone().unwrap_or(serde_json::json!(null))
        );
    }
    let _ = browser.close_process(&process.id);
}
