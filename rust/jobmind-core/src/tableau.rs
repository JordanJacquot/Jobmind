//! Le tableau de bord (étape 12 du plan, cadrage point 4) : les envois de la semaine et du mois, les réponses reçues et
//! le taux de réponse, les entretiens, les relances à faire, les offres nouvelles par source, les envois des huit
//! dernières semaines et la part de chaque statut. Tout est calculé ici, d'après les candidatures, les offres gardées,
//! le jour qu'il est et le délai de relance ; l'écran ne fait que dessiner.

use crate::candidature::Candidature;
use crate::jour::Jour;
use crate::offre::Offre;
use crate::statut::Statut;
use crate::suivi::{Action, situation};

/// Le nombre de semaines des barres des envois.
pub const SEMAINES: usize = 8;
/// Les offres « nouvelles » : ajoutées depuis ce nombre de jours (aujourd'hui compris).
pub const JOURS_NOUVELLES: i64 = 7;
/// Le nom des sources inconnues.
pub const AUTRES: &str = "Autres sites";

/// Une barre : son texte (« 9 nov. », « Cette semaine »), son nombre et sa hauteur (0 à 100, la plus haute à 100).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Barre {
    pub texte: String,
    pub nombre: usize,
    pub part: u32,
}

/// Le tableau de bord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tableau {
    /// Les candidatures envoyées cette semaine (depuis lundi) et ce mois-ci
    pub semaine: usize,
    pub mois: usize,
    /// Toutes les candidatures
    pub total: usize,
    /// Celles qui comptent pour le taux (toutes sauf les abandonnées) et celles qui ont eu une réponse
    pub comptees: usize,
    pub reponses: usize,
    /// Le taux de réponse en pour cent (rien sans candidature comptée)
    pub taux: Option<u32>,
    /// Les candidatures qui ont eu un entretien, et les entretiens encore à venir
    pub entretiens: usize,
    pub entretiens_a_venir: usize,
    /// Les relances à faire (et les candidatures à classer après une relance restée sans réponse)
    pub relances: usize,
    /// Les offres ajoutées ces 7 derniers jours (cachées exclues), et leur nombre par source (le plus grand d'abord)
    pub offres_nouvelles: usize,
    pub par_source: Vec<(String, usize)>,
    /// Les envois des huit dernières semaines, la plus ancienne d'abord
    pub barres: Vec<Barre>,
    /// Le nombre de candidatures à chaque statut (seulement les statuts présents, dans l'ordre de l'écran)
    pub statuts: Vec<(Statut, usize)>,
}

/// A eu une réponse (entretien, réponse positive ou négative).
fn repondue(s: Statut) -> bool {
    matches!(s, Statut::Entretien | Statut::Positive | Statut::Negative)
}

/// Le nom d'une source d'offres (« adzuna » : « Adzuna »).
pub fn nom_source(code: &str) -> &'static str {
    match code {
        "adzuna" => "Adzuna",
        "linkedin" => "LinkedIn",
        "indeed" => "Indeed",
        "wttj" => "Welcome to the Jungle",
        "francetravail" => "France Travail",
        _ => AUTRES,
    }
}

/// Le lundi de la semaine de ce jour.
pub fn lundi(j: Jour) -> Jour {
    j.plus(-(j.semaine() as i64))
}

