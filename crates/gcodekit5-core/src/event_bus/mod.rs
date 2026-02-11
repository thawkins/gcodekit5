//! # Event Bus Module
//!
//! Provides a unified event bus system for decoupled communication between
//! application components. See ADR-006 for design details.
//!
//! ## Overview
//!
//! The event bus enables publish/subscribe patterns across the application:
//! - Publishers emit typed events without knowing subscribers
//! - Subscribers filter and receive events of interest
//! - Supports both sync and async event handling
//!
//! ## Usage
//!
//! ```rust,ignore
//! use gcodekit5_core::event_bus::{event_bus, AppEvent, ConnectionEvent, EventFilter, EventCategory};
//!
//! // Subscribe to connection events
//! let subscription = event_bus().subscribe(
//!     EventFilter::Categories(vec![EventCategory::Connection]),
//!     |event| {
//!         if let AppEvent::Connection(conn) = event {
//!             println!("Connection event: {:?}", conn);
//!         }
//!     },
//! );
//!
//! // Publish an event
//! event_bus().publish(AppEvent::Connection(ConnectionEvent::Connected {
//!     port: "/dev/ttyUSB0".to_string(),
//!     firmware: "GRBL 1.1h".to_string(),
//! }));
//!
//! // Unsubscribe when done
//! event_bus().unsubscribe(subscription);
//! ```

mod bus;
mod events;

pub use bus::*;
pub use events::*;
