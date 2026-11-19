//! La note de pertinence d'une offre (étape 6 du plan) et sa raison en une ligne, calculées ici, en Rust, jamais par
//! l'IA : d'après « Ce que je cherche » (les postes, le lieu, les contrats, le télétravail, « aucun diplôme requis »,
//! « débutant accepté », le salaire minimum), le CV (ses mots, son plus haut diplôme) et l'offre (son titre, son
//! texte, son lieu, ce que [`crate::lecture`] y a lu).
//!
//! La note va de 0 à 100 :
//! - le poste : 35 si tous les mots d'un des postes cherchés sont dans le titre, 22 si la moitié, 14 s'ils ne sont que
//!   dans le texte ; aucun poste nulle part : l'offre est **hors sujet** (rangée à part, jamais effacée) ; aucun poste
//!   cherché (tous les métiers du lieu) : 25 pour toutes, jamais hors sujet ;
//! - le lieu : 20 dans la ville ou le rayon (un peu moins vers le bord), 6 jusqu'au double du rayon, 0 au-delà ; sans
//!   position connue, le département de la ville cherchée (12 dedans, 2 dehors) ; un département ou une région
//!   cherchés : 20 dedans, 0 dehors ; partout : 15 ; une offre en télétravail complet compte comme dans le rayon ;
//! - le contrat 10, le télétravail 5, le diplôme 10, l'expérience 10, le salaire 5, la ressemblance avec le CV 5 ;
//! - ce qui va contre « Ce que je cherche » retire des points : un diplôme demandé quand Jordan cherche « aucun diplôme
//!   requis » (ou plus haut que celui de son CV), une expérience exigée quand il cherche « débutant accepté » (20 chacun),
//!   un contrat qu'il ne veut pas (10), un salaire sous son minimum (5).
//!
//! Le niveau montré : « Très proche » (85 et plus), « Proche » (65 et plus), « Un peu », « Hors sujet ».

use crate::lecture::{self, Diplome, Experience};
use crate::lieux;
use crate::offre::{TeleOffre, cle_ville, distance_km, normaliser};
use crate::recherche::{Contrat, Lieu, Recherche, Teletravail};

/// La version du calcul : une note faite par une autre version est refaite.
pub const VERSION: u32 = 1;
/// La part du poste quand « Ce que je cherche » n'a aucun poste (tous les métiers du lieu) : ni hors sujet, ni « très
/// proche » sans le reste (lieu, diplôme, expérience).
pub const POSTE_SANS_MOTS: u32 = 25;

/// Le niveau montré à Jordan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Niveau {
    HorsSujet,
    UnPeu,
    Proche,
    TresProche,
}

impl Niveau {
    pub fn code(self) -> &'static str {
        match self {
            Niveau::TresProche => "tres_proche",
            Niveau::Proche => "proche",
            Niveau::UnPeu => "un_peu",
            Niveau::HorsSujet => "hors_sujet",
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Niveau::TresProche => "Très proche",
            Niveau::Proche => "Proche",
            Niveau::UnPeu => "Un peu",
            Niveau::HorsSujet => "Hors sujet",
        }
    }

    /// Le niveau d'une note (pas hors sujet).
    pub fn de(score: u8) -> Niveau {
        match score {
            85.. => Niveau::TresProche,
            65.. => Niveau::Proche,
            _ => Niveau::UnPeu,
        }
    }
}

/// Ce que Jordan cherche, prêt pour noter.
#[derive(Debug, Clone)]
pub struct Profil<'a> {
    pub recherche: &'a Recherche,
    /// Les mots du CV (remis au propre)
    pub mots_cv: Vec<String>,
    /// Le plus haut diplôme lu dans le CV
    pub diplome_cv: Option<Diplome>,
    /// Le centre de la ville cherchée, s'il est connu (la position médiane des offres qui y sont)
    pub centre: Option<(f64, f64)>,
    /// Le département de la ville cherchée, s'il est connu (celui des offres qui y sont)
    pub departement_ville: Option<String>,
}

