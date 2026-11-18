//! Ce que la page de Jobmind montre : l'état du kit d'écrans de la suite (format en tête de ui\kit.js).
//!
//! L'accueil est celui de la maquette C (maquettes\proposition-c.html, « les dossiers ») : trois onglets,
//! Candidatures, Offres et Mon profil, chacun avec une liste fine à gauche et le dossier ouvert à droite. Il est
//! dessiné par ui\jobmind.js à la place des tuiles du kit (bloc `accueil.perso`, voir « perso » en tête de kit.js) ;
//! le kit garde l'en-tête, le bas, le clavier et la manette, et les sons.
//!
//! Étape 1 : rien n'est encore rempli, chaque onglet montre son état vide (« Aucune candidature pour l'instant. »).
//! Mon profil montre déjà ses quatre rubriques (Mon CV, Mon style, Ce que je cherche, Réglages), vides elles aussi.
//!
//! Tous les textes vont à la page tels quels (elle les échappe) : en français simple, sans tiret long ni demi-cadratin.

use serde_json::{Value, json};

/// Les onglets, dans l'ordre.
pub const CANDIDATURES: &str = "candidatures";
pub const OFFRES: &str = "offres";
pub const PROFIL: &str = "profil";
pub const ONGLETS: [&str; 3] = [CANDIDATURES, OFFRES, PROFIL];

/// La phrase de l'onglet Candidatures, tant qu'il est vide.
pub const AUCUNE_CANDIDATURE: &str = "Aucune candidature pour l'instant.";
/// La phrase de l'onglet Offres, tant qu'il est vide.
pub const AUCUNE_OFFRE: &str = "Aucune offre pour l'instant.";

/// Un dossier vide (le grand panneau de droite) : son icône, sa phrase, ce qui viendra, et une note.
fn dossier_vide(icone: &str, titre: &str, texte: &str, note: &str) -> Value {
    json!({ "icone": icone, "titre": titre, "texte": texte, "note": note })
}

/// Une rubrique de Mon profil : une ligne de la liste et son dossier.
fn rubrique(id: &str, nom: &str, icone: &str, titre: &str, texte: &str) -> Value {
    json!({
        "id": id,
        "nom": nom,
        "sous": "Pas encore rempli",
        "icone": icone,
        "dossier": dossier_vide(icone, titre, texte, ""),
    })
}

/// L'accueil de Jobmind (bloc `accueil.perso`) : les trois onglets et ce que chacun montre.
pub fn accueil() -> Value {
    json!({
        "onglets": [
            {
                "id": CANDIDATURES,
                "nom": "Candidatures",
                "nombre": 0,
                "liste": "Mes candidatures",
                "vide": "Rien à suivre pour l'instant.",
                "rubriques": [],
                "dossier": dossier_vide(
                    "dossier",
                    AUCUNE_CANDIDATURE,
                    "Chaque candidature aura ici son dossier : ce que tu as envoyé, l'offre, les réponses reçues et \
                     la prochaine chose à faire.",
                    "Tout reste sur ce PC.",
                ),
            },
            {
                "id": OFFRES,
                "nom": "Offres",
                "nombre": 0,
                "liste": "Les offres",
                "vide": "Aucune offre gardée.",
                "rubriques": [],
                "dossier": dossier_vide(
                    "loupe",
                    AUCUNE_OFFRE,
                    "Jobmind rassemblera ici les offres de plusieurs sites, sans doublons, triées pour garder les \
                     plus intéressantes pour toi.",
                    "Chaque offre gardée reste lisible sur ce PC, même si elle disparaît du site.",
                ),
            },
            {
                "id": PROFIL,
                "nom": "Mon profil",
                "liste": "Mon profil",
                "vide": "",
                "rubriques": [
                    rubrique(
                        "cv",
                        "Mon CV",
                        "cv",
                        "Aucun CV pour l'instant.",
                        "Ton CV servira à préparer chaque candidature. Tu pourras le donner en PDF ou en Word, ou \
                         le coller.",
                    ),
                    rubrique(
                        "style",
                        "Mon style",
                        "plume",
                        "Aucun texte de toi pour l'instant.",
                        "Quelques textes écrits par toi apprendront à Jobmind ta façon d'écrire. Tes candidatures \
                         resteront toujours professionnelles.",
                    ),
                    rubrique(
                        "recherche",
                        "Ce que je cherche",
                        "cible",
                        "Rien de noté pour l'instant.",
                        "Les métiers, le lieu et les contrats que tu cherches serviront à trier les offres.",
                    ),
                    rubrique(
                        "reglages",
                        "Réglages",
                        "reglages",
                        "Rien à régler pour l'instant.",
                        "Les clés des sites d'offres et de l'IA, ta boîte Gmail et la sauvegarde viendront ici.",
                    ),
                ],
                "dossier": dossier_vide("profil", "Mon profil", "", ""),
            },
        ],
    })
}

