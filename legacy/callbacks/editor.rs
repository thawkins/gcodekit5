use std::rc::Rc;
use std::sync::{Arc, Mutex};
use slint::ComponentHandle;
use crate::slint_generatedMainWindow::MainWindow;
use gcodekit5_ui::GcodeEditor;
use gcodekit5::{DeviceConsoleManager as ConsoleManager, DeviceMessageType, Communicator};
use gcodekit5_ui::EditorBridge;
use gcodekit5::SerialCommunicator;
use tracing::warn;

pub fn register_callbacks(
    main_window: &MainWindow,
    gcode_editor: Rc<GcodeEditor>,
    console_manager: Arc<ConsoleManager>,
    editor_bridge: Rc<EditorBridge>,
    communicator: Arc<Mutex<SerialCommunicator>>,
) {
    // Set up menu-file-exit callback
    let communicator_clone = communicator.clone();
    main_window.on_menu_file_exit(move || {
        // Disconnect if connected before exiting
        let mut comm = communicator_clone.lock().unwrap();
        if comm.disconnect().is_err() {}
        std::process::exit(0);
    });

    // Set up menu-file-new callback
    let window_weak = main_window.as_weak();
    main_window.on_menu_file_new(move || {
        if let Some(window) = window_weak.upgrade() {
            let content = window.get_gcode_content();
            if !content.is_empty() {
                let should_clear = rfd::MessageDialog::new()
                    .set_title("Clear Editor?")
                    .set_description("The editor contains unsaved G-Code. Are you sure you want to clear it?")
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .set_level(rfd::MessageLevel::Warning)
                    .show();
                
                if should_clear != rfd::MessageDialogResult::Yes {
                    return;
                }
            }
            
            // Clear the editor content and reset filename
            window.set_gcode_filename(slint::SharedString::from("unknown.gcode"));
            window.invoke_clear_editor();
        }
    });

    // Set up menu-file-open callback
    let window_weak = main_window.as_weak();
    let gcode_editor_clone = gcode_editor.clone();
    let console_manager_clone = console_manager.clone();
    let editor_bridge_open = editor_bridge.clone();
    main_window.on_menu_file_open(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_is_busy(true);
        }

        // Open file dialog and load file
        match gcode_editor_clone.open_and_load_file() {
            Ok(path) => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let full_path = path.display().to_string();
                let line_count = gcode_editor_clone.get_line_count();

                // Log to device console
                console_manager_clone.add_message(
                    DeviceMessageType::Success,
                    format!("✓ File loaded: {}", file_name),
                );
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("  Lines: {}", line_count),
                );
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("  Path: {}", path.display()),
                );

                // Update UI with new content
                let content = gcode_editor_clone.get_plain_content();

                let preview = if content.len() > 100 {
                    format!("{}...", &content[..100])
                } else {
                    content.clone()
                };
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("CONTENT LENGTH: {} chars", content.len()),
                );
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("CONTENT PREVIEW: {}", preview),
                );

                if let Some(window) = window_weak.upgrade() {
                    // DEBUG: Log view switch
                    console_manager_clone.add_message(
                        DeviceMessageType::Output,
                        "DEBUG: Switching to gcode-editor view".to_string(),
                    );

                    // IMPORTANT: Switch to gcode-editor view to show the content
                    window.set_current_view(slint::SharedString::from("gcode-editor"));
                    window.set_gcode_focus_trigger(window.get_gcode_focus_trigger() + 1);

                    console_manager_clone.add_message(
                        DeviceMessageType::Output,
                        format!("DEBUG: Setting TextEdit content ({} chars)", content.len()),
                    );

                    // Load into custom editor
                    editor_bridge_open.load_text(&content);

                    window.set_gcode_content(slint::SharedString::from(content.clone()));

                    // Update custom editor state
                    let line_count = editor_bridge_open.line_count();
                    window.set_can_undo(editor_bridge_open.can_undo());
                    window.set_can_redo(editor_bridge_open.can_redo());
                    window.set_total_lines(line_count as i32);
                    super::super::helpers::update_visible_lines(&window, &editor_bridge_open);

                    // VERIFY: Log what was set
                    let verify_content = window.get_gcode_content();
                    console_manager_clone.add_message(
                        DeviceMessageType::Output,
                        format!(
                            "VERIFY: get_gcode_content returned {} chars",
                            verify_content.len()
                        ),
                    );

                    window.set_gcode_filename(slint::SharedString::from(&full_path));
                    window.set_connection_status(slint::SharedString::from(format!(
                        "Loaded: {} ({} lines)",
                        file_name, line_count
                    )));

                    // Render visualization
                    let width = window.get_visualizer_canvas_width();
                    let height = window.get_visualizer_canvas_height();
                    let max_intensity = window.get_visualizer_max_intensity();
                    window.invoke_refresh_visualization(width, height, max_intensity);

                    // DEBUG: Log console update
                    console_manager_clone.add_message(
                        DeviceMessageType::Output,
                        "DEBUG: TextEdit content set in view".to_string(),
                    );

                    // Update console display
                    let console_output = console_manager_clone.get_output();
                    window.set_console_output(slint::SharedString::from(console_output));
                }
            }
            Err(e) => {
                let error_msg = e.to_string();

                // Silently ignore dialog cancellations
                if error_msg.contains("cancelled") {
                    return;
                }

                warn!("Failed to open file: {}", e);

                // Log error to device console
                console_manager_clone.add_message(
                    DeviceMessageType::Error,
                    format!("✗ Failed to load file: {}", e),
                );

                if let Some(window) = window_weak.upgrade() {
                    window
                        .set_connection_status(slint::SharedString::from(format!("Error: {}", e)));
                    window.set_is_busy(false);

                    // Update console display
                    let console_output = console_manager_clone.get_output();
                    window.set_console_output(slint::SharedString::from(console_output));
                }
            }
        }

        // Always clear busy state at end
        if let Some(window) = window_weak.upgrade() {
            window.set_is_busy(false);
        }
    });

    // Set up menu-file-save callback
    let window_weak = main_window.as_weak();
    let gcode_editor_clone = gcode_editor.clone();
    let console_manager_clone = console_manager.clone();
    main_window.on_menu_file_save(move || {
        // Get current filename and content from window
        if let Some(window) = window_weak.upgrade() {
            let filename = window.get_gcode_filename().to_string();
            let current_content = window.get_gcode_content().to_string();

            // If it's "untitled.gcode", prompt for filename (treat as Save As)
            if filename.contains("untitled") {
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    "No file loaded. Use 'Save As' to save with a filename.",
                );
                window.set_connection_status(slint::SharedString::from(
                    "Please use 'Save As' to save the file",
                ));
                return;
            }

            // Save to current file with current content from TextEdit
            match gcode_editor_clone.save_file_with_content(&current_content) {
                Ok(_) => {
                    console_manager_clone.add_message(
                        DeviceMessageType::Success,
                        format!("✓ File saved: {}", filename),
                    );
                    window.set_connection_status(slint::SharedString::from(format!(
                        "Saved: {}",
                        filename
                    )));
                }
                Err(e) => {
                    warn!("Failed to save file: {}", e);
                    console_manager_clone
                        .add_message(DeviceMessageType::Error, format!("✗ Failed to save: {}", e));
                    window.set_connection_status(slint::SharedString::from(format!(
                        "Error saving file: {}",
                        e
                    )));
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up menu-file-save-as callback
    let window_weak = main_window.as_weak();
    let gcode_editor_clone = gcode_editor.clone();
    let console_manager_clone = console_manager.clone();
    main_window.on_menu_file_save_as(move || {
        if let Some(window) = window_weak.upgrade() {
            let current_content = window.get_gcode_content().to_string();

            // Use the editor's save_as_with_dialog_and_content method with current content
            match gcode_editor_clone.save_as_with_dialog_and_content(&current_content) {
                Ok(path) => {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let full_path = path.display().to_string();

                    console_manager_clone.add_message(
                        DeviceMessageType::Success,
                        format!("✓ File saved as: {}", file_name),
                    );
                    console_manager_clone.add_message(
                        DeviceMessageType::Output,
                        format!("  Path: {}", path.display()),
                    );

                    // Update filename in UI
                    window.set_gcode_filename(slint::SharedString::from(full_path));
                    window.set_connection_status(slint::SharedString::from(format!(
                        "Saved as: {}",
                        file_name
                    )));
                }
                Err(e) => {
                    warn!("Failed to save file as: {}", e);
                    console_manager_clone.add_message(
                        DeviceMessageType::Error,
                        format!("✗ Failed to save as: {}", e),
                    );
                    window
                        .set_connection_status(slint::SharedString::from(format!("Error: {}", e)));
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up undo callback for custom editor
    let window_weak = main_window.as_weak();
    let editor_bridge_undo = editor_bridge.clone();
    main_window.on_undo_requested(move || {
        if editor_bridge_undo.undo() {
            if let Some(window) = window_weak.upgrade() {
                // Update UI state
                window.set_can_undo(editor_bridge_undo.can_undo());
                window.set_can_redo(editor_bridge_undo.can_redo());

                // Update viewport if cursor moved off-screen
                let (start_line, _end_line) = editor_bridge_undo.viewport_range();
                window.set_visible_start_line(start_line as i32);

                super::super::helpers::update_visible_lines(&window, &editor_bridge_undo);

                // Update g-code content
                let content = editor_bridge_undo.get_text();
                window.set_gcode_content(slint::SharedString::from(content));

                // Update cursor position
                let (line, col) = editor_bridge_undo.cursor_position();
                window.set_cursor_line((line + 1) as i32);
                window.set_cursor_column((col + 1) as i32);
            }
        }
    });

    // Set up redo callback for custom editor
    let window_weak = main_window.as_weak();
    let editor_bridge_redo = editor_bridge.clone();
    main_window.on_redo_requested(move || {
        if editor_bridge_redo.redo() {
            if let Some(window) = window_weak.upgrade() {
                // Update UI state
                window.set_can_undo(editor_bridge_redo.can_undo());
                window.set_can_redo(editor_bridge_redo.can_redo());

                // Update viewport if cursor moved off-screen
                let (start_line, _end_line) = editor_bridge_redo.viewport_range();
                window.set_visible_start_line(start_line as i32);

                super::super::helpers::update_visible_lines(&window, &editor_bridge_redo);

                // Update g-code content
                let content = editor_bridge_redo.get_text();
                window.set_gcode_content(slint::SharedString::from(content));

                // Update cursor position
                let (line, col) = editor_bridge_redo.cursor_position();
                window.set_cursor_line((line + 1) as i32);
                window.set_cursor_column((col + 1) as i32);
            }
        }
    });

    // Set up scroll callback for custom editor
    let window_weak = main_window.as_weak();
    let editor_bridge_scroll = editor_bridge.clone();
    main_window.on_scroll_changed(move |line| {
        editor_bridge_scroll.scroll_to_line(line as usize);
        if let Some(window) = window_weak.upgrade() {
            super::super::helpers::update_visible_lines(&window, &editor_bridge_scroll);
        }
    });

    // Set up text-changed callback for custom editor
    let window_weak = main_window.as_weak();
    let editor_bridge_text = editor_bridge.clone();
    main_window.on_text_changed(move |_text| {
        if let Some(window) = window_weak.upgrade() {
            window.set_can_undo(editor_bridge_text.can_undo());
            window.set_can_redo(editor_bridge_text.can_redo());
        }
    });

    // Custom editor callbacks
    let window_weak = main_window.as_weak();
    let editor_bridge_clear = editor_bridge.clone();
    main_window.on_clear_editor(move || {
        editor_bridge_clear.load_text("");
        if let Some(window) = window_weak.upgrade() {
            let line_count = editor_bridge_clear.line_count();
            window.set_total_lines(line_count as i32);
            super::super::helpers::update_visible_lines(&window, &editor_bridge_clear);
        }
    });

    let window_weak = main_window.as_weak();
    let editor_bridge_append = editor_bridge.clone();
    main_window.on_append_gcode_line(move |line| {
        let current_text = editor_bridge_append.get_text();
        let new_text = if current_text.is_empty() {
            line.to_string()
        } else {
            format!("{}\n{}", current_text, line)
        };
        editor_bridge_append.load_text(&new_text);
        if let Some(window) = window_weak.upgrade() {
            let line_count = editor_bridge_append.line_count();
            window.set_total_lines(line_count as i32);
            super::super::helpers::update_visible_lines(&window, &editor_bridge_append);
        }
    });

    let window_weak = main_window.as_weak();
    let editor_bridge_loader = editor_bridge.clone();
    main_window.on_load_editor_text(move |text| {
        let shared = text.clone();
        let content = text.to_string();
        editor_bridge_loader.load_text(&content);
        // Scroll to top
        editor_bridge_loader.scroll_to_line(0);
        if let Some(window) = window_weak.upgrade() {
            window.set_gcode_content(shared);
            window.set_can_undo(editor_bridge_loader.can_undo());
            window.set_can_redo(editor_bridge_loader.can_redo());
            window.set_total_lines(editor_bridge_loader.line_count() as i32);
            // Start cursor at position (1, 1) - line 1, column 1
            window.set_cursor_line(1);
            window.set_cursor_column(1);
            super::super::helpers::update_visible_lines(&window, &editor_bridge_loader);
        }
    });

    // Text editing callbacks
    let window_weak = main_window.as_weak();
    let editor_bridge_insert = editor_bridge.clone();
    main_window.on_text_inserted(move |line, col, text| {
        let text_str = text.to_string();
        // Move cursor to the position where text should be inserted (convert 1-based to 0-based)
        let line_0based = (line - 1).max(0) as usize;
        let col_0based = (col - 1).max(0) as usize;
        editor_bridge_insert.set_cursor(line_0based, col_0based);
        // Now insert the text at the cursor position
        editor_bridge_insert.insert_text(&text_str);
        if let Some(window) = window_weak.upgrade() {
            window.set_can_undo(editor_bridge_insert.can_undo());
            window.set_can_redo(editor_bridge_insert.can_redo());
            window.set_total_lines(editor_bridge_insert.line_count() as i32);
            // Update viewport if cursor moved off-screen
            let (start_line, _end_line) = editor_bridge_insert.viewport_range();
            window.set_visible_start_line(start_line as i32);
            super::super::helpers::update_visible_lines(&window, &editor_bridge_insert);
            let (line, col) = editor_bridge_insert.cursor_position();
            window.set_cursor_line((line + 1) as i32);
            window.set_cursor_column((col + 1) as i32);
            let content = editor_bridge_insert.get_text();
            window.set_gcode_content(slint::SharedString::from(content));
        }
    });

    let window_weak = main_window.as_weak();
    let editor_bridge_delete = editor_bridge.clone();
    main_window.on_text_deleted(move |start_line, start_col, _end_line, end_col| {
        let count = (end_col - start_col).max(0) as usize;
        if count > 0 {
            // Move cursor to the position where deletion should occur (convert 1-based to 0-based)
            let line_0based = (start_line - 1).max(0) as usize;
            let col_0based = (start_col - 1).max(0) as usize;
            editor_bridge_delete.set_cursor(line_0based, col_0based);
            // Now delete from the cursor position
            editor_bridge_delete.delete_backward(count);
            if let Some(window) = window_weak.upgrade() {
                window.set_can_undo(editor_bridge_delete.can_undo());
                window.set_can_redo(editor_bridge_delete.can_redo());
                window.set_total_lines(editor_bridge_delete.line_count() as i32);
                // Update viewport if cursor moved off-screen
                let (start_line, _end_line) = editor_bridge_delete.viewport_range();
                window.set_visible_start_line(start_line as i32);
                super::super::helpers::update_visible_lines(&window, &editor_bridge_delete);
                let (line, col) = editor_bridge_delete.cursor_position();
                window.set_cursor_line((line + 1) as i32);
                window.set_cursor_column((col + 1) as i32);
                let content = editor_bridge_delete.get_text();
                window.set_gcode_content(slint::SharedString::from(content));
            }
        }
    });

    // Cursor navigation callback
    let window_weak = main_window.as_weak();
    let editor_bridge_cursor = editor_bridge.clone();
    main_window.on_cursor_moved(move |line, col| {
        // Handle line wrapping for arrow keys
        let mut line_0based = (line - 1).max(0) as usize;
        let mut col_0based = col - 1;

        // If col is 0 or negative and there's a line above, move to end of previous line
        if col_0based < 0 && line_0based > 0 {
            line_0based -= 1;
            // Get the previous line's length
            if let Some(prev_line) = editor_bridge_cursor.get_line_at(line_0based) {
                col_0based = (prev_line.len() - 1).max(0) as i32;
            }
        }
        // If col is beyond line length and there's a line below, move to start of next line
        else if line_0based < editor_bridge_cursor.line_count() - 1 {
            if let Some(curr_line) = editor_bridge_cursor.get_line_at(line_0based) {
                if col_0based >= curr_line.len() as i32 {
                    line_0based += 1;
                    col_0based = 0;
                }
            }
        }

        let col_0based = col_0based.max(0) as usize;
        editor_bridge_cursor.set_cursor(line_0based, col_0based);

        if let Some(window) = window_weak.upgrade() {
            // Use the values we calculated (with wrapping), not the clamped ones
            let display_line = (line_0based + 1) as i32;
            let display_col = (col_0based + 1) as i32;
            window.set_cursor_line(display_line);
            window.set_cursor_column(display_col);

            // Update viewport to keep cursor visible
            let (start_line, _end_line) = editor_bridge_cursor.viewport_range();
            window.set_visible_start_line(start_line as i32);

            // Update visible lines to show cursor
            super::super::helpers::update_visible_lines(&window, &editor_bridge_cursor);
        }
    });

    // End key pressed - move cursor to end of current line
    let window_weak = main_window.as_weak();
    let editor_bridge_end = editor_bridge.clone();
    main_window.on_end_key_pressed(move || {
        // Get current cursor position and move to end of line
        let (line, _col) = editor_bridge_end.cursor_position();
        // Get the length of the current line
        let text = editor_bridge_end.get_text();
        let lines: Vec<&str> = text.lines().collect();
        let line_end_col = if line < lines.len() {
            lines[line].len()
        } else {
            0
        };

        // Move cursor to end of line (convert to 0-based cursor position)
        editor_bridge_end.set_cursor(line, line_end_col);

        if let Some(window) = window_weak.upgrade() {
            // Get actual cursor position
            let (actual_line, actual_col) = editor_bridge_end.cursor_position();
            // Convert to 1-based for display
            window.set_cursor_line((actual_line + 1) as i32);
            window.set_cursor_column((actual_col + 1) as i32);

            // Update viewport
            let (start_line, _end_line) = editor_bridge_end.viewport_range();
            window.set_visible_start_line(start_line as i32);

            super::super::helpers::update_visible_lines(&window, &editor_bridge_end);
        }
    });

    // Ctrl+Home: Jump to beginning of file
    let window_weak = main_window.as_weak();
    let editor_bridge_ctrl_home = editor_bridge.clone();
    main_window.on_ctrl_home_pressed(move || {
        // Move to first line, first column
        editor_bridge_ctrl_home.set_cursor(0, 0);

        if let Some(window) = window_weak.upgrade() {
            // Get actual cursor position
            let (actual_line, actual_col) = editor_bridge_ctrl_home.cursor_position();
            // Convert to 1-based for display
            window.set_cursor_line((actual_line + 1) as i32);
            window.set_cursor_column((actual_col + 1) as i32);

            // Update viewport to top
            window.set_visible_start_line(0);

            super::super::helpers::update_visible_lines(&window, &editor_bridge_ctrl_home);
        }
    });

    // Ctrl+End: Jump to end of file
    let window_weak = main_window.as_weak();
    let editor_bridge_ctrl_end = editor_bridge.clone();
    main_window.on_ctrl_end_pressed(move || {
        // Get total lines and last line
        let line_count = editor_bridge_ctrl_end.line_count();
        let last_line = if line_count > 0 { line_count - 1 } else { 0 };

        // Get the length of the last line
        let text = editor_bridge_ctrl_end.get_text();
        let lines: Vec<&str> = text.lines().collect();
        let last_col = if last_line < lines.len() {
            lines[last_line].len()
        } else {
            0
        };

        // Move cursor to end of last line
        editor_bridge_ctrl_end.set_cursor(last_line, last_col);

        if let Some(window) = window_weak.upgrade() {
            // Get actual cursor position
            let (actual_line, actual_col) = editor_bridge_ctrl_end.cursor_position();
            // Convert to 1-based for display
            window.set_cursor_line((actual_line + 1) as i32);
            window.set_cursor_column((actual_col + 1) as i32);

            // Update viewport to show cursor
            let (start_line, _end_line) = editor_bridge_ctrl_end.viewport_range();
            window.set_visible_start_line(start_line as i32);

            super::super::helpers::update_visible_lines(&window, &editor_bridge_ctrl_end);
        }
    });

    // Mouse click callback - convert pixels to line/column
    main_window.on_mouse_clicked(move |_x, _y| {
        // TODO: Implement mouse-based cursor positioning
        // Currently clicking just focuses the editor for keyboard input
    });

    // Find callback
    let window_weak = main_window.as_weak();
    main_window.on_find_requested(move |_search| {
        if let Some(_window) = window_weak.upgrade() {
            // TODO: Implement find functionality in EditorBridge
        }
    });

    // Find and replace callback
    let window_weak = main_window.as_weak();
    main_window.on_replace_requested(move |_search, _replace| {
        if let Some(_window) = window_weak.upgrade() {
            // TODO: Implement find/replace functionality in EditorBridge
        }
    });

    // Set up menu-view-gcode-editor callback
    let window_weak = main_window.as_weak();
    main_window.on_menu_view_gcode_editor(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_connection_status(slint::SharedString::from("G-Code Editor activated"));
            // Trigger focus on the editor by incrementing the trigger counter
            window.set_gcode_focus_trigger(window.get_gcode_focus_trigger() + 1);
        }
    });

    // Debug callback for key-pressed events from editor
    main_window.on_key_pressed_event(move |_msg| {});

    // Ctrl+Home: Jump to beginning of file
    let window_weak = main_window.as_weak();
    let editor_bridge_ctrl_home = editor_bridge.clone();
    main_window.on_ctrl_home_pressed(move || {
        // Move to first line, first column
        editor_bridge_ctrl_home.set_cursor(0, 0);

        if let Some(window) = window_weak.upgrade() {
            // Get actual cursor position
            let (actual_line, actual_col) = editor_bridge_ctrl_home.cursor_position();
            // Convert to 1-based for display
            window.set_cursor_line((actual_line + 1) as i32);
            window.set_cursor_column((actual_col + 1) as i32);

            // Update viewport to top
            window.set_visible_start_line(0);

            super::super::helpers::update_visible_lines(&window, &editor_bridge_ctrl_home);
        }
    });

    // Ctrl+End: Jump to end of file
    let window_weak = main_window.as_weak();
    let editor_bridge_ctrl_end = editor_bridge.clone();
    main_window.on_ctrl_end_pressed(move || {
        // Get total lines and last line
        let line_count = editor_bridge_ctrl_end.line_count();
        let last_line = if line_count > 0 { line_count - 1 } else { 0 };

        // Get the length of the last line
        let text = editor_bridge_ctrl_end.get_text();
        let lines: Vec<&str> = text.lines().collect();
        let last_col = if last_line < lines.len() {
            lines[last_line].len()
        } else {
            0
        };

        // Move cursor to end of last line
        editor_bridge_ctrl_end.set_cursor(last_line, last_col);

        if let Some(window) = window_weak.upgrade() {
            // Get actual cursor position
            let (actual_line, actual_col) = editor_bridge_ctrl_end.cursor_position();
            // Convert to 1-based for display
            window.set_cursor_line((actual_line + 1) as i32);
            window.set_cursor_column((actual_col + 1) as i32);

            // Update viewport to show cursor
            let (start_line, _end_line) = editor_bridge_ctrl_end.viewport_range();
            window.set_visible_start_line(start_line as i32);

            super::super::helpers::update_visible_lines(&window, &editor_bridge_ctrl_end);
        }
    });

    // Mouse click callback - convert pixels to line/column
    main_window.on_mouse_clicked(move |_x, _y| {
        // TODO: Implement mouse-based cursor positioning
        // Currently clicking just focuses the editor for keyboard input
    });

    // Find callback
    let window_weak = main_window.as_weak();
    main_window.on_find_requested(move |_search| {
        if let Some(_window) = window_weak.upgrade() {
            // TODO: Implement find functionality in EditorBridge
        }
    });

    // Find and replace callback
    let window_weak = main_window.as_weak();
    main_window.on_replace_requested(move |_search, _replace| {
        if let Some(_window) = window_weak.upgrade() {
            // TODO: Implement find/replace functionality in EditorBridge
        }
    });

    // Set up menu-view-gcode-editor callback
    let window_weak = main_window.as_weak();
    main_window.on_menu_view_gcode_editor(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_connection_status(slint::SharedString::from("G-Code Editor activated"));
            // Trigger focus on the editor by incrementing the trigger counter
            window.set_gcode_focus_trigger(window.get_gcode_focus_trigger() + 1);
        }
    });

    // Debug callback for key-pressed events from editor
    main_window.on_key_pressed_event(move |_msg| {});

    // Debug callback for editor clicked events
    main_window.on_editor_clicked(move || {});
}
