# ADR-006: Unified Event Bus System

**Status**: Proposed  
**Date**: 2026-01-30  
**Author**: GCodeKit Contributors  
**Related**: ADR-004 (Interior Mutability), ADR-005 (Error Handling)

## Context

GCodeKit5 currently uses multiple event handling patterns across the codebase:

1. **Tokio broadcast channels** (`EventDispatcher`, `MessageDispatcher`) for pub/sub
2. **Trait-based listeners** (`CommunicatorListener`, `ControllerListener`) for observer pattern
3. **Closure callbacks** with `Arc<Mutex<Vec<Box<dyn Fn>>>>` for UI events
4. **GTK4 signals** via `connect_*()` methods
5. **RefCell callbacks** in UI components

This fragmentation leads to:
- Inconsistent event handling patterns across crates
- Difficulty in debugging event flow
- Tight coupling between components
- Complex callback chains that are hard to trace
- No unified way to subscribe to cross-cutting events

## Decision

We will implement a **Unified Event Bus System** that:
- Provides a consistent API for all application events
- Supports typed events with compile-time safety
- Enables decoupled communication between components
- Integrates with async Rust (tokio)
- Coexists with GTK4's signal system for UI-specific events

## Event Types

### Core Event Categories

```rust
/// Root event enum for all application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Machine connection events
    Connection(ConnectionEvent),
    /// Machine state and status
    Machine(MachineEvent),
    /// G-code file operations
    File(FileEvent),
    /// User interface events
    Ui(UiEvent),
    /// Communication layer events
    Communication(CommunicationEvent),
    /// Settings and configuration
    Settings(SettingsEvent),
    /// Error and diagnostic events
    Error(ErrorEvent),
}

#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    Connecting { port: String },
    Connected { port: String, firmware: String },
    Disconnected { port: String, reason: DisconnectReason },
    ConnectionFailed { port: String, error: String },
}

#[derive(Debug, Clone)]
pub enum MachineEvent {
    StateChanged { old: MachineState, new: MachineState },
    PositionUpdated { position: Position, source: PositionSource },
    AlarmTriggered { alarm: AlarmCode, message: String },
    AlarmCleared,
    ProbeTriggered { position: Position },
    LimitTriggered { axis: Axis, direction: Direction },
    SpindleChanged { rpm: f32, state: SpindleState },
    CoolantChanged { mist: bool, flood: bool },
    FeedOverrideChanged { percent: u8 },
    RapidOverrideChanged { percent: u8 },
    SpindleOverrideChanged { percent: u8 },
}

#[derive(Debug, Clone)]
pub enum FileEvent {
    Opened { path: PathBuf, lines: usize },
    Closed,
    Modified,
    Saved { path: PathBuf },
    ParseError { line: usize, error: String },
    StreamStarted { total_lines: usize },
    StreamProgress { current_line: usize, total_lines: usize },
    StreamCompleted { duration: Duration },
    StreamPaused,
    StreamResumed,
    StreamCancelled,
}

#[derive(Debug, Clone)]
pub enum CommunicationEvent {
    DataSent { data: String },
    DataReceived { data: String },
    Timeout { operation: String },
    BufferStatus { available: usize, total: usize },
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    ViewChanged { view: ViewType },
    ThemeChanged { theme: Theme },
    UnitsChanged { units: MeasurementSystem },
    ZoomChanged { level: f32 },
    SelectionChanged { selected: Vec<EntityId> },
}

#[derive(Debug, Clone)]
pub enum SettingsEvent {
    Loaded,
    Saved,
    Changed { key: String, value: SettingValue },
    ProfileChanged { profile: String },
}

#[derive(Debug, Clone)]
pub enum ErrorEvent {
    Warning { code: String, message: String },
    Error { code: String, message: String, recoverable: bool },
    Critical { code: String, message: String },
}
```

## Subscriber Interface

### Event Subscriber Trait

