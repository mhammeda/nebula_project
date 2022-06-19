//! Handlers for federated routes

pub mod client;
mod communities;
mod other;
mod posts;
mod users;

pub use {communities::*, other::*, posts::*, users::*};
