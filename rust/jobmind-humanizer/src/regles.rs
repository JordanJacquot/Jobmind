//! Les règles fixes de Jobmind (étape 9) : les tournures toutes faites des textes d'IA et les mots trop recherchés,
//! que Jordan voit et peut couper dans Mon profil, rubrique Mon style ; il peut y ajouter ses propres expressions à
//! éviter (avec un remplacement s'il veut), et tout remettre comme au départ. Une règle avec un remplacement est
//! appliquée d'elle-même (« afin de » devient « pour ») ; une règle sans remplacement est surlignée dans la lettre,
//! compte dans le score naturel, et Claude l'évite quand il réécrit.
//!
//! Une règle se cherche mot par mot, sans tenir compte des accents ni des majuscules ; dans les règles de départ, un mot
//! qui finit par `*` en couvre toutes les fins (« motiv* » : motivé, motivée, motivation), et `^` en tête veut dire « en
//! tête de phrase seulement ».

use jobmind_core::offre::normaliser;
use jobmind_core::style::PHRASE_DEFAUT;

use crate::texte::{Mot, debut_de_phrase, mots};

/// Le genre d'une règle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    /// Une tournure toute faite des textes d'IA
    Ia,
    /// Un mot trop recherché
    Pompeux,
    /// Une expression ajoutée par Jordan
    Perso,
}

impl Genre {
    pub fn code(self) -> &'static str {
        match self {
            Genre::Ia => "ia",
            Genre::Pompeux => "pompeux",
            Genre::Perso => "perso",
        }
    }

    /// Le titre de la liste de ce genre.
    pub fn titre(self) -> &'static str {
        match self {
            Genre::Ia => "Formules toutes faites",
            Genre::Pompeux => "Mots trop recherchés",
            Genre::Perso => "Mes expressions à éviter",
        }
    }
}

