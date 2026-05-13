---
description: "Increment patch version, commit, push, and tag the release"
user-invocable: true
disable-model-invocation: true
argument-hint: "[commit message]"
---

Perform a release of the ragent project by following these steps **in order**:

1. **Read the current version** from the workspace `Cargo.toml` (the `version = "..."` line near the top).

2. **Increment the least-significant version digit** by exactly 1.
   - For a version like `0.1.0-alpha.9`, increment the pre-release number → `0.1.0-alpha.10`.
   - For a version like `1.2.3`, increment the patch number → `1.2.4`.
   - For a version like `0.1.0-beta.2`, increment → `0.1.0-beta.3`.

3. **Update `Cargo.toml`** with the new version string (the workspace-level `version` field only).

4. **Run `cargo check`** to ensure the version change doesn't break the build.

5. **Update `RELEASE.md`** and `CHANGELOG.md` with the new version number and any recent changes.

6. **Stage all modified files** with `git add -A`.

7. **Commit** with the message: `Version: <new-version>` followed by any additional message the user provided via $ARGUMENTS.

8. **Push** to the remote with `git push`.

9. **Tag** the commit with `v<new-version>` (e.g. `v0.1.0-alpha.10`) and push the tag with `git push origin v<new-version>`.

10. **Report** the old version, new version, and the tag that was pushed.
