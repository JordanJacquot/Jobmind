//! La construction d'essai (option cargo `essai`), jamais livrée : elle tourne seulement sur un dossier d'essai et
//! sur un bureau Windows à part (outils\sur-bureau.ps1, outils\essai-fenetre.ps1, outils\essai-dans-mindset.ps1).
//! L'exe livré ne contient pas ce module : outils\verifier-sans-essai.ps1 cherche ses noms dans l'exe de publication.
//! Repris de mindcare-appli.
//!
//! Trois variables, toutes obligatoires (sinon le programme s'arrête aussitôt, sans rien faire) :
//! - `JOBMIND_DONNEES` : le dossier de données de Jobmind ; le dossier WebView2 se met à côté, dans son dossier
//!   parent (comme `<données de la suite>\Jobmind`) ;
//! - `JOBMIND_ESSAI_LOCAL` : le faux `%LOCALAPPDATA%` (suite.json, afficher-page.txt) ;
//! - `JOBMIND_ESSAI_NOM` : le nom des verrou et signal d'essai, `Local\JobmindEssai-<nom>-Instance` et
//!   `-Afficher` (jamais ceux de la vraie Jobmind, qui tourne peut-être sur le PC).
//!
//! En plus, dans cette construction : HKLM n'est jamais lu et seul ce programme peut être lancé
//! (windows\systeme.rs), le signal du vrai Mindset n'est jamais levé (pont.rs) ; un fichier `capture.demande` dans
//! le dossier de données fait écrire l'image de la fenêtre dans `capture.png`, à côté ; un fichier `ordre.demande`
//! (un ordre au kit en JSON) est envoyé à la page (ouvrir une page ou la fiche d'une carte, pour les captures).

use std::path::{Path, PathBuf};

use jobmind_core::candidature::{Entretien, Fiche, Moyen};
use jobmind_core::{Heure, Jour, Statut};
use jobmind_storage::candidatures;
use jobmind_storage::{Journal, Moment};
use jobmind_vie::Noms;

/// La demande de capture et l'image, dans le dossier de données d'essai.
pub const CAPTURE_DEMANDE: &str = "capture.demande";
pub const CAPTURE: &str = "capture.png";
/// Un ordre au kit (JSON) à envoyer à la page, pour les captures (ouvrir une page, la fiche d'une carte).
pub const ORDRE_DEMANDE: &str = "ordre.demande";
/// Au démarrage, ce fichier dans le dossier de données d'essai remplit la base de candidatures inventées (les
/// captures), à des statuts variés, datées d'après aujourd'hui ; il est supprimé ensuite.
pub const EXEMPLES_DEMANDE: &str = "exemples.demande";

