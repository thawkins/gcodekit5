use gcodekit5_designer::history::{ActionType, HistoryAction, HistoryTransaction, UndoRedoManager};

#[test]
fn test_create_history_action() {
    let action = HistoryAction::simple(ActionType::ShapeCreated, "Created rectangle".to_string());

    assert_eq!(action.action_type, ActionType::ShapeCreated);
    assert_eq!(action.description, "Created rectangle");
}

#[test]
fn test_undo_redo_manager_creation() {
    let manager = UndoRedoManager::new(50);
    assert!(!manager.can_undo());
    assert!(!manager.can_redo());
    assert_eq!(manager.undo_depth(), 0);
    assert_eq!(manager.redo_depth(), 0);
}

#[test]
fn test_record_single_action() {
    let mut manager = UndoRedoManager::new(50);
    let action = HistoryAction::simple(ActionType::ShapeCreated, "Create shape".to_string());

    manager.record(action);
    assert!(manager.can_undo());
    assert!(!manager.can_redo());
    assert_eq!(manager.undo_depth(), 1);
}

#[test]
fn test_undo_single_action() {
    let mut manager = UndoRedoManager::new(50);
    let action = HistoryAction::simple(ActionType::ShapeCreated, "Create shape".to_string());

    manager.record(action);
    assert!(manager.can_undo());

    let undone = manager.undo();
    assert!(undone.is_some());
    assert!(!manager.can_undo());
    assert!(manager.can_redo());
}

#[test]
fn test_redo_after_undo() {
    let mut manager = UndoRedoManager::new(50);
    let action = HistoryAction::simple(ActionType::ShapeCreated, "Create shape".to_string());

    manager.record(action);
    manager.undo();
    assert!(manager.can_redo());

    let redone = manager.redo();
    assert!(redone.is_some());
    assert!(manager.can_undo());
    assert!(!manager.can_redo());
}

#[test]
fn test_multiple_undo_redo() {
    let mut manager = UndoRedoManager::new(50);

    for i in 0..5 {
        let action = HistoryAction::simple(ActionType::ShapeCreated, format!("Create shape {}", i));
        manager.record(action);
    }

    assert_eq!(manager.undo_depth(), 5);
    assert_eq!(manager.redo_depth(), 0);

    // Undo all
    for _ in 0..5 {
        manager.undo();
    }

    assert_eq!(manager.undo_depth(), 0);
    assert_eq!(manager.redo_depth(), 5);

    // Redo all
    for _ in 0..5 {
        manager.redo();
    }

    assert_eq!(manager.undo_depth(), 5);
    assert_eq!(manager.redo_depth(), 0);
}

#[test]
fn test_redo_clears_on_new_action() {
    let mut manager = UndoRedoManager::new(50);

    let a1 = HistoryAction::simple(ActionType::ShapeCreated, "A".to_string());
    let a2 = HistoryAction::simple(ActionType::ShapeCreated, "B".to_string());
    let a3 = HistoryAction::simple(ActionType::ShapeCreated, "C".to_string());

    manager.record(a1);
    manager.record(a2);
    manager.undo();

    assert_eq!(manager.redo_depth(), 1);

    manager.record(a3);
    assert_eq!(manager.redo_depth(), 0);
}

#[test]
fn test_max_depth_limit() {
    let mut manager = UndoRedoManager::new(3);

    for i in 0..5 {
        let action = HistoryAction::simple(ActionType::ShapeCreated, format!("Action {}", i));
        manager.record(action);
    }

    assert_eq!(manager.undo_depth(), 3);
}

#[test]
fn test_clear_history() {
    let mut manager = UndoRedoManager::new(50);

    let action = HistoryAction::simple(ActionType::ShapeCreated, "Create".to_string());
    manager.record(action);

    assert!(manager.can_undo());

    manager.undo();

    assert!(!manager.can_undo());
    assert!(manager.can_redo());

    manager.clear();

    assert!(!manager.can_undo());
    assert!(!manager.can_redo());
    assert_eq!(manager.undo_depth(), 0);
    assert_eq!(manager.redo_depth(), 0);
}

#[test]
fn test_enable_disable_history() {
    let mut manager = UndoRedoManager::new(50);
    assert!(manager.is_enabled());

    let action = HistoryAction::simple(ActionType::ShapeCreated, "Create".to_string());

    manager.record(action);
    assert_eq!(manager.undo_depth(), 1);

    manager.disable();
    let action = HistoryAction::simple(ActionType::ShapeCreated, "Create2".to_string());
    manager.record(action);

    // Should still be 1 since disabled
    assert_eq!(manager.undo_depth(), 1);

    manager.enable();
    manager.record(HistoryAction::simple(
        ActionType::ShapeCreated,
        "Create3".to_string(),
    ));
    assert_eq!(manager.undo_depth(), 2);
}

#[test]
fn test_descriptions() {
    let mut manager = UndoRedoManager::new(50);

    let action = HistoryAction::simple(
        ActionType::ShapeMoved,
        "Move rectangle to (10, 20)".to_string(),
    );

    manager.record(action);
    assert_eq!(
        manager.undo_description(),
        Some("Move rectangle to (10, 20)".to_string())
    );

    manager.undo();
    assert_eq!(
        manager.redo_description(),
        Some("Move rectangle to (10, 20)".to_string())
    );
}

#[test]
fn test_transaction() {
    let mut txn = HistoryTransaction::new("Multi-shape create".to_string());

    let a1 = HistoryAction::simple(ActionType::ShapeCreated, "Shape 1".to_string());
    let a2 = HistoryAction::simple(ActionType::ShapeCreated, "Shape 2".to_string());

    txn.add_action(a1);
    txn.add_action(a2);

    assert_eq!(txn.action_count(), 2);

    let batch = txn.commit();
    assert_eq!(batch.action_type, ActionType::BatchOperation);
    assert_eq!(batch.description, "Multi-shape create");
}

#[test]
fn test_action_type_display() {
    assert_eq!(ActionType::ShapeCreated.to_string(), "Create Shape");
    assert_eq!(ActionType::ShapeMoved.to_string(), "Move Shape");
    assert_eq!(ActionType::ToolChanged.to_string(), "Change Tool");
}

#[test]
fn test_trim_to_depth() {
    let mut manager = UndoRedoManager::new(100);

    for i in 0..10 {
        let action = HistoryAction::simple(ActionType::ShapeCreated, format!("Action {}", i));
        manager.record(action);
    }

    assert_eq!(manager.undo_depth(), 10);

    manager.trim_to_depth(5);
    assert_eq!(manager.undo_depth(), 5);
}

#[test]
fn test_full_history() {
    let mut manager = UndoRedoManager::new(50);

    let a1 = HistoryAction::simple(ActionType::ShapeCreated, "A".to_string());
    let a2 = HistoryAction::simple(ActionType::ShapeMoved, "B".to_string());

    manager.record(a1);
    manager.record(a2);
    manager.undo();

    let history = manager.full_history();
    assert_eq!(history.len(), 2);
}

#[test]
fn test_serialization() {
    let action = HistoryAction::simple(ActionType::ShapeCreated, "Create rectangle".to_string());

    let json = serde_json::to_string(&action).expect("Failed to serialize");
    let deserialized: HistoryAction = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(action.description, deserialized.description);
    assert_eq!(action.action_type, deserialized.action_type);
}