impl<'a> Profil<'a> {
    pub fn nouveau(recherche: &'a Recherche, cv: &str) -> Profil<'a> {
        let mut mots_cv: Vec<String> = normaliser(cv).split(' ').filter(|m| m.len() >= 3).map(str::to_string).collect();
        mots_cv.sort();
        mots_cv.dedup();
        Profil {
            recherche,
            mots_cv,
            diplome_cv: lecture::diplome_le_plus_haut(cv),
            centre: None,
            departement_ville: None,
        }
    }
}

/// Ce qu'il faut savoir d'une offre pour la noter.
#[derive(Debug, Clone, Default)]
pub struct OffreANoter<'a> {
    pub titre: &'a str,
    pub description: &'a str,
    pub lieu: &'a str,
    pub departement: Option<&'a str>,
    pub region: Option<&'a str>,
    pub point: Option<(f64, f64)>,
    pub contrat: Option<Contrat>,
    pub teletravail: Option<TeleOffre>,
    pub salaire: &'a str,
    pub diplome: Option<Diplome>,
    pub experience: Option<Experience>,
}

/// La note d'une offre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub score: u8,
    pub niveau: Niveau,
    /// La raison en une ligne (« Aucun diplôme demandé, à 12 km, débutant accepté »)
    pub raison: String,
}

/// Les petits mots d'un poste qui ne disent rien du métier.
const PETITS: [&str; 14] = ["de", "d", "du", "des", "la", "le", "les", "l", "en", "et", "a", "au", "h", "f"];

fn mots_utiles(t: &str) -> Vec<String> {
    normaliser(t).split(' ').filter(|m| !m.is_empty() && !PETITS.contains(m)).map(str::to_string).collect()
}

/// Deux mots sont le même : égaux, au pluriel, ou les mêmes six premières lettres (« preparateur », « preparatrice »).
fn meme_mot(a: &str, b: &str) -> bool {
    a == b
        || a.strip_suffix('s') == Some(b)
        || b.strip_suffix('s') == Some(a)
        || (a.len() >= 6 && b.len() >= 6 && a.as_bytes()[..6] == b.as_bytes()[..6])
}

/// La part des mots du poste présents dans ces mots (0 à 1).
fn part(poste: &[String], mots: &[String]) -> f64 {
    if poste.is_empty() {
        return 0.0;
    }
    let n = poste.iter().filter(|p| mots.iter().any(|m| meme_mot(p, m))).count();
    n as f64 / poste.len() as f64
}

/// Un salaire lu dans un texte (« 21 800 à 23 500 € par an », « 1 800 € brut/mois », « 11,88 €/h ») : le plus haut,
/// en euros bruts par mois.
pub fn salaire_mensuel(t: &str) -> Option<u32> {
    let n = normaliser(t);
    let bas = t.to_lowercase();
    // Les nombres : des chiffres, avec des espaces ou des points pour les milliers, une virgule pour les centimes
    let mut nombres: Vec<f64> = Vec::new();
    let cs: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if !cs[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let mut entier = String::new();
        let mut decimales = String::new();
        while i < cs.len() {
            let c = cs[i];
            if c.is_ascii_digit() {
                entier.push(c);
                i += 1;
            } else if matches!(c, ' ' | '\u{a0}' | '\u{202f}' | '.')
                && i + 3 < cs.len() + 1
                && cs[i + 1..].iter().take(3).filter(|x| x.is_ascii_digit()).count() == 3
                && cs.get(i + 4).is_none_or(|x| !x.is_ascii_digit())
            {
                i += 1;
            } else if c == ',' && cs.get(i + 1).is_some_and(char::is_ascii_digit) {
                i += 1;
                while i < cs.len() && cs[i].is_ascii_digit() {
                    decimales.push(cs[i]);
                    i += 1;
                }
                break;
            } else {
                break;
            }
        }
        let mut x: f64 = entier.parse().unwrap_or(0.0);
        if !decimales.is_empty() {
            x += format!("0.{decimales}").parse::<f64>().unwrap_or(0.0);
        }
        // « 30k€ », « 30 k »
        let apres: String = cs[i..].iter().take(3).collect::<String>().to_lowercase();
        if apres.trim_start().starts_with('k') {
            x *= 1000.0;
        }
        if x > 0.0 {
            nombres.push(x);
        }
    }
    let haut = nombres.into_iter().fold(0.0f64, f64::max);
    if haut <= 0.0 {
        return None;
    }
    let par = |mots: &[&str]| mots.iter().any(|m| format!(" {n} ").contains(&format!(" {m} ")));
    let mensuel = if par(&["an", "ans", "annuel", "annuels", "annee", "brut annuel", "k"]) || bas.contains("/an") {
        haut / 12.0
    } else if par(&["h", "heure", "horaire", "heures"]) || bas.contains("/h") {
        haut * 151.67
    } else if par(&["mois", "mensuel", "mensuels"]) || bas.contains("/mois") {
        haut
    } else if haut > 8000.0 {
        haut / 12.0
    } else if haut < 100.0 {
        haut * 151.67
    } else {
        haut
    };
    (100.0..=100_000.0).contains(&mensuel).then_some(mensuel.round() as u32)
}

