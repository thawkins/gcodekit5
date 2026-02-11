//! Event Bus implementation.
//!
//! Provides the core EventBus struct and global instance for
//! application-wide event distribution.

use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use uuid::Uuid;

use super::events::{AppEvent, EventCategory};

/// Subscription handle for unsubscribing from events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(Uuid);

impl SubscriptionId {
    /// Create a new unique subscription ID
    fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sub({})", &self.0.to_string()[..8])
    }
}

/// Filter to receive only specific event types
#[derive(Debug, Clone, Default)]
pub enum EventFilter {
    /// Receive all events.
    #[default]
    All,
    /// Receive events matching any of these categories.
    Categories(Vec<EventCategory>),
}

impl EventFilter {
    /// Check if an event matches this filter
    pub fn matches(&self, event: &AppEvent) -> bool {
        match self {
            EventFilter::All => true,
            EventFilter::Categories(categories) => categories.contains(&event.category()),
        }
    }
}

/// Type alias for event handler functions
type EventHandler = Box<dyn Fn(AppEvent) + Send + Sync>;

/// Configuration for the event bus
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Channel capacity for broadcast.
    pub channel_capacity: usize,
    /// Whether to keep event history.
    pub enable_history: bool,
    /// Maximum number of events to retain in history.
    pub max_history_size: usize,
    /// How long to retain events in history.
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

/// Event with timestamp for history
#[derive(Debug, Clone)]
struct TimestampedEvent {
    event: AppEvent,
    timestamp: Instant,
}

/// Error types for event bus operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum EventBusError {
    /// No subscribers are listening
    #[error("No active subscribers")]
    NoSubscribers,
    /// Channel is closed
    #[error("Event channel is closed")]
    ChannelClosed,
    /// Channel is full (lagging)
    #[error("Event channel is full, {0} events dropped")]
    ChannelFull(u64),
}

/// Central event bus for application-wide event distribution
pub struct EventBus {
    /// Broadcast channel sender
    sender: broadcast::Sender<AppEvent>,
    /// Registered synchronous handlers
    handlers: Arc<RwLock<HashMap<SubscriptionId, (EventFilter, EventHandler)>>>,
    /// Event history (optional)
    history: Arc<RwLock<VecDeque<TimestampedEvent>>>,
    /// Configuration
    config: EventBusConfig,
}

impl EventBus {
    /// Create a new event bus with default configuration
    pub fn new() -> Self {
        Self::with_config(EventBusConfig::default())
    }

    /// Create a new event bus with custom configuration
    pub fn with_config(config: EventBusConfig) -> Self {
        let (sender, _) = broadcast::channel(config.channel_capacity);
        Self {
            sender,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(VecDeque::new())),
            config,
        }
    }

    /// Publish an event to all subscribers
    ///
    /// Returns the number of receivers that will receive the event,
    /// or an error if there are no subscribers or the channel is closed.
    pub fn publish(&self, event: AppEvent) -> Result<usize, EventBusError> {
        // Add to history if enabled
        if self.config.enable_history {
            self.add_to_history(&event);
        }

        // Call synchronous handlers
        let handlers = self.handlers.read();
        for (_, (filter, handler)) in handlers.iter() {
            if filter.matches(&event) {
                handler(event.clone());
            }
        }

        // Send via broadcast channel for async receivers
        match self.sender.send(event) {
            Ok(count) => Ok(count),
            Err(_) => {
                // No receivers, but handlers may have been called
                if handlers.is_empty() {
                    Err(EventBusError::NoSubscribers)
                } else {
                    Ok(0)
                }
            }
        }
    }

    /// Subscribe to events with a synchronous handler
    ///
    /// The handler will be called on the publishing thread, so it should
    /// return quickly to avoid blocking event dispatch.
    pub fn subscribe<F>(&self, filter: EventFilter, handler: F) -> SubscriptionId
    where
        F: Fn(AppEvent) + Send + Sync + 'static,
    {
        let id = SubscriptionId::new();
        let mut handlers = self.handlers.write();
        handlers.insert(id, (filter, Box::new(handler)));
        tracing::debug!("Subscription {} added", id);
        id
    }

    /// Get a receiver for manual event polling
    ///
    /// This is useful for async contexts where you want to receive events
    /// in a tokio task.
    pub fn receiver(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    /// Unsubscribe from events
    ///
    /// Returns true if the subscription was found and removed.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        let mut handlers = self.handlers.write();
        let removed = handlers.remove(&id).is_some();
        if removed {
            tracing::debug!("Subscription {} removed", id);
        }
        removed
    }

    /// Get the number of active subscriptions
    pub fn subscriber_count(&self) -> usize {
        self.handlers.read().len()
    }

    /// Get recent event history (if enabled)
    ///
    /// Returns events since the given instant, or all history if None.
    pub fn history(&self, since: Option<Instant>) -> Vec<AppEvent> {
        if !self.config.enable_history {
            return Vec::new();
        }

        let history = self.history.read();
        match since {
            Some(since) => history
                .iter()
                .filter(|e| e.timestamp >= since)
                .map(|e| e.event.clone())
                .collect(),
            None => history.iter().map(|e| e.event.clone()).collect(),
        }
    }

    /// Clear event history
    pub fn clear_history(&self) {
        let mut history = self.history.write();
        history.clear();
    }

    /// Get the current configuration
    pub fn config(&self) -> &EventBusConfig {
        &self.config
    }

    /// Add an event to history, maintaining size and age limits
    fn add_to_history(&self, event: &AppEvent) {
        let mut history = self.history.write();
        let now = Instant::now();

        // Add new event
        history.push_back(TimestampedEvent {
            event: event.clone(),
            timestamp: now,
        });

        // Remove old events
        let retention = self.config.history_retention;
        while history
            .front()
            .is_some_and(|e| now.duration_since(e.timestamp) > retention)
        {
            history.pop_front();
        }

        // Enforce max size
        while history.len() > self.config.max_history_size {
            history.pop_front();
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("subscribers", &self.subscriber_count())
            .field("config", &self.config)
            .finish()
    }
}

