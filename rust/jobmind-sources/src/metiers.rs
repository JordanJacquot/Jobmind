//! Les métiers proposés pendant la frappe (0.1.8) : dans la case « poste » d'une recherche, Jordan écrit « helpdesk »
//! et Jobmind propose les vrais noms de métier (« Technicien helpdesk », « Technicien de help desk en informatique »)
//! puis les métiers voisins, ceux de la même famille (« Technicien support informatique »).
//!
//! La table (`metiers.txt`, une ligne par nom de métier : `code de la famille;nom officiel`, environ 14 300 noms dans
//! 1 900 familles, 750 Ko) est l'arborescence principale du ROME (Répertoire opérationnel des métiers et des emplois,
//! France Travail, version d'octobre 2026), publiée en données ouvertes sur data.gouv.fr ; elle est embarquée dans
//! l'exe, jamais téléchargée pendant que Jobmind tourne. La première ligne d'une famille est son titre.
//!
//! Le code de la famille ne sort jamais d'ici : il sert seulement à trouver les métiers voisins. Les sites reçoivent
//! des mots (le nom choisi), pas un numéro.
//!
//! Les noms officiels sont écrits aux deux genres (« Technicien / Technicienne helpdesk ») ; [`court`] en fait le nom
//! qu'on tape sur un site d'emploi (« Technicien helpdesk »).

use crate::page::simple;

const TABLE: &str = include_str!("metiers.txt");

/// Au plus ce nombre de propositions.
pub const PROPOSITIONS_MAX: usize = 8;
/// Au plus ce nombre de noms trouvés directement quand la famille a aussi des métiers voisins à montrer.
const DIRECTS_MAX: usize = 5;

/// Un métier proposé pendant la frappe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposition {
    /// Ce qui va dans la case, et ce que les sites recevront : « Technicien helpdesk »
    pub nom: String,
    /// Le titre de sa famille de métiers : « Conseiller support technique informatique »
    pub famille: String,
}

