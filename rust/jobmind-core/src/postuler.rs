//! L'envoi d'une candidature sur le site de l'offre (étape 11 du plan), sans réseau ni navigateur : ce que Jordan
//! tape dans le récapitulatif vérifié ([`verifier_saisie`], [`verifier_telephone`]), ses coordonnées trouvées dans son
//! CV ([`mail_dans`], [`telephone_dans`]), une question du site rapprochée d'une réponse préparée par Claude
//! ([`reponse_pour`]) et le choix d'une liste qui va avec une réponse ([`option_pour`]).
//!
//! Tout est prudent : une question du site qui ne ressemble pas assez à une question préparée, ou un choix qui ne se
//! lit pas clairement dans la réponse, ne donnent rien ; Jordan complète alors lui-même dans le récapitulatif. Rien
//! n'est inventé : Jobmind ne met dans un champ que ce qui vient du profil, du CV, de la lettre gardée, d'une réponse
//! préparée ou de ce que Jordan a tapé.

use crate::envoi::adresse_valide;

/// La plus longue valeur courte (un nom, un téléphone, une réponse d'une ligne).
pub const COURT_MAX: usize = 300;
/// La plus longue valeur longue (une lettre, une réponse en plusieurs lignes).
pub const LONG_MAX: usize = 8000;