/// Des candidatures inventées (entreprises et personnes qui n'existent pas), comme celles de la maquette C : (jours
/// depuis l'envoi, entreprise, poste, ville, moyen, source, statuts suivants avec le nombre de jours avant aujourd'hui,
/// entretien dans N jours à cette heure et ce lieu, notes).
type Exemple = (
    i64,
    &'static str,
    &'static str,
    &'static str,
    Moyen,
    &'static str,
    &'static [(Statut, i64)],
    Option<(i64, u32, u32, &'static str)>,
    &'static str,
);
const EXEMPLES: [Exemple; 10] = [
    (16, "Transports Moselle Fret", "Agent de quai", "Woippy", Moyen::Site, "France Travail", &[], None, ""),
    (
        13,
        "Logistique des Trois Frontières",
        "Préparateur de commandes",
        "Thionville",
        Moyen::Mail,
        "Adzuna",
        &[(Statut::Entretien, 6)],
        Some((2, 9, 30, "Zone Actipôle, Thionville")),
        "Appel de la responsable. Entretien sur place, apporter le CV et le CACES. Demander les horaires exacts.",
    ),
    (14, "Résidence Les Tilleuls", "Agent de service", "Pont-à-Mousson", Moyen::Site, "Indeed", &[], None, ""),
    (2, "Netéa Services", "Agent d'entretien", "Metz", Moyen::Mail, "LinkedIn", &[], None, ""),
    (6, "Cartonnerie de la Seille", "Opérateur de production", "Metz", Moyen::MainPropre, "", &[], None, ""),
    (
        19,
        "Brico Lorraine",
        "Vendeur rayon outillage",
        "Nancy",
        Moyen::Site,
        "France Travail",
        &[(Statut::Relancee, 5)],
        None,
        "Relance par téléphone, la responsable était absente.",
    ),
    (
        27,
        "Hôtel des Remparts",
        "Réceptionniste de nuit",
        "Nancy",
        Moyen::Mail,
        "Indeed",
        &[(Statut::Relancee, 15)],
        None,
        "",
    ),
    (
        15,
        "Avenir Numérique Est",
        "Technicien support",
        "Nancy",
        Moyen::Site,
        "Adzuna",
        &[(Statut::Positive, 1)],
        None,
        "Premier échange positif, ils veulent me revoir.",
    ),
    (
        29,
        "Réseaux Moselle Télécom",
        "Technicien réseaux",
        "Thionville",
        Moyen::Site,
        "Adzuna",
        &[(Statut::Negative, 8)],
        None,
        "Ils ont pris quelqu'un avec plus d'expérience en fibre.",
    ),
    (
        41,
        "Les Halles de Sarreguemines",
        "Employé libre-service",
        "Sarreguemines",
        Moyen::Autre,
        "",
        &[(Statut::Relancee, 27), (Statut::SansReponse, 10)],
        None,
        "",
    ),
];

/// Ce que dit la lettre d'exemple (professionnelle, sans rien d'inventé sur la personne).
fn lettre(poste: &str) -> String {
    format!(
        "Madame, Monsieur,\n\nVotre offre « {poste} » a retenu toute mon attention. J'ai travaillé deux ans en entrepôt, \
         en équipe, avec des horaires du matin.\n\nJe suis disponible immédiatement et je serais heureux de vous \
         rencontrer pour un entretien.\n\nJe vous prie d'agréer, Madame, Monsieur, mes salutations distinguées."
    )
}

/// Remplit la base d'essai avec [`EXEMPLES`] si `exemples.demande` est là (puis le supprime). Seulement dans la
/// construction d'essai, seulement dans son dossier de données.
pub fn exemples_demandes(dossier: &Path, base: Option<&jobmind_storage::Base>, maintenant: Moment, journal: &Journal) {
    let demande = dossier.join(EXEMPLES_DEMANDE);
    if !demande.is_file() {
        return;
    }
    let _ = std::fs::remove_file(&demande);
    let Some(b) = base else { return };
    match remplir(b.connexion(), maintenant) {
        Ok(n) => journal.ecrire(&format!("Essai : {n} candidatures d'exemple ajoutées")),
        Err(e) => journal.ecrire(&format!("Essai : exemples non ajoutés ({e})")),
    }
}

fn a_ce_jour(j: Jour, h: u32) -> Moment {
    Moment::nouveau(j.annee, j.mois, j.jour, h, 0, 0).unwrap_or(Moment {
        annee: 2026,
        mois: 1,
        jour: 1,
        heure: 0,
        minute: 0,
        seconde: 0,
    })
}

fn remplir(c: &jobmind_storage::rusqlite::Connection, maintenant: Moment) -> jobmind_storage::rusqlite::Result<usize> {
    let auj = maintenant.jour();
    for (depuis, entreprise, poste, ville, moyen, source, statuts, entretien, notes) in EXEMPLES {
        let envoyee_le = auj.plus(-depuis);
        let f = Fiche {
            entreprise: entreprise.into(),
            poste: poste.into(),
            ville: ville.into(),
            envoyee_le,
            moyen,
            source: source.into(),
            lien: String::new(),
            envoye: lettre(poste),
            cv: "CV 2026.pdf".into(),
            notes: notes.into(),
        };
        let id = candidatures::ajouter(c, &f, a_ce_jour(envoyee_le, 10))?;
        for (s, il_y_a) in statuts {
            let e = match (s, entretien) {
                (Statut::Entretien, Some((dans, h, m, lieu))) => {
                    Some(Entretien { jour: auj.plus(dans), heure: Heure::nouvelle(h, m), lieu: lieu.into() })
                }
                _ => None,
            };
            candidatures::changer_statut(c, id, *s, e.as_ref(), a_ce_jour(auj.plus(-il_y_a), 11))?;
        }
    }
    Ok(EXEMPLES.len())
}

/// Ce que l'essai donne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Essai {
    pub donnees: PathBuf,
    pub local: String,
    pub nom: String,
}

