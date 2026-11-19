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
    let l: Vec<Commune> = index()
        .iter()
        .filter(|(_, k)| nom.is_empty() || *k == nom)
        .map(|(c, _)| *c)
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
    index().iter().filter(|(_, k)| !nom.is_empty() && *k == nom).map(|(c, _)| *c).collect()
}

/// Le nom seul d'une ville tapée ou prise dans les propositions : « Sarrebourg (57400) », « Metz (57) » : « Sarrebourg »,
/// « Metz » (pour les sites qui cherchent par nom, et les formulaires de candidature). Un texte sans code postal ni
/// département entre parenthèses à la fin est rendu tel quel (sans les espaces autour).
pub fn nom_seul(ville: &str) -> String {
    let t = ville.trim();
    if let Some(i) = t.rfind('(')
        && let Some(dedans) = t[i + 1..].strip_suffix(')')
    {
        let d = dedans.trim();
        if (2..=5).contains(&d.len())
            && d.chars().all(|c| c.is_ascii_alphanumeric())
            && d.chars().any(|c| c.is_ascii_digit())
            && !t[..i].trim().is_empty()
        {
            return t[..i].trim().to_string();
        }
    }
    t.to_string()
}

/// Une commune proposée pendant la frappe : « Sarrebourg (57400) ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proposition {
    pub nom: &'static str,
    /// Le code postal montré : celui qui commence comme ce qui est tapé, sinon le premier de la commune
    pub code_postal: &'static str,
    pub insee: &'static str,
}

impl Proposition {
    /// Ce qui va dans la case : « Sarrebourg (57400) » ([`trouver`] le relit et reprend cette commune).
    pub fn texte(&self) -> String {
        format!("{} ({})", self.nom, self.code_postal)
    }
}

/// Au plus ce nombre de propositions.
pub const PROPOSITIONS_MAX: usize = 8;

/// Chaque commune, avec son nom remis au propre (fait une fois, au premier appel : environ 35 000 noms).
fn index() -> &'static [(Commune, String)] {
    static INDEX: std::sync::OnceLock<Vec<(Commune, String)>> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| communes().map(|c| (c, cle_nom(c.nom))).collect())
}

