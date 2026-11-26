//! Les communes de France (19/11/2026) : leur nom, leurs codes postaux et leur code INSEE, pour chercher sur France
//! Travail autour d'une ville (son paramètre `lieux` attend le code INSEE de la commune : Sarrebourg, 57630, et non son
//! code postal, 57400 ; vu en vrai sur le site).
//!
//! 0.2.0 : chaque ligne porte aussi la position de la mairie de la commune (`;latitude;longitude`, de la même source ;
//! le centre géographique pour les six communes sans mairie connue : la mairie est au centre-ville, le centre
//! géographique d'une grande commune peut en être à plusieurs kilomètres), pour dire à
//! quelle distance de la ville cherchée se trouve une offre (voir [`centre`]) : la table fait environ 1,4 Mo.
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
        let mut p = l.split(';');
        Some(Commune { insee: p.next()?, codes_postaux: p.next()?, nom: p.next()? })
    })
}

/// 0.2.0 : le centre d'une commune (latitude, longitude), d'après son code INSEE ; rien si elle n'est pas dans la
/// table ou n'a pas de centre.
pub fn centre(insee: &str) -> Option<(f64, f64)> {
    static CENTRES: std::sync::OnceLock<std::collections::HashMap<&'static str, (f64, f64)>> =
        std::sync::OnceLock::new();
    CENTRES
        .get_or_init(|| {
            TABLE
                .lines()
                .filter_map(|l| {
                    let mut p = l.split(';');
                    let insee = p.next()?;
                    let (la, lo) = (p.nth(2)?.parse::<f64>().ok()?, p.next()?.parse::<f64>().ok()?);
                    Some((insee, (la, lo)))
                })
                .collect()
        })
        .get(insee)
        .copied()
}

