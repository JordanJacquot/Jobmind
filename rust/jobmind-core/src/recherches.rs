//! Les recherches gardées (étape 4 du plan) : un nom, des mots, un lieu. Elles viennent de « Ce que je cherche » (une
//! par poste, [`depuis_profil`]) ou sont tapées à la main ([`verifier`]). « Chercher maintenant » envoie l'une d'elles
//! aux sources, qui la reçoivent en mots ([`lieu_pour_source`]).

use crate::candidature::{trop_long, une_ligne};
use crate::lieux;
use crate::recherche::{GenreLieu, Lieu, RAYON_MAX, Recherche, SANS_POSTE_NI_LIEU, VILLE_MAX};

/// Au plus ce nombre de recherches gardées.
pub const NOMBRE_MAX: usize = 20;
pub const NOM_MAX: usize = 60;
pub const MOTS_MAX: usize = 120;

/// Une recherche à garder, vérifiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NouvelleRecherche {
    pub nom: String,
    pub mots: String,
    pub lieu: Lieu,
}

/// Ce que la page a envoyé, tel quel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SaisieRechercheGardee {
    pub nom: String,
    pub mots: String,
    pub lieu: String,
    pub ville: String,
    pub rayon: i64,
    pub departement: String,
    pub region: String,
}

/// Un nom de ville acceptable (comme « Ce que je cherche »).
fn ville_valable(v: &str) -> bool {
    v.chars().any(char::is_alphabetic)
        && v.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '\u{2019}' | '.' | '(' | ')'))
}

/// Le nom d'une recherche sans mots : « Tous les métiers autour de Sarrebourg », « Tous les métiers, 57 Moselle ».
pub fn nom_sans_mots(l: &Lieu) -> String {
    let n = match l {
        Lieu::Ville { nom, .. } => format!("Tous les métiers autour de {nom}"),
        Lieu::Departement(d) => format!("Tous les métiers, {}", d.libelle()),
        Lieu::Region(r) => format!("Tous les métiers, {}", r.nom),
        Lieu::Partout => "Tous les métiers".into(),
    };
    n.chars().take(NOM_MAX).collect()
}

/// Vérifie une recherche tapée. Sans nom, elle prend ses mots. Sans mots, elle prend toutes les offres du lieu, quel que
/// soit le métier : il faut alors un lieu. L'erreur est une phrase pour la bulle de l'écran.
pub fn verifier(s: &SaisieRechercheGardee) -> Result<NouvelleRecherche, String> {
    let mots = une_ligne(&s.mots);
    if trop_long(&mots, MOTS_MAX) {
        return Err(format!("Les mots sont trop longs ({MOTS_MAX} caractères au plus)."));
    }
    let nom = une_ligne(&s.nom);
    if trop_long(&nom, NOM_MAX) {
        return Err(format!("Le nom est trop long ({NOM_MAX} caractères au plus)."));
    }
    let lieu = match GenreLieu::depuis_code(&s.lieu).ok_or("Choisis où chercher.")? {
        GenreLieu::Ville => {
            let v = une_ligne(&s.ville);
            if v.is_empty() {
                return Err("Écris la ville autour de laquelle chercher.".into());
            }
            if trop_long(&v, VILLE_MAX) || !ville_valable(&v) {
                return Err("Le nom de la ville semble faux.".into());
            }
            let rayon = u32::try_from(s.rayon).ok().filter(|r| (1..=RAYON_MAX).contains(r));
            Lieu::Ville { nom: v, rayon: rayon.ok_or(format!("Choisis une distance ({RAYON_MAX} km au plus)."))? }
        }
        GenreLieu::Departement => {
            Lieu::Departement(lieux::departement(&s.departement).ok_or("Choisis un département dans la liste.")?)
        }
        GenreLieu::Region => Lieu::Region(lieux::region(&s.region).ok_or("Choisis une région dans la liste.")?),
        GenreLieu::Partout => Lieu::Partout,
    };
    if mots.is_empty() && lieu == Lieu::Partout {
        return Err(SANS_POSTE_NI_LIEU.into());
    }
    let nom = match nom {
        n if !n.is_empty() => n,
        _ if mots.is_empty() => nom_sans_mots(&lieu),
        _ => mots.chars().take(NOM_MAX).collect(),
    };
    Ok(NouvelleRecherche { nom, mots, lieu })
}

/// Les recherches tirées de « Ce que je cherche » : une par poste, au lieu choisi ; sans poste, une seule, pour tous les
/// métiers du lieu.
pub fn depuis_profil(r: &Recherche) -> Vec<NouvelleRecherche> {
    if r.postes.is_empty() {
        if r.lieu == Lieu::Partout {
            return Vec::new();
        }
        return vec![NouvelleRecherche { nom: nom_sans_mots(&r.lieu), mots: String::new(), lieu: r.lieu.clone() }];
    }
    r.postes
        .iter()
        .take(NOMBRE_MAX)
        .map(|p| {
            let mots: String = p.chars().take(MOTS_MAX).collect();
            let mut nom: String = mots.chars().take(NOM_MAX).collect();
            if let Some(c) = nom.chars().next() {
                nom = c.to_uppercase().chain(nom.chars().skip(1)).collect();
            }
            NouvelleRecherche { nom, mots, lieu: r.lieu.clone() }
        })
        .collect()
}

