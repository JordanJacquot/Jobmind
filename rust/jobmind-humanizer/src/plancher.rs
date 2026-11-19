//! Le plancher professionnel (étape 9, demande de Jordan du 18/11) : quoi qu'il arrive, une candidature reste polie et
//! correcte. Le style de Jordan règle le vocabulaire, la longueur des phrases et les exemples, jamais le niveau de
//! langue. Ces règles ne se coupent pas.
//!
//! - corrigé de lui-même ([`corriger`]) : les tirets longs, les points d'exclamation, les points de suspension, les
//!   émojis, les espaces en trop ;
//! - assuré dans la version dans mon style ([`assurer`]) : la formule d'appel et la formule de politesse ;
//! - signalé ([`problemes`]) : un mot familier, le tutoiement, une formule d'appel ou de politesse absente (dans une
//!   lettre que Jordan a écrite lui-même, Jobmind ne la change pas : il le lui dit).

use crate::texte::{Mot, est_appel, est_politesse, mots, structure};

/// Les mots familiers ou oraux (sur le texte normalisé) ; `*` : toutes les fins.
const FAMILIERS: [&str; 30] = [
    "salut",
    "coucou",
    "bisou*",
    "bises",
    "ouais",
    "boulot*",
    "taf",
    "bosser",
    "bouffe",
    "kiff*",
    "cool",
    "sympa",
    "super",
    "mdr",
    "lol",
    "nickel",
    "ok",
    "okay",
    "hyper",
    "carrement",
    "truc*",
    "machin*",
    "pote*",
    "mec",
    "nan",
    "bah",
    "ben",
    "perso",
    "chouette",
    "genial*",
];

/// Les tournures orales de plusieurs mots.
const ORAUX: [&str; 5] = ["du coup", "en gros", "trop bien", "a plus", "ca roule"];

/// La formule d'appel ajoutée quand elle manque.
pub const APPEL: &str = "Madame, Monsieur,";
/// La formule de politesse ajoutée quand elle manque.
pub const POLITESSE: &str = "Je vous prie d'agréer, Madame, Monsieur, l'expression de mes salutations distinguées.";

/// Ce qui ne va pas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probleme {
    /// Un mot familier ou oral (sa place, en octets, et le mot)
    Familier(usize, usize, String),
    /// Le tutoiement (sa place et le mot)
    Tutoiement(usize, usize, String),
    SansAppel,
    SansPolitesse,
}

impl Probleme {
    pub fn code(&self) -> &'static str {
        match self {
            Probleme::Familier(..) => "familier",
            Probleme::Tutoiement(..) => "tutoiement",
            Probleme::SansAppel => "sans_appel",
            Probleme::SansPolitesse => "sans_politesse",
        }
    }

    /// La phrase pour Jordan.
    pub fn texte(&self) -> String {
        match self {
            Probleme::Familier(_, _, m) => format!("« {m} » est trop familier pour une candidature."),
            Probleme::Tutoiement(_, _, m) => format!("« {m} » : dans une candidature, on vouvoie toujours."),
            Probleme::SansAppel => format!("Il manque la formule d'appel, par exemple « {APPEL} »."),
            Probleme::SansPolitesse => "Il manque la formule de politesse à la fin.".into(),
        }
    }

    /// Sa place dans le texte, s'il en a une.
    pub fn place(&self) -> Option<(usize, usize)> {
        match self {
            Probleme::Familier(d, f, _) | Probleme::Tutoiement(d, f, _) => Some((*d, *f)),
            _ => None,
        }
    }
}

fn familier(m: &str) -> bool {
    FAMILIERS.iter().any(|f| match f.strip_suffix('*') {
        Some(debut) => m.starts_with(debut),
        None => m == *f,
    })
}

/// Les mots familiers et le tutoiement d'un texte.
pub fn mots_a_revoir(t: &str) -> Vec<Probleme> {
    let l: Vec<Mot> = mots(t);
    let mut p = Vec::new();
    let mut i = 0;
    while i < l.len() {
        let m = &l[i];
        // (deux mots : « du coup »)
        if let Some(s) = l.get(i + 1)
            && ORAUX.contains(&format!("{} {}", m.norme, s.norme).as_str())
        {
            p.push(Probleme::Familier(m.debut, s.fin, t[m.debut..s.fin].to_string()));
            i += 2;
            continue;
        }
        let elide = t[m.fin..].starts_with(['\'', '\u{2019}']);
        if ["tu", "toi", "tes"].contains(&m.norme.as_str()) || (m.norme == "t" && elide) {
            p.push(Probleme::Tutoiement(m.debut, m.fin, t[m.debut..m.fin].to_string()));
        } else if familier(&m.norme) {
            p.push(Probleme::Familier(m.debut, m.fin, t[m.debut..m.fin].to_string()));
        }
        i += 1;
    }
    p
}

