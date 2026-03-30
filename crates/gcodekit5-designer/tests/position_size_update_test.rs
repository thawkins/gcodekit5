/// Test for verifying that width/height edits don't cause unwanted position changes
///
/// This test verifies the fix for the bug where editing width or height in the inspector
/// would cause shapes to move to incorrect positions. The bug was in the
/// `calculate_position_and_size_updates` method which incorrectly used the provided
/// x/y values even when update_position=false.

#[cfg(test)]
mod position_size_update_tests {
    use gcodekit5_designer::canvas::Canvas;
    use gcodekit5_designer::model::DesignerShape;

    #[test]
    fn test_resize_width_preserves_position() {
        // Create a canvas and add a rectangle
        let mut canvas = Canvas::new();

        // add_rectangle takes top-left corner coordinates
        // Create rectangle with top-left at (20, 30), width 60, height 40
        // This will have center at (50, 50)
        let id = canvas.add_rectangle(20.0, 30.0, 60.0, 40.0);

        // Select the shape
        canvas.select_shape(id, false);

        // Get initial center position (which is what the UI displays)
        let shape = canvas.get_shape(id).expect("shape not found");
        let (initial_min_x, initial_min_y, initial_max_x, initial_max_y) = shape.shape.bounds();
        let initial_center_x = (initial_min_x + initial_max_x) / 2.0;
        let initial_center_y = (initial_min_y + initial_max_y) / 2.0;
        let initial_width = initial_max_x - initial_min_x;
        let initial_height = initial_max_y - initial_min_y;

        // Verify initial values
        assert!(
            (initial_center_x - 50.0).abs() < 0.01,
            "Initial center X should be 50, got {}",
            initial_center_x
        );
        assert!(
            (initial_center_y - 50.0).abs() < 0.01,
            "Initial center Y should be 50, got {}",
            initial_center_y
        );
        assert!(
            (initial_width - 60.0).abs() < 0.01,
            "Initial width should be 60"
        );
        assert!(
            (initial_height - 40.0).abs() < 0.01,
            "Initial height should be 40"
        );

        // Resize width to 80, keeping center position unchanged (update_position=false, update_size=true)
        // Note: We pass the center coordinates (50, 50) which the UI would pass
        let updates = canvas.calculate_position_and_size_updates(
            initial_center_x, // center x (passed from UI but ignored when update_position=false)
            initial_center_y, // center y (passed from UI but ignored when update_position=false)
            80.0,             // new width
            initial_height,   // height (unchanged)
            false,            // update_position=false - preserve center
            true,             // update_size=true
        );

        // Verify we got an update
        assert_eq!(updates.len(), 1, "Should have one update");

        let (updated_id, updated_obj) = &updates[0];
        assert_eq!(*updated_id, id, "Updated ID should match");

        // Get new bounds and calculate center
        let (new_min_x, new_min_y, new_max_x, new_max_y) = updated_obj.shape.bounds();
        let new_center_x = (new_min_x + new_max_x) / 2.0;
        let new_center_y = (new_min_y + new_max_y) / 2.0;
        let new_width = new_max_x - new_min_x;
        let new_height = new_max_y - new_min_y;

        // Verify CENTER is preserved (this is what matters for the user!)
        assert!(
            (new_center_x - initial_center_x).abs() < 0.01,
            "Center X should be preserved. Expected {}, got {}",
            initial_center_x,
            new_center_x
        );
        assert!(
            (new_center_y - initial_center_y).abs() < 0.01,
            "Center Y should be preserved. Expected {}, got {}",
            initial_center_y,
            new_center_y
        );

        // Verify width changed
        assert!(
            (new_width - 80.0).abs() < 0.01,
            "Width should be 80. Got {}",
            new_width
        );

        // Verify height unchanged
        assert!(
            (new_height - initial_height).abs() < 0.01,
            "Height should remain {}. Got {}",
            initial_height,
            new_height
        );
    }

