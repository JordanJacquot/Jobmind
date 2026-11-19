//! Le score « naturel » d'une lettre (étape 9), calculé ici, jamais par Claude : 100 au départ, des points en moins
//! pour chaque signe d'un texte d'IA, chacun simple à expliquer à Jordan :
//!
//! - une formule toute faite (règles de départ ou les siennes) : 8 points (56 au plus) ;
//! - un mot trop recherché : 4 points (20 au plus) ;
//! - une phrase plus longue que la longueur permise : 6 points (24 au plus) ;
//! - des phrases qui ont presque toutes la même longueur (au moins 5 phrases ; l'écart à la moyenne divisé par la
//!   moyenne sous 0,22 : 12 points, sous 0,32 : 6 points) ;
//! - un mot de 6 lettres ou plus qui revient 3 fois ou plus : 3 points (12 au plus) ;
//! - une liste de trois qualités (« rigoureux, ponctuel et motivé ») : 4 points (8 au plus) ;
//! - un tiret long : 4 points (8 au plus).
//!
//! La formule d'appel, la formule de politesse et la signature ne comptent pas. Les mots familiers ne baissent pas le
//! score (ils relèvent du plancher professionnel, montré à part).

use std::collections::HashMap;

use crate::regles::{Genre, Regles};
use crate::surlignage::{GenreMarque, marques};
use crate::texte::{est_appel, est_politesse, mots, nombre_de_mots, phrases};

/// Une raison du score : son code, la phrase pour Jordan, les points retirés.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raison {
    pub code: &'static str,
    pub texte: String,
    pub points: u32,
}

/// Le score naturel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Score {
    /// De 0 à 100
    pub note: u8,
    /// « Naturel », « Plutôt naturel », « Un peu IA », « Sonne comme une IA »
    pub mot: &'static str,
    /// Toutes les raisons, les plus lourdes d'abord
    pub raisons: Vec<Raison>,
}

impl Score {
    /// Les 3 raisons principales en mots simples (ou « rien ne sonne comme une IA »).
    pub fn principales(&self) -> Vec<String> {
        if self.raisons.is_empty() {
            return vec!["Rien ne sonne comme un texte d'IA.".into()];
        }
        self.raisons.iter().take(3).map(|r| r.texte.clone()).collect()
    }
}

/// Le mot qui va avec une note.
pub fn mot(note: u8) -> &'static str {
    match note {
        80.. => "Naturel",
        60..=79 => "Plutôt naturel",
        40..=59 => "Un peu IA",
        _ => "Sonne comme une IA",
    }
}

/// Les mots longs qui reviennent sans être un signe d'IA.
const MOTS_LIBRES: [&str; 12] = [
    "madame",
    "monsieur",
    "votre",
    "notre",
    "depuis",
    "pendant",
    "toujours",
    "plusieurs",
    "beaucoup",
    "candidat",
    "entretien",
    "aupres",
];

fn pluriel(n: usize, un: &str, plusieurs: &str) -> String {
    if n == 1 { un.to_string() } else { format!("{n} {plusieurs}") }
}

/// La phrase compte dans le score (pas l'appel, la politesse ou une signature de 1 à 3 mots).
pub fn phrase_du_corps(p: &str) -> bool {
    nombre_de_mots(p) > 3 && !est_appel(p) && !est_politesse(p)
}

/// Le score naturel d'un texte, avec ces règles.
pub fn score(t: &str, regles: &Regles) -> Score {
    let mut raisons: Vec<Raison> = Vec::new();
    let ms = marques(t, regles);
    let exemple = |g: GenreMarque| ms.iter().find(|m| m.genre == g).map(|m| t[m.debut..m.fin].to_string());
    // Les règles
    for (g, code, un, plusieurs, par, plafond) in [
        (GenreMarque::Ia, "ia", "Une formule toute faite", "formules toutes faites", 8, 56),
        (GenreMarque::Perso, "perso", "Une expression que tu veux éviter", "expressions que tu veux éviter", 8, 56),
        (GenreMarque::Pompeux, "pompeux", "Un mot trop recherché", "mots trop recherchés", 4, 20),
    ] {
        let n = ms.iter().filter(|m| m.genre == g).count();
        if n > 0 {
            let ex = exemple(g).map(|e| format!(", comme « {e} »")).unwrap_or_default();
            raisons.push(Raison {
                code,
                texte: format!("{}{ex}.", pluriel(n, un, plusieurs)),
                points: (n as u32 * par).min(plafond),
            });
        }
    }
    // Les phrases trop longues
    let longues = ms.iter().filter(|m| m.genre == GenreMarque::Longue).count();
    if longues > 0 {
        raisons.push(Raison {
            code: "longues",
            texte: format!(
                "{} (plus de {} mots).",
                pluriel(longues, "Une phrase trop longue", "phrases trop longues"),
                regles.phrase_max
            ),
            points: (longues as u32 * 6).min(24),
        });
    }
    // Des phrases toutes de la même longueur
    let longueurs: Vec<f64> = phrases(t)
        .into_iter()
        .map(|(d, f)| &t[d..f])
        .filter(|p| phrase_du_corps(p))
        .map(|p| nombre_de_mots(p) as f64)
        .collect();
    if longueurs.len() >= 5 {
        let moyenne = longueurs.iter().sum::<f64>() / longueurs.len() as f64;
        let ecart = (longueurs.iter().map(|x| (x - moyenne).powi(2)).sum::<f64>() / longueurs.len() as f64).sqrt();
        let v = if moyenne > 0.0 { ecart / moyenne } else { 1.0 };
        let points = if v < 0.22 {
            12
        } else if v < 0.32 {
            6
        } else {
            0
        };
        if points > 0 {
            raisons.push(Raison {
                code: "egales",
                texte: "Les phrases ont presque toutes la même longueur.".into(),
                points,
            });
        }
    }
    // Les répétitions (dans le corps de la lettre)
    let corps: String =
        phrases(t).into_iter().map(|(d, f)| &t[d..f]).filter(|p| phrase_du_corps(p)).collect::<Vec<_>>().join(" ");
    let mut comptes: HashMap<String, (usize, String)> = HashMap::new();
    for m in mots(&corps) {
        if m.norme.chars().count() < 6 || MOTS_LIBRES.contains(&m.norme.as_str()) {
            continue;
        }
        let cle: String = m.norme.chars().take(6).collect();
        let e = comptes.entry(cle).or_insert((0, corps[m.debut..m.fin].to_lowercase()));
        e.0 += 1;
    }
    let mut repetes: Vec<(usize, String)> = comptes.into_values().filter(|(n, _)| *n >= 3).collect();
    repetes.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    if let Some((n, m)) = repetes.first() {
        raisons.push(Raison {
            code: "repetitions",
            texte: if repetes.len() == 1 {
                format!("« {m} » revient {n} fois.")
            } else {
                format!("Des mots reviennent souvent, comme « {m} » ({n} fois).")
            },
            points: (repetes.len() as u32 * 3).min(12),
        });
    }
    // Les listes de trois qualités
    let trois = ms.iter().filter(|m| m.genre == GenreMarque::Trois).count();
    if trois > 0 {
        raisons.push(Raison {
            code: "trois",
            texte: format!(
                "{}, un tic des textes d'IA.",
                pluriel(trois, "Une liste de trois qualités", "listes de trois qualités")
            ),
            points: (trois as u32 * 4).min(8),
        });
    }
    // Les tirets longs
    let tirets = t.matches(['\u{2014}', '\u{2013}']).count();
    if tirets > 0 {
        raisons.push(Raison {
            code: "tirets",
            texte: "Des tirets longs, fréquents dans les textes d'IA.".into(),
            points: (tirets as u32 * 4).min(8),
        });
    }
    raisons.sort_by_key(|r| std::cmp::Reverse(r.points));
    let total: u32 = raisons.iter().map(|r| r.points).sum();
    let note = 100u32.saturating_sub(total) as u8;
    Score { note, mot: mot(note), raisons }
}

