//! Type aliases for commonly used complex types.
//!
//! This module provides type aliases to improve code readability by giving
//! meaningful names to complex nested types commonly used throughout the codebase.
//!
//! ## Rationale
//!
//! Complex types like `Rc<RefCell<Option<Box<dyn Fn()>>>>` are hard to read and
//! understand at a glance. Type aliases provide:
//! - **Readability**: `UiCallback` is clearer than the full type
//! - **Consistency**: Same pattern used the same way across crates
//! - **Documentation**: Alias names convey intent
//! - **Refactoring**: Change the underlying type in one place
//!
//! ## Usage
//!
//! ```rust,ignore
//! use gcodekit5_core::types::*;
//!
//! // Instead of: Rc<RefCell<MyState>>
//! let state: Shared<MyState> = Shared::new(RefCell::new(MyState::default()));
//!
//! // Instead of: Arc<Mutex<Vec<String>>>
//! let queue: ThreadSafeVec<String> = Arc::new(Mutex::new(Vec::new()));
//! ```

use parking_lot::{Mutex, RwLock};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;

// =============================================================================
// SINGLE-THREADED SHARED TYPES (Rc<RefCell<T>>)
// =============================================================================

/// A reference-counted, interior-mutable wrapper for single-threaded sharing.
///
/// Use when you need to share mutable state within a single thread (e.g., GTK UI).
/// This is the fundamental building block for UI state management.
///
/// # Example
/// ```rust,ignore
/// let state: Shared<AppState> = Shared::new(RefCell::new(AppState::default()));
/// state.borrow_mut().update();
/// ```
pub type Shared<T> = Rc<RefCell<T>>;

/// An optional shared reference, for lazily-initialized shared state.
///
/// Common pattern for optional UI components or deferred initialization.
pub type SharedOption<T> = Rc<RefCell<Option<T>>>;

/// A shared vector for single-threaded collection management.
pub type SharedVec<T> = Rc<RefCell<Vec<T>>>;

/// A shared hash map for single-threaded key-value storage.
pub type SharedHashMap<K, V> = Rc<RefCell<HashMap<K, V>>>;

// =============================================================================
// THREAD-SAFE SHARED TYPES (Arc<Mutex<T>> / Arc<RwLock<T>>)
// =============================================================================

/// A thread-safe, mutex-protected wrapper for cross-thread sharing.
///
/// Use when you need to share mutable state across threads (e.g., async tasks).
/// Uses `parking_lot::Mutex` for better performance than `std::sync::Mutex`.
///
/// # Example
/// ```rust,ignore
/// let state: ThreadSafe<AppState> = Arc::new(Mutex::new(AppState::default()));
/// state.lock().update();
/// ```
pub type ThreadSafe<T> = Arc<Mutex<T>>;

/// A thread-safe optional wrapper for lazily-initialized cross-thread state.
pub type ThreadSafeOption<T> = Arc<Mutex<Option<T>>>;

/// A thread-safe vector for cross-thread collection management.
pub type ThreadSafeVec<T> = Arc<Mutex<Vec<T>>>;

/// A thread-safe deque for cross-thread queue/buffer management.
pub type ThreadSafeDeque<T> = Arc<Mutex<VecDeque<T>>>;

/// A thread-safe hash map for cross-thread key-value storage.
pub type ThreadSafeMap<K, V> = Arc<Mutex<HashMap<K, V>>>;

/// A thread-safe reader-writer lock wrapper for read-heavy workloads.
///
/// Use when reads greatly outnumber writes. Multiple readers can access
/// concurrently, but writes require exclusive access.
pub type ThreadSafeRw<T> = Arc<RwLock<T>>;

/// A thread-safe reader-writer hash map.
pub type ThreadSafeRwMap<K, V> = Arc<RwLock<HashMap<K, V>>>;

// =============================================================================
// CALLBACK TYPES
// =============================================================================

/// A simple callback with no parameters or return value.
///
/// Thread-safe, suitable for cross-thread event notification.
pub type Callback = Box<dyn Fn() + Send + Sync>;

/// A callback that receives a single parameter.
///
/// Thread-safe, suitable for cross-thread data notification.
pub type DataCallback<T> = Box<dyn Fn(T) + Send + Sync>;

/// A callback that receives two parameters.
pub type DataCallback2<T, U> = Box<dyn Fn(T, U) + Send + Sync>;

