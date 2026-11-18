//! Lit un fichier comme Jobmind lit un CV, et montre ce qui en sort (les mesures sur des fichiers jamais vus) :
//!
//!   cargo run -p jobmind-lecture --example lire -- <fichier> [<fichier>...]
//!
//! Pour chaque fichier : le format, le temps, le nombre de caractères, puis le texte remis au propre (ou la phrase
//! d'erreur que Jobmind montrerait).

fn main() {
    for chemin in std::env::args().skip(1) {
        println!("=== {chemin}");
        let octets = match std::fs::read(&chemin) {
            Ok(o) => o,
            Err(e) => {
                println!("illisible : {e}");
                continue;
            }
        };
        let debut = std::time::Instant::now();
        let lu = jobmind_lecture::lire(&octets);
        let duree = debut.elapsed();
        match lu {
            Ok(l) => {
                let propre = jobmind_core::profil::nettoyer_texte(&l.texte);
                println!(
                    "format {}, {} octets, lu en {} ms, {} caractères bruts, {} remis au propre",
                    l.format.nom(),
                    octets.len(),
                    duree.as_millis(),
                    l.texte.chars().count(),
                    propre.chars().count()
                );
                println!("{propre}");
                if l.format == jobmind_lecture::Format::Pdf
                    && let Ok((_, e)) = jobmind_lecture::pdf::texte_mesure(&octets, jobmind_lecture::pdf::SEUIL_ESPACE)
                {
                    // (les écarts entre morceaux de texte, en part de la taille des lettres)
                    let n = |a: f64, b: f64| e.iter().filter(|x| **x > a && **x <= b).count();
                    println!(
                        "écarts : {} morceaux ; 0 à 0,1 : {} ; 0,1 à 0,18 : {} ; 0,18 à 0,25 : {} ; 0,25 à 0,4 : {} ; plus : {}",
                        e.len(),
                        n(-0.01, 0.1),
                        n(0.1, 0.18),
                        n(0.18, 0.25),
                        n(0.25, 0.4),
                        n(0.4, f64::MAX)
                    );
                }
            }
            Err(e) => println!("refusé en {} ms : {} | {}", duree.as_millis(), e.raison(), e.message()),
        }
    }
}
