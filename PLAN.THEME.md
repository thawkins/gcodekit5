# Plan: Switchable Themes (Dark/Light/System)

This plan outlines the steps to implement switchable themes in the GCodeKit5 application. The goal is to support "Dark", "Light", and "System" (OS default) themes, selectable via the existing User Preferences.

## Strategy

The implementation will rely on GTK's native theming capabilities and named colors. Instead of maintaining separate CSS files for each theme, we will refactor the existing `style.css` to use GTK named colors (e.g., `@theme_bg_color`, `@window_fg_color`). This allows the application to automatically adapt its appearance when the GTK theme variant is switched between Dark and Light.

The specific "Theme" preference (Dark/Light/System) will control the `gtk-application-prefer-dark-theme` property of the application settings at startup.

## Tasks

### 1. Persistence & Configuration Loading
- [ ] **Modify `crates/gcodekit5-ui/src/gtk_app.rs` to load configuration at startup.**
    - Currently, the application starts with default settings. It must load the user's persistent configuration file (`config.json`) during the startup phase.
    - Use `gcodekit5_settings::manager::SettingsManager` to locate the config file.
    - Use `gcodekit5_settings::persistence::SettingsPersistence::load_from_file` to attempt loading the config.
    - Fallback to defaults if the file does not exist.

### 2. Theme Switching Logic
- [ ] **Implement Theme Application Logic in `gtk_app.rs`.**
    - Create a helper function (or inline logic) typically within the `startup` or `activate` phase.
    - Read the `theme` string from the loaded `UiSettings` (`"Dark"`, `"Light"`, or `"System"`).
    - Access the global GTK settings: `gtk::Settings::default()`.
    - Apply the logic:
        - **"Dark"**: Set `gtk-application-prefer-dark-theme` to `true`.
        - **"Light"**: Set `gtk-application-prefer-dark-theme` to `false`.
        - **"System"**: Ideally, adhere to the system default. In pure GTK4 setups without LibAdwaita, setting `gtk-application-prefer-dark-theme` to `false` typically defaults to the system's "Light" variation or the default GTK theme, while `true` forces the "Dark" variant. Determine if explicit "System" detection is needed or if leaving the property unset (if possible) works. *Fall back to "Dark" as default if System detection is complex in v1.*
    - *Note: Since restart is acceptable for v1, applying this only at startup is sufficient.*

### 3. Refactor CSS for Theme Compatibility
- [ ] **Refactor `crates/gcodekit5-ui/src/ui/gtk/style.css`.**
    - Replace hardcoded hex colors with GTK named colors to ensure the UI adapts to the selected theme variant.
    - **Mappings to replace:**
        - Backgrounds (`#1e1e1e`, `#2d2d2d`, etc.) -> `@theme_bg_color`, `@window_bg_color`, or `@view_bg_color`.
        - Foregrounds/Text (`#ffffff`, `white`) -> `@theme_fg_color`, `@window_fg_color`.
        - Borders -> `@borders`.
        - Selected/Active states -> `@theme_selected_bg_color`, `@theme_selected_fg_color`.
    - **Specific Components:**
        - `visualizer-container`, `designer-toolbox`, `status-bar`: Ensure they use theme-aware colors.
        - `designer-canvas`: Decide if this should remain a constant color (e.g., light gray paper) or invert. If it represents a physical workspace, a constant light color might be preferred, or a specific `@canvas_color`.
    - **Custom Colors:**
        - For semantic colors like Success/Error (`#2ecc71`, `#e74c3c`), consider using variables or ensuring they look good on both light and dark backgrounds.
        - Define CSS variables for any hardcoded colors that don't map neatly to GTK defaults, and set their values based on the active theme if necessary (though named colors are preferred).

### 4. Verification
- [ ] **Verify "Dark" Mode**: App looks as it currently does (or very close).
- [ ] **Verify "Light" Mode**: App background becomes light, text becomes dark chosen from the GTK light theme palette.
- [ ] **Verify "System" Mode**: App matches the OS theme setting (if OS is set to Light > App is Light).
- [ ] **Verify Persistence**: Changing the setting in Preferences -> restarting the app -> preserves the choice.

## Future Improvements (v2)
- Implement runtime theme switching without restart (listening to config changes).
- Integrate LibAdwaita for better System theme detection and mobile-friendly controls.
