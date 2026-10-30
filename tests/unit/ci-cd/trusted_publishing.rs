//! Guard the release boundary and authentication precedence for both release modes.
use std::fs;

fn workflow() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/release.yml"
    ))
    .unwrap()
    .replace("\r\n", "\n")
}

fn job<'a>(workflow: &'a str, name: &str) -> &'a str {
    let start = workflow.find(&format!("  {name}:\n")).unwrap();
    let rest = &workflow[start..];
    let end = rest[1..].find("\n  # ===").map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

fn step<'a>(job: &'a str, name: &str) -> &'a str {
    let start = job.find(&format!("      - name: {name}\n")).unwrap();
    let rest = &job[start..];
    let end = rest[1..].find("\n      - ").map_or(rest.len(), |i| i + 1);
    &rest[..end]
}

#[test]
fn release_authentication_prefers_oidc_and_preserves_both_secret_fallbacks() {
    let workflow = workflow();
    let global_env = workflow.split("\njobs:\n").next().unwrap();
    assert!(!global_env.contains("secrets.CARGO_"));

    for name in ["auto-release", "manual-release"] {
        let job = job(&workflow, name);
        assert!(job.contains("      id-token: write\n"));
        let auth = step(job, "Authenticate with crates.io (trusted publishing)");
        assert!(auth.contains(
            "uses: rust-lang/crates-io-auth-action@c6f97d42243bad5fab37ca0427f495c86d5b1a18"
        ));
        assert!(auth.contains("id: prm_auth\n"));
        assert!(auth.contains("continue-on-error: true\n"));
        let publish = step(job, "Publish to Crates.io");
        assert!(publish.contains("run: rust-script scripts/publish-crate.rs\n"));
        assert!(publish.contains("CARGO_REGISTRY_TOKEN: ${{ steps.prm_auth.outputs.token || secrets.CARGO_REGISTRY_TOKEN || secrets.CARGO_TOKEN }}"));
        assert!(job.find(auth).unwrap() < job.find(publish).unwrap());
        let condition = |block: &str| {
            block
                .lines()
                .find(|line| line.trim_start().starts_with("if:"))
                .unwrap()
                .trim()
                .to_owned()
        };
        assert_eq!(condition(auth), condition(publish));
        let expected_condition = if name == "auto-release" {
            "if: steps.check.outputs.should_release == 'true' && steps.check.outputs.crate_published != 'true'"
        } else {
            "if: steps.version.outputs.version_committed == 'true' || steps.version.outputs.already_released == 'true'"
        };
        assert_eq!(condition(auth), expected_condition);
        assert!(job.find("- name: Build release").unwrap() < job.find(auth).unwrap());
        assert!(job.find("- name: Check crate package size").unwrap() < job.find(auth).unwrap());
    }
}

#[test]
fn oidc_release_jobs_require_validated_builds_and_do_not_publish_on_pull_requests() {
    let workflow = workflow();
    let build = job(&workflow, "build");
    assert!(build.contains("needs.lint.result == 'success'"));
    assert!(build.contains("needs.test.result == 'success'"));
    for name in ["auto-release", "manual-release"] {
        let job = job(&workflow, name);
        assert!(job.contains("needs: [lint, test, build]"));
        assert!(job.contains("needs.build.result == 'success'"));
        let gate = job.split("    steps:\n").next().unwrap();
        assert!(!gate.contains("github.event_name == 'pull_request'"));
        if name == "auto-release" {
            assert!(gate.contains("github.event_name == 'push'"));
            assert!(gate.contains("github.ref == 'refs/heads/main'"));
        } else {
            assert!(gate.contains("github.event_name == 'workflow_dispatch'"));
            assert!(gate.contains("github.event.inputs.release_mode == 'instant'"));
        }
    }
}
