//! Une candidature suivie à la main (étape 2 du plan) : ce que Jordan a envoyé, à qui, quand et comment, son statut et
//! sa frise. La saisie de la page est vérifiée ici ([`verifier`]), jamais crue telle quelle.

use crate::jour::{Heure, Jour};
use crate::statut::Statut;

/// Par où la candidature est partie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Moyen {
    Mail,
    Site,
    MainPropre,
    Autre,
}

impl Moyen {
    /// Dans l'ordre de l'écran.
    pub const TOUS: [Moyen; 4] = [Moyen::Mail, Moyen::Site, Moyen::MainPropre, Moyen::Autre];

    /// Le code gardé dans la base.
    pub fn code(self) -> &'static str {
        match self {
            Moyen::Mail => "mail",
            Moyen::Site => "site",
            Moyen::MainPropre => "main_propre",
            Moyen::Autre => "autre",
        }
    }

    /// Le nom du choix à l'écran.
    pub fn nom(self) -> &'static str {
        match self {
            Moyen::Mail => "Par mail",
            Moyen::Site => "Sur le site",
            Moyen::MainPropre => "En main propre",
            Moyen::Autre => "Autrement",
        }
    }

    /// Dans une phrase, après « envoyée » (« envoyée par mail ») ; rien pour « Autrement ».
    pub fn dans_phrase(self) -> Option<&'static str> {
        match self {
            Moyen::Mail => Some("par mail"),
            Moyen::Site => Some("sur le site"),
            Moyen::MainPropre => Some("en main propre"),
            Moyen::Autre => None,
        }
    }

    pub fn depuis_code(code: &str) -> Option<Moyen> {
        Moyen::TOUS.into_iter().find(|m| m.code() == code)
    }
}

/// Un entretien : son jour, son heure si on la connaît, son lieu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entretien {
    pub jour: Jour,
    pub heure: Option<Heure>,
    pub lieu: String,
}

impl Entretien {
    /// « mercredi 20 novembre à 9 h 30 » (ou « aujourd'hui à 9 h 30 », « demain »).
    pub fn quand(&self, aujourdhui: Jour) -> String {
        let jour = match aujourdhui.jours_jusqua(self.jour) {
            0 => "aujourd'hui".to_string(),
            1 => "demain".to_string(),
            _ => self.jour.long(aujourdhui),
        };
        match self.heure {
            Some(h) => format!("{jour} à {}", h.texte()),
            None => jour,
        }
    }

    /// Ce que la base garde : « 2026-11-20T09:30:00 », ou le jour seul.
    pub fn moment_iso(&self) -> String {
        match self.heure {
            Some(h) => format!("{}T{}:00", self.jour.iso(), h.hhmm()),
            None => self.jour.iso(),
        }
    }

    /// D'après ce que la base garde ([`Entretien::moment_iso`]).
    pub fn depuis_iso(moment: &str, lieu: &str) -> Option<Entretien> {
        let jour = Jour::lire_iso(moment)?;
        let heure = moment.get(11..16).and_then(Heure::lire);
        Some(Entretien { jour, heure, lieu: lieu.to_string() })
    }
}

/// Une candidature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidature {
    pub id: i64,
    pub entreprise: String,
    pub poste: String,
    pub ville: String,
    pub statut: Statut,
    /// Le jour où le statut a changé pour la dernière fois (le jour d'envoi pour « Envoyée »)
    pub statut_le: Jour,
    pub envoyee_le: Jour,
    pub moyen: Moyen,
    /// Où l'offre a été vue (LinkedIn, Indeed...)
    pub source: String,
    /// Le lien de l'offre (vide : aucun)
    pub lien: String,
    /// Ce qui a été envoyé (la lettre, le message)
    pub envoye: String,
    /// Le CV utilisé (son nom)
    pub cv: String,
    pub notes: String,
    pub relance_le: Option<Jour>,
    pub entretien: Option<Entretien>,
}

/// Un point de la frise : un statut daté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evenement {
    pub jour: Jour,
    pub statut: Statut,
    /// Un détail (l'entretien : son jour et son lieu)
    pub texte: String,
}

