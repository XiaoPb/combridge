// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    combridge_rust_lib::run()
}

pub mod commands;
pub mod device;
pub mod error;
pub mod protocol;
pub mod serial;
pub mod state;
pub mod system;
pub mod websocket;

pub use commands::
    ble, protocol, serial, state, system, websocket,
;
