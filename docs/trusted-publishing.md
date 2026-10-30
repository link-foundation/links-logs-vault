# crates.io trusted publishing migration

Both release jobs in `.github/workflows/release.yml` request a short-lived
crates.io token through the pinned `rust-lang/crates-io-auth-action`. Authentication
runs after the release build and package-size check, immediately before publishing.
Only the release jobs receive `id-token: write`. Pull requests run validation but
do not run either release job.

The publish step chooses credentials in this order:

1. The OIDC authentication step's `token` output.
2. The existing `CARGO_REGISTRY_TOKEN` secret.
3. The existing `CARGO_TOKEN` secret.

Authentication has `continue-on-error: true` so a missing publisher registration
or unavailable OIDC service can fall back to an existing secret. A failed publish
still fails the job. An invalid nonempty OIDC token does not cause a second publish
attempt with a secret. Registry tokens are scoped to the publish step rather than
being passed to every validation job.

## Register the publisher

The migration that opened PR #3 calls for rerunning `setup --migrate` using the
original publishing migration tool. That tool is not part of this repository;
run it in the authenticated environment where the migration was initiated and
review the resulting registration. If registering directly in the crate's
[crates.io settings](https://crates.io/crates/links-logs-vault/settings), use:

| Field | Value |
| --- | --- |
| Crate | `links-logs-vault` |
| GitHub owner | `link-foundation` |
| GitHub repository | `links-logs-vault` |
| Workflow filename | `release.yml` |
| Environment | Leave unset; these jobs do not specify an environment |

Register the caller workflow above, rather than the authentication action's own
repository or a different reusable workflow. See the official
[trusted publishing documentation](https://crates.io/docs/trusted-publishing).

## Verify the migration

After registration and review/merge, the patch changelog fragment triggers the
automatic release on `main`. Manual instant releases use the same authentication
and publish sequence, after lint, tests and build pass. Version numbers are
computed by the existing release scripts; do not edit `Cargo.toml` manually.

In a release run, verify that **Authenticate with crates.io (trusted publishing)**
succeeds without its failure being tolerated, then **Publish to Crates.io** reports
`publish_result=success`. With a successful authentication action, the token
expression selects its OIDC token before either secret. Preserve the run URL,
commit SHA, crate version and authentication/publish job evidence in PR #3.
A green pull-request run skips release jobs and does not verify OIDC publishing.
A successful publish after authentication failure proves only token fallback.

Keep both legacy secrets as the requested fallback. Do not remove them as part of
this migration. Do not claim live OIDC verification based solely on the
regression tests: publisher registration and a real release require registry
owner access and a GitHub Actions runtime.
