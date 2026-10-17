//! The plugin as a standalone app (no host): `alea_standalone --help`.
//! `ALEA_WORK=/path/to/work.alea` loads a work on start.

fn main() {
    nih_plug::nih_export_standalone::<alea_plugin::Alea>();
}