/// Un texte remis au propre pour comparer : minuscules, sans accent, tout ce qui n'est ni lettre ni chiffre en espace.
pub fn cle(t: &str) -> String {
    let s: String = simple(t)
        .chars()
        .map(|c| match c {
            'œ' => 'o',
            'æ' => 'a',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Le nom officiel aux deux genres, ramené au nom qu'on tape : « Technicien / Technicienne helpdesk » donne
/// « Technicien helpdesk », « Directeur adjoint / Directrice adjointe de magasin » donne « Directeur adjoint de
/// magasin ». Les mots du second genre sont ceux qui commencent comme les derniers mots du premier (trois lettres ;
/// parfois tout le nom est répété, « Agent commercial en machines-outils / Agente commerciale en machines-outils ») ;
/// faute de ressemblance (« Homme / Femme de ménage »), un seul mot.
pub fn court(nom: &str) -> String {
    let Some((avant, apres)) = nom.split_once(" / ") else { return nom.trim().to_string() };
    let a: Vec<&str> = avant.split_whitespace().collect();
    let b: Vec<&str> = apres.split_whitespace().collect();
    let debut = |m: &str| -> String { cle(m).chars().take(3).collect() };
    let mut k = 1;
    for n in (2..=a.len().min(b.len())).rev() {
        if (0..n).all(|i| debut(a[a.len() - n + i]) == debut(b[i])) {
            k = n;
            break;
        }
    }
    let mut mots = a;
    mots.extend(b.into_iter().skip(k));
    // (un second « / » plus loin dans le nom, trois noms en tout dans la table : le reste est gardé tel quel)
    mots.join(" ")
}

struct Ligne {
    code: &'static str,
    nom: String,
    cle: String,
    /// La clé sans espace : « help desk » et « helpdesk » se trouvent l'un l'autre
    colle: String,
}

/// Chaque nom de métier, remis au propre (fait une fois, au premier appel).
fn index() -> &'static [Ligne] {
    static INDEX: std::sync::OnceLock<Vec<Ligne>> = std::sync::OnceLock::new();
    INDEX.get_or_init(|| {
        TABLE
            .lines()
            .filter_map(|l| l.split_once(';'))
            .map(|(code, nom)| {
                let nom = court(nom);
                let cle = cle(&nom);
                Ligne { code, colle: cle.replace(' ', ""), cle, nom }
            })
            .collect()
    })
}

/// Le nombre de noms de métier de la table.
pub fn nombre() -> usize {
    index().len()
}

/// Les métiers proposés pour ce qui est tapé (au plus `max`), dès 2 lettres. D'abord les noms qui correspondent : celui
/// qui commence par ce qui est tapé, puis celui dont chaque mot tapé commence un de ses mots (« tech info » :
/// Technicien informatique), puis celui qui le contient une fois les espaces retirées (« helpdesk » : Technicien de
/// help desk) ; à égalité, le nom le plus court. Ensuite les métiers voisins : les autres noms de la famille du premier
/// trouvé, son titre d'abord. Sans tenir compte des majuscules ni des accents ; un nom n'est proposé qu'une fois.
pub fn proposer(tape: &str, max: usize) -> Vec<Proposition> {
    let tape = cle(tape);
    let colle = tape.replace(' ', "");
    if colle.chars().count() < 2 || max == 0 {
        return Vec::new();
    }
    let mots: Vec<&str> = tape.split(' ').collect();
    let table = index();
    // (rang, longueur, place dans la table)
    let mut directs: Vec<(u8, usize, usize)> = Vec::new();
    for (i, l) in table.iter().enumerate() {
        let rang = if l.cle.starts_with(&tape) {
            0
        } else if mots.iter().all(|m| l.cle.split(' ').any(|x| x.starts_with(m))) {
            1
        } else if l.colle.contains(&colle) {
            2
        } else {
            continue;
        };
        directs.push((rang, l.cle.len(), i));
    }
    directs.sort();
    let Some(&(_, _, premier)) = directs.first() else { return Vec::new() };
    let famille = table[premier].code;
    let titre = |code: &str| table.iter().find(|l| l.code == code).map(|l| l.nom.clone()).unwrap_or_default();
    let mut vus: Vec<&str> = Vec::new();
    let mut r: Vec<Proposition> = Vec::new();
    let mut ajouter = |i: usize, r: &mut Vec<Proposition>| {
        let l = &table[i];
        if r.len() < max && !vus.contains(&l.cle.as_str()) {
            vus.push(&l.cle);
            r.push(Proposition { nom: l.nom.clone(), famille: titre(l.code) });
        }
    };
    let voisins: Vec<usize> = (0..table.len()).filter(|&i| table[i].code == famille).collect();
    // (quand la famille a d'autres noms à montrer, quelques places leur sont gardées)
    let garde = if voisins.len() > 1 { DIRECTS_MAX.min(max) } else { max };
    for &(_, _, i) in directs.iter() {
        if r.len() >= garde {
            break;
        }
        ajouter(i, &mut r);
    }
    for &i in &voisins {
        ajouter(i, &mut r);
    }
    for &(_, _, i) in directs.iter() {
        ajouter(i, &mut r);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noms(tape: &str) -> Vec<String> {
        proposer(tape, PROPOSITIONS_MAX).into_iter().map(|p| p.nom).collect()
    }

    #[test]
    fn la_table() {
        assert!(nombre() > 14_000, "{}", nombre());
        // Chaque ligne : un code de famille (une lettre, quatre chiffres) et un nom, sans tiret long
        for l in TABLE.lines() {
            let (code, nom) = l.split_once(';').unwrap_or_else(|| panic!("{l}"));
            assert!(code.len() == 5 && code.starts_with(|c: char| c.is_ascii_uppercase()), "{l}");
            assert!(code[1..].chars().all(|c| c.is_ascii_digit()), "{l}");
            assert!(!nom.trim().is_empty() && !nom.contains(['\u{2013}', '\u{2014}', '<', '>', ';']), "{l}");
        }
    }

    #[test]
    fn le_nom_qu_on_tape() {
        assert_eq!(court("Technicien / Technicienne helpdesk"), "Technicien helpdesk");
        assert_eq!(
            court("Chef / Cheffe d'équipe de help desk en informatique"),
            "Chef d'équipe de help desk en informatique"
        );
        assert_eq!(court("Directeur adjoint / Directrice adjointe de magasin"), "Directeur adjoint de magasin");
        assert_eq!(court("Homme / Femme de ménage"), "Homme de ménage");
        assert_eq!(
            court("Agent commercial en machines-outils / Agente commerciale en machines-outils"),
            "Agent commercial en machines-outils"
        );
        assert_eq!(court("Support aux utilisateurs en informatique"), "Support aux utilisateurs en informatique");
        // Aucun nom de la table ne garde de « / » de genre au début, et aucun n'est vide
        for l in index() {
            assert!(!l.nom.is_empty() && !l.cle.is_empty(), "{}", l.code);
        }
    }

    /// Le cas de Jordan : « helpdesk » propose les noms du métier, puis ses voisins du support informatique.
    #[test]
    fn helpdesk() {
        let l = noms("helpdesk");
        assert_eq!(l[0], "Technicien helpdesk", "{l:?}");
        assert!(l.contains(&"Technicien de help desk en informatique".to_string()), "{l:?}");
        assert!(l.contains(&"Conseiller support technique informatique".to_string()), "les voisins : {l:?}");
        assert_eq!(l.len(), PROPOSITIONS_MAX);
        let p = &proposer("helpdesk", 8)[0];
        assert_eq!(p.famille, "Conseiller support technique informatique");
        // En deux mots, avec une majuscule ou un accent : pareil
        assert_eq!(noms("Help Desk")[0..1], noms("help desk")[0..1]);
        assert!(noms("help desk").iter().any(|n| n == "Technicien helpdesk"));
    }

    #[test]
    fn d_autres_metiers() {
        assert_eq!(noms("technicien informatique")[0], "Technicien informatique");
        assert!(noms("tech info").iter().any(|n| n == "Technicien informatique"), "{:?}", noms("tech info"));
        assert!(noms("cariste").iter().any(|n| n.starts_with("Cariste")), "{:?}", noms("cariste"));
        assert!(noms("boulang")[0].starts_with("Boulanger"), "{:?}", noms("boulang"));
        assert!(noms("vendeur").iter().all(|n| !n.contains(" / ")), "{:?}", noms("vendeur"));
        // Un nom n'est proposé qu'une fois
        for t in ["helpdesk", "vendeur", "agent", "infirm", "chef"] {
            let l = noms(t);
            let mut u = l.clone();
            u.sort();
            u.dedup();
            assert_eq!(u.len(), l.len(), "{t} : {l:?}");
            assert!(l.len() <= PROPOSITIONS_MAX);
        }
    }

    /// Contre-épreuve : trop court, ou rien qui ressemble, ne propose rien.
    #[test]
    fn rien_a_proposer() {
        assert!(noms("h").is_empty());
        assert!(noms("  ").is_empty());
        assert!(noms("zzqxw").is_empty());
        assert!(proposer("helpdesk", 0).is_empty());
    }

    /// Pour regarder ce qui est proposé (lancé à la main : `cargo test -p jobmind-sources apercu -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn apercu() {
        for t in [
            "helpdesk",
            "support info",
            "admin sys",
            "réseau",
            "devops",
            "dev web",
            "caissier",
            "aide soign",
            "chauffeur pl",
            "serveur",
            "secrétaire",
            "agent de quai",
            "préparateur de commandes",
            "soudeur",
            "électricien",
            "comptable",
            "rh",
            "it",
            "commercial",
            "manut",
        ] {
            println!("== {t}");
            for p in proposer(t, PROPOSITIONS_MAX) {
                println!("   {}   [{}]", p.nom, p.famille);
            }
        }
    }
}
