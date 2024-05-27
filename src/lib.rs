#![warn(
    anonymous_parameters,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

//! This is a basic implementation of a task which can be used but with Standard (Linux,Windows, MacOs) and Wasm (Web).
//! A task means here a function which can be run in the background (Standard:Thread, Wasm: WebWorker).
mod task;
#[cfg(target_arch = "wasm32")]
pub use task::{gloo_worker, WebWorker};
pub use task::{Function, JobState, Task, TaskPool, Ticket};

mod ongoing_task;
#[cfg(target_arch = "wasm32")]
pub use ongoing_task::WebWorkerBackground;
pub use ongoing_task::{BackgroundFunction, BackgroundTask, StateProgress, StateTrait};
