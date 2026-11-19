//! Le texte d'une lettre découpé : ses mots (leur forme sans accents ni majuscules, et leur place exacte dans le
//! texte), ses phrases, et sa structure (la formule d'appel, les paragraphes, la formule de politesse et la signature).

use jobmind_core::offre::normaliser;

/// Un mot : sa forme normalisée et sa place dans le texte (en octets, `fin` exclue).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mot {
    pub norme: String,
    pub debut: usize,
    pub fin: usize,
}

/// Les mots d'un texte (une apostrophe ou un trait d'union sépare deux mots : « d'une » donne « d » et « une »).
pub fn mots(t: &str) -> Vec<Mot> {
    let mut l = Vec::new();
    let mut courant: Option<Mot> = None;
    for (i, c) in t.char_indices() {
        let mut tampon = [0u8; 4];
        let n = normaliser(c.encode_utf8(&mut tampon));
        let n = n.trim();
        if n.is_empty() {
            if let Some(m) = courant.take() {
                l.push(m);
            }
            continue;
        }
        match courant.as_mut() {
            Some(m) => {
                m.norme.push_str(n);
                m.fin = i + c.len_utf8();
            }
            None => courant = Some(Mot { norme: n.to_string(), debut: i, fin: i + c.len_utf8() }),
        }
    }
    if let Some(m) = courant {
        l.push(m);
    }
    l
}

/// Le mot `i` commence une phrase : c'est le premier, ou ce qui le sépare du mot d'avant contient une fin de phrase.
pub fn debut_de_phrase(t: &str, l: &[Mot], i: usize) -> bool {
    i == 0 || t[l[i - 1].fin..l[i].debut].contains(['.', '!', '?', '\n', '\u{2026}'])
}

/// Les abréviations après lesquelles un point ne finit pas la phrase.
const ABREVIATIONS: [&str; 8] = ["m", "mme", "etc", "cf", "ex", "st", "ste", "env"];