/// Le genre de marque qui va avec une règle.
pub fn genre_de(g: Genre) -> GenreMarque {
    match g {
        Genre::Ia => GenreMarque::Ia,
        Genre::Pompeux => GenreMarque::Pompeux,
        Genre::Perso => GenreMarque::Perso,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IA: &str = "Madame, Monsieur,\n\nC'est avec un grand intérêt que je vous adresse ma candidature pour le poste \
                      de préparateur de commandes au sein de votre entreprise. Fort de mon expérience dans la logistique, \
                      je suis convaincu que mon profil constitue un véritable atout pour votre équipe dynamique. \
                      Rigoureux, ponctuel et motivé, je saurai relever de nouveaux défis avec enthousiasme et efficacité. \
                      Mon expérience en entrepôt m'a permis de développer une solide expérience de la préparation. \
                      Je suis convaincu que cette expérience sera une valeur ajoutée pour votre entreprise.\n\nJe vous prie \
                      d'agréer, Madame, Monsieur, l'expression de mes salutations distinguées.";

    const HUMAIN: &str = "Madame, Monsieur,\n\nJ'ai vu votre annonce pour un poste de préparateur de commandes à Woippy. \
                          Je l'ai fait deux ans en intérim. Les commandes, le scanner, le filmage des palettes : je connais. \
                          J'ai aussi le CACES 1 et 3, à jour. Je peux commencer dès lundi, en équipe du matin ou de l'après-midi.\
                          \n\nJe vous prie d'agréer, Madame, Monsieur, mes salutations distinguées.\nJordan Exemple";

    #[test]
    fn une_lettre_d_ia_et_une_lettre_humaine() {
        let r = Regles::default();
        let ia = score(IA, &r);
        let h = score(HUMAIN, &r);
        assert!(ia.note < 40, "{ia:?}");
        assert!(h.note >= 80, "{h:?}");
        assert_eq!(ia.mot, "Sonne comme une IA");
        assert!(h.raisons.iter().all(|x| x.code == "egales"), "{h:?}");
        assert_eq!(score("Je viens.", &r).principales(), ["Rien ne sonne comme un texte d'IA."]);
        let p = ia.principales();
        assert_eq!(p.len(), 3);
        assert!(p[0].contains("formules toutes faites"), "{p:?}");
        assert!(ia.raisons.iter().any(|x| x.code == "trois"));
        assert!(ia.raisons.iter().any(|x| x.code == "repetitions"), "{:?}", ia.raisons);
        for x in &ia.raisons {
            assert!(!x.texte.contains(['\u{2013}', '\u{2014}']));
        }
    }

    #[test]
    fn longues_et_egales() {
        let r = Regles::construire(&[], &[], 12);
        let t = "Je conduis des chariots depuis des années dans des entrepôts de la région de Metz avec soin et calme. \
                 Ma phrase est courte.";
        let s = score(t, &r);
        assert!(s.raisons.iter().any(|x| x.code == "longues" && x.texte.contains("plus de 12 mots")));
        let egales = "Je range les palettes. Je charge les camions. Je compte les colis. Je lis les bons. Je scanne les \
                      codes.";
        assert!(score(egales, &Regles::default()).raisons.iter().any(|x| x.code == "egales"));
        assert_eq!(score("", &Regles::default()).note, 100);
        assert_eq!(mot(80), "Naturel");
        assert_eq!(mot(79), "Plutôt naturel");
        assert_eq!(mot(40), "Un peu IA");
    }
}
