//! Placeholder APIs not wired into the TCP proxy yet (breakpoints, session state, UI).
//!
//! These live under one module so the crate root stays focused on the shipped proxy, recorder,
//! SVD, and RTOS paths. Submodules remain **`pub`** and are re-exported at the crate root for
//! stability (`rsgdb::breakpoints`, `rsgdb::state`, `rsgdb::ui`).

pub mod breakpoints;
pub mod state;
pub mod ui;
