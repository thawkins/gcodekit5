// Export a compatibility shim for the types that used to live in `src/app/types.rs`.
// The actual types have been moved into crate-local `types.rs` modules for
// `gcodekit5-ui` (UI param models) and top-level `src/types.rs` for application state.
pub use crate::types::GcodeSendState;
pub use gcodekit5_ui::types::{VectorEngravingParams, BitmapEngravingParams, TabbedBoxParams, JigsawPuzzleParams};

