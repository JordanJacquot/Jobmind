//! Le surlignage (étape 9) : les passages d'une lettre qui font « texte d'IA » selon les règles (une formule toute
//! faite, un mot trop recherché, une expression que Jordan veut éviter, une liste de trois qualités, une phrase trop
//! longue) et ceux qui passent sous le plancher professionnel (un mot familier, le tutoiement), chacun avec sa raison ;
//! puis le texte découpé en morceaux pour l'écran, avec aussi les phrases « à vérifier » (rien d'inventé, étape 8).

use crate::plancher::mots_a_revoir;
use crate::regles::Regles;
use crate::score::{genre_de, phrase_du_corps};
use crate::texte::{mots, nombre_de_mots, phrases};

/// Le genre d'un passage surligné (du plus important au moins important).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GenreMarque {
    /// Sous le plancher professionnel : un mot familier, le tutoiement
    Plancher,
    Ia,
    Perso,
    Pompeux,
    /// Une liste de trois qualités
    Trois,
    /// Une phrase trop longue
    Longue,
}

impl GenreMarque {
    pub fn code(self) -> &'static str {
        match self {
            GenreMarque::Plancher => "plancher",
            GenreMarque::Ia => "ia",
            GenreMarque::Perso => "perso",
            GenreMarque::Pompeux => "pompeux",
            GenreMarque::Trois => "trois",
            GenreMarque::Longue => "longue",
        }
    }
}

/// Un passage surligné : sa place (en octets), son genre et sa raison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marque {
    pub debut: usize,
    pub fin: usize,
    pub genre: GenreMarque,
    pub raison: String,
}

/// Les adjectifs d'une liste de trois qualités finissent souvent ainsi (sur le mot normalisé).
const FINS_DE_QUALITE: [&str; 14] =
    ["eux", "euse", "if", "ive", "ant", "ante", "ent", "ente", "ible", "able", "ee", "e", "el", "eur"];

fn qualite(m: &str) -> bool {
    m.chars().count() >= 5 && FINS_DE_QUALITE.iter().any(|f| m.ends_with(f))
}

/// Tous les passages surlignés d'un texte, dans l'ordre.
pub fn marques(t: &str, regles: &Regles) -> Vec<Marque> {
    let mut l: Vec<Marque> = Vec::new();
    for p in mots_a_revoir(t) {
        if let Some((d, f)) = p.place() {
            l.push(Marque { debut: d, fin: f, genre: GenreMarque::Plancher, raison: p.texte() });
        }
    }
    for x in regles.trouver(t) {
        let r = &regles.liste[x.regle];
        l.push(Marque { debut: x.debut, fin: x.fin, genre: genre_de(r.genre), raison: r.raison.clone() });
    }
    // Les listes de trois qualités : « rigoureux, ponctuel et motivé »
    let ms = mots(t);
    for i in 0..ms.len().saturating_sub(3) {
        let (a, b, et, c) = (&ms[i], &ms[i + 1], &ms[i + 2], &ms[i + 3]);
        if t[a.fin..b.debut].trim() == ","
            && t[b.fin..et.debut].trim().is_empty()
            && et.norme == "et"
            && t[et.fin..c.debut].trim().is_empty()
            && qualite(&a.norme)
            && qualite(&b.norme)
            && qualite(&c.norme)
        {
            l.push(Marque {
                debut: a.debut,
                fin: c.fin,
                genre: GenreMarque::Trois,
                raison: "Trois qualités à la suite : un exemple concret dit plus.".into(),
            });
        }
    }
    for (d, f) in phrases(t) {
        let p = &t[d..f];
        let n = nombre_de_mots(p);
        if phrase_du_corps(p) && n > regles.phrase_max {
            l.push(Marque {
                debut: d,
                fin: f,
                genre: GenreMarque::Longue,
                raison: format!("Phrase trop longue : {n} mots (au plus {}).", regles.phrase_max),
            });
        }
    }
    l.sort_by_key(|m| (m.debut, m.genre));
    l
}

/// Un morceau de texte pour l'écran : la phrase à vérifier qui le couvre (son numéro), le passage surligné le plus
/// important qui le couvre (son numéro dans la liste des marques).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Morceau {
    pub texte: String,
    pub verif: Option<usize>,
    pub marque: Option<usize>,
}

