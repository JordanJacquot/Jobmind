//! Les communes de France (19/11/2026) : leur nom, leurs codes postaux et leur code INSEE, pour chercher sur France
//! Travail autour d'une ville (son paramètre `lieux` attend le code INSEE de la commune : Sarrebourg, 57630, et non son
//! code postal, 57400 ; vu en vrai sur le site).
//!
//! La table (`communes.txt`, une ligne par commune : `code INSEE;codes postaux;nom`, environ 860 Ko) vient de l'API
//! Découpage administratif (geo.api.gouv.fr, données de l'INSEE et de La Poste), sous Licence Ouverte Etalab 2.0 ;
//! elle est embarquée dans l'exe, jamais téléchargée pendant que Jobmind tourne.
//!
//! La recherche est tolérante : majuscules, accents, tirets, apostrophes, « St » pour « Saint » ; un code postal
//! (« 57400 Sarrebourg ») ou un département (« Sarrebourg (57) ») départage plusieurs communes du même nom. Plusieurs
//! communes possibles sans rien pour les départager : rien (l'appelant fait sans).

use crate::page::simple;

const TABLE: &str = include_str!("communes.txt");

/// Une commune de la table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Commune {
    /// Le code INSEE (« 57630 »)
    pub insee: &'static str,
    /// Ses codes postaux, séparés par des virgules (« 57400 »)
    pub codes_postaux: &'static str,
    pub nom: &'static str,
}

impl Commune {
    /// Son département (« 57 », « 2A », « 971 »).
    pub fn departement(&self) -> &'static str {
        if self.insee.starts_with("97") { &self.insee[..3] } else { &self.insee[..2] }
    }
}

fn communes() -> impl Iterator<Item = Commune> {
    TABLE.lines().filter_map(|l| {
        let mut p = l.splitn(3, ';');
        Some(Commune { insee: p.next()?, codes_postaux: p.next()?, nom: p.next()? })
    })
}

/// Un nom remis au propre pour comparer : minuscules, sans accent, tirets et apostrophes en espaces, « st » et
/// « ste » écrits en entier.
pub fn cle_nom(t: &str) -> String {
    let s: String = simple(t)
        .chars()
        .map(|c| match c {
            'œ' => 'o',
            'æ' => 'a',
            'ÿ' => 'y',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    s.split_whitespace()
        .map(|m| match m {
            "st" => "saint",
            "ste" => "sainte",
            m => m,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Ce qu'une ville tapée dit d'elle : son nom, un code postal (5 chiffres), un département (« (57) »).
fn lire_ville(t: &str) -> (String, Option<String>, Option<String>) {
    let mut cp = None;
    let mut dep = None;
    let mut nom = Vec::new();
    for m in t.split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')') {
        if m.is_empty() {
            continue;
        }
        if m.len() == 5 && m.chars().all(|c| c.is_ascii_digit()) {
            cp = Some(m.to_string());
        } else if (m.len() == 2 || m.len() == 3)
            && (m.chars().all(|c| c.is_ascii_digit()) || ["2a", "2b"].contains(&m.to_ascii_lowercase().as_str()))
        {
            dep = Some(m.to_ascii_uppercase());
        } else {
            nom.push(m);
        }
    }
    (cle_nom(&nom.join(" ")), cp, dep)
}

/// La commune d'une ville tapée (« Sarrebourg », « 57400 Sarrebourg », « St-Avold », « Saint-Denis (93) ») ; rien si
/// elle n'existe pas, ou si plusieurs communes portent ce nom sans rien pour les départager.
pub fn trouver(ville: &str) -> Option<Commune> {
    let (nom, cp, dep) = lire_ville(ville);
    if nom.is_empty() && cp.is_none() {
        return None;
    }
    let l: Vec<Commune> = communes()
        .filter(|c| nom.is_empty() || cle_nom(c.nom) == nom)
        .filter(|c| cp.as_deref().is_none_or(|p| c.codes_postaux.split(',').any(|x| x == p)))
        .filter(|c| dep.as_deref().is_none_or(|d| c.departement() == d))
        .collect();
    match l.as_slice() {
        [c] => Some(*c),
        _ => None,
    }
}

/// Les communes de ce nom (pour dire lesquelles existent quand il y en a plusieurs).
pub fn homonymes(ville: &str) -> Vec<Commune> {
    let (nom, _, _) = lire_ville(ville);
    communes().filter(|c| !nom.is_empty() && cle_nom(c.nom) == nom).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validée par des communes connues (codes INSEE et postaux publiés par l'INSEE et La Poste).
    #[test]
    fn communes_connues() {
        for (ville, insee, cp) in [
            ("Sarrebourg", "57630", "57400"),
            ("Metz", "57463", "57000"),
            ("Nancy", "54395", "54000"),
            ("Strasbourg", "67482", "67000"),
            ("SARREBOURG", "57630", "57400"),
            ("57400 Sarrebourg", "57630", "57400"),
            ("St-Avold", "57606", "57500"),
            ("saint avold", "57606", "57500"),
            ("Pont-à-Mousson", "54431", "54700"),
            ("pont a mousson", "54431", "54700"),
            ("Thionville (57)", "57672", "57100"),
        ] {
            let c = trouver(ville).unwrap_or_else(|| panic!("{ville}"));
            assert_eq!(c.insee, insee, "{ville}");
            assert!(c.codes_postaux.split(',').any(|x| x == cp), "{ville} {}", c.codes_postaux);
        }
        assert_eq!(trouver("Metz").unwrap().departement(), "57");
        assert!(communes().count() > 34_000);
        assert!(TABLE.len() < 1_000_000);
    }

    /// Plusieurs communes du même nom : le code postal ou le département départage ; sans eux, rien.
    #[test]
    fn homonymes_departages() {
        assert!(homonymes("Saint-Denis").len() > 1);
        assert_eq!(trouver("Saint-Denis"), None);
        assert_eq!(trouver("Saint-Denis (93)").unwrap().insee, "93066");
        assert_eq!(trouver("93200 Saint-Denis").unwrap().insee, "93066");
        assert_eq!(trouver("Atlantide-sur-Mer"), None);
        assert_eq!(trouver(""), None);
        assert_eq!(trouver("()"), None);
        assert_eq!(cle_nom("L'Hôpital"), "l hopital");
    }
}