/// Les communes proposées pour ce qui est tapé dans une case de ville (au plus `max`) : dès 2 lettres ou 2 chiffres.
/// Les plus pertinentes d'abord : le nom exact, puis le nom qui commence par ce qui est tapé, puis un mot du nom qui
/// commence ainsi (« avold » : Saint-Avold), puis le nom qui le contient ; sans tenir compte des majuscules, des
/// accents, des tirets ni des apostrophes, « st » pour « saint ». À pertinence égale, les villes à plusieurs codes
/// postaux (les grandes) d'abord, puis le nom le plus court, puis l'ordre alphabétique. Un code postal tapé, même en partie (« 574 »,
/// « Sarrebourg 57400 », « Sarrebourg (57400) ») : seulement les communes d'un code qui commence ainsi, ce code montré.
/// Une ligne par commune.
pub fn proposer(tape: &str, max: usize) -> Vec<Proposition> {
    let mut cp: Option<String> = None;
    let mut mots = Vec::new();
    for m in tape.split(|c: char| c.is_whitespace() || c == ',' || c == '(' || c == ')') {
        if m.is_empty() {
            continue;
        }
        if (1..=5).contains(&m.len()) && m.chars().all(|c| c.is_ascii_digit()) && cp.is_none() {
            cp = Some(m.to_string());
        } else {
            mots.push(m);
        }
    }
    let nom = cle_nom(&mots.join(" "));
    let cp = cp.filter(|p| p.len() >= 2 || !nom.is_empty());
    if nom.chars().count() < 2 && cp.as_ref().is_none_or(|p| p.len() < 2) {
        return Vec::new();
    }
    if max == 0 {
        return Vec::new();
    }
    let dans_mot = format!(" {nom}");
    // (pertinence du nom, code exact, nombre de codes postaux, nom remis au propre, la proposition)
    let mut l: Vec<(u8, bool, usize, &str, Proposition)> = Vec::new();
    for (c, k) in index() {
        let rang = if nom.is_empty() || *k == nom {
            0
        } else if k.starts_with(&nom) {
            1
        } else if k.contains(&dans_mot) {
            2
        } else if k.contains(&nom) {
            3
        } else {
            continue;
        };
        let code = match &cp {
            Some(p) => match c.codes_postaux.split(',').find(|x| x.starts_with(p.as_str())) {
                Some(x) => x,
                None => continue,
            },
            None => c.codes_postaux.split(',').next().unwrap_or_default(),
        };
        let exact = cp.as_deref().is_some_and(|p| p == code);
        let n = c.codes_postaux.split(',').count();
        l.push((rang, !exact, n, k.as_str(), Proposition { nom: c.nom, code_postal: code, insee: c.insee }));
    }
    l.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.cmp(&b.1))
            .then(if nom.is_empty() { a.4.code_postal.cmp(b.4.code_postal) } else { std::cmp::Ordering::Equal })
            .then(b.2.cmp(&a.2))
            .then(a.3.len().cmp(&b.3.len()))
            .then(a.3.cmp(b.3))
            .then(a.4.code_postal.cmp(b.4.code_postal))
    });
    l.into_iter().take(max).map(|x| x.4).collect()
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

    fn textes(tape: &str) -> Vec<String> {
        proposer(tape, PROPOSITIONS_MAX).iter().map(Proposition::texte).collect()
    }

    /// Les villes proposées pendant la frappe : le nom exact, puis le début du nom, puis un mot du nom, puis le reste ;
    /// sans majuscules, accents ni tirets, « st » pour « saint » ; 8 au plus.
    #[test]
    fn propositions_pendant_la_frappe() {
        assert_eq!(textes("Sarreb")[0], "Sarrebourg (57400)");
        assert_eq!(textes("SARREBOURG")[0], "Sarrebourg (57400)");
        assert_eq!(textes("metz")[0], "Metz (57000)");
        assert_eq!(textes("st avold")[0], "Saint-Avold (57500)");
        assert_eq!(textes("St-Avold")[0], "Saint-Avold (57500)");
        assert_eq!(textes("avold")[0], "Saint-Avold (57500)", "un mot du nom");
        assert_eq!(textes("pont a mous")[0], "Pont-à-Mousson (54700)");
        assert_eq!(textes("Pont-à-Mousson")[0], "Pont-à-Mousson (54700)");
        // (le début du nom passe avant un mot du nom, qui passe avant le milieu d'un mot)
        let l = proposer("sarre", 50);
        let rang = |p: &Proposition| {
            let k = cle_nom(p.nom);
            if k == "sarre" {
                0
            } else if k.starts_with("sarre") {
                1
            } else if k.contains(" sarre") {
                2
            } else {
                3
            }
        };
        assert!(l.windows(2).all(|w| rang(&w[0]) <= rang(&w[1])), "{l:?}");
        assert!(l.iter().all(|p| cle_nom(p.nom).contains("sarre")));
        // (à pertinence égale, les grandes villes, à plusieurs codes postaux, d'abord)
        assert_eq!(textes("nan")[..2], ["Nantes (44000)", "Nancy (54000)"]);
        assert_eq!(textes("pari")[0], "Paris (75001)");
        // 8 au plus, et rien avant 2 lettres
        assert_eq!(textes("sa").len(), PROPOSITIONS_MAX);
        assert!(textes("s").is_empty() && textes("").is_empty() && textes("  - ").is_empty() && textes("5").is_empty());
        assert!(textes("zzqx").is_empty());
        assert!(proposer("metz", 0).is_empty());
    }

    /// Un code postal tapé, même en partie : les communes de ce code, ce code montré.
    #[test]
    fn propositions_par_code_postal() {
        let l = proposer("574", 30);
        assert_eq!(l.len(), 30);
        assert!(l.iter().all(|p| p.code_postal.starts_with("574")), "{l:?}");
        assert!(l.windows(2).all(|w| w[0].code_postal <= w[1].code_postal), "{l:?}");
        let l = proposer("57400", 50);
        assert!(l.iter().all(|p| p.code_postal == "57400"));
        assert!(l.iter().any(|p| p.nom == "Sarrebourg"));
        assert_eq!(textes("Sarrebourg 574"), ["Sarrebourg (57400)"]);
        assert_eq!(textes("Sarrebourg (57400)"), ["Sarrebourg (57400)"]);
        // (une ville à plusieurs codes : le code tapé est celui montré)
        assert_eq!(textes("Metz 57070")[0], "Metz (57070)");
        assert_eq!(textes("Paris 7501")[0], "Paris (75010)");
        assert!(textes("Sarrebourg 13").is_empty(), "pas de Sarrebourg dans les Bouches-du-Rhône");
    }

    /// Ce qui est mis dans la case (« Nom (code postal) ») est relu par [`trouver`], qui reprend la même commune,
    /// homonymes compris (Saint-Jean, Saint-Denis) ; vérifié aussi sur des communes prises dans toute la table.
    #[test]
    fn la_proposition_choisie_est_comprise() {
        for nom in ["Saint-Aubin", "Beaulieu", "Sainte-Colombe"] {
            let l = proposer(nom, 50);
            let exacts: Vec<&Proposition> = l.iter().filter(|p| p.nom == nom).collect();
            assert!(exacts.len() >= 10, "{l:?}");
            assert!(l.iter().take(exacts.len()).all(|p| p.nom == nom), "les homonymes exacts d'abord");
            for p in exacts {
                assert_eq!(trouver(&p.texte()).map(|c| c.insee), Some(p.insee), "{}", p.texte());
            }
            assert_eq!(trouver(nom), None, "sans code postal, rien pour les départager");
        }
        assert_eq!(textes("Saint-Jean")[0], "Saint-Jean (31240)");
        assert_eq!(trouver("Saint-Jean (31240)").unwrap().insee, "31488");
        for p in proposer("Saint-Denis", 8) {
            assert_eq!(trouver(&p.texte()).map(|c| c.insee), Some(p.insee), "{}", p.texte());
        }
        assert_eq!(trouver("Saint-Denis (93200)").unwrap().insee, "93066");
        // (une commune sur 300 de la table, chacun de ses codes postaux)
        let mut n = 0;
        for c in communes().step_by(300) {
            for cp in c.codes_postaux.split(',') {
                let t = Proposition { nom: c.nom, code_postal: cp, insee: c.insee }.texte();
                assert_eq!(trouver(&t).map(|x| x.insee), Some(c.insee), "{t}");
                n += 1;
            }
        }
        assert!(n > 100);
        // (la forme proposée est aussi celle que les sites qui cherchent par nom reçoivent sans le code)
        assert_eq!(nom_seul("Sarrebourg (57400)"), "Sarrebourg");
        assert_eq!(nom_seul(" Metz (57) "), "Metz");
        assert_eq!(nom_seul("Ajaccio (2A)"), "Ajaccio");
        assert_eq!(nom_seul("Sarrebourg"), "Sarrebourg");
        assert_eq!(nom_seul("Fontaine (Isère)"), "Fontaine (Isère)");
        assert_eq!(nom_seul("(57400)"), "(57400)");
    }

    /// Rapide : la table remise au propre une fois, puis chaque lettre tapée parcourt la table en mémoire.
    #[test]
    fn propositions_rapides() {
        let _ = proposer("me", 8);
        let t = std::time::Instant::now();
        for tape in ["sa", "sai", "sain", "saint", "saint j", "saint je", "574", "le", "la", "mo"] {
            let _ = proposer(tape, PROPOSITIONS_MAX);
        }
        // (10 frappes ; en construction de test, sans optimisation : bien moins en vrai)
        let ms = t.elapsed().as_millis();
        assert!(ms < 500, "{ms} ms");
    }
}
