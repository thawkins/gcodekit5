# Architecture Decision Records (ADRs)

This directory contains Architecture Decision Records (ADRs) for GCodeKit5. ADRs document significant architectural decisions made during development, including the context, decision, and consequences.

## Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [ADR-001](ADR-001-gtk4-ui-framework.md) | GTK4 UI Framework | Accepted | 2026-01-26 |
| [ADR-002](ADR-002-coordinate-system.md) | Coordinate System (Y-flip) | Accepted | 2026-01-26 |
| [ADR-003](ADR-003-modular-crates.md) | Modular Crates Structure | Accepted | 2026-01-26 |
| [ADR-004](ADR-004-interior-mutability.md) | Interior Mutability Patterns | Accepted | 2026-01-26 |
| [ADR-005](ADR-005-error-handling.md) | Error Handling Strategy | Accepted | 2026-01-26 |
| [ADR-006](ADR-006-event-bus-system.md) | Unified Event Bus System | Proposed | 2026-01-30 |

## ADR Template

When creating a new ADR, use this template:

```markdown
# ADR-NNN: Title

## Status
[Proposed | Accepted | Deprecated | Superseded by ADR-XXX]

## Context
What is the issue that we're seeing that is motivating this decision or change?

## Decision
What is the change that we're proposing and/or doing?

## Consequences
What becomes easier or more difficult to do because of this change?

## Alternatives Considered
What other options were evaluated?

## References
Links to relevant documentation, issues, or discussions.
```

## References

- [ADR GitHub Organization](https://adr.github.io/)
- [Michael Nygard's ADR Article](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