/// Une règle de départ : son code, son genre, l'expression montrée, ce qui se cherche, le remplacement, la raison.
struct Depart(&'static str, Genre, &'static str, &'static str, Option<&'static str>, &'static str);

const CREUSE: &str = "Formule toute faite : un fait concret de ton parcours dit plus.";
const IA: &str = "Tournure typique des textes d'IA.";
const QUALITES: &str = "Qualités creuses : un exemple concret les montre mieux.";
const LIAISON: &str = "Mot de liaison des textes d'IA, inutile ici.";
const RECHERCHE: &str = "Mot trop recherché pour une lettre simple.";
const VIDE: &str = "Ce mot n'ajoute rien.";
const DANS: &str = "Plus simple : « dans ».";
const POUR: &str = "Plus simple : « pour ».";

/// Les règles de départ, fixées avant toute mesure (étape 9, 19/11/2026).
const DEPARTS: &[Depart] = &[
    Depart("ia-fort-de", Genre::Ia, "fort de mon expérience", "fort* de mon*", None, CREUSE),
    Depart("ia-fort-d", Genre::Ia, "fort d'une solide expérience", "fort* d une", None, CREUSE),
    Depart("ia-convaincu", Genre::Ia, "je suis convaincu", "je suis convaincu*", None, CREUSE),
    Depart("ia-persuade", Genre::Ia, "je suis persuadé", "je suis persuad*", None, CREUSE),
    Depart("ia-dyn-motive", Genre::Ia, "dynamique et motivé", "dynamique* et motiv*", None, QUALITES),
    Depart("ia-motive-dyn", Genre::Ia, "motivé et dynamique", "motiv* et dynamique*", None, QUALITES),
    Depart("ia-rigoureux", Genre::Ia, "rigoureux et motivé", "rigoureu* et motiv*", None, QUALITES),
    Depart("ia-defis", Genre::Ia, "relever de nouveaux défis", "relever de nouveaux defis", None, CREUSE),
    Depart("ia-defi", Genre::Ia, "relever ce défi", "relever ce* defi*", None, CREUSE),
    Depart("ia-valeur", Genre::Ia, "valeur ajoutée", "valeur ajoutee", None, IA),
    Depart("ia-atout", Genre::Ia, "un véritable atout", "veritable atout*", None, IA),
    Depart("ia-atout-majeur", Genre::Ia, "un atout majeur", "atout* majeur*", None, IA),
    Depart("ia-profit", Genre::Ia, "mettre à profit", "mettre a profit", None, IA),
    Depart(
        "ia-service",
        Genre::Ia,
        "mettre mes compétences au service",
        "mettre mes competences au service",
        None,
        CREUSE,
    ),
    Depart("ia-grand-interet", Genre::Ia, "c'est avec un grand intérêt", "c est avec un grand interet", None, IA),
    Depart("ia-enthousiasme", Genre::Ia, "avec enthousiasme", "avec enthousiasme", None, IA),
    Depart("ia-vif", Genre::Ia, "un vif intérêt", "vif interet", None, IA),
    Depart("ia-permets", Genre::Ia, "je me permets de", "je me permets", None, IA),
    Depart("ia-hesitez", Genre::Ia, "n'hésitez pas à", "n hesitez pas", None, IA),
    Depart("ia-entiere", Genre::Ia, "entière disposition", "entiere disposition", Some("disposition"), VIDE),
    Depart("ia-passionne", Genre::Ia, "passionné par", "passionn* par", None, IA),
    Depart("ia-constante", Genre::Ia, "en constante évolution", "en constante evolution", None, IA),
    Depart("ia-perpetuelle", Genre::Ia, "en perpétuelle évolution", "en perpetuelle evolution", None, IA),
    Depart("ia-stimulant", Genre::Ia, "environnement stimulant", "environnement stimulant", None, IA),
    Depart("ia-investir", Genre::Ia, "m'investir pleinement", "m investir pleinement", None, IA),
    Depart("ia-activement", Genre::Ia, "contribuer activement", "contribuer activement", None, IA),
    Depart("ia-continuite", Genre::Ia, "s'inscrit dans la continuité", "s inscrit dans la continuite", None, IA),
    Depart("ia-adequation", Genre::Ia, "en adéquation avec", "en adequation avec", None, IA),
    Depart("ia-solide-exp", Genre::Ia, "solide expérience", "solide* experience*", None, CREUSE),
    Depart("ia-solides-comp", Genre::Ia, "solides compétences", "solide* competence*", None, CREUSE),
    Depart("ia-riche", Genre::Ia, "riche expérience", "riche* experience*", None, CREUSE),
    Depart("ia-eventail", Genre::Ia, "large éventail", "large eventail", None, IA),
    Depart("ia-proposition", Genre::Ia, "force de proposition", "force de proposition", None, IA),
    Depart("ia-aisance", Genre::Ia, "aisance relationnelle", "aisance relationnelle", None, IA),
    Depart("ia-aigu", Genre::Ia, "sens aigu", "sens aigu", None, IA),
    Depart("ia-souligner", Genre::Ia, "je tiens à souligner", "je tiens a souligner", None, IA),
    Depart("ia-important", Genre::Ia, "il est important de souligner", "important de souligner", None, IA),
    Depart("ia-passion", Genre::Ia, "véritable passion", "veritable passion", None, IA),
    Depart("ia-attention", Genre::Ia, "a retenu toute mon attention", "a retenu toute mon attention", None, CREUSE),
    Depart("ia-savoir-etre", Genre::Ia, "savoir-être", "savoir etre", None, IA),
    Depart("ia-monde", Genre::Ia, "Dans un monde...", "^dans un monde", None, IA),
    Depart("ia-parti", Genre::Ia, "tirer parti", "tirer parti", None, IA),
    Depart("ia-ravi", Genre::Ia, "je serais ravi", "je serais ravi*", None, IA),
    Depart("ia-unique", Genre::Ia, "une opportunité unique", "opportunite* unique*", None, IA),
    Depart("ia-sein-de", Genre::Ia, "au sein de", "au sein de", Some("dans"), DANS),
    Depart("ia-sein-du", Genre::Ia, "au sein du", "au sein du", Some("dans le"), DANS),
    Depart("ia-sein-des", Genre::Ia, "au sein des", "au sein des", Some("dans les"), DANS),
    Depart("ia-sein-d", Genre::Ia, "au sein d'une", "au sein d", Some("dans"), DANS),
    Depart("ia-afin-de", Genre::Ia, "afin de", "afin de", Some("pour"), POUR),
    Depart("ia-afin-d", Genre::Ia, "afin d'", "afin d", Some("pour"), POUR),
    Depart("ia-en-effet", Genre::Ia, "En effet, en tête de phrase", "^en effet", Some(""), LIAISON),
    Depart("ia-ailleurs", Genre::Ia, "Par ailleurs, en tête de phrase", "^par ailleurs", Some(""), LIAISON),
    Depart("ia-de-plus", Genre::Ia, "De plus, en tête de phrase", "^de plus", Some(""), LIAISON),
    Depart("ia-outre", Genre::Ia, "En outre, en tête de phrase", "^en outre", Some(""), LIAISON),
    Depart("ia-surcroit", Genre::Ia, "de surcroît", "de surcroit", Some(""), LIAISON),
    Depart("po-synergie", Genre::Pompeux, "synergie", "synergie*", None, RECHERCHE),
    Depart("po-proactif", Genre::Pompeux, "proactif", "proacti*", None, RECHERCHE),
    Depart("po-optimal", Genre::Pompeux, "optimal", "optimal*", None, RECHERCHE),
    Depart("po-perenne", Genre::Pompeux, "pérenne", "perenn*", None, RECHERCHE),
    Depart("po-incontournable", Genre::Pompeux, "incontournable", "incontournable*", None, RECHERCHE),
    Depart("po-innovant", Genre::Pompeux, "innovant", "innovant*", None, RECHERCHE),
    Depart("po-excellence", Genre::Pompeux, "excellence", "excellence", None, RECHERCHE),
    Depart("po-challenge", Genre::Pompeux, "challenge", "challeng*", None, RECHERCHE),
    Depart("po-impactant", Genre::Pompeux, "impactant", "impactant*", None, RECHERCHE),
    Depart("po-holistique", Genre::Pompeux, "holistique", "holistique*", None, RECHERCHE),
    Depart("po-opportunite", Genre::Pompeux, "opportunité", "opportunite*", None, RECHERCHE),
    Depart("po-adaptabilite", Genre::Pompeux, "adaptabilité", "adaptabilite", None, RECHERCHE),
    Depart("po-leadership", Genre::Pompeux, "leadership", "leadership", None, RECHERCHE),
    Depart("po-accru", Genre::Pompeux, "accru", "accru*", None, RECHERCHE),
    Depart("po-meticuleux", Genre::Pompeux, "méticuleux", "meticuleu*", None, RECHERCHE),
    Depart("po-prestigieux", Genre::Pompeux, "prestigieux", "prestigieu*", None, RECHERCHE),
    Depart("po-renommee", Genre::Pompeux, "renommée", "renommee*", None, RECHERCHE),
    Depart("po-expertise", Genre::Pompeux, "expertise", "expertise*", None, RECHERCHE),
    Depart("po-envergure", Genre::Pompeux, "envergure", "envergure", None, RECHERCHE),
    Depart("po-dynamisme", Genre::Pompeux, "dynamisme", "dynamisme", None, RECHERCHE),
    Depart("po-veritablement", Genre::Pompeux, "véritablement", "veritablement", Some(""), VIDE),
    Depart("po-pleinement", Genre::Pompeux, "pleinement", "pleinement", Some(""), VIDE),
    Depart("po-parfaitement", Genre::Pompeux, "parfaitement", "parfaitement", Some(""), VIDE),
    Depart("po-indeniablement", Genre::Pompeux, "indéniablement", "indeniablement", Some(""), VIDE),
    Depart(
        "po-particulierement",
        Genre::Pompeux,
        "tout particulièrement",
        "tout particulierement",
        Some("particulièrement"),
        VIDE,
    ),
];

/// Un mot d'une règle : exact, ou le début d'un mot (`*`).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Bout {
    texte: String,
    debut_seul: bool,
}