/// Global event bus instance
static EVENT_BUS: OnceLock<EventBus> = OnceLock::new();

/// Get or initialize the global event bus
///
/// This is the primary way to access the event bus throughout the application.
pub fn event_bus() -> &'static EventBus {
    EVENT_BUS.get_or_init(EventBus::new)
}

/// Initialize the global event bus with custom configuration
///
/// Must be called before any calls to `event_bus()`. Returns an error if
/// the event bus has already been initialized.
pub fn init_event_bus(config: EventBusConfig) -> Result<(), EventBusConfig> {
    EVENT_BUS
        .set(EventBus::with_config(config))
        .map_err(|bus| bus.config.clone())
}

/// Convenience macro to publish an event to the global event bus
#[macro_export]
macro_rules! emit {
    ($event:expr) => {
        $crate::event_bus::event_bus().publish($event)
    };
}

/// Convenience macro to subscribe to events on the global event bus
#[macro_export]
macro_rules! on_event {
    ($filter:expr, $handler:expr) => {
        $crate::event_bus::event_bus().subscribe($filter, $handler)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::events::{ConnectionEvent, MachineEvent};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_subscribe_and_unsubscribe() {
        let bus = EventBus::new();

        let id = bus.subscribe(EventFilter::All, |_| {});
        assert_eq!(bus.subscriber_count(), 1);

        assert!(bus.unsubscribe(id));
        assert_eq!(bus.subscriber_count(), 0);

        // Double unsubscribe should return false
        assert!(!bus.unsubscribe(id));
    }

    #[test]
    fn test_event_delivery() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let _id = bus.subscribe(EventFilter::All, move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        let event = AppEvent::Connection(ConnectionEvent::Connected {
            port: "/dev/ttyUSB0".to_string(),
            firmware: "GRBL".to_string(),
        });

        bus.publish(event).expect("Should publish");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_filtering() {
        let bus = EventBus::new();
        let connection_count = Arc::new(AtomicUsize::new(0));
        let machine_count = Arc::new(AtomicUsize::new(0));

        let cc = connection_count.clone();
        bus.subscribe(
            EventFilter::Categories(vec![EventCategory::Connection]),
            move |_| {
                cc.fetch_add(1, Ordering::SeqCst);
            },
        );

        let mc = machine_count.clone();
        bus.subscribe(
            EventFilter::Categories(vec![EventCategory::Machine]),
            move |_| {
                mc.fetch_add(1, Ordering::SeqCst);
            },
        );

        // Publish connection event
        bus.publish(AppEvent::Connection(ConnectionEvent::Connected {
            port: "test".to_string(),
            firmware: "test".to_string(),
        }))
        .ok();

        // Publish machine event
        bus.publish(AppEvent::Machine(MachineEvent::AlarmCleared))
            .ok();

        assert_eq!(connection_count.load(Ordering::SeqCst), 1);
        assert_eq!(machine_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_event_history() {
        let config = EventBusConfig {
            enable_history: true,
            max_history_size: 10,
            ..Default::default()
        };
        let bus = EventBus::with_config(config);

        // Publish some events
        for i in 0..5 {
            bus.publish(AppEvent::Machine(MachineEvent::FeedOverrideChanged {
                percent: i as u8,
            }))
            .ok();
        }

        let history = bus.history(None);
        assert_eq!(history.len(), 5);

        bus.clear_history();
        assert_eq!(bus.history(None).len(), 0);
    }

    #[test]
    fn test_history_max_size() {
        let config = EventBusConfig {
            enable_history: true,
            max_history_size: 5,
            ..Default::default()
        };
        let bus = EventBus::with_config(config);

        // Publish more events than max
        for i in 0..10 {
            bus.publish(AppEvent::Machine(MachineEvent::FeedOverrideChanged {
                percent: i as u8,
            }))
            .ok();
        }

        let history = bus.history(None);
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_filter_matches() {
        let event = AppEvent::Connection(ConnectionEvent::Connected {
            port: "test".to_string(),
            firmware: "test".to_string(),
        });

        assert!(EventFilter::All.matches(&event));
        assert!(EventFilter::Categories(vec![EventCategory::Connection]).matches(&event));
        assert!(!EventFilter::Categories(vec![EventCategory::Machine]).matches(&event));
        assert!(
            EventFilter::Categories(vec![EventCategory::Connection, EventCategory::Machine])
                .matches(&event)
        );
    }

    #[tokio::test]
    async fn test_async_receiver() {
        let bus = EventBus::new();
        let mut receiver = bus.receiver();

        // Publish from another task
        let event = AppEvent::Connection(ConnectionEvent::Connected {
            port: "test".to_string(),
            firmware: "test".to_string(),
        });
        bus.publish(event.clone()).ok();

        // Receive async
        let received = receiver.try_recv();
        assert!(received.is_ok());

        if let Ok(AppEvent::Connection(ConnectionEvent::Connected { port, .. })) = received {
            assert_eq!(port, "test");
        } else {
            panic!("Wrong event received");
        }
    }
}
