# Spawn Prompts: Parallel Feature Team

## api-builder

```json
{
  "team_name": "parallel-feature",
  "teammate_name": "api-builder",
  "agent_type": "general",
  "prompt": "Implement backend/API changes for the feature. Keep interfaces stable, add validation, and provide migration notes for callers."
}
```

## ui-builder

```json
{
  "team_name": "parallel-feature",
  "teammate_name": "ui-builder",
  "agent_type": "general",
  "prompt": "Implement UI and interaction changes for the feature. Follow existing patterns, keep accessibility and state transitions explicit."
}
```

## tests-builder

```json
{
  "team_name": "parallel-feature",
  "teammate_name": "tests-builder",
  "agent_type": "general",
  "prompt": "Add integration and regression tests for the feature across API and UI behavior. Prioritize deterministic tests and clear failure messages."
}
```