```rust
use async_trait::async_trait;

/// Type-erased event handler
pub type EventHandler = Box<dyn Fn(AppEvent) + Send + Sync>;

/// Async event handler for background processing
pub type AsyncEventHandler = Box<dyn Fn(AppEvent) -> BoxFuture<'static, ()> + Send + Sync>;

/// Subscription handle for unsubscribing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptionId(Uuid);

/// Filter to receive only specific event types
#[derive(Debug, Clone)]
pub enum EventFilter {
    /// Receive all events
    All,
    /// Receive events matching any of these categories
    Categories(Vec<EventCategory>),
    /// Custom filter function
    Custom(Arc<dyn Fn(&AppEvent) -> bool + Send + Sync>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    Connection,
    Machine,
    File,
    Communication,
    Ui,
    Settings,
    Error,
}

impl AppEvent {
    pub fn category(&self) -> EventCategory {
        match self {
            AppEvent::Connection(_) => EventCategory::Connection,
            AppEvent::Machine(_) => EventCategory::Machine,
            AppEvent::File(_) => EventCategory::File,
            AppEvent::Communication(_) => EventCategory::Communication,
            AppEvent::Ui(_) => EventCategory::Ui,
            AppEvent::Settings(_) => EventCategory::Settings,
            AppEvent::Error(_) => EventCategory::Error,
        }
    }
}
```

## Event Bus API

### EventBus Struct

```rust
use tokio::sync::broadcast;
use std::sync::Arc;
use parking_lot::RwLock;

/// Central event bus for application-wide event distribution
pub struct EventBus {
    /// Broadcast channel for event distribution
    sender: broadcast::Sender<AppEvent>,
    /// Registered synchronous handlers
    sync_handlers: Arc<RwLock<HashMap<SubscriptionId, (EventFilter, EventHandler)>>>,
    /// Registered async handlers
    async_handlers: Arc<RwLock<HashMap<SubscriptionId, (EventFilter, AsyncEventHandler)>>>,
    /// Event history for replay (optional, bounded)
    history: Arc<RwLock<VecDeque<(Instant, AppEvent)>>>,
    /// Configuration
    config: EventBusConfig,
}

#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Channel capacity for broadcast
    pub channel_capacity: usize,
    /// Whether to keep event history
    pub enable_history: bool,
    /// Maximum history size
    pub max_history_size: usize,
    /// History retention duration
    pub history_retention: Duration,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 1024,
            enable_history: false,
            max_history_size: 1000,
            history_retention: Duration::from_secs(300),
        }
    }
}

impl EventBus {
    /// Create a new event bus with default configuration
    pub fn new() -> Self;
    
    /// Create with custom configuration
    pub fn with_config(config: EventBusConfig) -> Self;
    
    /// Publish an event to all subscribers
    pub fn publish(&self, event: AppEvent) -> Result<usize, EventBusError>;
    
    /// Publish an event asynchronously
    pub async fn publish_async(&self, event: AppEvent) -> Result<usize, EventBusError>;
    
    /// Subscribe to events with a synchronous handler
    pub fn subscribe<F>(&self, filter: EventFilter, handler: F) -> SubscriptionId
    where
        F: Fn(AppEvent) + Send + Sync + 'static;
    
    /// Subscribe to events with an async handler
    pub fn subscribe_async<F, Fut>(&self, filter: EventFilter, handler: F) -> SubscriptionId
    where
        F: Fn(AppEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static;
    
    /// Get a receiver for manual event polling
    pub fn receiver(&self) -> broadcast::Receiver<AppEvent>;
    
    /// Unsubscribe from events
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool;
    
    /// Get recent event history (if enabled)
    pub fn history(&self, since: Option<Instant>) -> Vec<AppEvent>;
    
    /// Clear event history
    pub fn clear_history(&self);
}

/// Global event bus instance
static EVENT_BUS: OnceLock<EventBus> = OnceLock::new();

/// Get or initialize the global event bus
pub fn event_bus() -> &'static EventBus {
    EVENT_BUS.get_or_init(EventBus::new)
}
```

### Convenience Macros