/// Une règle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regle {
    /// « ia-fort-de » ; une règle de Jordan : « perso-<id> »
    pub code: String,
    pub genre: Genre,
    /// L'expression telle qu'on la montre
    pub motif: String,
    /// Le remplacement (rien : surlignée seulement ; vide : retirée)
    pub remplacement: Option<String>,
    pub raison: String,
    pub active: bool,
    bouts: Vec<Bout>,
    en_tete: bool,
}

impl Regle {
    /// Une règle de Jordan (déjà vérifiée par jobmind-core::style::verifier_regle).
    pub fn perso(id: i64, motif: &str, remplacement: Option<&str>, active: bool) -> Regle {
        let raison = match remplacement {
            Some(r) if !r.is_empty() => format!("Tu préfères « {r} »."),
            Some(_) => "Tu as demandé de la retirer.".into(),
            None => "Tu as demandé de l'éviter.".into(),
        };
        let (bouts, en_tete) = bouts(&normaliser(motif), false);
        Regle {
            code: format!("perso-{id}"),
            genre: Genre::Perso,
            motif: motif.to_string(),
            remplacement: remplacement.map(str::to_string),
            raison,
            active,
            bouts,
            en_tete,
        }
    }

    /// Ce qui est cherché est vide (la règle ne trouvera jamais rien).
    pub fn vide(&self) -> bool {
        self.bouts.is_empty()
    }
}