/// Un nom remis au propre pour comparer : minuscules, sans accent, tirets et apostrophes en espaces, « st » et
/// « ste » écrits en entier.
pub fn cle_nom(t: &str) -> String {
    let s: String = simple(t)
        .chars()
        // (« œ » et « æ » sont déjà écrits « oe » et « ae » par `simple` : « Vandœuvre » et « Vandoeuvre » donnent
        // la même clé)
        .map(|c| match c {
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

/// Paris, Lyon et Marseille : le nom remis au propre, le début de leurs codes postaux (suivi du numéro de
/// l'arrondissement sur deux chiffres : « 75015 », « 69003 », « 13008 »), leur département et leur nombre
/// d'arrondissements. La table n'a que la commune entière (75056, 69123, 13055), avec les codes postaux de tous ses
/// arrondissements : « Paris 15 » est donc la commune de Paris.
const GRANDES_VILLES: [(&str, &str, u32, u32); 3] =
    [("paris", "750", 75, 20), ("lyon", "690", 69, 9), ("marseille", "130", 13, 16)];

/// Le numéro d'arrondissement écrit dans un mot remis au propre (« 15 », « 03 », « 15e », « 1er », « 3eme ») : le
/// numéro, et s'il est écrit avec sa fin (« e », « er », « eme »...).
fn numero_arrondissement(mot: &str) -> Option<(u32, bool)> {
    let chiffres = mot.chars().take_while(char::is_ascii_digit).count();
    if !(1..=2).contains(&chiffres) {
        return None;
    }
    let fin = &mot[chiffres..];
    if !matches!(fin, "" | "e" | "er" | "ere" | "eme" | "ieme" | "nd" | "nde") {
        return None;
    }
    Some((mot[..chiffres].parse().ok()?, !fin.is_empty()))
}

/// Les mots d'une ville tapée ou écrite par un site, remis au propre, sans ce qui n'est pas la commune.
struct MotsVille {
    /// Les mots qui restent (le nom, un code postal, un département), dans l'ordre
    mots: Vec<String>,
    /// « Cedex » était écrit (le code postal est alors souvent celui d'un service de courrier, absent de la table)
    cedex: bool,
    /// L'arrondissement de Paris, Lyon ou Marseille qui était écrit : la ville (son rang dans [`GRANDES_VILLES`]) et
    /// le numéro
    arrondissement: Option<(usize, u32)>,
}

/// Retire d'une ville « cedex » et son numéro (« Metz Cedex 1 »), et l'arrondissement de Paris, Lyon ou Marseille
/// (« Paris 15 », « 69 - LYON 03 », « Paris 15e Arrondissement », « Marseille 8ème ») : un nombre qui suit le nom
/// d'une de ces trois villes est un arrondissement s'il en a le numéro, pas un département. « Marseille 13 » et
/// « Lyon 69 » gardent leur nombre : c'est leur département.
fn mots_ville(t: &str) -> MotsVille {
    let cle = cle_nom(t);
    let mut mots: Vec<&str> = cle.split(' ').filter(|m| !m.is_empty()).collect();
    let nombre = |m: &str| m.chars().all(|c| c.is_ascii_digit());
    let mut cedex = false;
    if let Some(k) = mots.iter().position(|m| *m == "cedex") {
        cedex = true;
        mots.remove(k);
        // (le numéro du cedex : un chiffre, ou deux chiffres quand le département ou le code postal est écrit
        // ailleurs ; « Saint-Denis Cedex (93) » garde son département)
        let ailleurs = mots.iter().enumerate().any(|(i, m)| i != k && m.len() >= 2 && nombre(m));
        if mots
            .get(k)
            .is_some_and(|m| nombre(m) && (m.len() == 1 || (m.len() == 2 && (ailleurs || m.starts_with('0')))))
        {
            mots.remove(k);
        }
    }
    let mut arrondissement = None;
    // (le nom : le premier mot qui n'est pas un nombre ; devant lui, un code postal ou un département)
    if let Some(k) = mots.iter().position(|m| !nombre(m))
        && let Some(g) = GRANDES_VILLES.iter().position(|v| v.0 == mots[k])
    {
        let (_, _, dep, max) = GRANDES_VILLES[g];
        let mut garder = vec![true; mots.len()];
        let mut numero = None;
        let mut bon = true;
        for (i, m) in mots.iter().enumerate().skip(k + 1) {
            if matches!(*m, "arrondissement" | "arrondissements" | "arr" | "arrdt") {
                garder[i] = false;
            } else if let Some((n, ecrit)) = numero_arrondissement(m)
                && (1..=max).contains(&n)
                && (ecrit || n != dep)
            {
                garder[i] = false;
                numero = numero.or(Some(n));
            } else if !nombre(m) {
                // (un autre mot : une autre commune, « Paris-l'Hôpital », « Marseille-en-Beauvaisis »)
                bon = false;
                break;
            }
        }
        if bon {
            let mut i = 0;
            mots.retain(|_| {
                i += 1;
                garder[i - 1]
            });
            arrondissement = numero.map(|n| (g, n));
        }
    }
    MotsVille { mots: mots.into_iter().map(str::to_string).collect(), cedex, arrondissement }
}

/// Ce qu'une ville tapée dit d'elle.
struct VilleLue {
    /// Son nom remis au propre
    nom: String,
    /// Un code postal (5 chiffres)
    cp: Option<String>,
    /// Un département (« (57) », « 75 - PARIS ») : le premier écrit
    dep: Option<String>,
    cedex: bool,
}

fn lire_ville(t: &str) -> VilleLue {
    let m = mots_ville(t);
    let mut cp = None;
    let mut dep: Option<String> = None;
    let mut nom = Vec::new();
    for m in &m.mots {
        if m.len() == 5 && m.chars().all(|c| c.is_ascii_digit()) {
            cp = Some(m.clone());
        } else if (m.len() == 2 || m.len() == 3) && (m.chars().all(|c| c.is_ascii_digit()) || m == "2a" || m == "2b") {
            // (le premier département lu est gardé : dans « 57 - METZ 02 », le second nombre n'en est pas un)
            if dep.is_none() {
                dep = Some(m.to_ascii_uppercase());
            }
        } else {
            nom.push(m.as_str());
        }
    }
    VilleLue { nom: nom.join(" "), cp, dep, cedex: m.cedex }
}

/// Les communes de ce nom, de ce code postal et de ce département (ce qui est donné).
fn communes_de(nom: &str, cp: Option<&str>, dep: Option<&str>) -> Vec<Commune> {
    index()
        .iter()
        .filter(|(_, k)| nom.is_empty() || *k == nom)
        .map(|(c, _)| *c)
        .filter(|c| cp.is_none_or(|p| c.codes_postaux.split(',').any(|x| x == p)))
        .filter(|c| dep.is_none_or(|d| c.departement() == d))
        .collect()
}

/// La commune d'une ville tapée (« Sarrebourg », « 57400 Sarrebourg », « St-Avold », « Saint-Denis (93) ») ou écrite
/// par un site (« 75 - PARIS 15 », « Paris 15e Arrondissement », « 57000 Metz Cedex 1 ») ; rien si elle n'existe pas,
/// ou si plusieurs communes portent ce nom sans rien pour les départager. Paris, Lyon et Marseille : la commune
/// entière, quel que soit l'arrondissement.
pub fn trouver(ville: &str) -> Option<Commune> {
    let v = lire_ville(ville);
    if v.nom.is_empty() && v.cp.is_none() {
        return None;
    }
    let mut l = communes_de(&v.nom, v.cp.as_deref(), v.dep.as_deref());
    // (« 57045 Metz Cedex 1 » : le code d'un service de courrier n'est celui d'aucune commune ; son département reste)
    if l.is_empty()
        && v.cedex
        && !v.nom.is_empty()
        && let Some(d) = v.cp.as_deref().and_then(crate::page::departement_postal)
        && v.dep.as_deref().is_none_or(|x| x == d)
    {
        l = communes_de(&v.nom, None, Some(&d));
    }
    // (« Metz Cedex 15 », sans rien d'autre : ce nombre était le numéro du cedex, pas un département)
    if l.is_empty() && v.cedex && !v.nom.is_empty() && v.cp.is_none() && v.dep.is_some() {
        l = communes_de(&v.nom, None, None);
    }
    match l.as_slice() {
        [c] => Some(*c),
        _ => None,
    }
}

/// Les communes de ce nom (pour dire lesquelles existent quand il y en a plusieurs).
pub fn homonymes(ville: &str) -> Vec<Commune> {
    let nom = lire_ville(ville).nom;
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
/// Une ligne par commune. Un arrondissement de Paris, Lyon ou Marseille (« Paris 15 », « Lyon 3e ») : la ville, avec
/// le code postal de l'arrondissement.
pub fn proposer(tape: &str, max: usize) -> Vec<Proposition> {
    let lus = mots_ville(tape);
    let mut cp: Option<String> = None;
    let mut mots = Vec::new();
    for m in &lus.mots {
        if (1..=5).contains(&m.len()) && m.chars().all(|c| c.is_ascii_digit()) && cp.is_none() {
            cp = Some(m.clone());
        } else {
            mots.push(m.as_str());
        }
    }
    // (« Paris 15 », « Lyon 3e » : le code postal de l'arrondissement, « 75015 », « 69003 »)
    if cp.is_none()
        && let Some((g, n)) = lus.arrondissement
    {
        cp = Some(format!("{}{n:02}", GRANDES_VILLES[g].1));
    }
    let nom = mots.join(" ");
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

    /// 0.2.0 : le centre des communes, et les noms toujours lus sans leur position.
    #[test]
    fn centre_des_communes() {
        let (la, lo) = centre("57630").expect("Sarrebourg");
        assert!((la - 48.73).abs() < 0.02 && (lo - 7.05).abs() < 0.02, "{la} {lo}");
        let (la, lo) = centre("57463").expect("Metz");
        assert!((la - 49.12).abs() < 0.02 && (lo - 6.18).abs() < 0.02, "{la} {lo}");
        assert_eq!(centre("00000"), None);
        assert_eq!(centre(""), None);
        assert!(communes().all(|c| !c.nom.contains(';') && centre(c.insee).is_some()));
        assert_eq!(trouver("Sarrebourg").map(|c| c.nom), Some("Sarrebourg"));
    }

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
        assert!(TABLE.len() < 1_600_000);
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

    /// Audit du 26/11/2026 : « œ » et « æ » s'écrivent aussi « oe » et « ae » ; les deux écritures trouvent la
    /// commune, tapées ou écrites par un site.
    #[test]
    fn oe_et_ae_dans_les_noms() {
        for (ville, insee) in [
            ("Vandœuvre-lès-Nancy", "54547"),
            ("Vandoeuvre-lès-Nancy", "54547"),
            ("VANDOEUVRE LES NANCY", "54547"),
            ("VANDŒUVRE-LÈS-NANCY", "54547"),
            ("54 - VANDOEUVRE LES NANCY", "54547"),
            ("Schœneck", "57638"),
            ("Schoeneck", "57638"),
            ("Œting", "57521"),
            ("Oeting", "57521"),
            ("OETING (57)", "57521"),
        ] {
            assert_eq!(trouver(ville).map(|c| c.insee), Some(insee), "{ville}");
        }
        assert_eq!(cle_nom("Vandœuvre"), cle_nom("Vandoeuvre"));
        assert_eq!(cle_nom("Œting"), "oeting");
        assert_eq!(cle_nom("Ældre"), "aeldre");
        assert_eq!(textes("vandoeuvre")[0], "Vandœuvre-lès-Nancy (54500)");
        assert_eq!(textes("Vandœ")[0], "Vandœuvre-lès-Nancy (54500)");
        assert_eq!(textes("oeting")[0], "Œting (57600)");
        // (contre-épreuve : l'ancienne écriture sans « e » ne trouve plus rien, et les autres noms ne bougent pas)
        assert_eq!(trouver("Vandouvre-lès-Nancy"), None);
        assert_eq!(cle_nom("L'Haÿ-les-Roses"), "l hay les roses");
        assert_eq!(trouver("L'Haÿ-les-Roses").map(|c| c.insee), Some("94038"));
        // (chaque commune de la table se retrouve toujours par son nom et son premier code postal)
        for c in communes().filter(|c| c.nom.contains(['œ', 'Œ', 'æ', 'Æ'])) {
            let cp = c.codes_postaux.split(',').next().unwrap();
            assert_eq!(trouver(&format!("{} ({cp})", c.nom)).map(|x| x.insee), Some(c.insee), "{}", c.nom);
            let deux = c.nom.replace('œ', "oe").replace('Œ', "Oe").replace('æ', "ae").replace('Æ', "Ae");
            assert_eq!(trouver(&format!("{deux} ({cp})")).map(|x| x.insee), Some(c.insee), "{deux}");
        }
    }

    /// Audit du 26/11/2026 : les arrondissements de Paris, Lyon et Marseille. La table n'a que la commune entière :
    /// c'est elle qui est trouvée, et la distance se calcule depuis son centre.
    #[test]
    fn arrondissements_de_paris_lyon_marseille() {
        for (ville, insee) in [
            ("75 - PARIS 15", "75056"),
            ("69 - LYON 03", "69123"),
            ("13 - MARSEILLE 08", "13055"),
            ("13 - MARSEILLE 13", "13055"),
            ("Paris 15e Arrondissement", "75056"),
            ("PARIS 15E ARRONDISSEMENT", "75056"),
            ("Paris 1er", "75056"),
            ("Paris 1er Arrondissement (75)", "75056"),
            ("Lyon 3ème", "69123"),
            ("Lyon 3e arrondissement", "69123"),
            ("Lyon 03", "69123"),
            ("Lyon 3", "69123"),
            ("Marseille 8ème arrondissement", "13055"),
            ("Marseille 13", "13055"),
            ("Marseille 13e", "13055"),
            ("Paris 15", "75056"),
            ("Paris (15e)", "75056"),
            ("75015 Paris 15", "75056"),
            ("Paris 15 (75015)", "75056"),
            ("Paris-15", "75056"),
            // (contre-épreuves : ce qui marchait)
            ("Paris", "75056"),
            ("Paris (75)", "75056"),
            ("Paris (75015)", "75056"),
            ("75 - PARIS", "75056"),
            ("Lyon", "69123"),
            ("Lyon (69)", "69123"),
            ("69003 Lyon", "69123"),
            ("Marseille (13)", "13055"),
            ("Marseille (13008)", "13055"),
            ("Paris-l'Hôpital", "71343"),
            ("Paris-l'Hôpital (71)", "71343"),
            ("Marseille-en-Beauvaisis", "60387"),
        ] {
            assert_eq!(trouver(ville).map(|c| c.insee), Some(insee), "{ville}");
        }
        for insee in ["75056", "69123", "13055"] {
            assert!(centre(insee).is_some(), "{insee}");
        }
        // (un arrondissement qui n'existe pas, une ville du mauvais département : toujours rien)
        assert_eq!(trouver("Paris 25e"), None);
        assert_eq!(trouver("Lyon 12"), None);
        assert_eq!(trouver("Paris (69)"), None);
        assert_eq!(trouver("69 - PARIS 15"), None);
        // (le nombre qui suit une autre ville reste un département)
        assert_eq!(trouver("Saint-Denis 93").map(|c| c.insee), Some("93066"));
        assert_eq!(trouver("Metz 15"), None);
        // (le premier département lu est gardé)
        assert_eq!(trouver("57 - METZ 02").map(|c| c.insee), Some("57463"));
        assert_eq!(trouver("93 - SAINT DENIS 02").map(|c| c.insee), Some("93066"));
        assert_eq!(homonymes("75 - PARIS 15").len(), 1);
        // Les propositions pendant la frappe : la ville, avec le code postal de l'arrondissement
        assert_eq!(textes("Paris 15"), ["Paris (75015)"]);
        assert_eq!(textes("Lyon 3"), ["Lyon (69003)"]);
        assert_eq!(textes("Lyon 3e"), ["Lyon (69003)"]);
        assert_eq!(textes("Paris 1er arrondissement"), ["Paris (75001)"]);
        assert_eq!(textes("marseille 8ème"), ["Marseille (13008)"]);
        assert_eq!(textes("Paris 16"), ["Paris (75016)"]);
        for t in ["Paris 15", "Lyon 3", "marseille 8ème"] {
            let p = proposer(t, 1)[0];
            assert_eq!(trouver(&p.texte()).map(|c| c.insee), Some(p.insee), "{t}");
        }
        // (contre-épreuves : un code postal tapé en partie reste un code postal)
        assert_eq!(textes("Paris 7501")[0], "Paris (75010)");
        assert_eq!(textes("Paris 75")[0], "Paris (75001)");
        assert_eq!(textes("Marseille 13")[0], "Marseille (13001)");
        assert_eq!(textes("Lyon 69")[0], "Lyon (69001)");
        assert_eq!(textes("Lyon 69003"), ["Lyon (69003)"]);
        assert!(textes("Paris 25").is_empty() && textes("Lyon 12").is_empty());
    }

    /// Audit du 26/11/2026 : « Cedex » et son numéro ne font pas partie du nom.
    #[test]
    fn cedex_retire() {
        for (ville, insee) in [
            ("57000 Metz Cedex 1", "57463"),
            ("57000 METZ CEDEX 01", "57463"),
            ("Metz Cedex", "57463"),
            ("Metz cedex 3", "57463"),
            ("54 - NANCY CEDEX", "54395"),
            ("Saint-Denis Cedex (93)", "93066"),
            // (le code d'un service de courrier, absent de la table : le département du code départage)
            ("57045 Metz Cedex 1", "57463"),
            ("93207 Saint-Denis Cedex", "93066"),
            ("75 - PARIS CEDEX 15", "75056"),
            ("Metz Cedex 15", "57463"),
            ("METZ CEDEX 03", "57463"),
        ] {
            assert_eq!(trouver(ville).map(|c| c.insee), Some(insee), "{ville}");
        }
        assert_eq!(textes("Metz Cedex 1")[0], "Metz (57000)");
        assert_eq!(textes("57000 Metz Cedex")[0], "Metz (57000)");
        // (contre-épreuves : sans « cedex », un code postal qui n'est pas celui de la ville ne donne rien ; avec,
        // un code d'un autre département non plus, et les homonymes restent sans réponse)
        assert_eq!(trouver("57045 Metz"), None);
        assert_eq!(trouver("54045 Metz Cedex"), None);
        assert_eq!(trouver("Saint-Denis Cedex"), None);
        assert_eq!(trouver("Cedex"), None);
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