```rust
/// Publish an event to the global event bus
#[macro_export]
macro_rules! emit {
    ($event:expr) => {
        $crate::event_bus().publish($event)
    };
}

/// Subscribe to events with a filter
#[macro_export]
macro_rules! on_event {
    ($filter:expr, $handler:expr) => {
        $crate::event_bus().subscribe($filter, $handler)
    };
}
```

## Usage Patterns

### Pattern 1: Simple Event Subscription

```rust
use gcodekit5_core::events::*;

// Subscribe to all connection events
let subscription = event_bus().subscribe(
    EventFilter::Categories(vec![EventCategory::Connection]),
    |event| {
        if let AppEvent::Connection(conn_event) = event {
            match conn_event {
                ConnectionEvent::Connected { port, firmware } => {
                    tracing::info!("Connected to {} running {}", port, firmware);
                }
                ConnectionEvent::Disconnected { port, reason } => {
                    tracing::info!("Disconnected from {}: {:?}", port, reason);
                }
                _ => {}
            }
        }
    },
);

// Later: unsubscribe
event_bus().unsubscribe(subscription);
```

### Pattern 2: Async Event Handler

```rust
// Subscribe with async handler for database logging
let subscription = event_bus().subscribe_async(
    EventFilter::All,
    |event| async move {
        if let Err(e) = database.log_event(&event).await {
            tracing::error!("Failed to log event: {}", e);
        }
    },
);
```

### Pattern 3: Component Integration

```rust
pub struct StatusBar {
    subscription: Option<SubscriptionId>,
    // ... other fields
}

impl StatusBar {
    pub fn new() -> Self {
        let subscription = event_bus().subscribe(
            EventFilter::Categories(vec![
                EventCategory::Connection,
                EventCategory::Machine,
            ]),
            |event| {
                // Update status bar based on event
                Self::handle_event(event);
            },
        );
        
        Self {
            subscription: Some(subscription),
            // ...
        }
    }
}

impl Drop for StatusBar {
    fn drop(&mut self) {
        if let Some(id) = self.subscription.take() {
            event_bus().unsubscribe(id);
        }
    }
}
```

### Pattern 4: Event Publishing

```rust
// From communication layer
impl SerialConnection {
    async fn handle_data(&self, data: &str) {
        // Process data...
        
        // Emit event
        emit!(AppEvent::Communication(CommunicationEvent::DataReceived {
            data: data.to_string(),
        }));
        
        // If it's a position update
        if let Some(position) = parse_position(data) {
            emit!(AppEvent::Machine(MachineEvent::PositionUpdated {
                position,
                source: PositionSource::StatusReport,
            }));
        }
    }
}
```

### Pattern 5: GTK4 Integration

```rust
// Bridge GTK signals to event bus
impl MainWindow {
    fn setup_actions(&self) {
        let connect_action = gio::ActionEntry::builder("connect")
            .activate(|_, _, _| {
                // GTK action triggers event
                emit!(AppEvent::Ui(UiEvent::ActionTriggered {
                    action: "connect".to_string(),
                }));
            })
            .build();
            
        // Subscribe to events for UI updates
        let label = self.status_label.clone();
        event_bus().subscribe(
            EventFilter::Categories(vec![EventCategory::Connection]),
            move |event| {
                glib::idle_add_local_once({
                    let label = label.clone();
                    let event = event.clone();
                    move || {
                        // Update UI on main thread
                        if let AppEvent::Connection(conn) = event {
                            match conn {
                                ConnectionEvent::Connected { .. } => {
                                    label.set_text("Connected");
                                }
                                ConnectionEvent::Disconnected { .. } => {
                                    label.set_text("Disconnected");
                                }
                                _ => {}
                            }
                        }
                    }
                });
            },
        );
    }
}
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              GCodeKit5 Event Flow                            │
└─────────────────────────────────────────────────────────────────────────────┘

┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   GTK4 UI    │    │ Communication│    │   G-Code     │    │  Settings    │
│  Signals     │    │    Layer     │    │   Parser     │    │   Manager    │
└──────┬───────┘    └──────┬───────┘    └──────┬───────┘    └──────┬───────┘
       │                   │                   │                   │
       │ emit!()           │ emit!()           │ emit!()           │ emit!()
       │                   │                   │                   │
       ▼                   ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│                            EventBus (Global Singleton)                       │
│                                                                             │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐         │
│  │ broadcast::     │    │ sync_handlers   │    │ async_handlers  │         │
│  │ Sender<AppEvent>│───▶│ HashMap<Id,     │    │ HashMap<Id,     │         │
│  │                 │    │   (Filter, Fn)> │    │   (Filter, Fut)>│         │
│  └─────────────────┘    └─────────────────┘    └─────────────────┘         │
│                                                                             │
│  ┌─────────────────┐                                                        │
│  │ Event History   │ (optional, for debugging/replay)                       │
│  │ VecDeque<Event> │                                                        │
│  └─────────────────┘                                                        │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
       │                   │                   │                   │
       │ filtered events   │                   │                   │
       ▼                   ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Status Bar  │    │   Console    │    │  Visualizer  │    │   Logger     │
│  Subscriber  │    │  Subscriber  │    │  Subscriber  │    │  Subscriber  │
└──────────────┘    └──────────────┘    └──────────────┘    └──────────────┘
```

