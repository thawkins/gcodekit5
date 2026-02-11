# Dependency Management Guide

This document describes the dependency management process for GCodeKit5.

## Automated Updates (Dependabot)

Dependabot is configured to automatically create PRs for dependency updates:

- **Cargo dependencies**: Weekly on Mondays at 09:00 UTC
- **GitHub Actions**: Weekly on Mondays at 09:00 UTC  
- **npm dependencies**: Weekly on Mondays at 09:00 UTC

Minor and patch updates are grouped together to reduce PR noise.

### Reviewing Dependabot PRs

1. **Minor/Patch updates**: Generally safe to merge after CI passes
2. **Major updates**: Review changelog for breaking changes before merging
3. **Security updates**: Prioritize and merge promptly after CI passes

## Monthly Manual Review

In addition to automated updates, perform a monthly manual review to:

- Check for outdated dependencies that Dependabot may have missed
- Review security advisories
- Evaluate major version updates
- Clean up unused dependencies

### Prerequisites

Install the required cargo tools:

```bash
cargo install cargo-outdated cargo-audit
```

### Running the Review

Use the provided script:

```bash
./scripts/monthly-dependency-review.sh
```

This will:
1. Check for outdated dependencies with `cargo outdated`
2. Run security audit with `cargo audit`
3. Check for duplicate dependencies
4. Generate a report in `target/temp/dependency-review-YYYY-MM.md`

### Applying Updates

To apply safe (minor/patch) updates:

```bash
./scripts/monthly-dependency-review.sh --update
```

For major version updates, update manually:

```bash
# Edit Cargo.toml to update version
cargo update -p <package>
cargo test
```

## Security Vulnerability Response

When security vulnerabilities are discovered:

1. **Critical/High severity**: Fix within 24 hours
2. **Medium severity**: Fix within 1 week
3. **Low severity**: Fix in next monthly review

### Checking for Vulnerabilities

```bash
cargo audit
```

### Fixing Vulnerabilities

1. Update the affected dependency if a fix is available
2. If no fix available, evaluate:
   - Can we replace the dependency?
   - Can we work around the vulnerable code path?
   - Document the risk if we must accept it temporarily

## Dependency Guidelines

### Adding New Dependencies

Before adding a new dependency:

1. **Evaluate necessity**: Can we implement this ourselves reasonably?
2. **Check maintenance status**: Is the crate actively maintained?
3. **Review security**: Run `cargo audit` after adding
4. **Check license compatibility**: Ensure license is compatible (MIT, Apache 2.0, BSD)
5. **Consider size**: Avoid heavy dependencies for simple features

### Preferred Crates

| Category | Preferred Crate(s) |
|----------|-------------------|
| Error handling | `thiserror`, `anyhow` |
| Async runtime | `tokio` |
| Serialization | `serde`, `serde_json` |
| Logging | `tracing` |
| HTTP client | `reqwest` |
| Date/time | `chrono` |
| CLI parsing | `clap` |
| Testing | Built-in `#[test]`, `proptest` for property testing |

### Version Constraints

Use appropriate version constraints in `Cargo.toml`:

```toml
# Prefer: Compatible updates (most common)
serde = "1.0"

# When needed: Exact version (rare, avoid if possible)
some-crate = "=1.2.3"

# For development dependencies: More relaxed
[dev-dependencies]
proptest = "1"
```

## Duplicate Dependencies

Some duplicate dependencies are unavoidable due to transitive dependencies. Current known duplicates:

- `bitflags` (1.x vs 2.x) - ecosystem-wide migration in progress
- `itertools` - different deps require different versions
- `syn`/`quote` - different proc-macro crates use different versions

To check for duplicates:

```bash
cargo tree --duplicates
```

## Monthly Review Schedule

| Month | Reviewer | Status |
|-------|----------|--------|
| January 2026 | TBD | Pending |
| February 2026 | TBD | Pending |
| ... | ... | ... |

## Related Documentation

- [Dependency Audit](dependency_audit.md) - Full audit of all dependencies
- [REMEDIATION_PLAN.md](../REMEDIATION_PLAN.md) - Dependency remediation tasks
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
