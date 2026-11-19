//! Le style de Jordan (étape 9 du plan) : ce qu'il règle lui-même dans Mon profil, rubrique Mon style, vérifié ici
//! (l'intensité de la réécriture, la longueur des phrases, ses propres expressions à éviter).
//!
//! Les règles elles-mêmes, le plancher professionnel, le score naturel et la réécriture sont dans jobmind-humanizer.

/// L'intensité de la réécriture dans le style de Jordan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intensite {
    /// Ses règles seulement, sans Claude (marche sans clé)
    Aucune,
    /// Claude change le moins possible
    Legere,
    /// Claude reprend ses mots et son rythme
    #[default]
    Moyenne,
    /// Claude réécrit chaque phrase à sa façon
    Forte,
}

impl Intensite {
    pub const TOUTES: [Intensite; 4] = [Intensite::Aucune, Intensite::Legere, Intensite::Moyenne, Intensite::Forte];

    /// Le code gardé dans la base (la colonne `style_intensite` de la migration 8).
    pub fn code(self) -> &'static str {
        match self {
            Intensite::Aucune => "aucune",
            Intensite::Legere => "legere",
            Intensite::Moyenne => "moyenne",
            Intensite::Forte => "forte",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Intensite> {
        Intensite::TOUTES.into_iter().find(|i| i.code() == c.trim())
    }

    /// Le nom à l'écran.
    pub fn nom(self) -> &'static str {
        match self {
            Intensite::Aucune => "Mes règles seulement",
            Intensite::Legere => "Légère",
            Intensite::Moyenne => "Moyenne",
            Intensite::Forte => "Forte",
        }
    }

    /// Ce qu'elle fait, en une phrase.
    pub fn explication(self) -> &'static str {
        match self {
            Intensite::Aucune => "Jobmind applique seulement tes règles, sans Claude. Marche sans clé.",
            Intensite::Legere => {
                "Claude change le moins possible : il retire les formules toutes faites et coupe les phrases trop longues."
            }
            Intensite::Moyenne => "Claude reprend tes mots et le rythme de tes textes, chaque idée reste à sa place.",
            Intensite::Forte => "Claude réécrit chaque phrase à ta façon, sans changer les faits.",
        }
    }

    /// Claude est nécessaire.
    pub fn avec_claude(self) -> bool {
        self != Intensite::Aucune
    }
}

/// La longueur de phrase permise (en mots) : au moins, au plus, et celle de départ.
pub const PHRASE_MIN: usize = 12;
pub const PHRASE_PLUS: usize = 45;
pub const PHRASE_DEFAUT: usize = 25;

/// Une expression à éviter de Jordan, au plus (en caractères), et son remplacement.
pub const MOTIF_MAX: usize = 60;
/// Les expressions de Jordan, au plus.
pub const PERSO_MAX: usize = 60;

/// La longueur de phrase demandée, ramenée entre [`PHRASE_MIN`] et [`PHRASE_PLUS`].
pub fn verifier_phrase_max(n: i64) -> usize {
    n.clamp(PHRASE_MIN as i64, PHRASE_PLUS as i64) as usize
}

fn propre(t: &str) -> String {
    t.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Vérifie une expression à éviter et son remplacement (facultatif) : une ligne, au moins une lettre, au plus
/// [`MOTIF_MAX`] caractères chacun, et un remplacement différent de l'expression.
pub fn verifier_regle(motif: &str, remplacement: &str) -> Result<(String, Option<String>), String> {
    let m = propre(motif);
    let r = propre(remplacement);
    if !m.chars().any(char::is_alphabetic) {
        return Err("Écris l'expression à éviter, par exemple : je me permets de.".into());
    }
    if m.chars().count() > MOTIF_MAX || r.chars().count() > MOTIF_MAX {
        return Err(format!("C'est trop long : {MOTIF_MAX} caractères au plus."));
    }
    if crate::offre::normaliser(&m).split_whitespace().collect::<Vec<_>>()
        == crate::offre::normaliser(&r).split_whitespace().collect::<Vec<_>>()
    {
        return Err("Le remplacement est le même que l'expression.".into());
    }
    Ok((m, if r.is_empty() { None } else { Some(r) }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saisies() {
        assert_eq!(verifier_phrase_max(3), PHRASE_MIN);
        assert_eq!(verifier_phrase_max(300), PHRASE_PLUS);
        assert_eq!(verifier_phrase_max(20), 20);
        assert_eq!(verifier_regle("  je me\tpermets  de ", "").unwrap(), ("je me permets de".to_string(), None));
        assert_eq!(verifier_regle("afin de", "pour").unwrap().1.as_deref(), Some("pour"));
        assert!(verifier_regle("  ", "").is_err());
        assert!(verifier_regle("123", "").is_err());
        assert!(verifier_regle(&"a".repeat(61), "").is_err());
        assert!(verifier_regle("Afin de", "afin  de").is_err());
        for i in Intensite::TOUTES {
            assert_eq!(Intensite::depuis_code(i.code()), Some(i));
            assert!(!i.explication().contains(['\u{2013}', '\u{2014}']));
        }
        assert_eq!(Intensite::depuis_code("x"), None);
        assert!(!Intensite::Aucune.avec_claude() && Intensite::Forte.avec_claude());
    }
}
