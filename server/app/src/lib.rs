#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Application entry point. Wires config, the simulation, networking, and the
//! shutdown path into the first runnable vertical slice: a client connects, logs
//! in offline, enters play, and receives a `JoinGame`, a position sync, and the
//! flat-world spawn chunks.
//!
//! - [`AppConfig`] is the minimal, validated server configuration.
//! - [`run`] builds the world, starts the simulation, binds the listener, and
//!   begins accepting connections, returning a [`RunningServer`].
//! - [`build_command_tree`] exposes the `/spawn` + `/gamemode` command set for
//!   direct server-side tests.

mod bucket_completion;
mod bucket_raycast;
mod chunk_preparation;
mod cli;
mod combat;
mod combat_completion;
mod combat_wear;
mod command;
mod config;
mod connection;
mod consumption;
mod death_inventory;
mod departure_vitals;
mod driver;
mod furnace_window;
mod gameplay;
mod ground_persistence;
mod ground_runtime;
mod ground_view;
mod held_drop;
mod inventory;
mod inventory_budget;
mod java_bridge;
mod light_view;
mod lighting;
mod mining_wear;
mod mining_witness;
mod mob_command;
mod mob_runtime_app;
mod mob_view;
mod observe;
mod operators;
mod persistence;
mod placement_completion;
mod player_admission;
mod player_data;
mod plugins;
mod registries;
mod render_descriptors;
mod server;
mod storage_worker;
mod window;
mod world;

pub use cli::{load_or_init_config, Cli};
pub use command::build_command_tree;
pub use config::AppConfig;
pub use server::{run, RunningServer};