/// Les mots d'un motif normalisé (`*` permis seulement dans les règles de départ).
fn bouts(n: &str, etoiles: bool) -> (Vec<Bout>, bool) {
    let en_tete = etoiles && n.trim_start().starts_with('^');
    let n = n.replace('^', " ");
    let l = n
        .split_whitespace()
        .map(|m| {
            if etoiles && let Some(x) = m.strip_suffix('*') {
                Bout { texte: x.to_string(), debut_seul: true }
            } else {
                Bout { texte: m.to_string(), debut_seul: false }
            }
        })
        .filter(|b| !b.texte.is_empty())
        .collect();
    (l, en_tete)
}

/// Ce qui se cherche dans une règle de départ (son motif normalisé, `*` et `^` gardés).
fn cherche_depart(c: &str) -> (Vec<Bout>, bool) {
    // (normaliser change « * » et « ^ » en espaces : ils sont remis à leur place mot par mot)
    let en_tete = c.starts_with('^');
    let l = c
        .trim_start_matches('^')
        .split_whitespace()
        .map(|m| Bout {
            texte: normaliser(m.trim_end_matches('*')).split_whitespace().collect::<Vec<_>>().join(""),
            debut_seul: m.ends_with('*'),
        })
        .collect();
    (l, en_tete)
}

/// Les règles de Jordan : celles de départ (coupées ou non) et les siennes, et la longueur de phrase permise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regles {
    pub liste: Vec<Regle>,
    pub phrase_max: usize,
}

impl Default for Regles {
    fn default() -> Self {
        Regles::construire(&[], &[], PHRASE_DEFAUT)
    }
}

/// Une règle trouvée dans un texte : sa place (en octets) et son numéro dans la liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trouvee {
    pub debut: usize,
    pub fin: usize,
    pub regle: usize,
}

/// Une règle de Jordan gardée : son id, l'expression, le remplacement, active ou non.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perso {
    pub id: i64,
    pub motif: String,
    pub remplacement: Option<String>,
    pub active: bool,
}

impl Regles {
    /// Les règles de départ, sauf celles que Jordan a coupées (`coupees` : leurs codes), puis les siennes.
    pub fn construire(coupees: &[String], perso: &[Perso], phrase_max: usize) -> Regles {
        let mut liste: Vec<Regle> = DEPARTS
            .iter()
            .map(|d| {
                let (bouts, en_tete) = cherche_depart(d.3);
                Regle {
                    code: d.0.to_string(),
                    genre: d.1,
                    motif: d.2.to_string(),
                    remplacement: d.4.map(str::to_string),
                    raison: d.5.to_string(),
                    active: !coupees.iter().any(|c| c == d.0),
                    bouts,
                    en_tete,
                }
            })
            .collect();
        liste.extend(perso.iter().map(|p| Regle::perso(p.id, &p.motif, p.remplacement.as_deref(), p.active)));
        Regles { liste, phrase_max }
    }

