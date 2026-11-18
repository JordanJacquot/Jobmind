//! Jobmind en Rust : le programme Mindset.exe du dossier de Jobmind. Tout est dans la bibliothèque
//! (src\programme.rs) : le démarrage, la fenêtre « Jobmind » dans Mindset, sa vie et sa fin.

// Pas de console : c'est une appli à fenêtre (même en construction de mise au point, aucune console ne s'ouvre).
#![windows_subsystem = "windows"]
#![deny(unsafe_code)]

fn main() {
    jobmind_app::programme::lancer()
}