/// L'état de la page : le nom, la version et le logo de l'appli, et l'accueil (les trois onglets). Le thème, les
/// couleurs d'un pack et ses images s'ajoutent par l'habillage ([`crate::Page::habiller`]).
pub fn etat(version: &str) -> Value {
    json!({
        "app": { "nom": crate::NOM_APPLI, "version": version, "logo": "logo.svg" },
        "accueil": {
            // (aucune tuile : les onglets de la maquette C prennent toute la place)
            "tuiles": [],
            "ronds": [],
            "perso": accueil(),
        },
        "pages": {},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tous les textes d'une valeur (récursivement).
    fn textes(v: &Value, l: &mut Vec<String>) {
        match v {
            Value::String(s) => l.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| textes(x, l)),
            Value::Object(o) => o.values().for_each(|x| textes(x, l)),
            _ => {}
        }
    }

    #[test]
    fn trois_onglets_vides() {
        let e = etat("0.1.0");
        assert_eq!(e["app"], json!({ "nom": "Jobmind", "version": "0.1.0", "logo": "logo.svg" }));
        assert_eq!(e["accueil"]["tuiles"], json!([]));
        let o = e["accueil"]["perso"]["onglets"].as_array().unwrap();
        let ids: Vec<&str> = o.iter().map(|x| x["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ONGLETS);
        let noms: Vec<&str> = o.iter().map(|x| x["nom"].as_str().unwrap()).collect();
        assert_eq!(noms, ["Candidatures", "Offres", "Mon profil"]);
        assert_eq!(o[0]["dossier"]["titre"], AUCUNE_CANDIDATURE);
        assert_eq!(o[1]["dossier"]["titre"], AUCUNE_OFFRE);
        assert_eq!(o[0]["nombre"], 0);
        assert!(o[0]["rubriques"].as_array().unwrap().is_empty());
        // Mon profil : ses quatre rubriques, chacune avec son dossier vide
        let r = o[2]["rubriques"].as_array().unwrap();
        let noms: Vec<&str> = r.iter().map(|x| x["nom"].as_str().unwrap()).collect();
        assert_eq!(noms, ["Mon CV", "Mon style", "Ce que je cherche", "Réglages"]);
        for x in r {
            assert!(x["dossier"]["titre"].as_str().unwrap().ends_with("pour l'instant."), "{x}");
        }
        assert!(e["pages"].as_object().unwrap().is_empty());
    }

    /// Règle de la suite : aucun tiret long ni demi-cadratin, aucun HTML (la page écrit les textes tels quels), des
    /// ids que la page accepte.
    #[test]
    fn textes_simples() {
        let mut l = Vec::new();
        textes(&etat("0.1.0"), &mut l);
        for t in l {
            assert!(!t.contains(['\u{2013}', '\u{2014}', '<', '>']), "{t}");
            assert!(!t.contains("  "), "{t}");
        }
        let e = etat("0.1.0");
        for o in e["accueil"]["perso"]["onglets"].as_array().unwrap() {
            for x in std::iter::once(o).chain(o["rubriques"].as_array().unwrap()) {
                let id = x["id"].as_str().unwrap();
                assert!(!id.is_empty() && id.bytes().all(|c| c.is_ascii_lowercase()), "{id}");
            }
        }
    }

    /// Le logo de l'état est bien livré avec les pages (ui\), et la page charge l'accueil de Jobmind ; chaque icône
    /// envoyée est connue de la page.
    #[test]
    fn le_logo_et_l_accueil_existent() {
        let ui = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui");
        let e = etat("0.1.0");
        assert!(ui.join(e["app"]["logo"].as_str().unwrap()).is_file());
        let index = std::fs::read_to_string(ui.join("index.html")).unwrap();
        for f in ["jobmind.css", "jobmind.js"] {
            assert!(ui.join(f).is_file(), "{f}");
            assert!(index.contains(f), "index.html ne charge pas {f}");
        }
        let js = std::fs::read_to_string(ui.join("jobmind.js")).unwrap();
        let mut icones = Vec::new();
        for o in e["accueil"]["perso"]["onglets"].as_array().unwrap() {
            icones.push(o["dossier"]["icone"].as_str().unwrap().to_string());
            for r in o["rubriques"].as_array().unwrap() {
                icones.push(r["icone"].as_str().unwrap().to_string());
            }
        }
        for i in icones {
            assert!(js.contains(&format!("  {i}: ")), "icône {i} inconnue de jobmind.js");
        }
        // L'état entier reste léger (envoyé à chaque changement)
        assert!(e.to_string().len() < 20_000);
    }
}