/// Un texte remis à plat pour comparer : minuscules, sans accents, lettres et chiffres séparés par une espace.
pub fn plat(t: &str) -> String {
    let mut s = String::with_capacity(t.len());
    for c in t.chars() {
        let c = match c {
            'à' | 'â' | 'ä' | 'á' | 'À' | 'Â' | 'Ä' | 'Á' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'î' | 'ï' | 'í' | 'Î' | 'Ï' => 'i',
            'ô' | 'ö' | 'ó' | 'Ô' | 'Ö' => 'o',
            'ù' | 'û' | 'ü' | 'ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'œ' | 'Œ' => 'o',
            'ÿ' => 'y',
            x => x,
        };
        for x in c.to_lowercase() {
            s.push(if x.is_alphanumeric() { x } else { ' ' });
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Les mots qui ne disent rien d'une question.
const VIDES: [&str; 46] = [
    "le", "la", "les", "un", "une", "des", "de", "du", "d", "l", "et", "ou", "a", "au", "aux", "en", "pour", "par",
    "sur", "dans", "avec", "vous", "votre", "vos", "est", "etes", "avez", "quel", "quelle", "quels", "quelles", "que",
    "qu", "ce", "cette", "il", "y", "se", "s", "pas", "ne", "your", "you", "do", "the", "of",
];

fn mots_utiles(t: &str) -> Vec<String> {
    plat(t)
        .split(' ')
        .filter(|m| !m.is_empty() && !VIDES.contains(m))
        // (un mot au pluriel compte comme au singulier)
        .map(|m| if m.len() > 3 && m.ends_with('s') { m[..m.len() - 1].to_string() } else { m.to_string() })
        .collect()
}

/// La question préparée (son rang) qui répond à une question du site : la plus proche, si elles partagent assez de
/// mots utiles (au moins deux, et au moins la moitié de ceux de la plus courte). Rien sinon.
pub fn reponse_pour(question: &str, preparees: &[&str]) -> Option<usize> {
    let a = mots_utiles(question);
    if a.is_empty() {
        return None;
    }
    let mut meilleure: Option<(usize, f64)> = None;
    for (i, q) in preparees.iter().enumerate() {
        let b = mots_utiles(q);
        if b.is_empty() {
            continue;
        }
        let communs = a.iter().filter(|m| b.contains(m)).count();
        let court = a.len().min(b.len());
        let note = communs as f64 / court as f64;
        if communs >= 2.min(court) && note >= 0.5 && meilleure.is_none_or(|(_, n)| note > n) {
            meilleure = Some((i, note));
        }
    }
    meilleure.map(|(i, _)| i)
}

/// Le choix d'une liste (ou d'un groupe de boutons) qui va avec une réponse : « Oui » ou « Non » quand la réponse
/// commence ainsi, sinon le choix dont le texte entier se trouve dans la réponse (le plus long). Rien si ce n'est pas
/// clair (deux choix différents possibles, ou aucun).
pub fn option_pour(reponse: &str, options: &[&str]) -> Option<usize> {
    let r = plat(reponse);
    if r.is_empty() {
        return None;
    }
    let premier = r.split(' ').next().unwrap_or_default();
    let oui_non = |o: &str| -> Option<bool> {
        match plat(o).as_str() {
            "oui" | "yes" => Some(true),
            "non" | "no" => Some(false),
            _ => None,
        }
    };
    let dit = match premier {
        "oui" | "yes" => Some(true),
        "non" | "no" => Some(false),
        _ => None,
    };
    if let Some(d) = dit {
        let l: Vec<usize> = (0..options.len()).filter(|i| oui_non(options[*i]) == Some(d)).collect();
        if l.len() == 1 {
            return Some(l[0]);
        }
    }
    let mut trouves: Vec<(usize, usize)> = Vec::new();
    for (i, o) in options.iter().enumerate() {
        let p = plat(o);
        if p.len() < 2 || oui_non(o).is_some() {
            continue;
        }
        if format!(" {r} ").contains(&format!(" {p} ")) {
            trouves.push((i, p.len()));
        }
    }
    trouves.sort_by_key(|x| std::cmp::Reverse(x.1));
    match trouves.as_slice() {
        [] => None,
        [x] => Some(x.0),
        [x, y, ..] if x.1 > y.1 && plat(options[x.0]).contains(&plat(options[y.0])) => Some(x.0),
        _ => None,
    }
}

/// Vérifie une valeur tapée par Jordan pour un champ du site : sans caractère de contrôle (les retours à la ligne
/// gardés seulement pour une valeur longue), sans tiret long, coupée aux bords, pas trop longue. Vide : rien.
pub fn verifier_saisie(t: &str, long: bool) -> Result<String, String> {
    let max = if long { LONG_MAX } else { COURT_MAX };
    let s: String = t
        .replace("\r\n", "\n")
        .chars()
        .filter_map(|c| match c {
            '\u{2014}' | '\u{2013}' => Some('-'),
            '\n' if long => Some('\n'),
            '\n' | '\t' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect();
    let s = s.trim().to_string();
    if s.chars().count() > max {
        return Err(format!("C'est trop long : {max} caractères au plus."));
    }
    Ok(s)
}

/// Vérifie un téléphone : des chiffres (10 pour un numéro français, 8 à 15 avec l'indicatif), des espaces, des
/// points, des tirets, des parenthèses, un « + » au début. Rendu tel que Jordan l'a écrit, remis au propre.
pub fn verifier_telephone(t: &str) -> Result<String, String> {
    let s = verifier_saisie(t, false)?;
    let chiffres = s.chars().filter(char::is_ascii_digit).count();
    let permis = s.chars().enumerate().all(|(i, c)| c.is_ascii_digit() || " .-()".contains(c) || (c == '+' && i == 0));
    if !permis || !(8..=15).contains(&chiffres) || (!s.starts_with('+') && !s.starts_with("00") && chiffres != 10) {
        return Err("Ce numéro ne semble pas juste. Il s'écrit comme 06 12 34 56 78.".into());
    }
    Ok(s)
}

/// Vérifie une adresse mail tapée : en minuscules, bien formée.
pub fn verifier_mail(t: &str) -> Result<String, String> {
    let a = verifier_saisie(t, false)?.to_lowercase();
    if !adresse_valide(&a) {
        return Err("Cette adresse ne semble pas juste. Elle s'écrit comme prenom.nom@exemple.fr.".into());
    }
    Ok(a)
}

/// La première adresse mail écrite dans un texte (le CV), telle quelle.
pub fn mail_dans(texte: &str) -> Option<String> {
    texte
        .split(|c: char| c.is_whitespace() || "<>()[]{};,\"'".contains(c))
        .map(|m| m.trim_end_matches(['.', ':']))
        .find(|m| m.contains('@') && adresse_valide(&m.to_lowercase()))
        .map(str::to_lowercase)
}

/// Le premier numéro de téléphone français écrit dans un texte (le CV) : « 06 12 34 56 78 », « 06.12.34.56.78 »,
/// « +33 6 12 34 56 78 », tel qu'il est écrit.
pub fn telephone_dans(texte: &str) -> Option<String> {
    let c: Vec<char> = texte.chars().collect();
    let mut i = 0;
    while i < c.len() {
        let debut_ok = i == 0 || !c[i - 1].is_ascii_digit();
        if debut_ok && (c[i] == '0' || c[i] == '+') {
            let mut j = i;
            let mut chiffres = String::new();
            while j < c.len() && (c[j].is_ascii_digit() || " .-".contains(c[j]) || (j == i && c[j] == '+')) {
                if c[j].is_ascii_digit() {
                    chiffres.push(c[j]);
                }
                if chiffres.len() >= 12 {
                    break;
                }
                j += 1;
            }
            let fini = j >= c.len() || !c[j].is_ascii_digit();
            let fr = chiffres.len() == 10 && chiffres.starts_with('0') && chiffres[1..2] != *"0";
            let inter = c[i] == '+' && chiffres.len() == 11 && chiffres.starts_with("33");
            if fini && (fr || inter) {
                let t: String = c[i..j].iter().collect();
                return Some(t.trim_end_matches([' ', '.', '-']).to_string());
            }
        }
        i += 1;
    }
    None
}

/// Un nom complet coupé en prénom et nom (« Jordan Exemple » : « Jordan », « Exemple ») ; un seul mot : le nom.
pub fn separer_nom(complet: &str) -> (String, String) {
    let t = complet.split_whitespace().collect::<Vec<_>>();
    match t.as_slice() {
        [] => (String::new(), String::new()),
        [n] => (String::new(), (*n).to_string()),
        [p, reste @ ..] => ((*p).to_string(), reste.join(" ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plat_retire_accents_et_signes() {
        assert_eq!(plat("Êtes-vous disponible ?"), "etes vous disponible");
        assert_eq!(plat("Prétentions  salariales*"), "pretentions salariales");
    }

    #[test]
    fn questions_rapprochees() {
        let preparees = [
            "Quelle est votre disponibilité ?",
            "Avez-vous le permis B ?",
            "Quelles sont vos prétentions salariales ?",
        ];
        assert_eq!(reponse_pour("Disponibilité", &preparees), Some(0));
        assert_eq!(reponse_pour("Êtes-vous titulaire du permis B ?", &preparees), Some(1));
        assert_eq!(reponse_pour("Prétentions salariales (brut annuel)", &preparees), Some(2));
        assert_eq!(reponse_pour("Lien vers votre profil LinkedIn", &preparees), None);
        assert_eq!(reponse_pour("", &preparees), None);
    }

    #[test]
    fn choix_d_une_liste() {
        assert_eq!(option_pour("Oui, j'ai le permis B depuis 2019.", &["Oui", "Non"]), Some(0));
        assert_eq!(option_pour("Non, pas encore.", &["Choisir", "Oui", "Non"]), Some(2));
        assert_eq!(option_pour("Je suis disponible immédiatement.", &["Immédiatement", "Sous 1 mois"]), Some(0));
        assert_eq!(option_pour("Je peux commencer dans un mois.", &["Immédiatement", "Sous 1 mois"]), None);
        assert_eq!(option_pour("Avec un CACES 3 et un CACES 5", &["CACES 3", "CACES 5"]), None);
        assert_eq!(option_pour("", &["Oui", "Non"]), None);
    }

    #[test]
    fn saisies_verifiees() {
        assert_eq!(verifier_saisie("  Metz\n", false).unwrap(), "Metz");
        assert_eq!(verifier_saisie("Ligne 1\r\nLigne 2", true).unwrap(), "Ligne 1\nLigne 2");
        assert_eq!(verifier_saisie("a\u{2014}b", false).unwrap(), "a-b");
        assert!(verifier_saisie(&"x".repeat(301), false).is_err());
        assert_eq!(verifier_telephone("06 12 34 56 78").unwrap(), "06 12 34 56 78");
        assert_eq!(verifier_telephone("+33 6 12 34 56 78").unwrap(), "+33 6 12 34 56 78");
        assert!(verifier_telephone("06 12").is_err());
        assert!(verifier_telephone("06 12 34 56 7a").is_err());
        assert_eq!(verifier_mail(" Jordan.Exemple@Exemple.fr ").unwrap(), "jordan.exemple@exemple.fr");
        assert!(verifier_mail("jordan@").is_err());
    }

    #[test]
    fn coordonnees_du_cv() {
        let cv = "Jordan Exemple\nMetz (57) - jordan.exemple@exemple.fr - Tél. : 06.12.34.56.78\nCACES 1, 3 et 5 depuis 2018";
        assert_eq!(mail_dans(cv).as_deref(), Some("jordan.exemple@exemple.fr"));
        assert_eq!(telephone_dans(cv).as_deref(), Some("06.12.34.56.78"));
        assert_eq!(telephone_dans("Tel +33 6 12 34 56 78").as_deref(), Some("+33 6 12 34 56 78"));
        assert_eq!(telephone_dans("SIRET 0123456789012, de 2018 à 2020"), None);
        assert_eq!(mail_dans("pas d'adresse ici"), None);
        assert_eq!(separer_nom("Jordan  Exemple"), ("Jordan".into(), "Exemple".into()));
        assert_eq!(separer_nom("Exemple"), (String::new(), "Exemple".into()));
    }
}
