//! Les pages des sites que Jobmind n'arrive pas à lire, gardées sur ce PC pour corriger la lecture (0.1.6, accord de
//! Jordan le 21/11/2026 : Indeed et LinkedIn avaient changé, plus aucune offre lue). Une copie par site et par sorte
//! de page (offre, liste), remplacée à chaque fois, dans `<données>\pages-illisibles\` ; rien n'est envoyé nulle part.
//! Seulement LinkedIn, Indeed, Welcome to the Jungle et France Travail ; jamais une page de connexion ou de
//! vérification. À retirer une fois la lecture corrigée.

use std::path::{Path, PathBuf};

use jobmind_sources::page::{PageRefusee, Site};
use jobmind_sources::sites::{Comprise, Ecran};

/// Le dossier des copies, dans les données de Jobmind.
pub const DOSSIER: &str = "pages-illisibles";
/// Une copie au plus de cette taille (une page plus grosse est coupée).
const TAILLE_MAX: usize = 4_000_000;

/// Le nom de la copie d'une page mal lue : « indeed-offre.html », « linkedin-liste.html »... Rien pour une page bien
/// lue, d'un autre site, ou qui n'est ni une offre ni une liste de résultats.
pub fn nom_copie(c: &Comprise) -> Option<String> {
    if c.site == Site::Autre || c.ecran != Ecran::Normale {
        return None;
    }
    let sorte = if matches!(c.offre, Err(PageRefusee::PasUneOffre(_))) && c.liste.resultats.is_empty() {
        // (une offre que les règles ne lisent pas : sa page a un titre, mais ni données ni blocs reconnus)
        if !c.trace.contains("pas_une_offre") && !adresse_d_offre(&c.adresse) {
            return None;
        }
        "offre"
    } else if c.liste.lu_par == "liens" {
        // (une liste lue seulement par ses liens : les blocs du site ne sont plus reconnus)
        "liste"
    } else {
        return None;
    };
    Some(format!("{}-{sorte}.html", c.site.code()))
}

/// L'adresse d'une offre sur un site connu.
fn adresse_d_offre(u: &str) -> bool {
    ["/jobs/view/", "viewjob", "/offres/recherche/detail/", "/companies/"].iter().any(|m| u.contains(m))
}

/// Garde la copie d'une page mal lue (son adresse en tête, en commentaire). Rend son chemin, rien sinon.
pub fn garder(donnees: &Path, c: &Comprise, html: &str) -> Option<PathBuf> {
    let nom = nom_copie(c)?;
    let dossier = donnees.join(DOSSIER);
    std::fs::create_dir_all(&dossier).ok()?;
    let mut fin = html.len().min(TAILLE_MAX);
    while !html.is_char_boundary(fin) {
        fin -= 1;
    }
    let adresse = c.adresse.replace("--", "- -");
    let texte = format!("<!-- Jobmind : page illisible, {adresse} -->\n{}", &html[..fin]);
    let chemin = dossier.join(nom);
    std::fs::write(&chemin, texte).ok()?;
    Some(chemin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobmind_sources::sites::comprendre;

    fn dossier(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jobmind-illisibles-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn copie_des_pages_mal_lues_seulement() {
        let d = dossier("copie");
        // Une offre LinkedIn d'une forme inconnue : gardée, remplacée la fois suivante
        let u = "https://www.linkedin.com/jobs/view/4485721882/";
        let inconnue = "<html><head><title>Poste | Exemple | LinkedIn</title></head><body><p>Rien</p></body></html>";
        let c = comprendre(u, inconnue);
        assert_eq!(nom_copie(&c).as_deref(), Some("linkedin-offre.html"), "{}", c.trace);
        let chemin = garder(&d, &c, inconnue).unwrap();
        assert_eq!(chemin, d.join(DOSSIER).join("linkedin-offre.html"));
        let lu = std::fs::read_to_string(&chemin).unwrap();
        assert!(lu.starts_with("<!-- Jobmind : page illisible, https://www.linkedin.com/jobs/view/4485721882/ -->"));
        assert!(lu.ends_with(inconnue));
        garder(&d, &c, "<html><body>autre</body></html>").unwrap();
        assert!(std::fs::read_to_string(&chemin).unwrap().ends_with("<html><body>autre</body></html>"));
        // Une page d'un autre site : rien
        let c = comprendre("https://www.exemple.fr/offre/1", inconnue);
        assert_eq!(nom_copie(&c), None);
        assert!(garder(&d, &c, inconnue).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
