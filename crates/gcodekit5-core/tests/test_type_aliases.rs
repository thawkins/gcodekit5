//! Tests for `gcodekit5_core::types::aliases` (moved from the inline test module).

use gcodekit5_core::types::aliases::*;
use std::cell::RefCell;
use std::rc::Rc;

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
