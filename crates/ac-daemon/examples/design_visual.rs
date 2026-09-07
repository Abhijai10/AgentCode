use serde_json::json;

// A tiny visual smoke for the design-system renderer: renders the same
// rich spec through three personalities and writes the HTML files for
// browser inspection (deterministic, no provider).
fn main() {
    let spec = json!({
        "headline": "Ship governed code, not promises",
        "subheadline": "AgentCode plans, executes and verifies missions with real evidence — every claim traceable to tool output.",
        "primary_cta": "Start a mission",
        "secondary_cta": "See evidence",
        "hero_image_idea": "",
        "section_ideas": ["Kernel-gated missions", "Evidence-first verification", "Worktree isolation"],
        "audience": "Engineering teams who audit everything",
        "voice": "Confident, technical, zero fluff",
        "personality": "editorial",
        "stats": [
            {"value": "470", "label": "tests green"},
            {"value": "0", "label": "unverified claims"},
            {"value": "24/7", "label": "local daemon"}
        ],
        "feature_details": [
            {"title": "Kernel authority", "body": "One writer owns mission and completion truth."},
            {"title": "Evidence chain", "body": "Every task attaches real tool output as receipts."},
            {"title": "Isolated worktrees", "body": "Your repo is untouched until you approve the merge."}
        ],
        "testimonial": {"quote": "The evidence chain ended our review arguments. We ship what's proven.", "author": "Staff engineer, platform team"}
    });
    for (p, layout) in [
        ("editorial", "hero_left"),
        ("bold", "hero_center"),
        ("technical", "hero_split"),
    ] {
        let html = ac_daemon::render_mockup_html_for_visual_check(
            &spec, layout, "#ffffff", "#0f172a", "#2563eb", p,
        );
        let out = format!("/tmp/agentcode-design-{p}.html");
        std::fs::write(&out, html).expect("write");
        println!("wrote {out}");
    }
}
