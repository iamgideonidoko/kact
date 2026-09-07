---
title: Architecture decisions
description: Durable project decisions and their consequences.
---

Record a decision when it has meaningful alternatives, consequences, or migration cost. Routine implementation details do not need a record.

## 0001: Command-first interface

**Status:** Accepted

### Context

Kact must integrate with shells and external keyboard remappers while preserving normal typing.

### Decision

Expose cursor actions through the CLI and private local Unix socket. Keep global shortcuts disabled by default; configured bindings only add optional local or global activation.

### Consequences

External tools can trigger every action without native keyboard capture. Commands are a stable compatibility boundary and need clear validation and error reporting.
