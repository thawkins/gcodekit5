# ADR-005: Error Handling Strategy

## Status
Accepted

## Context

GCodeKit5 handles errors from many sources:

- Serial port communication failures
- G-code parsing errors
- File I/O errors
- Configuration validation errors
- Firmware protocol errors
- UI state errors

A consistent error handling strategy is needed to:
- Provide actionable error messages to users
- Enable proper error recovery
- Support debugging and logging
- Maintain clean code without excessive error handling boilerplate

## Decision

We adopt a **layered error handling strategy**:

### 1. Library Crates: Use `thiserror` for Typed Errors

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommunicationError {
    #[error("Serial port not found: {port}")]
    PortNotFound { port: String },
    
    #[error("Connection timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    
    #[error("Protocol error: {message}")]
    Protocol { message: String },
    
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

### 2. Application Layer: Use `anyhow` for Context

```rust
use anyhow::{Context, Result};

fn load_device_profile(path: &Path) -> Result<DeviceProfile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read device profile: {}", path.display()))?;
    
    serde_json::from_str(&content)
        .with_context(|| "Invalid JSON in device profile")
}
```

### 3. UI Layer: Convert to User-Friendly Messages

```rust
fn show_error_to_user(error: &anyhow::Error) {
    let message = format_error_chain(error);
    show_error_dialog(&message);
}

fn format_error_chain(error: &anyhow::Error) -> String {
    let mut messages = vec![error.to_string()];
    let mut source = error.source();
    while let Some(err) = source {
        messages.push(err.to_string());
        source = err.source();
    }
    messages.join("\n  Caused by: ")
}
```

### Guidelines

| Context | Approach | Crate |
|---------|----------|-------|
| Library public API | Typed errors with `thiserror` | thiserror |
| Internal functions | `Result<T, E>` with `?` propagation | std |
| Application/binary | `anyhow::Result` with context | anyhow |
| Unrecoverable bugs | `panic!` / `unreachable!` | std |
| Optional features | `Option<T>` | std |

### Banned Patterns

```rust
// ❌ Don't use unwrap() without justification
let value = map.get("key").unwrap();

// ✅ Use expect() with explanation if truly unreachable
let value = map.get("key").expect("key always present after init");

// ✅ Or handle the error properly
let value = map.get("key").ok_or_else(|| anyhow!("Missing key"))?;
```

## Consequences

### Positive
- Clear error types in library APIs
- Rich context in error messages
- Error chains preserve root cause
- Consistent patterns across codebase
- User-friendly error display

### Negative
- Two error crates to learn (thiserror + anyhow)
- Some boilerplate for typed errors
- Must remember to add context at boundaries

### Neutral
- Errors are not silently swallowed
- Stack traces available in debug builds

## Alternatives Considered

1. **Only anyhow**: Simpler but loses type information for matching.

2. **Only thiserror**: More boilerplate for application code.

3. **Custom error types everywhere**: Maximum control but significant boilerplate.

4. **error-chain (deprecated)**: Was popular but now unmaintained.

## References

- [thiserror Documentation](https://docs.rs/thiserror)
- [anyhow Documentation](https://docs.rs/anyhow)
- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Error Handling in a Correctness-Critical Rust Project](https://sled.rs/errors.html)
