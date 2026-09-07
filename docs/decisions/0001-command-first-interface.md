# 0001: Use a command-first interface

**Status:** Accepted

## Context

Kact must integrate with shells and external keyboard remappers while preserving normal typing.

## Decision

Expose cursor actions through the CLI and local Unix socket. Keep global shortcuts disabled by default; configured bindings only add optional local or global activation.

## Consequences

External tools can trigger every action without native keyboard capture. Commands become a stable compatibility boundary and must retain clear validation and error reporting.