/// Le tableau de bord d'aujourd'hui.
pub fn calculer(liste: &[Candidature], offres: &[Offre], aujourdhui: Jour, delai: u32) -> Tableau {
    let debut_semaine = lundi(aujourdhui);
    let comptees: Vec<&Candidature> = liste.iter().filter(|c| c.statut != Statut::Abandonnee).collect();
    let reponses = comptees.iter().filter(|c| repondue(c.statut)).count();
    let taux = (!comptees.is_empty()).then(|| ((reponses * 100 + comptees.len() / 2) / comptees.len()) as u32);
    let relances = liste
        .iter()
        .filter(|c| {
            matches!(
                situation(c, aujourdhui, delai).action,
                Some(Action::Relancer { .. }) | Some(Action::ClasserSansReponse { .. })
            )
        })
        .count();
    let entretiens = liste.iter().filter(|c| c.statut == Statut::Entretien || c.entretien.is_some()).count();
    let entretiens_a_venir = liste
        .iter()
        .filter(|c| c.statut == Statut::Entretien && c.entretien.as_ref().is_some_and(|e| e.jour >= aujourdhui))
        .count();
    // Les offres nouvelles, par source
    let depuis = aujourdhui.plus(1 - JOURS_NOUVELLES);
    let mut par_source: Vec<(String, usize)> = Vec::new();
    let mut offres_nouvelles = 0;
    for o in offres.iter().filter(|o| !o.cachee && o.ajoutee_le >= depuis && o.ajoutee_le <= aujourdhui) {
        offres_nouvelles += 1;
        let nom = nom_source(o.sources.first().map_or("", String::as_str));
        match par_source.iter_mut().find(|(n, _)| n == nom) {
            Some((_, k)) => *k += 1,
            None => par_source.push((nom.to_string(), 1)),
        }
    }
    // (le plus grand d'abord ; « Autres sites » toujours en dernier)
    par_source.sort_by(|a, b| (a.0 == AUTRES).cmp(&(b.0 == AUTRES)).then(b.1.cmp(&a.1)).then(a.0.cmp(&b.0)));
    // Les barres des huit dernières semaines
    let mut barres: Vec<Barre> = (0..SEMAINES as i64)
        .rev()
        .map(|i| {
            let debut = debut_semaine.plus(-7 * i);
            let fin = debut.plus(6);
            let nombre = liste.iter().filter(|c| c.envoyee_le >= debut && c.envoyee_le <= fin).count();
            let texte = if i == 0 { "Cette semaine".to_string() } else { debut.court(aujourdhui) };
            Barre { texte, nombre, part: 0 }
        })
        .collect();
    let haut = barres.iter().map(|b| b.nombre).max().unwrap_or(0);
    for b in &mut barres {
        b.part = (b.nombre * 100 + haut / 2).checked_div(haut).unwrap_or(0) as u32;
    }
    let statuts = Statut::TOUS
        .iter()
        .map(|s| (*s, liste.iter().filter(|c| c.statut == *s).count()))
        .filter(|(_, n)| *n > 0)
        .collect();
    Tableau {
        semaine: liste.iter().filter(|c| c.envoyee_le >= debut_semaine && c.envoyee_le <= aujourdhui).count(),
        mois: liste
            .iter()
            .filter(|c| c.envoyee_le.annee == aujourdhui.annee && c.envoyee_le.mois == aujourdhui.mois)
            .count(),
        total: liste.len(),
        comptees: comptees.len(),
        reponses,
        taux,
        entretiens,
        entretiens_a_venir,
        relances,
        offres_nouvelles,
        par_source,
        barres,
        statuts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidature::{Entretien, Moyen};

    fn j(m: u32, d: u32) -> Jour {
        Jour::nouveau(2026, m, d).unwrap()
    }

    fn c(id: i64, statut: Statut, envoyee: Jour) -> Candidature {
        Candidature {
            id,
            entreprise: format!("Entreprise {id}"),
            poste: "Agent de quai".into(),
            ville: "Metz".into(),
            statut,
            statut_le: envoyee,
            envoyee_le: envoyee,
            moyen: Moyen::Mail,
            source: String::new(),
            lien: String::new(),
            envoye: String::new(),
            cv: String::new(),
            notes: String::new(),
            relance_le: None,
            entretien: None,
        }
    }

    fn o(id: i64, source: &str, ajoutee: Jour, cachee: bool) -> Offre {
        Offre {
            id,
            titre: "Offre".into(),
            ajoutee_le: ajoutee,
            sources: vec![source.into()],
            cachee,
            ..Offre::default()
        }
    }

    /// Une base remplie à la main : chaque chiffre recompté ici à la main.
    #[test]
    fn les_chiffres_d_une_base_remplie_a_la_main() {
        // Jeudi 19 novembre 2026 : la semaine commence le lundi 16
        let auj = j(11, 19);
        assert_eq!(lundi(auj), j(11, 16));
        let mut l = vec![
            c(1, Statut::Envoyee, j(11, 19)),    // cette semaine, ce mois
            c(2, Statut::Envoyee, j(11, 16)),    // cette semaine (lundi), ce mois
            c(3, Statut::Envoyee, j(11, 15)),    // le dimanche d'avant ; 4 jours : pas encore de relance
            c(4, Statut::Envoyee, j(11, 1)),     // relance à faire (18 jours)
            c(5, Statut::Relancee, j(10, 20)),   // relancée depuis longtemps : à classer
            c(6, Statut::Entretien, j(10, 28)),  // entretien à venir
            c(7, Statut::Positive, j(10, 2)),    // réponse, après un entretien passé
            c(8, Statut::Negative, j(9, 25)),    // réponse
            c(9, Statut::Abandonnee, j(11, 18)), // pas comptée pour le taux, mais envoyée cette semaine
            c(10, Statut::SansReponse, j(9, 1)), // comptée, sans réponse ; hors des 8 semaines
        ];
        l[4].relance_le = Some(j(10, 30));
        l[5].entretien = Some(Entretien { jour: j(11, 24), heure: None, lieu: String::new() });
        l[6].entretien = Some(Entretien { jour: j(10, 12), heure: None, lieu: String::new() });
        let offres = vec![
            o(1, "adzuna", j(11, 19), false),
            o(2, "adzuna", j(11, 13), false), // le premier des 7 jours (13 au 19)
            o(3, "indeed", j(11, 17), false),
            o(4, "linkedin", j(11, 18), false),
            o(5, "adzuna", j(11, 12), false), // trop vieille
            o(6, "indeed", j(11, 18), true),  // cachée
            o(7, "page", j(11, 18), false),   // un autre site
            o(8, "indeed", j(11, 20), false), // demain : jamais comptée
            o(9, "jooble", j(11, 18), false), // une source retirée (une ancienne base) : un autre site
        ];
        let t = calculer(&l, &offres, auj, 14);
        assert_eq!((t.semaine, t.mois, t.total), (3, 5, 10));
        assert_eq!((t.comptees, t.reponses, t.taux), (9, 3, Some(33)));
        assert_eq!((t.entretiens, t.entretiens_a_venir), (2, 1));
        assert_eq!(t.relances, 2);
        assert_eq!(t.offres_nouvelles, 6);
        assert_eq!(
            t.par_source,
            vec![("Adzuna".to_string(), 2), ("Indeed".into(), 1), ("LinkedIn".into(), 1), (AUTRES.into(), 2)]
        );
        // Les barres : les 8 semaines, du lundi 28 septembre à cette semaine
        let textes: Vec<&str> = t.barres.iter().map(|b| b.texte.as_str()).collect();
        assert_eq!(
            textes,
            ["28 sept.", "5 oct.", "12 oct.", "19 oct.", "26 oct.", "2 nov.", "9 nov.", "Cette semaine"]
        );
        let nombres: Vec<usize> = t.barres.iter().map(|b| b.nombre).collect();
        assert_eq!(nombres, [1, 0, 0, 1, 2, 0, 1, 3]);
        let parts: Vec<u32> = t.barres.iter().map(|b| b.part).collect();
        assert_eq!(parts, [33, 0, 0, 33, 67, 0, 33, 100]);
        assert_eq!(
            t.statuts,
            vec![
                (Statut::Envoyee, 4),
                (Statut::Relancee, 1),
                (Statut::Entretien, 1),
                (Statut::Positive, 1),
                (Statut::Negative, 1),
                (Statut::SansReponse, 1),
                (Statut::Abandonnee, 1)
            ]
        );
    }

    #[test]
    fn rien_du_tout() {
        let t = calculer(&[], &[], j(11, 19), 14);
        assert_eq!((t.semaine, t.mois, t.total, t.taux, t.relances, t.offres_nouvelles), (0, 0, 0, None, 0, 0));
        assert_eq!(t.barres.len(), SEMAINES);
        assert!(t.barres.iter().all(|b| b.part == 0));
        assert!(t.statuts.is_empty() && t.par_source.is_empty());
        // Seulement des abandonnées : aucun taux
        let t = calculer(&[c(1, Statut::Abandonnee, j(11, 2))], &[], j(11, 19), 14);
        assert_eq!(t.taux, None);
    }

    /// Le passage d'une année (la semaine du lundi 28 décembre au dimanche 3 janvier).
    #[test]
    fn a_cheval_sur_deux_annees() {
        let auj = Jour::nouveau(2027, 1, 2).unwrap();
        assert_eq!(lundi(auj), Jour::nouveau(2026, 12, 28).unwrap());
        let l = [c(1, Statut::Envoyee, Jour::nouveau(2026, 12, 29).unwrap()), c(2, Statut::Envoyee, auj)];
        let t = calculer(&l, &[], auj, 14);
        assert_eq!((t.semaine, t.mois), (2, 1));
        assert_eq!(t.barres.last().unwrap().nombre, 2);
        assert_eq!(t.barres[6].texte, "21 déc. 2026");
    }
}
