// Release builds use the windows subsystem so launching the app does not
// open a console window; debug builds keep the console for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    nod_desktop_lib::run()
}