## Comparison to Current Approach

| Aspect | Current Approach | Unified Event Bus |
|--------|-----------------|-------------------|
| **Consistency** | Multiple patterns (callbacks, traits, channels) | Single consistent API |
| **Type Safety** | Varies by pattern | Full compile-time type safety |
| **Debugging** | Hard to trace event flow | Optional history, centralized logging |
| **Coupling** | Components often know about each other | Fully decoupled via events |
| **Threading** | Mixed sync/async handling | Unified async-first with sync support |
| **GTK Integration** | Direct signal connections | Bridge pattern with thread-safe updates |
| **Performance** | Good (direct calls) | Good (broadcast channel overhead minimal) |
| **Memory** | Lower (no central storage) | Slightly higher (event bus singleton) |
| **Testing** | Requires mocking multiple interfaces | Easy to inject test event bus |

### Migration Strategy

1. **Phase 1**: Implement core `EventBus` in `gcodekit5-core`
2. **Phase 2**: Add event types incrementally
3. **Phase 3**: Bridge existing listeners to emit events
4. **Phase 4**: Migrate subscribers to use event bus
5. **Phase 5**: Deprecate old listener patterns (optional)

### Coexistence

The event bus is designed to **coexist** with:
- GTK4 signals (for UI-internal events)
- Existing `CommunicatorListener` (can emit to event bus)
- Existing `ControllerListener` (can emit to event bus)

## Consequences

### Positive
- Consistent event handling across the application
- Easier to add new subscribers without modifying publishers
- Better testability through event injection
- Optional event history for debugging
- Clear event documentation through enum types

### Negative
- Additional abstraction layer
- Slight memory overhead for event bus singleton
- Learning curve for new pattern
- Need to bridge with GTK4 signals

### Risks
- Over-use of events for everything (use sparingly for cross-cutting concerns)
- Event storms if not careful with high-frequency events
- Memory leaks if subscriptions not cleaned up

## Implementation Notes

### Crate Location
The event bus should be implemented in `gcodekit5-core` as it's a fundamental infrastructure component.

### Dependencies
- `tokio` (broadcast channel)
- `parking_lot` (RwLock for handlers)
- `uuid` (subscription IDs)

### Performance Considerations
- Use `broadcast::channel` for efficient multi-subscriber delivery
- Filter events before calling handlers to reduce overhead
- Consider debouncing for high-frequency events (position updates)

## References

- [Tokio Broadcast Channel](https://docs.rs/tokio/latest/tokio/sync/broadcast/)
- [Event-Driven Architecture](https://martinfowler.com/articles/201701-event-driven.html)
- [Observer Pattern in Rust](https://rust-unofficial.github.io/patterns/patterns/behavioural/observer.html)