impl Essai {
    /// Les noms du verrou et du signal d'essai.
    pub fn noms(&self) -> Noms {
        Noms {
            verrou: format!(r"Local\JobmindEssai-{}-Instance", self.nom),
            signal: format!(r"Local\JobmindEssai-{}-Afficher", self.nom),
        }
    }
}

/// Un nom d'essai : lettres, chiffres et tirets, 40 au plus.
fn nom_valable(n: &str) -> bool {
    (1..=40).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

/// Les réglages de l'essai, d'après les trois valeurs (lues dans l'environnement par [`lire`]).
pub fn depuis(donnees: Option<String>, local: Option<String>, nom: Option<String>) -> Result<Essai, String> {
    let donnees = donnees.filter(|d| Path::new(d).is_absolute()).ok_or("JOBMIND_DONNEES manque ou incomplet")?;
    let local = local.filter(|d| Path::new(d).is_absolute()).ok_or("JOBMIND_ESSAI_LOCAL manque ou incomplet")?;
    let nom = nom.filter(|n| nom_valable(n)).ok_or("JOBMIND_ESSAI_NOM manque ou mal écrit")?;
    Ok(Essai { donnees: PathBuf::from(donnees), local, nom })
}

/// Les réglages de l'essai, lus dans l'environnement.
pub fn lire() -> Result<Essai, String> {
    let v = |n: &str| std::env::var(n).ok();
    depuis(v("JOBMIND_DONNEES"), v("JOBMIND_ESSAI_LOCAL"), v("JOBMIND_ESSAI_NOM"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reglages_d_essai() {
        let s = |t: &str| Some(t.to_string());
        let e = depuis(s(r"C:\Temp\essai\Donnees\Jobmind"), s(r"C:\Temp\essai\Local"), s("essai-42")).unwrap();
        assert_eq!(e.noms().verrou, r"Local\JobmindEssai-essai-42-Instance");
        assert_eq!(e.noms().signal, r"Local\JobmindEssai-essai-42-Afficher");
        // Jamais les vrais noms
        assert!(!e.noms().verrou.starts_with(r"Local\Jobmind-"));
        // Chaque valeur manquante ou mal écrite arrête l'essai
        assert!(depuis(None, s(r"C:\L"), s("a")).is_err());
        assert!(depuis(s(r"Donnees\Jobmind"), s(r"C:\L"), s("a")).is_err());
        assert!(depuis(s(r"C:\D"), None, s("a")).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), None).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), s(r"a\..\Jobmind")).is_err());
        assert!(depuis(s(r"C:\D"), s(r"C:\L"), s("")).is_err());
    }

    /// Les exemples des captures : à des statuts variés, dans les trois groupes de la liste, et un entretien à venir.
    #[test]
    fn exemples_varies() {
        let m = Moment::nouveau(2026, 11, 18, 9, 0, 0).unwrap();
        let (b, _) = jobmind_storage::Base::en_memoire(m).unwrap();
        assert_eq!(remplir(b.connexion(), m).unwrap(), 10);
        let (l, _) = candidatures::lister(b.connexion()).unwrap();
        assert_eq!(l.len(), 10);
        let groupes: Vec<(jobmind_core::suivi::Groupe, usize)> =
            jobmind_core::suivi::ranger(&l, m.jour(), 14).iter().map(|(g, x)| (*g, x.len())).collect();
        use jobmind_core::suivi::Groupe::*;
        assert_eq!(groupes, [(AFaire, 4), (EnAttente, 3), (Terminee, 3)]);
        for s in Statut::TOUS.iter().filter(|s| **s != Statut::Abandonnee) {
            assert!(l.iter().any(|c| c.statut == *s), "{s:?}");
        }
    }
}
