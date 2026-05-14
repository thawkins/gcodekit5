# Parallel Feature Team Example

A 3-teammate implementation split:

- API teammate
- UI teammate
- tests teammate

This pattern is useful for shipping one feature in parallel tracks with dependency-aware tasking.

## Quick run

1. Create team:

```text
/team create parallel-feature
```

2. Spawn teammates from `spawn-prompts.md`.

3. Add tasks from `task-seed.json` (note dependency chain).

4. Track progress:

```text
/team status
/team tasks
```