/// Tout ce qui ne va pas dans une lettre (les mots à revoir, puis l'appel et la politesse absents).
pub fn problemes(lettre: &str) -> Vec<Probleme> {
    let mut p = mots_a_revoir(lettre);
    let s = structure(lettre);
    if lettre.trim().is_empty() {
        return p;
    }
    if s.appel.is_none() {
        p.push(Probleme::SansAppel);
    }
    if s.fin.is_none() && !est_politesse(lettre) {
        p.push(Probleme::SansPolitesse);
    }
    p
}

/// Corrige ce qui se corrige sans rien changer au sens : un tiret long entre deux espaces devient une virgule (collé :
/// un trait d'union), un point d'exclamation un point, des points de suspension un point, les émojis partent, les
/// espaces en trop aussi.
pub fn corriger(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    let c: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < c.len() {
        let x = c[i];
        match x {
            '\u{2014}' | '\u{2013}' => {
                let espace_avant = s.ends_with(' ');
                let espace_apres = c.get(i + 1).is_some_and(|y| *y == ' ');
                if espace_avant && espace_apres {
                    s.pop();
                    s.push(',');
                } else {
                    s.push('-');
                }
            }
            '!' => {
                while s.ends_with(' ') {
                    s.pop();
                }
                if !s.ends_with(['.', '!', '?']) {
                    s.push('.');
                }
            }
            '\u{2026}' => {
                if !s.ends_with('.') {
                    s.push('.');
                }
            }
            '.' if s.ends_with('.') => {}
            x if est_emoji(x) => {}
            ' ' | '\u{a0}' if s.ends_with(' ') || s.ends_with('\n') || s.is_empty() => {}
            '\u{a0}' => s.push(' '),
            '\n' => {
                while s.ends_with(' ') {
                    s.pop();
                }
                s.push('\n');
            }
            x => s.push(x),
        }
        i += 1;
    }
    // (« , , » laissé par deux tirets, une espace avant une virgule ou un point)
    s.replace(" ,", ",").replace(",,", ",").replace(" .", ".").trim().to_string()
}

fn est_emoji(c: char) -> bool {
    matches!(c as u32, 0x1F300..=0x1FAFF | 0x2600..=0x27BF | 0xFE0F | 0x200D)
}

/// La lettre avec sa formule d'appel et sa formule de politesse (ajoutées si elles manquent).
pub fn assurer(lettre: &str) -> String {
    let t = lettre.trim();
    let mut blocs: Vec<String> = Vec::new();
    let premier = crate::texte::blocs(t).into_iter().next().unwrap_or_default();
    if !est_appel(&premier) {
        blocs.push(APPEL.to_string());
    }
    blocs.push(t.to_string());
    if !est_politesse(t) {
        blocs.push(POLITESSE.to_string());
    }
    blocs.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mots_familiers_et_tutoiement() {
        let t = "Salut, du coup j'ai trouvé ce boulot sympa. Tu verras, t'es au courant. Le ton reste sobre.";
        let p = mots_a_revoir(t);
        let codes: Vec<String> =
            p.iter().map(|x| format!("{}:{}", x.code(), &t[x.place().unwrap().0..x.place().unwrap().1])).collect();
        assert_eq!(
            codes,
            [
                "familier:Salut",
                "familier:du coup",
                "familier:boulot",
                "familier:sympa",
                "tutoiement:Tu",
                "tutoiement:t"
            ]
        );
        // « genre », « ton », « une bosse » ne comptent pas ; « super » seul oui
        assert!(mots_a_revoir("Ce genre de poste, sur ce ton, avec une bosse.").is_empty());
        assert_eq!(mots_a_revoir("Un super poste.").len(), 1);
    }

    #[test]
    fn les_problemes_d_une_lettre() {
        assert_eq!(problemes("J'aime ce poste."), vec![Probleme::SansAppel, Probleme::SansPolitesse]);
        assert!(problemes("Madame, Monsieur,\n\nJ'aime ce poste.\n\nCordialement,\nJordan").is_empty());
        assert!(problemes("").is_empty());
        for p in [Probleme::SansAppel, Probleme::SansPolitesse, Probleme::Familier(0, 1, "x".into())] {
            assert!(!p.texte().contains(['\u{2013}', '\u{2014}']));
        }
    }

    #[test]
    fn corrections() {
        assert_eq!(corriger("J'aime ce poste \u{2014} vraiment !"), "J'aime ce poste, vraiment.");
        assert_eq!(corriger("Un bon\u{2013}poste... Merci !!  Vraiment\u{2026}"), "Un bon-poste. Merci. Vraiment.");
        assert_eq!(corriger("Bravo \u{1F600} à vous  tous."), "Bravo à vous tous.");
        assert_eq!(corriger("Ligne un  \nLigne deux"), "Ligne un\nLigne deux");
    }

    #[test]
    fn assurer_l_appel_et_la_politesse() {
        let t = assurer("J'aime ce poste.");
        assert!(t.starts_with(APPEL) && t.ends_with(POLITESSE));
        let l = "Bonjour Madame Durand,\n\nJ'aime ce poste.\n\nCordialement,\nJordan";
        assert_eq!(assurer(l), l);
    }
}