/// A progress callback receiving (current, total) values.
///
/// Common for file operations, downloads, and long-running tasks.
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// A result callback receiving a Result type.
pub type ResultCallback<T, E> = Box<dyn Fn(Result<T, E>) + Send + Sync>;

/// A UI callback stored in RefCell for GTK signal handlers.
///
/// Single-threaded, suitable for GTK callbacks that capture UI state.
pub type UiCallback = Rc<RefCell<Option<Box<dyn Fn()>>>>;

/// A UI callback with a single parameter.
pub type UiDataCallback<T> = Rc<RefCell<Option<Box<dyn Fn(T)>>>>;

/// A UI callback with two parameters.
pub type UiDataCallback2<T, U> = Rc<RefCell<Option<Box<dyn Fn(T, U)>>>>;

// =============================================================================
// LISTENER/HANDLER TYPES
// =============================================================================

/// A list of callbacks for multi-listener patterns.
pub type CallbackList = ThreadSafeVec<Callback>;

/// A map of subscription IDs to callbacks for event bus patterns.
pub type SubscriptionMap<K, V> = ThreadSafeRwMap<K, V>;

// =============================================================================
// DYNAMIC DISPATCH TYPES (Box<dyn T>)
// =============================================================================
// These types use trait objects for legitimate runtime polymorphism.
// Each is documented with why dynamic dispatch is necessary.

/// A boxed dynamically-typed iterator.
///
/// Used when the concrete iterator type varies at runtime (e.g., forward vs reverse).
/// This is necessary because `Range<T>` and `Rev<Range<T>>` are different types.
///
/// # When to use
/// - Conditional iteration direction (forward/reverse)
/// - Iterator type determined at runtime
///
/// # Example
/// ```rust,ignore
/// let iter: BoxedIterator<u32> = if ascending {
///     Box::new(0..10)
/// } else {
///     Box::new((0..10).rev())
/// };
/// ```
pub type BoxedIterator<T> = Box<dyn Iterator<Item = T>>;

/// A boxed error type for simplified error handling.
///
/// Use `anyhow::Error` for most cases. This type is provided for compatibility
/// with APIs that require `Box<dyn Error>`.
///
/// # When to prefer alternatives
/// - Use `anyhow::Result<T>` for application code
/// - Use `thiserror` for library error types
/// - Use this only when interfacing with external APIs
pub type BoxedError = Box<dyn std::error::Error>;

/// A boxed error type that is thread-safe.
///
/// Use when errors need to cross thread boundaries.
pub type BoxedSendError = Box<dyn std::error::Error + Send + Sync>;

/// A result type with boxed error for legacy compatibility.
///
/// Prefer `anyhow::Result<T>` for new code.
pub type BoxedResult<T> = Result<T, BoxedError>;

/// A result type with thread-safe boxed error.
pub type BoxedSendResult<T> = Result<T, BoxedSendError>;

// =============================================================================
// CONSTRUCTOR HELPERS
// =============================================================================

/// Create a new `Shared<T>` from a value.
///
/// # Example
/// ```rust,ignore
/// let state = shared(AppState::default());
/// ```
#[inline]
pub fn shared<T>(value: T) -> Shared<T> {
    Rc::new(RefCell::new(value))
}

/// Create a new `SharedOption<T>` initialized to `None`.
#[inline]
pub fn shared_none<T>() -> SharedOption<T> {
    Rc::new(RefCell::new(None))
}

/// Create a new `SharedOption<T>` initialized to `Some(value)`.
#[inline]
pub fn shared_some<T>(value: T) -> SharedOption<T> {
    Rc::new(RefCell::new(Some(value)))
}

/// Create a new `ThreadSafe<T>` from a value.
///
/// # Example
/// ```rust,ignore
/// let state = thread_safe(AppState::default());
/// ```
#[inline]
pub fn thread_safe<T>(value: T) -> ThreadSafe<T> {
    Arc::new(Mutex::new(value))
}

/// Create a new `ThreadSafeOption<T>` initialized to `None`.
#[inline]
pub fn thread_safe_none<T>() -> ThreadSafeOption<T> {
    Arc::new(Mutex::new(None))
}

/// Create a new `ThreadSafeOption<T>` initialized to `Some(value)`.
#[inline]
pub fn thread_safe_some<T>(value: T) -> ThreadSafeOption<T> {
    Arc::new(Mutex::new(Some(value)))
}

