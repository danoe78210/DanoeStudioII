// Empêche l'ouverture d'une console supplémentaire en build release (Windows).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    danoe_studio_lib::run()
}