/// Le texte d'un lieu pour une raison : « en Moselle » ne se dit pas pour tous ; on dit « dans ton département ».
fn ville_nom(l: &Lieu) -> String {
    match l {
        Lieu::Ville { nom, .. } => nom.clone(),
        _ => String::new(),
    }
}

/// La note d'une offre, et sa raison.
pub fn noter(p: &Profil, o: &OffreANoter) -> Note {
    let r = p.recherche;
    let titre = mots_utiles(o.titre);
    let texte = mots_utiles(o.description);
    // Le poste
    let mut poste = 0u32;
    let mut poste_texte = false;
    for x in &r.postes {
        let m = mots_utiles(x);
        if m.is_empty() {
            continue;
        }
        let (dans_titre, dans_texte) = (part(&m, &titre), part(&m, &texte));
        let s = if dans_titre >= 1.0 {
            35
        } else if dans_titre >= 0.5 {
            22
        } else if dans_texte >= 1.0 {
            14
        } else {
            0
        };
        if s > poste {
            poste = s;
            poste_texte = s == 14;
        }
    }
    // (sans poste ni mot : tous les métiers du lieu ; jamais hors sujet faute de poste, une part moyenne pour tous)
    if r.postes.iter().all(|x| mots_utiles(x).is_empty()) {
        poste = POSTE_SANS_MOTS;
    }
    if poste == 0 {
        let noms: Vec<&str> = r.postes.iter().take(2).map(String::as_str).collect();
        let suite = if r.postes.len() > 2 { "..." } else { "" };
        return Note {
            score: 0,
            niveau: Niveau::HorsSujet,
            raison: format!("Aucun de tes postes dans cette offre ({}{suite})", noms.join(", ")),
        };
    }
    let mut score = poste;
    // (les points retirés à la fin)
    let mut malus = 0u32;
    // (ce qui va dans la raison : d'abord le diplôme, le lieu, l'expérience ; puis ce qui ne va pas)
    let mut raison: Vec<String> = Vec::new();
    let mut apres: Vec<String> = Vec::new();
    // Le diplôme
    match o.diplome {
        Some(Diplome::Aucun) => {
            score += 10;
            raison.push(Diplome::Aucun.demande().into());
        }
        None => score += if r.sans_diplome { 5 } else { 6 },
        Some(d) => {
            let trop = r.sans_diplome || p.diplome_cv.is_some_and(|c| d > c);
            if trop {
                score += 1;
                malus += 20;
                raison.push(d.demande().into());
            } else if p.diplome_cv.is_some() {
                score += 9;
            } else {
                score += 6;
                apres.push(d.demande().into());
            }
        }
    }
    // Le lieu
    let tele_complet = o.teletravail == Some(TeleOffre::Complet);
    match &r.lieu {
        Lieu::Partout => score += 15,
        _ if tele_complet => {
            score += 20;
            raison.push("en télétravail".into());
        }
        Lieu::Ville { nom, rayon } => {
            let cle = cle_ville(nom);
            let rayon_f = f64::from((*rayon).max(1));
            if !cle.is_empty() && cle_ville(o.lieu) == cle {
                score += 20;
                raison.push(format!("à {}", ville_nom(&r.lieu)));
            } else if let (Some(c), Some(pt)) = (p.centre, o.point) {
                let d = distance_km(c, pt);
                let km = d.round() as i64;
                if d <= rayon_f {
                    score += (20.0 - 6.0 * d / rayon_f).round() as u32;
                    raison.push(format!("à {km} km"));
                } else if d <= 2.0 * rayon_f {
                    score += 6;
                    raison.push(format!("à {km} km, plus loin que tes {rayon} km"));
                } else {
                    raison.push(format!("à {km} km, loin de {nom}"));
                }
            } else if let (Some(dv), Some(dep)) = (p.departement_ville.as_deref(), o.departement) {
                if dv == dep {
                    score += 12;
                    let n = lieux::departement(dep).map_or(dep.to_string(), |x| x.libelle());
                    raison.push(format!("dans le {n}"));
                } else {
                    score += 2;
                    raison.push(format!("loin de {nom}"));
                }
            } else {
                score += 8;
            }
        }
        Lieu::Departement(d) => match o.departement {
            Some(x) if x == d.code => {
                score += 20;
                raison.push(format!("dans le {}", d.libelle()));
            }
            Some(_) => raison.push(format!("hors du {}", d.libelle())),
            None => score += 8,
        },
        Lieu::Region(g) => match o.region {
            Some(x) if x == g.code => {
                score += 20;
                raison.push(format!("en {}", g.nom));
            }
            Some(_) => raison.push(format!("hors de la région {}", g.nom)),
            None => score += 8,
        },
    }
    // L'expérience
    match o.experience {
        Some(Experience::Debutant) => {
            score += 10;
            raison.push(Experience::Debutant.demande());
        }
        Some(e @ Experience::Souhaitee) => {
            score += 8;
            apres.push(e.demande());
        }
        Some(e) => {
            score += if r.debutant { 1 } else { 6 };
            if r.debutant {
                malus += 20;
            }
            if r.debutant { raison.push(e.demande()) } else { apres.push(e.demande()) }
        }
        None => score += 6,
    }
    if poste_texte {
        apres.insert(0, "le poste est dans le texte, pas dans le titre".into());
    }
    // Le contrat
    match o.contrat {
        _ if r.contrats.is_empty() => score += 7,
        Some(c) if r.contrats.contains(&c) => score += 10,
        Some(c) => {
            malus += 10;
            apres.insert(0, format!("en {}, pas dans tes contrats", c.nom()));
        }
        None => score += 5,
    }
    // Le télétravail
    score += match (r.teletravail, o.teletravail) {
        (Teletravail::PeuImporte, _) => 4,
        (Teletravail::Possible, Some(_)) | (Teletravail::Complet, Some(TeleOffre::Complet)) => 5,
        (Teletravail::Complet, Some(TeleOffre::Partiel)) => 2,
        (Teletravail::Possible, None) => 2,
        (Teletravail::Complet, None) => {
            apres.push("pas en télétravail".into());
            0
        }
    };
    // Le salaire
    match (r.salaire_min, salaire_mensuel(o.salaire)) {
        (None, _) => score += 4,
        (Some(min), Some(s)) if s >= min => score += 5,
        (Some(_), Some(_)) => {
            malus += 5;
            apres.insert(0, "salaire sous ton minimum".into());
        }
        (Some(_), None) => score += 3,
    }
    // La ressemblance avec le CV
    if !p.mots_cv.is_empty() && !titre.is_empty() {
        let x = part(&titre, &p.mots_cv);
        if x >= 0.5 {
            score += 5;
            apres.push("proche de ton CV".into());
        } else if x > 0.0 {
            score += 2;
        }
    } else {
        score += 2;
    }
    let score = score.saturating_sub(malus).min(100) as u8;
    raison.extend(apres);
    raison.truncate(4);
    let mut t = raison.join(", ");
    if let Some(c) = t.chars().next() {
        t = c.to_uppercase().collect::<String>() + &t[c.len_utf8()..];
    }
    Note { score, niveau: Niveau::de(score), raison: t }
}

/// L'empreinte de ce qui fait la note (« Ce que je cherche », le CV, les versions des règles) : une autre empreinte,
/// et les notes sont refaites.
pub fn signature(r: &Recherche, cv: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let tout = format!("{VERSION}|{}|{r:?}|{cv}", lecture::VERSION_REGLES);
    for b in tout.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests;
