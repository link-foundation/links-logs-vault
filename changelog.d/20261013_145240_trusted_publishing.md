---
bump: patch
---

### Changed
- Prefer GitHub Actions trusted publishing for crates.io releases, retaining CARGO_REGISTRY_TOKEN and CARGO_TOKEN secrets as fallbacks when OIDC authentication is unavailable.