/// Le lieu en mots pour une source, et le rayon : la ville et son rayon ; le nom du département ou de la région ;
/// rien (partout en France).
pub fn lieu_pour_source(l: &Lieu) -> (String, u32) {
    match l {
        Lieu::Ville { nom, rayon } => (nom.clone(), *rayon),
        Lieu::Departement(d) => (d.nom.to_string(), 0),
        Lieu::Region(r) => (r.nom.to_string(), 0),
        Lieu::Partout => (String::new(), 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recherche::{SaisieRecherche, verifier_recherche};

    fn saisie() -> SaisieRechercheGardee {
        SaisieRechercheGardee {
            nom: String::new(),
            mots: "  agent   de quai ".into(),
            lieu: "ville".into(),
            ville: "Metz".into(),
            rayon: 30,
            ..SaisieRechercheGardee::default()
        }
    }

    #[test]
    fn une_recherche_tapee() {
        let r = verifier(&saisie()).unwrap();
        assert_eq!((r.nom.as_str(), r.mots.as_str()), ("agent de quai", "agent de quai"));
        assert_eq!(lieu_pour_source(&r.lieu), ("Metz".to_string(), 30));
        let r = verifier(&SaisieRechercheGardee {
            nom: "Quai".into(),
            lieu: "departement".into(),
            departement: "57".into(),
            ..saisie()
        })
        .unwrap();
        assert_eq!(r.nom, "Quai");
        assert_eq!(lieu_pour_source(&r.lieu), ("Moselle".to_string(), 0));
        let r = verifier(&SaisieRechercheGardee { lieu: "partout".into(), ..saisie() }).unwrap();
        assert_eq!(lieu_pour_source(&r.lieu), (String::new(), 0));
        for (s, attendu) in [
            (
                SaisieRechercheGardee { mots: " ".into(), lieu: "partout".into(), ..saisie() },
                "Écris un poste, ou choisis un lieu",
            ),
            (SaisieRechercheGardee { mots: "x".repeat(130), ..saisie() }, "trop longs"),
            (SaisieRechercheGardee { nom: "n".repeat(70), ..saisie() }, "nom est trop long"),
            (SaisieRechercheGardee { lieu: "lune".into(), ..saisie() }, "où chercher"),
            (SaisieRechercheGardee { ville: "<b>".into(), ..saisie() }, "semble faux"),
            (SaisieRechercheGardee { rayon: 0, ..saisie() }, "distance"),
            (SaisieRechercheGardee { lieu: "region".into(), region: "x".into(), ..saisie() }, "région"),
        ] {
            let e = verifier(&s).unwrap_err();
            assert!(e.contains(attendu), "{e} / {attendu}");
        }
    }

    #[test]
    fn depuis_ce_que_je_cherche() {
        let r = verifier_recherche(&SaisieRecherche {
            postes: "agent de quai, cariste".into(),
            lieu: "region".into(),
            region: "grand_est".into(),
            teletravail: "peu_importe".into(),
            ..SaisieRecherche::default()
        })
        .unwrap();
        let l = depuis_profil(&r);
        assert_eq!(l.iter().map(|x| x.nom.as_str()).collect::<Vec<_>>(), ["Agent de quai", "Cariste"]);
        assert_eq!(l[1].mots, "cariste");
        assert_eq!(lieu_pour_source(&l[0].lieu), ("Grand Est".to_string(), 0));
    }

    /// Sans poste ni mot, avec un lieu : une seule recherche, pour tous les métiers du lieu (demande de Jordan, 19/11).
    #[test]
    fn sans_poste_avec_un_lieu() {
        let r = verifier(&SaisieRechercheGardee { mots: "  ".into(), ville: "Sarrebourg".into(), ..saisie() }).unwrap();
        assert_eq!((r.nom.as_str(), r.mots.as_str()), ("Tous les métiers autour de Sarrebourg", ""));
        assert_eq!(lieu_pour_source(&r.lieu), ("Sarrebourg".to_string(), 30));
        let r = verifier(&SaisieRechercheGardee { mots: String::new(), nom: "Le coin".into(), ..saisie() }).unwrap();
        assert_eq!(r.nom, "Le coin");
        let d = SaisieRechercheGardee {
            mots: String::new(),
            lieu: "departement".into(),
            departement: "57".into(),
            ..saisie()
        };
        assert_eq!(verifier(&d).unwrap().nom, "Tous les métiers, 57 Moselle");
        let p = verifier_recherche(&SaisieRecherche {
            postes: String::new(),
            lieu: "ville".into(),
            ville: "Sarrebourg".into(),
            rayon: 20,
            teletravail: "peu_importe".into(),
            sans_diplome: true,
            debutant: true,
            ..SaisieRecherche::default()
        })
        .unwrap();
        let l = depuis_profil(&p);
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].nom.as_str(), l[0].mots.as_str()), ("Tous les métiers autour de Sarrebourg", ""));
        assert_eq!(l[0].lieu, Lieu::Ville { nom: "Sarrebourg".into(), rayon: 20 });
    }
}
