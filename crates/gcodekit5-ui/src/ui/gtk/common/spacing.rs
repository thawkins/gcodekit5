//! # Shared Spacing and Margin Constants
//!
//! Standardized spacing values for consistent UI layout across the application.
//!
//! Usage:
//! ```rust
//! use crate::ui::gtk::common::spacing;
//! widget.set_margin_top(spacing::MEDIUM);
//! box_layout.set_spacing(spacing::SMALL);
//! ```

/// Extra small spacing (2px) - for tight packing
pub const EXTRA_SMALL: i32 = 2;

/// Small spacing (4px) - for minimal padding
pub const SMALL: i32 = 4;

/// Default/medium spacing (8px) - standard widget padding
pub const MEDIUM: i32 = 8;

/// Large spacing (12px) - for section separation
pub const LARGE: i32 = 12;

/// Extra large spacing (16px) - for major section gaps
pub const EXTRA_LARGE: i32 = 16;

/// Huge spacing (24px) - for page-level margins
pub const HUGE: i32 = 24;

/// Button internal spacing (6px) - consistent button content spacing
pub const BUTTON_INTERNAL: i32 = 6;

/// Icon to label spacing (6px) - standard icon+text button spacing
pub const ICON_TO_LABEL: i32 = 6;

/// Form row spacing (8px) - spacing between form rows
pub const FORM_ROW: i32 = 8;

/// Panel padding (12px) - standard panel content padding
pub const PANEL: i32 = 12;

/// Dialog content margin (24px) - standard dialog padding
pub const DIALOG_CONTENT: i32 = 24;

/// Toolbar spacing (6px) - spacing in toolbars
pub const TOOLBAR: i32 = 6;

/// List item spacing (4px) - spacing between list items
pub const LIST_ITEM: i32 = 4;

/// Card padding (16px) - padding inside card containers
pub const CARD: i32 = 16;

/// Header spacing (12px) - spacing for headers
pub const HEADER: i32 = 12;
