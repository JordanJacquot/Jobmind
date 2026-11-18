//! Les statuts d'une candidature (cadrage, point 4) : Envoyée, Relancée, Entretien, Réponse positive, Réponse
//! négative, Sans réponse, Abandonnée. Le code (`envoyee`...) est ce que la base garde ; le nom, ce que l'écran montre.

/// Le statut d'une candidature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Statut {
    Envoyee,
    Relancee,
    Entretien,
    Positive,
    Negative,
    SansReponse,
    Abandonnee,
}

impl Statut {
    /// Tous les statuts, dans l'ordre de l'écran.
    pub const TOUS: [Statut; 7] = [
        Statut::Envoyee,
        Statut::Relancee,
        Statut::Entretien,
        Statut::Positive,
        Statut::Negative,
        Statut::SansReponse,
        Statut::Abandonnee,
    ];

    /// Le code gardé dans la base.
    pub fn code(self) -> &'static str {
        match self {
            Statut::Envoyee => "envoyee",
            Statut::Relancee => "relancee",
            Statut::Entretien => "entretien",
            Statut::Positive => "positive",
            Statut::Negative => "negative",
            Statut::SansReponse => "sans_reponse",
            Statut::Abandonnee => "abandonnee",
        }
    }

    /// Le nom montré à l'écran.
    pub fn nom(self) -> &'static str {
        match self {
            Statut::Envoyee => "Envoyée",
            Statut::Relancee => "Relancée",
            Statut::Entretien => "Entretien",
            Statut::Positive => "Réponse positive",
            Statut::Negative => "Réponse négative",
            Statut::SansReponse => "Sans réponse",
            Statut::Abandonnee => "Abandonnée",
        }
    }

    /// Le statut d'un code de la base (un code inconnu : aucun).
    pub fn depuis_code(code: &str) -> Option<Statut> {
        Statut::TOUS.into_iter().find(|s| s.code() == code)
    }

    /// Une candidature terminée (rangée dans « Terminées ») : plus rien à attendre.
    pub fn termine(self) -> bool {
        matches!(self, Statut::Positive | Statut::Negative | Statut::SansReponse | Statut::Abandonnee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_et_noms() {
        for s in Statut::TOUS {
            assert_eq!(Statut::depuis_code(s.code()), Some(s));
            assert!(s.code().bytes().all(|o| o.is_ascii_lowercase() || o == b'_'), "{}", s.code());
            assert!(!s.nom().contains(['\u{2013}', '\u{2014}']));
        }
        assert_eq!(Statut::depuis_code("Envoyee"), None);
        assert_eq!(Statut::depuis_code(""), None);
        assert!(!Statut::Envoyee.termine() && !Statut::Entretien.termine());
        assert!(Statut::Abandonnee.termine());
    }
}