    /// Les codes des règles de départ.
    pub fn codes_de_depart() -> Vec<&'static str> {
        DEPARTS.iter().map(|d| d.0).collect()
    }

    /// Les règles actives trouvées dans un texte, dans l'ordre, sans chevauchement (à une même place, la plus longue).
    pub fn trouver(&self, t: &str) -> Vec<Trouvee> {
        let l = mots(t);
        let mut toutes: Vec<Trouvee> = Vec::new();
        for (k, r) in self.liste.iter().enumerate().filter(|(_, r)| r.active && !r.vide()) {
            for i in 0..l.len() {
                if r.en_tete && !debut_de_phrase(t, &l, i) {
                    continue;
                }
                if let Some(f) = correspond(&l, i, &r.bouts) {
                    toutes.push(Trouvee { debut: l[i].debut, fin: l[f].fin, regle: k });
                }
            }
        }
        toutes.sort_by_key(|x| (x.debut, std::cmp::Reverse(x.fin)));
        let mut gardees: Vec<Trouvee> = Vec::new();
        for x in toutes {
            if gardees.last().is_none_or(|g| x.debut >= g.fin) {
                gardees.push(x);
            }
        }
        gardees
    }

    /// Les expressions actives sans remplacement (à éviter : données à Claude pour la réécriture).
    pub fn a_eviter(&self) -> Vec<String> {
        self.liste.iter().filter(|r| r.active && r.remplacement.is_none()).map(|r| r.motif.clone()).collect()
    }

    /// Applique les remplacements des règles actives (« afin de » devient « pour » ; une règle au remplacement vide
    /// retire l'expression, et la virgule qui la suivait). Rend le texte et le nombre de changements.
    pub fn appliquer(&self, t: &str) -> (String, usize) {
        let trouvees: Vec<Trouvee> =
            self.trouver(t).into_iter().filter(|x| self.liste[x.regle].remplacement.is_some()).collect();
        if trouvees.is_empty() {
            return (t.to_string(), 0);
        }
        let mut s = String::with_capacity(t.len());
        let mut ici = 0;
        let mut majuscule_a_venir = false;
        for x in &trouvees {
            if x.debut < ici {
                continue;
            }
            let avant = &t[ici..x.debut];
            push_avec_majuscule(&mut s, avant, &mut majuscule_a_venir);
            let original = &t[x.debut..x.fin];
            let tete = original.chars().next().is_some_and(char::is_uppercase);
            let mut fin = x.fin;
            // (une élision : « afin d'avoir » garde son espace : « pour avoir »)
            let elision = t[fin..].starts_with(['\'', '\u{2019}']);
            if elision {
                fin += t[fin..].chars().next().map_or(0, char::len_utf8);
            }
            let r = self.liste[x.regle].remplacement.clone().unwrap_or_default();
            if r.is_empty() {
                // (retirée : la virgule et les espaces qui suivent aussi ; la suite prend la majuscule)
                let reste = &t[fin..];
                let saute = reste.len() - reste.trim_start_matches([',', ' ']).len();
                fin += saute;
                if tete {
                    majuscule_a_venir = true;
                } else if s.ends_with(' ') && !t[fin..].starts_with([' ', '.', ',']) {
                    // (« je suis pleinement prêt » : une seule espace)
                } else if s.ends_with(' ') {
                    s.pop();
                }
            } else {
                let mut r = r;
                if tete {
                    r = majuscule(&r);
                }
                if elision {
                    r.push(' ');
                }
                s.push_str(&r);
            }
            ici = fin;
        }
        push_avec_majuscule(&mut s, &t[ici..], &mut majuscule_a_venir);
        (s, trouvees.len())
    }
}

