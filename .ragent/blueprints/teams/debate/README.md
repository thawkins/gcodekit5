# Debate Team Blueprint

A 3-teammate adversarial analysis team inspired by structured analytic techniques
(competing hypotheses, red-team/blue-team). Teammates challenge each other's
findings via peer-to-peer messaging to converge on well-tested conclusions.

## Teammates

| Name | Role |
|------|------|
| **advocate** | Builds the initial analysis and proposes conclusions |
| **critic** | Challenges the advocate's findings, identifies weaknesses and blind spots |
| **synthesizer** | Reconciles competing arguments and produces a balanced final report |

## How it works

1. **Advocate** analyses the target and produces initial findings.
2. **Critic** reviews the advocate's conclusions and sends counter-arguments via P2P messaging.
3. **Advocate** responds to challenges, strengthening or revising claims.
4. **Synthesizer** reads both perspectives and produces a consensus report.

## Quick run

1. Create the team:

```text
/team create debate
```

2. Monitor peer-to-peer exchanges:

```text
/team status
```

Look for 🔀 P2P messages in the log showing cross-teammate collaboration.

3. The synthesizer's final output is the team deliverable.
