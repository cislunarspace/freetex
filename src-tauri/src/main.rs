// Prevents additional console window on Windows in release, DO NOT DELETE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    freetex_lib::run()
}