/// Les places de chaque phrase à vérifier dans le texte (la première fois qu'elle paraît, sans chevauchement) :
/// (début, fin, son numéro).
pub fn places(t: &str, a_verifier: &[(usize, String)]) -> Vec<(usize, usize, usize)> {
    let mut l: Vec<(usize, usize, usize)> = Vec::new();
    for (n, p) in a_verifier {
        if p.is_empty() {
            continue;
        }
        let mut depart = 0;
        while let Some(x) = t[depart..].find(p.as_str()) {
            let d = depart + x;
            let f = d + p.len();
            if l.iter().all(|(a, b, _)| f <= *a || d >= *b) {
                l.push((d, f, *n));
                break;
            }
            depart = f;
        }
    }
    l.sort_unstable();
    l
}

/// Découpe un texte en morceaux : chaque morceau garde la même phrase à vérifier et le même passage surligné (le plus
/// important quand plusieurs se recouvrent).
pub fn morceaux(t: &str, verif: &[(usize, usize, usize)], ms: &[Marque]) -> Vec<Morceau> {
    let mut bornes: Vec<usize> = vec![0, t.len()];
    for (d, f, _) in verif {
        bornes.push(*d);
        bornes.push(*f);
    }
    for m in ms {
        bornes.push(m.debut);
        bornes.push(m.fin);
    }
    bornes.retain(|b| *b <= t.len() && t.is_char_boundary(*b));
    bornes.sort_unstable();
    bornes.dedup();
    let mut l: Vec<Morceau> = Vec::new();
    for w in bornes.windows(2) {
        let (d, f) = (w[0], w[1]);
        if d == f {
            continue;
        }
        let v = verif.iter().find(|(a, b, _)| *a <= d && f <= *b).map(|x| x.2);
        let m = ms
            .iter()
            .enumerate()
            .filter(|(_, m)| m.debut <= d && f <= m.fin)
            .min_by_key(|(_, m)| (m.genre, m.fin - m.debut))
            .map(|(i, _)| i);
        match l.last_mut() {
            Some(x) if x.verif == v && x.marque == m => x.texte.push_str(&t[d..f]),
            _ => l.push(Morceau { texte: t[d..f].to_string(), verif: v, marque: m }),
        }
    }
    if l.is_empty() {
        l.push(Morceau { texte: String::new(), verif: None, marque: None });
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_marques() {
        let r = Regles::construire(&[], &[], 12);
        let t = "Salut. Fort de mon expérience, je suis rigoureux, ponctuel et motivé pour ce poste dans votre belle \
                 entreprise de Metz.";
        let m = marques(t, &r);
        let g: Vec<(&str, &str)> = m.iter().map(|x| (x.genre.code(), &t[x.debut..x.fin])).collect();
        assert!(g.contains(&("plancher", "Salut")), "{g:?}");
        assert!(g.contains(&("ia", "Fort de mon")), "{g:?}");
        assert!(g.contains(&("trois", "rigoureux, ponctuel et motivé")), "{g:?}");
        assert!(g.iter().any(|(c, x)| *c == "longue" && x.starts_with("Fort")), "{g:?}");
        // « Madame, Monsieur, » et la politesse ne sont jamais des phrases trop longues
        let p = "Je vous prie d'agréer, Madame, Monsieur, l'expression de mes salutations les plus distinguées et respectueuses.";
        assert!(marques(p, &r).is_empty());
    }

    #[test]
    fn les_morceaux() {
        let r = Regles::default();
        let t = "Fort de mon expérience, je viens. J'ai encadré six personnes.";
        let ms = marques(t, &r);
        let v = places(t, &[(0, "J'ai encadré six personnes.".to_string())]);
        let l = morceaux(t, &v, &ms);
        let tout: String = l.iter().map(|m| m.texte.as_str()).collect();
        assert_eq!(tout, t);
        assert_eq!(l[0].texte, "Fort de mon");
        assert!(l[0].marque.is_some() && l[0].verif.is_none());
        assert_eq!(l.last().unwrap().texte, "J'ai encadré six personnes.");
        assert_eq!(l.last().unwrap().verif, Some(0));
        assert_eq!(morceaux("", &[], &[]).len(), 1);
        assert_eq!(morceaux("Rien.", &[], &[]), vec![Morceau { texte: "Rien.".into(), verif: None, marque: None }]);
    }
}