fn push_avec_majuscule(s: &mut String, t: &str, a_venir: &mut bool) {
    if *a_venir {
        let mut fait = false;
        for c in t.chars() {
            if !fait && c.is_alphabetic() {
                s.extend(c.to_uppercase());
                fait = true;
            } else {
                s.push(c);
            }
        }
        if fait {
            *a_venir = false;
        }
    } else {
        s.push_str(t);
    }
}

/// La première lettre en majuscule.
pub fn majuscule(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(p) => p.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

/// La règle correspond à partir du mot `i` : rend le numéro de son dernier mot.
fn correspond(l: &[Mot], i: usize, b: &[Bout]) -> Option<usize> {
    if i + b.len() > l.len() || b.is_empty() {
        return None;
    }
    for (j, x) in b.iter().enumerate() {
        let m = &l[i + j].norme;
        let ok = if x.debut_seul { m.starts_with(&x.texte) } else { *m == x.texte };
        if !ok {
            return None;
        }
    }
    Some(i + b.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Regles {
        Regles::default()
    }

    #[test]
    fn les_regles_de_depart_sont_saines() {
        let codes = Regles::codes_de_depart();
        let mut uniques = codes.clone();
        uniques.sort_unstable();
        uniques.dedup();
        assert_eq!(uniques.len(), codes.len(), "codes en double");
        for x in &r().liste {
            assert!(!x.vide(), "{}", x.code);
            assert!(!x.motif.contains(['\u{2013}', '\u{2014}']) && !x.raison.contains(['\u{2013}', '\u{2014}']));
        }
    }

    #[test]
    fn trouver_sans_accents_ni_majuscules() {
        let t = "Fort de mon expérience, je suis CONVAINCUE que mon profil est un véritable atout.";
        let l = r().trouver(t);
        let vus: Vec<&str> = l.iter().map(|x| &t[x.debut..x.fin]).collect();
        assert_eq!(vus, ["Fort de mon", "je suis CONVAINCUE", "véritable atout"]);
        // « ^ » : en tête de phrase seulement ; « de plus en plus » au milieu ne compte pas
        let t = "De plus, j'aime ce métier. Il y a de plus en plus de travail.";
        let l = r().trouver(t);
        assert_eq!(l.len(), 1);
        assert_eq!(&t[l[0].debut..l[0].fin], "De plus");
    }

    #[test]
    fn remplacer_et_retirer() {
        let (t, n) = r().appliquer("Afin d'avoir le poste, je travaille au sein d'une équipe et au sein du magasin.");
        assert_eq!(t, "Pour avoir le poste, je travaille dans une équipe et dans le magasin.");
        assert_eq!(n, 3);
        let (t, _) = r().appliquer("Madame,\n\nEn effet, j'ai conduit des chariots. Je suis pleinement prêt.");
        assert_eq!(t, "Madame,\n\nJ'ai conduit des chariots. Je suis prêt.");
        let (t, _) = r().appliquer("Je reste à votre entière disposition.");
        assert_eq!(t, "Je reste à votre disposition.");
        let (t, n) = r().appliquer("Rien à changer ici.");
        assert_eq!((t.as_str(), n), ("Rien à changer ici.", 0));
    }

    #[test]
    fn couper_et_ajouter() {
        let perso = [
            Perso { id: 3, motif: "Je me permets".into(), remplacement: None, active: true },
            Perso { id: 4, motif: "chariot élévateur".into(), remplacement: Some("chariot".into()), active: true },
            Perso { id: 5, motif: "rien".into(), remplacement: None, active: false },
        ];
        let regles = Regles::construire(&["ia-afin-de".to_string()], &perso, 20);
        let (t, _) = regles.appliquer("Afin de conduire un chariot élévateur, rien.");
        assert_eq!(t, "Afin de conduire un chariot, rien.");
        assert!(regles.a_eviter().contains(&"Je me permets".to_string()));
        assert!(!regles.a_eviter().contains(&"rien".to_string()));
        assert_eq!(regles.liste.last().unwrap().code, "perso-5");
        assert_eq!(regles.phrase_max, 20);
    }
}
