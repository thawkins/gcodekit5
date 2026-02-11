# ADR-004: Interior Mutability Patterns

## Status
Accepted

## Context

GTK4's architecture requires shared ownership of widgets and state:

- Widgets are reference-counted (`glib::Object` descendants)
- Signal handlers (closures) need access to application state
- Multiple UI components may need to read/write the same state
- Rust's ownership rules normally prevent shared mutable access

Traditional Rust patterns (single owner, explicit borrowing) don't work well with GTK's callback-heavy architecture.

## Decision

We use **interior mutability patterns** with the following conventions:

### 1. RefCell for Single-Threaded UI State

```rust
use std::cell::RefCell;
use std::rc::Rc;

struct AppState {
    inner: Rc<RefCell<AppStateInner>>,
}

struct AppStateInner {
    machine_position: Position,
    connection_status: ConnectionStatus,
    // ...
}
```

### 2. Mutex for Cross-Thread State

```rust
use std::sync::{Arc, Mutex};

struct SharedDeviceState {
    inner: Arc<Mutex<DeviceStateInner>>,
}
```

### 3. GLib Properties for Widget State

```rust
// Using glib::Properties derive macro
#[derive(Properties)]
#[properties(wrapper_type = super::MyWidget)]
pub struct MyWidget {
    #[property(get, set)]
    value: Cell<f64>,
}
```

### Guidelines

1. **Prefer `Rc<RefCell<T>>` for UI state** accessed only from the main thread
2. **Use `Arc<Mutex<T>>` for state** shared between UI and async/worker threads
3. **Keep borrows short** - borrow, copy/clone needed data, drop borrow
4. **Never hold borrows across await points** or GTK callbacks
5. **Use `Cell<T>` for Copy types** when full RefCell isn't needed

## Consequences

### Positive
- Enables GTK's callback-based architecture
- State can be shared across signal handlers
- Works naturally with GTK's reference counting
- Panic on borrow violation catches bugs early (vs. undefined behavior)

### Negative
- Runtime borrow checking (RefCell can panic)
- More verbose than direct mutation
- Easy to accidentally create long-lived borrows
- Deadlock potential with Mutex if not careful

### Neutral
- Common pattern in GTK-rs applications
- Requires discipline to avoid borrow conflicts

## Alternatives Considered

1. **Message Passing (Channels)**: Cleaner but adds complexity; GTK already has glib::MainContext for this.

2. **Global Static State**: Works but harder to test and manage lifetimes.

3. **Pure Functional Updates**: Doesn't fit GTK's imperative widget model.

4. **Elm-like Architecture (Relm4)**: Considered but adds abstraction layer; direct GTK gives more control.

## Best Practices

```rust
// ✅ Good: Short borrow, copy data out
fn get_position(&self) -> Position {
    self.inner.borrow().machine_position
}

// ❌ Bad: Holding borrow while doing other work
fn process(&self) {
    let borrowed = self.inner.borrow();
    self.update_ui(&borrowed.data);  // May trigger callbacks!
    // borrowed still held - callbacks might try to borrow too
}

// ✅ Good: Clone data, release borrow, then use
fn process(&self) {
    let data = self.inner.borrow().data.clone();
    drop(self.inner.borrow());  // Explicit release
    self.update_ui(&data);  // Safe - no active borrow
}
```

## References

- [Rust Interior Mutability](https://doc.rust-lang.org/book/ch15-05-interior-mutability.html)
- [GTK-rs Patterns](https://gtk-rs.org/gtk4-rs/stable/latest/book/g_object_memory_management.html)
- [RefCell Documentation](https://doc.rust-lang.org/std/cell/struct.RefCell.html)