    #[test]
    fn test_resize_height_preserves_position() {
        // Create a canvas and add a rectangle
        let mut canvas = Canvas::new();

        // add_rectangle takes top-left corner coordinates
        // Create rectangle with top-left at (75, 85), width 50, height 30
        // This will have center at (100, 100)
        let id = canvas.add_rectangle(75.0, 85.0, 50.0, 30.0);

        // Select the shape
        canvas.select_shape(id, false);

        // Get initial center position
        let shape = canvas.get_shape(id).expect("shape not found");
        let (initial_min_x, initial_min_y, initial_max_x, initial_max_y) = shape.shape.bounds();
        let initial_center_x = (initial_min_x + initial_max_x) / 2.0;
        let initial_center_y = (initial_min_y + initial_max_y) / 2.0;
        let initial_width = initial_max_x - initial_min_x;
        let _initial_height = initial_max_y - initial_min_y;

        // Verify initial values
        assert!(
            (initial_center_x - 100.0).abs() < 0.01,
            "Initial center X should be 100, got {}",
            initial_center_x
        );
        assert!(
            (initial_center_y - 100.0).abs() < 0.01,
            "Initial center Y should be 100, got {}",
            initial_center_y
        );

        // Resize height to 60, keeping center position unchanged (update_position=false, update_size=true)
        let updates = canvas.calculate_position_and_size_updates(
            initial_center_x, // center x
            initial_center_y, // center y
            initial_width,    // width (unchanged)
            60.0,             // new height
            false,            // update_position=false - preserve center
            true,             // update_size=true
        );

        // Verify we got an update
        assert_eq!(updates.len(), 1, "Should have one update");

        let (updated_id, updated_obj) = &updates[0];
        assert_eq!(*updated_id, id, "Updated ID should match");

        // Get new bounds and calculate center
        let (new_min_x, new_min_y, new_max_x, new_max_y) = updated_obj.shape.bounds();
        let new_center_x = (new_min_x + new_max_x) / 2.0;
        let new_center_y = (new_min_y + new_max_y) / 2.0;
        let new_width = new_max_x - new_min_x;
        let new_height = new_max_y - new_min_y;

        // Verify CENTER is preserved
        assert!(
            (new_center_x - initial_center_x).abs() < 0.01,
            "Center X should be preserved. Expected {}, got {}",
            initial_center_x,
            new_center_x
        );
        assert!(
            (new_center_y - initial_center_y).abs() < 0.01,
            "Center Y should be preserved. Expected {}, got {}",
            initial_center_y,
            new_center_y
        );

        // Verify width unchanged
        assert!(
            (new_width - initial_width).abs() < 0.01,
            "Width should remain {}. Got {}",
            initial_width,
            new_width
        );

        // Verify height changed
        assert!(
            (new_height - 60.0).abs() < 0.01,
            "Height should be 60. Got {}",
            new_height
        );
    }

    #[test]
    fn test_move_position_updates_correctly() {
        // Create a canvas and add a rectangle
        let mut canvas = Canvas::new();

        // add_rectangle takes top-left corner coordinates
        // Create rectangle with top-left at (20, 30), width 60, height 40
        // This will have center at (50, 50)
        let id = canvas.add_rectangle(20.0, 30.0, 60.0, 40.0);

        // Select the shape
        canvas.select_shape(id, false);

        // Get initial dimensions
        let shape = canvas.get_shape(id).expect("shape not found");
        let (initial_min_x, initial_min_y, initial_max_x, initial_max_y) = shape.shape.bounds();
        let initial_width = initial_max_x - initial_min_x;
        let initial_height = initial_max_y - initial_min_y;

        // Move to new CENTER position (100, 150) without resizing (update_position=true, update_size=false)
        let updates = canvas.calculate_position_and_size_updates(
            100.0,          // new center x
            150.0,          // new center y
            initial_width,  // width (unchanged)
            initial_height, // height (unchanged)
            true,           // update_position=true - move the center
            false,          // update_size=false
        );

        // Verify we got an update
        assert_eq!(updates.len(), 1, "Should have one update");

        let (updated_id, updated_obj) = &updates[0];
        assert_eq!(*updated_id, id, "Updated ID should match");

        // Get new bounds and calculate center
        let (new_min_x, new_min_y, new_max_x, new_max_y) = updated_obj.shape.bounds();
        let new_center_x = (new_min_x + new_max_x) / 2.0;
        let new_center_y = (new_min_y + new_max_y) / 2.0;
        let new_width = new_max_x - new_min_x;
        let new_height = new_max_y - new_min_y;

        // Verify CENTER position changed to target
        assert!(
            (new_center_x - 100.0).abs() < 0.01,
            "Center X should be 100. Got {}",
            new_center_x
        );
        assert!(
            (new_center_y - 150.0).abs() < 0.01,
            "Center Y should be 150. Got {}",
            new_center_y
        );

        // Verify size unchanged
        assert!(
            (new_width - initial_width).abs() < 0.01,
            "Width should remain {}. Got {}",
            initial_width,
            new_width
        );
        assert!(
            (new_height - initial_height).abs() < 0.01,
            "Height should remain {}. Got {}",
            initial_height,
            new_height
        );
    }
}