/// Les phrases d'un texte : leurs places (en octets), sans les espaces autour. Une phrase finit par un point, un point
/// d'exclamation ou d'interrogation, des points de suspension (suivis d'une espace ou de la fin), ou un retour à la
/// ligne ; « M. » et « etc. » ne la finissent pas.
pub fn phrases(t: &str) -> Vec<(usize, usize)> {
    let mut l = Vec::new();
    let mut debut = 0;
    let c: Vec<(usize, char)> = t.char_indices().collect();
    let mut i = 0;
    while i < c.len() {
        let (pos, ch) = c[i];
        let fin = match ch {
            '\n' => Some(pos),
            '.' | '!' | '?' | '\u{2026}' => {
                // (les signes collés : « ?! », « ... », un guillemet ou une parenthèse fermante)
                let mut j = i + 1;
                while j < c.len() && matches!(c[j].1, '.' | '!' | '?' | '\u{2026}' | '»' | '"' | ')' | '\u{201d}') {
                    j += 1;
                }
                let suivi = j >= c.len() || c[j].1.is_whitespace();
                let mot_avant = t[..pos].rsplit(|x: char| !x.is_alphanumeric()).next().unwrap_or_default();
                let abreviation = ch == '.'
                    && (ABREVIATIONS.contains(&normaliser(mot_avant).trim())
                        || (mot_avant.chars().count() == 1 && mot_avant.chars().all(char::is_uppercase)));
                // (un nombre décimal : « 3.5 » n'est pas suivi d'une espace, il ne coupe donc rien)
                if suivi && !abreviation {
                    i = j - 1;
                    Some(c[j - 1].0 + c[j - 1].1.len_utf8())
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(f) = fin {
            pousser(t, debut, f, &mut l);
            debut = f;
        }
        i += 1;
    }
    pousser(t, debut, t.len(), &mut l);
    l
}

fn pousser(t: &str, d: usize, f: usize, l: &mut Vec<(usize, usize)>) {
    let morceau = &t[d..f];
    let gauche = morceau.len() - morceau.trim_start().len();
    let droite = morceau.trim_end().len();
    if droite > gauche {
        l.push((d + gauche, d + droite));
    }
}

/// Le nombre de mots d'un texte (ceux qui contiennent une lettre ou un chiffre ; « l'offre » compte pour un).
pub fn nombre_de_mots(t: &str) -> usize {
    t.split_whitespace().filter(|m| m.chars().any(char::is_alphanumeric)).count()
}

/// Les mots qui disent une formule de politesse (sur le texte normalisé).
const POLITESSE: [&str; 9] = [
    "salutations",
    "cordialement",
    "agreer",
    "consideration",
    "respectueus",
    "sentiments",
    "bien a vous",
    "mes respects",
    "bien sincerement",
];

/// Le bloc contient une formule de politesse.
pub fn est_politesse(bloc: &str) -> bool {
    let n = format!(" {} ", normaliser(bloc).split_whitespace().collect::<Vec<_>>().join(" "));
    POLITESSE.iter().any(|p| n.contains(&format!(" {p}")))
}

/// Le bloc est une formule d'appel : une seule ligne courte qui finit par une virgule, ou qui commence par Madame,
/// Monsieur ou Bonjour.
pub fn est_appel(bloc: &str) -> bool {
    let b = bloc.trim();
    if b.contains('\n') || nombre_de_mots(b) > 8 {
        return false;
    }
    let n = normaliser(b);
    let premier = n.split_whitespace().next().unwrap_or_default();
    b.ends_with(',') || ["madame", "monsieur", "bonjour", "messieurs", "mesdames"].contains(&premier)
}

/// La structure d'une lettre : la formule d'appel, les paragraphes (chacun ses phrases), et la fin (la formule de
/// politesse, la signature).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Structure {
    pub appel: Option<String>,
    pub paragraphes: Vec<Vec<String>>,
    pub fin: Option<String>,
}

/// Les blocs d'un texte (séparés par une ligne vide).
pub fn blocs(t: &str) -> Vec<String> {
    let t = t.replace("\r\n", "\n");
    let mut l: Vec<String> = Vec::new();
    let mut courant: Vec<&str> = Vec::new();
    for ligne in t.split('\n') {
        if ligne.trim().is_empty() {
            if !courant.is_empty() {
                l.push(courant.join("\n").trim().to_string());
                courant.clear();
            }
        } else {
            courant.push(ligne);
        }
    }
    if !courant.is_empty() {
        l.push(courant.join("\n").trim().to_string());
    }
    l
}

/// Découpe une lettre : le premier bloc est la formule d'appel s'il en a l'air ; la fin commence au dernier bloc qui
/// contient une formule de politesse (les blocs d'après : la signature).
pub fn structure(t: &str) -> Structure {
    let mut b = blocs(t);
    let appel = if b.first().is_some_and(|x| est_appel(x)) { Some(b.remove(0)) } else { None };
    let fin = match b.iter().rposition(|x| est_politesse(x)) {
        // (la politesse est dans le seul paragraphe : elle n'est pas retirée, le corps resterait vide)
        Some(i) if i > 0 || b.len() > 1 || appel.is_some() => Some(b.split_off(i).join("\n\n")),
        _ => None,
    };
    let paragraphes = b
        .iter()
        .map(|x| phrases(x).into_iter().map(|(d, f)| x[d..f].replace('\n', " ")).collect::<Vec<_>>())
        .filter(|p: &Vec<String>| !p.is_empty())
        .collect();
    Structure { appel, paragraphes, fin }
}

impl Structure {
    /// La lettre en un texte : l'appel, les paragraphes (une ligne vide entre deux), la fin.
    pub fn texte(&self) -> String {
        let mut l: Vec<String> = Vec::new();
        if let Some(a) = &self.appel {
            l.push(a.clone());
        }
        for p in &self.paragraphes {
            let t = p.join(" ");
            if !t.trim().is_empty() {
                l.push(t);
            }
        }
        if let Some(f) = &self.fin {
            l.push(f.clone());
        }
        l.join("\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mots_et_places() {
        let t = "L'équipe d'Été, « au sein » de Cœur.";
        let l = mots(t);
        let n: Vec<&str> = l.iter().map(|m| m.norme.as_str()).collect();
        assert_eq!(n, ["l", "equipe", "d", "ete", "au", "sein", "de", "coeur"]);
        assert_eq!(&t[l[1].debut..l[1].fin], "équipe");
        assert_eq!(&t[l[7].debut..l[7].fin], "Cœur");
        assert!(debut_de_phrase(t, &l, 0) && !debut_de_phrase(t, &l, 3));
    }

    #[test]
    fn les_phrases() {
        let t = "Bonjour M. Durand. J'ai 3.5 ans d'expérience, etc. Vraiment ?! Oui... Fin\nNouvelle ligne";
        let p: Vec<&str> = phrases(t).into_iter().map(|(d, f)| &t[d..f]).collect();
        assert_eq!(
            p,
            ["Bonjour M. Durand.", "J'ai 3.5 ans d'expérience, etc. Vraiment ?!", "Oui...", "Fin", "Nouvelle ligne"]
        );
        assert!(phrases("   ").is_empty());
        assert_eq!(nombre_de_mots("Je suis l'agent - ici."), 4);
    }

    #[test]
    fn la_structure() {
        let t = "Madame, Monsieur,\n\nPremière phrase. Deuxième phrase.\n\nTroisième.\n\nJe vous prie d'agréer, Madame, \
                 Monsieur, mes salutations distinguées.\nJordan Exemple";
        let s = structure(t);
        assert_eq!(s.appel.as_deref(), Some("Madame, Monsieur,"));
        assert_eq!(s.paragraphes, vec![vec!["Première phrase.", "Deuxième phrase."], vec!["Troisième."]]);
        assert!(s.fin.as_deref().unwrap().ends_with("Jordan Exemple"));
        assert_eq!(s.texte(), t);
        // Sans appel ni politesse
        let s = structure("Une lettre sans rien.\n\nEt la suite.");
        assert!(s.appel.is_none() && s.fin.is_none());
        assert_eq!(s.paragraphes.len(), 2);
        // Une seule ligne avec la politesse : rien n'est retiré
        let s = structure("Merci, cordialement.");
        assert!(s.fin.is_none() && s.paragraphes.len() == 1);
        assert_eq!(structure("").paragraphes.len(), 0);
    }
}