/// Les longueurs permises (en caractères).
pub const ENTREPRISE_MAX: usize = 120;
pub const POSTE_MAX: usize = 120;
pub const VILLE_MAX: usize = 80;
pub const SOURCE_MAX: usize = 60;
pub const LIEN_MAX: usize = 500;
pub const ENVOYE_MAX: usize = 8000;
pub const CV_MAX: usize = 120;
pub const NOTES_MAX: usize = 4000;
pub const LIEU_MAX: usize = 120;

/// Ce que la page a envoyé pour une candidature, tel quel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Saisie {
    pub entreprise: String,
    pub poste: String,
    pub ville: String,
    pub date: String,
    pub moyen: String,
    pub source: String,
    pub lien: String,
    pub envoye: String,
    pub cv: String,
    pub notes: String,
}

/// Une saisie vérifiée : prête à être gardée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fiche {
    pub entreprise: String,
    pub poste: String,
    pub ville: String,
    pub envoyee_le: Jour,
    pub moyen: Moyen,
    pub source: String,
    pub lien: String,
    pub envoye: String,
    pub cv: String,
    pub notes: String,
}

/// Un texte d'une ligne : espaces en trop retirés, caractères de contrôle remplacés par une espace.
fn une_ligne(t: &str) -> String {
    let t: String = t.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Un texte de plusieurs lignes : fins de ligne « \n », autres caractères de contrôle retirés, espaces autour retirés.
fn plusieurs_lignes(t: &str) -> String {
    let t = t.replace("\r\n", "\n").replace('\r', "\n");
    let t: String = t.chars().filter(|&c| c == '\n' || c == '\t' || !c.is_control()).collect();
    t.trim().to_string()
}

fn trop_long(t: &str, max: usize) -> bool {
    t.chars().count() > max
}

/// Un lien d'offre acceptable : vide, ou une adresse « http:// » ou « https:// » sans espace.
pub fn lien_valable(l: &str) -> bool {
    l.is_empty()
        || ((l.starts_with("https://") || l.starts_with("http://"))
            && l.len() > 10
            && !l.chars().any(|c| c.is_whitespace() || c.is_control()))
}

/// Vérifie une saisie : l'entreprise et le poste sont obligatoires, la date d'envoi est un vrai jour qui n'est pas
/// dans le futur, le moyen est connu, le lien est une adresse web, chaque texte a une longueur raisonnable. L'erreur est
/// une phrase pour la bulle de l'écran.
pub fn verifier(s: &Saisie, aujourdhui: Jour) -> Result<Fiche, String> {
    let entreprise = une_ligne(&s.entreprise);
    let poste = une_ligne(&s.poste);
    let ville = une_ligne(&s.ville);
    let source = une_ligne(&s.source);
    let lien = s.lien.trim().to_string();
    let cv = une_ligne(&s.cv);
    let envoye = plusieurs_lignes(&s.envoye);
    let notes = plusieurs_lignes(&s.notes);
    if entreprise.is_empty() {
        return Err("Écris le nom de l'entreprise.".into());
    }
    if poste.is_empty() {
        return Err("Écris le poste.".into());
    }
    for (t, max, quoi) in [
        (&entreprise, ENTREPRISE_MAX, "Le nom de l'entreprise"),
        (&poste, POSTE_MAX, "Le poste"),
        (&ville, VILLE_MAX, "La ville"),
        (&source, SOURCE_MAX, "La source"),
        (&lien, LIEN_MAX, "Le lien de l'offre"),
        (&cv, CV_MAX, "Le nom du CV"),
        (&envoye, ENVOYE_MAX, "Ce que tu as envoyé"),
        (&notes, NOTES_MAX, "Les notes"),
    ] {
        if trop_long(t, max) {
            return Err(format!("{quoi} est trop long ({max} caractères au plus)."));
        }
    }
    let envoyee_le = Jour::lire_saisie(&s.date).ok_or("La date d'envoi s'écrit comme 18/11/2026.")?;
    if envoyee_le > aujourdhui {
        return Err("La date d'envoi ne peut pas être après aujourd'hui.".into());
    }
    let moyen = Moyen::depuis_code(&s.moyen).ok_or("Choisis par où tu l'as envoyée.")?;
    if !lien_valable(&lien) {
        return Err("Le lien de l'offre doit commencer par https://".into());
    }
    Ok(Fiche { entreprise, poste, ville, envoyee_le, moyen, source, lien, envoye, cv, notes })
}

/// Les jours possibles pour un entretien : d'un an en arrière à trois ans devant (le calendrier de l'écran ne montre
/// que ceux-là, [`verifier_entretien`] refuse les autres).
pub fn bornes_entretien(aujourdhui: Jour) -> (Jour, Jour) {
    (aujourdhui.plus(-366), aujourdhui.plus(3 * 366))
}

/// Les jours que le calendrier de la date d'envoi laisse choisir : des trois dernières années à aujourd'hui (une date
/// plus ancienne se tape encore à la main ; [`verifier`] refuse seulement une date après aujourd'hui).
pub fn bornes_envoi(aujourdhui: Jour) -> (Jour, Jour) {
    (aujourdhui.plus(-3 * 366), aujourdhui)
}

/// Vérifie un entretien tapé : un vrai jour (jamais plus d'un an en arrière), une heure si elle est donnée, un lieu
/// d'une ligne.
pub fn verifier_entretien(date: &str, heure: &str, lieu: &str, aujourdhui: Jour) -> Result<Entretien, String> {
    let jour = Jour::lire_saisie(date).ok_or("La date de l'entretien s'écrit comme 25/11/2026.")?;
    let (min, max) = bornes_entretien(aujourdhui);
    if jour < min || jour > max {
        return Err("La date de l'entretien semble fausse.".into());
    }
    let heure = match heure.trim() {
        "" => None,
        h => Some(Heure::lire(h).ok_or("L'heure s'écrit comme 9 h 30.")?),
    };
    let lieu = une_ligne(lieu);
    if trop_long(&lieu, LIEU_MAX) {
        return Err(format!("Le lieu est trop long ({LIEU_MAX} caractères au plus)."));
    }
    Ok(Entretien { jour, heure, lieu })
}

/// Vérifie des notes tapées dans le dossier.
pub fn verifier_notes(notes: &str) -> Result<String, String> {
    let n = plusieurs_lignes(notes);
    if trop_long(&n, NOTES_MAX) {
        return Err(format!("Les notes sont trop longues ({NOTES_MAX} caractères au plus)."));
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auj() -> Jour {
        Jour::nouveau(2026, 11, 18).unwrap()
    }

    fn saisie() -> Saisie {
        Saisie {
            entreprise: "  Transports   Moselle Fret ".into(),
            poste: "Agent de quai".into(),
            ville: "Woippy".into(),
            date: "02/11/2026".into(),
            moyen: "site".into(),
            source: "Indeed".into(),
            lien: " https://exemple.fr/offre/42 ".into(),
            envoye: "Madame, Monsieur,\r\n\r\nJe vous adresse ma candidature.\u{7}\n".into(),
            cv: "CV 2026.pdf".into(),
            notes: String::new(),
        }
    }

    #[test]
    fn une_saisie_correcte() {
        let f = verifier(&saisie(), auj()).unwrap();
        assert_eq!(f.entreprise, "Transports Moselle Fret");
        assert_eq!(f.envoyee_le, Jour::nouveau(2026, 11, 2).unwrap());
        assert_eq!(f.moyen, Moyen::Site);
        assert_eq!(f.lien, "https://exemple.fr/offre/42");
        assert_eq!(f.envoye, "Madame, Monsieur,\n\nJe vous adresse ma candidature.");
        // Aujourd'hui est permis, et le reste peut être vide
        let s = Saisie {
            entreprise: "A".into(),
            poste: "B".into(),
            date: "18/11/2026".into(),
            moyen: "autre".into(),
            ..Saisie::default()
        };
        assert!(verifier(&s, auj()).is_ok());
    }

    /// Contre-épreuve : chaque faute est refusée, avec une phrase simple.
    #[test]
    fn les_saisies_fausses_sont_refusees() {
        type Changer = fn(&mut Saisie);
        let cas: [(Changer, &str); 9] = [
            (|s| s.entreprise = "   ".into(), "Écris le nom de l'entreprise."),
            (|s| s.poste = String::new(), "Écris le poste."),
            (|s| s.date = "31/11/2026".into(), "La date d'envoi s'écrit comme 18/11/2026."),
            (|s| s.date = "19/11/2026".into(), "La date d'envoi ne peut pas être après aujourd'hui."),
            (|s| s.moyen = "pigeon".into(), "Choisis par où tu l'as envoyée."),
            (|s| s.lien = "javascript:alert(1)".into(), "Le lien de l'offre doit commencer par https://"),
            (|s| s.lien = "https://a b.fr/x".into(), "Le lien de l'offre doit commencer par https://"),
            (|s| s.entreprise = "x".repeat(121), "Le nom de l'entreprise est trop long (120 caractères au plus)."),
            (|s| s.envoye = "x".repeat(8001), "Ce que tu as envoyé est trop long (8000 caractères au plus)."),
        ];
        for (changer, attendu) in cas {
            let mut s = saisie();
            changer(&mut s);
            assert_eq!(verifier(&s, auj()), Err(attendu.to_string()));
        }
        for m in Moyen::TOUS {
            assert_eq!(Moyen::depuis_code(m.code()), Some(m));
            assert!(!m.nom().contains(['\u{2013}', '\u{2014}']));
        }
    }

    #[test]
    fn entretiens() {
        let e = verifier_entretien("20/11/2026", "9h30", " Zone  Actipôle ", auj()).unwrap();
        assert_eq!(e.lieu, "Zone Actipôle");
        assert_eq!(e.quand(auj()), "vendredi 20 novembre à 9 h 30");
        assert_eq!(e.moment_iso(), "2026-11-20T09:30:00");
        assert_eq!(Entretien::depuis_iso(&e.moment_iso(), "Zone Actipôle"), Some(e.clone()));
        let sans_heure = verifier_entretien("19/11/2026", "", "", auj()).unwrap();
        assert_eq!(sans_heure.quand(auj()), "demain");
        assert_eq!(sans_heure.moment_iso(), "2026-11-19");
        assert_eq!(Entretien::depuis_iso("2026-11-19", ""), Some(sans_heure));
        assert_eq!(verifier_entretien("18/11/2026", "14", "", auj()).unwrap().quand(auj()), "aujourd'hui à 14 h");
        assert!(verifier_entretien("20/11/2026", "25h", "", auj()).is_err());
        assert!(verifier_entretien("", "", "", auj()).is_err());
        assert!(verifier_entretien("20/11/2036", "", "", auj()).is_err());
        assert!(verifier_notes(&"x".repeat(4001)).is_err());
        assert_eq!(verifier_notes(" a\r\nb ").unwrap(), "a\nb");
        assert!(lien_valable("") && lien_valable("http://exemple.fr") && !lien_valable("https://"));
    }

    /// Le calendrier de l'écran écrit le jour cliqué comme « 05/01/2027 » ([`Jour::saisie`]) : chaque jour qu'il laisse
    /// choisir passe la vérification, le jour d'avant le premier et celui d'après le dernier sont refusés.
    #[test]
    fn les_jours_du_calendrier_sont_acceptes() {
        let (min, max) = bornes_entretien(auj());
        assert_eq!((min.iso(), max.iso()), ("2025-11-17".to_string(), "2029-11-20".to_string()));
        for j in [min, auj(), auj().plus(1), max] {
            assert_eq!(verifier_entretien(&j.saisie(), "", "", auj()).unwrap().jour, j, "{}", j.saisie());
        }
        assert!(verifier_entretien(&min.plus(-1).saisie(), "", "", auj()).is_err());
        assert!(verifier_entretien(&max.plus(1).saisie(), "", "", auj()).is_err());
        let (min, max) = bornes_envoi(auj());
        assert_eq!((min.iso(), max.iso()), ("2023-11-16".to_string(), "2026-11-18".to_string()));
        let s = |d: Jour| Saisie { date: d.saisie(), ..saisie() };
        for j in [min, auj().plus(-16), max] {
            assert_eq!(verifier(&s(j), auj()).unwrap().envoyee_le, j, "{}", j.saisie());
        }
        assert!(verifier(&s(max.plus(1)), auj()).is_err());
    }
}