/// Create a new empty `ThreadSafeVec<T>`.
#[inline]
pub fn thread_safe_vec<T>() -> ThreadSafeVec<T> {
    Arc::new(Mutex::new(Vec::new()))
}

/// Create a new empty `ThreadSafeDeque<T>`.
#[inline]
pub fn thread_safe_deque<T>() -> ThreadSafeDeque<T> {
    Arc::new(Mutex::new(VecDeque::new()))
}

/// Create a new empty `ThreadSafeMap<K, V>`.
#[inline]
pub fn thread_safe_map<K, V>() -> ThreadSafeMap<K, V> {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Create a new `ThreadSafeRw<T>` from a value.
#[inline]
pub fn thread_safe_rw<T>(value: T) -> ThreadSafeRw<T> {
    Arc::new(RwLock::new(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_creation() {
        let value: Shared<i32> = shared(42);
        assert_eq!(*value.borrow(), 42);

        *value.borrow_mut() = 100;
        assert_eq!(*value.borrow(), 100);
    }

    #[test]
    fn test_shared_option() {
        let opt: SharedOption<String> = shared_none();
        assert!(opt.borrow().is_none());

        *opt.borrow_mut() = Some("hello".to_string());
        assert_eq!(opt.borrow().as_ref().map(|s| s.as_str()), Some("hello"));
    }

    #[test]
    fn test_thread_safe_creation() {
        let value: ThreadSafe<i32> = thread_safe(42);
        assert_eq!(*value.lock(), 42);

        *value.lock() = 100;
        assert_eq!(*value.lock(), 100);
    }

    #[test]
    fn test_thread_safe_vec() {
        let vec: ThreadSafeVec<String> = thread_safe_vec();
        vec.lock().push("item1".to_string());
        vec.lock().push("item2".to_string());

        assert_eq!(vec.lock().len(), 2);
    }

    #[test]
    fn test_thread_safe_deque() {
        let deque: ThreadSafeDeque<i32> = thread_safe_deque();
        deque.lock().push_back(1);
        deque.lock().push_back(2);
        deque.lock().push_front(0);

        assert_eq!(deque.lock().len(), 3);
        assert_eq!(deque.lock().pop_front(), Some(0));
    }

    #[test]
    fn test_thread_safe_map() {
        let map: ThreadSafeMap<String, i32> = thread_safe_map();
        map.lock().insert("key1".to_string(), 1);
        map.lock().insert("key2".to_string(), 2);

        assert_eq!(map.lock().get("key1"), Some(&1));
    }

    #[test]
    fn test_thread_safe_rw() {
        let value: ThreadSafeRw<i32> = thread_safe_rw(42);

        // Multiple readers
        assert_eq!(*value.read(), 42);
        assert_eq!(*value.read(), 42);

        // Writer
        *value.write() = 100;
        assert_eq!(*value.read(), 100);
    }

    #[test]
    fn test_ui_callback() {
        let callback: UiCallback = Rc::new(RefCell::new(None));
        assert!(callback.borrow().is_none());

        let counter = shared(0);
        let counter_clone = counter.clone();

        *callback.borrow_mut() = Some(Box::new(move || {
            *counter_clone.borrow_mut() += 1;
        }));

        // Call the callback
        if let Some(ref cb) = *callback.borrow() {
            cb();
        }

        assert_eq!(*counter.borrow(), 1);
    }

    #[test]
    fn test_boxed_iterator() {
        // Forward iterator
        let forward: BoxedIterator<u32> = Box::new(0..5);
        let result: Vec<u32> = forward.collect();
        assert_eq!(result, vec![0, 1, 2, 3, 4]);

        // Reverse iterator
        let reverse: BoxedIterator<u32> = Box::new((0..5).rev());
        let result: Vec<u32> = reverse.collect();
        assert_eq!(result, vec![4, 3, 2, 1, 0]);

        // Conditional direction (simulating bidirectional scanning)
        let ascending = true;
        let iter: BoxedIterator<u32> = if ascending {
            Box::new(0..3)
        } else {
            Box::new((0..3).rev())
        };
        let result: Vec<u32> = iter.collect();
        assert_eq!(result, vec![0, 1, 2]);
    }

    #[test]
    fn test_boxed_result() {
        fn fallible_operation(succeed: bool) -> BoxedResult<i32> {
            if succeed {
                Ok(42)
            } else {
                Err("operation failed".into())
            }
        }

        assert!(fallible_operation(true).is_ok());
        assert!(fallible_operation(false).is_err());
    }
}
