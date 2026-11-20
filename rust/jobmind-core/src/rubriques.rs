//! Le texte d'une offre rangé en onglets (0.1.4) : « Le poste », « Ce qu'ils demandent », « Horaires et salaire »,
//! « L'entreprise » ([`decouper`]).
//!
//! Rien n'est inventé ni réécrit : chaque valeur est un morceau du texte de l'offre, au plus nettoyé des espaces, des
//! puces et des deux-points. Les rubriques sont reconnues par leur titre ([`TITRES`], facile à compléter) :
//! - une ligne courte (6 mots au plus), seule sur sa ligne, éventuellement terminée par « : », dont le texte remis au
//!   propre (sans accents, majuscules ni ponctuation) est dans la liste, ouvre une rubrique ;
//! - une ligne « Libellé : valeur » dont le libellé est dans la liste (« Qualification : Employé qualifié ») ouvre une
//!   rubrique dont la valeur est la première ligne.
//!
//! Le texte avant la première rubrique va dans « Le poste ». Sous une rubrique, les lignes à puce ou plusieurs lignes
//! courtes deviennent des puces, un paragraphe devient la valeur. « Exigé » ou « Souhaité » en fin de ligne (comme
//! France Travail l'écrit : « B - Véhicule léger Exigé », « 2 ans - Exigé ») est retiré du texte et noté à part.
//! Aucune ligne n'est perdue : seuls le titre de l'offre répété en tête et les titres de rubrique disparaissent.

use crate::offre::normaliser;

/// Les onglets, dans l'ordre : leur code et leur nom.
pub const ONGLETS: [(&str, &str); 4] = [
    ("poste", "Le poste"),
    ("demande", "Ce qu'ils demandent"),
    ("conditions", "Horaires et salaire"),
    ("entreprise", "L'entreprise"),
];

/// Les titres de rubrique reconnus, par onglet (même ordre que [`ONGLETS`]). Ils sont comparés remis au propre :
/// majuscules, accents, ponctuation et « : » final ne comptent pas (« QUI SOMMES-NOUS ? » est « Qui sommes-nous »).
pub const TITRES: [&[&str]; 4] = [
    // Le poste
    &[
        "Missions",
        "Mission",
        "Vos missions",
        "Votre mission",
        "Vos missions principales",
        "Missions principales",
        "Description du poste",
        "Descriptif du poste",
        "Le poste",
        "Poste",
        "Description",
        "Description de l'offre",
        "Descriptif de l'offre",
        "Activités",
        "Vos activités",
        "Vos responsabilités",
        "Responsabilités",
        "Votre quotidien",
        "Lieu de travail",
        "Lieu",
        "Informations complémentaires",
    ],
    // Ce qu'ils demandent
    &[
        "Profil souhaité",
        "Profil recherché",
        "Profil du candidat",
        "Votre profil",
        "Profil",
        "Expérience",
        "Expériences",
        "Formation",
        "Formations",
        "Diplôme",
        "Savoirs et savoir-faire",
        "Savoir-faire",
        "Compétences",
        "Vos compétences",
        "Compétences requises",
        "Savoir-être professionnels",
        "Savoir-être",
        "Qualités",
        "Qualités requises",
        "Permis",
        "Langues",
        "Langue",
        "Qualification",
        "Prérequis",
    ],
    // Horaires et salaire
    &[
        "Type de contrat",
        "Contrat",
        "Durée du contrat",
        "Durée de la mission",
        "Date de début",
        "Durée du travail",
        "Temps de travail",
        "Horaires",
        "Horaire",
        "Horaires de travail",
        "Salaire",
        "Salaire brut",
        "Rémunération",
        "Rémunération et avantages",
        "Salaire et avantages",
        "Avantages",
        "Nous offrons",
        "Nous vous offrons",
        "Ce que nous offrons",
        "Ce que nous vous offrons",
        "Déplacements",
        "Conditions de travail",
    ],
    // L'entreprise
    &[
        "Entreprise",
        "L'entreprise",
        "Notre entreprise",
        "Description de l'entreprise",
        "Présentation de l'entreprise",
        "Qui sommes-nous",
        "À propos",
        "À propos de nous",
        "Secteur d'activité",
        "Taille",
        "Taille de l'entreprise",
        "Employeur",
        "Notre client",
    ],
];

/// Un titre de rubrique a 6 mots au plus.
const MOTS_TITRE: usize = 6;

/// Une ligne courte (une puce possible) a 10 mots au plus.
const MOTS_COURTE: usize = 10;

/// Exigé ou souhaité, comme France Travail l'écrit en fin de ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Exigence {
    Exige,
    Souhaite,
}

impl Exigence {
    /// « exige », « souhaite ».
    pub fn code(self) -> &'static str {
        match self {
            Exigence::Exige => "exige",
            Exigence::Souhaite => "souhaite",
        }
    }
}

/// Un onglet de l'offre ouverte et ses rangs, dans l'ordre du texte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Onglet {
    /// « poste », « demande », « conditions », « entreprise »
    pub id: &'static str,
    /// « Le poste »...
    pub nom: &'static str,
    pub rangs: Vec<Rang>,
}

/// Une rubrique de l'offre : son titre tel qu'écrit (vide pour le texte d'avant la première rubrique), sa valeur
/// (un paragraphe, les lignes séparées par un retour) et ses puces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rang {
    pub nom: String,
    pub valeur: String,
    pub puces: Vec<String>,
    pub exigence: Option<Exigence>,
}

/// Une ligne de contenu, en attendant de ranger sa rubrique.
struct Ligne {
    texte: String,
    puce: bool,
    exigence: Option<Exigence>,
    /// une ligne vide la précède
    paragraphe: bool,
}

/// La rubrique en cours de lecture.
struct Rubrique {
    onglet: usize,
    nom: String,
    lignes: Vec<Ligne>,
    /// le texte d'avant la première rubrique (pas d'exigence lue, pas de puces sans marque)
    intro: bool,
}

impl Rubrique {
    fn nouvelle(onglet: usize, nom: String) -> Rubrique {
        Rubrique { onglet, nom, lignes: Vec::new(), intro: false }
    }

    fn ajouter(&mut self, ligne: &str, paragraphe: bool) {
        let (puce, texte) = sans_puce(ligne);
        if texte.is_empty() {
            return;
        }
        if self.intro {
            self.lignes.push(Ligne { texte: texte.to_string(), puce, exigence: None, paragraphe });
            return;
        }
        let (reste, exigence) = exigence_finale(texte);
        if reste.is_empty() {
            // (« Exigé » seul sur sa ligne : il va avec la ligne d'avant)
            if let Some(d) = self.lignes.last_mut().filter(|d| d.exigence.is_none()) {
                d.exigence = exigence;
                return;
            }
            self.lignes.push(Ligne { texte: texte.to_string(), puce, exigence: None, paragraphe });
            return;
        }
        self.lignes.push(Ligne { texte: reste, puce, exigence, paragraphe });
    }
}

/// Range le texte d'une offre en onglets (voir le module). Un onglet vide n'est pas rendu ; un texte sans rubrique
/// reconnue donne un seul onglet « Le poste » avec tout le texte ; un texte vide ne donne rien.
pub fn decouper(titre: &str, texte: &str) -> Vec<Onglet> {
    let mut lignes: Vec<&str> = texte.lines().map(str::trim).collect();
    if let Some(i) = lignes.iter().position(|l| !l.is_empty())
        && meme_titre(lignes[i], titre)
    {
        lignes[i] = "";
    }
    if lignes.iter().all(|l| l.is_empty()) {
        return Vec::new();
    }
    if !lignes.iter().any(|l| titre_de(l).is_some() || libelle(l).is_some()) {
        let rang = Rang { valeur: paragraphes(&lignes), ..Rang::default() };
        return vec![Onglet { id: ONGLETS[0].0, nom: ONGLETS[0].1, rangs: vec![rang] }];
    }
    let mut rangs: [Vec<Rang>; 4] = Default::default();
    let mut cours = Rubrique { intro: true, ..Rubrique::nouvelle(0, String::new()) };
    let mut blanc = false;
    for l in lignes {
        if l.is_empty() {
            blanc = true;
            continue;
        }
        if let Some((onglet, nom)) = titre_de(l) {
            fermer(&mut rangs, std::mem::replace(&mut cours, Rubrique::nouvelle(onglet, nom)));
        } else if let Some((onglet, nom, valeur)) = libelle(l) {
            fermer(&mut rangs, std::mem::replace(&mut cours, Rubrique::nouvelle(onglet, nom)));
            cours.ajouter(valeur, false);
        } else {
            cours.ajouter(l, blanc);
        }
        blanc = false;
    }
    fermer(&mut rangs, cours);
    ONGLETS
        .iter()
        .zip(rangs)
        .filter(|(_, r)| !r.is_empty())
        .map(|((id, nom), rangs)| Onglet { id, nom, rangs })
        .collect()
}

/// Les horaires en court pour les infos de l'offre : la première ligne de la rubrique « Horaires » ou « Durée du
/// travail », coupée au mot vers 40 caractères.
pub fn horaires(onglets: &[Onglet]) -> Option<String> {
    let r = onglets.iter().flat_map(|o| &o.rangs).find(|r| {
        let n = normaliser(&r.nom);
        n.starts_with("horaire") || n == "duree du travail" || n == "temps de travail"
    })?;
    let premiere =
        r.valeur.lines().map(str::trim).find(|l| !l.is_empty()).or_else(|| r.puces.first().map(String::as_str))?;
    Some(couper(premiere, 40))
}

/// Coupe un texte au mot, vers `max` caractères (« … » à la fin s'il est coupé).
fn couper(t: &str, max: usize) -> String {
    if t.chars().count() <= max {
        return t.to_string();
    }
    let mut s = String::new();
    for m in t.split_whitespace() {
        let n = s.chars().count();
        if n > 0 && n + 1 + m.chars().count() > max {
            break;
        }
        if n > 0 {
            s.push(' ');
        }
        s.push_str(m);
    }
    if s.chars().count() > max {
        s = s.chars().take(max).collect();
    }
    s.push('…');
    s
}

/// Range une rubrique lue : un rang par suite de lignes de même exigence (« Permis B Exigé » et « Permis C
/// Souhaité » restent distincts).
fn fermer(rangs: &mut [Vec<Rang>; 4], r: Rubrique) {
    let mut debut = 0;
    while debut < r.lignes.len() {
        let ex = r.lignes[debut].exigence;
        let fin = r.lignes[debut..].iter().position(|l| l.exigence != ex).map_or(r.lignes.len(), |p| debut + p);
        let groupe = &r.lignes[debut..fin];
        let marques = groupe.iter().any(|l| l.puce);
        let courtes = !r.intro && groupe.len() >= 2 && groupe.iter().all(|l| mots(&l.texte) <= MOTS_COURTE);
        let mut rang = Rang { nom: r.nom.clone(), exigence: ex, ..Rang::default() };
        let mut texte: Vec<&Ligne> = Vec::new();
        for l in groupe {
            if (marques && l.puce) || (!marques && courtes) {
                rang.puces.push(l.texte.clone());
            } else {
                texte.push(l);
            }
        }
        for (i, l) in texte.iter().enumerate() {
            if i > 0 {
                rang.valeur.push_str(if l.paragraphe { "\n\n" } else { "\n" });
            }
            rang.valeur.push_str(&l.texte);
        }
        rangs[r.onglet].push(rang);
        debut = fin;
    }
}

/// Les lignes remises en un texte : un retour entre deux lignes, une ligne vide entre deux paragraphes.
fn paragraphes(lignes: &[&str]) -> String {
    let mut s = String::new();
    let mut blanc = false;
    for l in lignes {
        if l.is_empty() {
            blanc = true;
            continue;
        }
        if !s.is_empty() {
            s.push_str(if blanc { "\n\n" } else { "\n" });
        }
        s.push_str(l);
        blanc = false;
    }
    s
}

fn mots(t: &str) -> usize {
    t.split_whitespace().count()
}

/// La ligne est-elle le titre de l'offre (sans tenir compte de « H/F ») ?
fn meme_titre(ligne: &str, titre: &str) -> bool {
    fn sans_hf(t: &str) -> String {
        let mut m: Vec<&str> = t.split(' ').collect();
        while m.last().is_some_and(|x| matches!(*x, "h" | "f" | "hf" | "fh" | "x" | "m")) {
            m.pop();
        }
        m.join(" ")
    }
    let t = sans_hf(&normaliser(titre));
    !t.is_empty() && sans_hf(&normaliser(ligne)) == t
}

/// L'onglet d'un titre de rubrique, remis au propre.
fn onglet_du_titre(t: &str) -> Option<usize> {
    let n = normaliser(t);
    if n.is_empty() || mots(t) > MOTS_TITRE {
        return None;
    }
    TITRES.iter().position(|liste| liste.iter().any(|x| normaliser(x) == n))
}

/// Une ligne qui est un titre de rubrique : son onglet et son nom nettoyé.
fn titre_de(ligne: &str) -> Option<(usize, String)> {
    if sans_puce(ligne).0 {
        return None;
    }
    onglet_du_titre(ligne).map(|o| (o, nettoyer(ligne)))
}

/// Une ligne « Libellé : valeur » au libellé connu : son onglet, le libellé nettoyé et la valeur.
fn libelle(ligne: &str) -> Option<(usize, String, &str)> {
    if sans_puce(ligne).0 {
        return None;
    }
    let (l, v) = ligne.split_once(':')?;
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    onglet_du_titre(l).map(|o| (o, nettoyer(l), v))
}

/// Un titre sans les espaces, les « : », les étoiles ni les dièses autour.
fn nettoyer(t: &str) -> String {
    t.trim_matches(|c: char| c.is_whitespace() || matches!(c, ':' | '*' | '#' | '.' | '-' | '_')).to_string()
}

/// Les marques de puce reconnues en tête de ligne.
const PUCES: [char; 10] = ['•', '·', '▪', '◦', '►', '▸', '→', '✓', '\u{2013}', '\u{2014}'];

/// La ligne sans sa marque de puce (« - », « • », « * »...), et si elle en avait une.
fn sans_puce(ligne: &str) -> (bool, &str) {
    let l = ligne.trim();
    if let Some(r) = l.strip_prefix(PUCES) {
        return (true, r.trim());
    }
    for m in ["- ", "* ", "+ ", "o "] {
        if let Some(r) = l.strip_prefix(m)
            && (m != "o " || r.chars().next().is_some_and(char::is_uppercase))
        {
            return (true, r.trim());
        }
    }
    (false, l)
}

fn exigence_du_mot(mot: &str) -> Option<Exigence> {
    match normaliser(mot).as_str() {
        "exige" | "exigee" | "exiges" | "exigees" => Some(Exigence::Exige),
        "souhaite" | "souhaitee" | "souhaites" | "souhaitees" => Some(Exigence::Souhaite),
        _ => None,
    }
}

/// Les séparateurs avant « Exigé » : « 2 ans - Exigé », « Permis B : souhaité ».
fn sans_separateur(t: &str) -> Option<&str> {
    t.trim_end().strip_suffix(['-', ':', ',', '\u{2013}', '\u{2014}']).map(str::trim_end)
}

/// Les mots qui, juste avant « souhaitée », en font une phrase (« une expérience est souhaitée ») : rien n'est retiré.
const AVANT_PHRASE: [&str; 13] = [
    "est",
    "sont",
    "serait",
    "seraient",
    "sera",
    "seront",
    "etre",
    "fortement",
    "vivement",
    "tres",
    "vous",
    "nous",
    "on",
];

/// « Exigé » ou « Souhaité » en fin de ligne, retiré du texte : seul, entre parenthèses, après « - » ou « : », ou
/// après une espace dans une ligne courte qui n'est pas une phrase. Sinon la ligne reste telle quelle.
fn exigence_finale(ligne: &str) -> (String, Option<Exigence>) {
    let rien = (ligne.to_string(), None);
    let s = ligne.trim_end_matches(|c: char| c == '.' || c == '!' || c.is_whitespace());
    if let Some(dedans) = s.strip_suffix(')') {
        let Some(i) = dedans.rfind('(') else { return rien };
        let Some(e) = exigence_du_mot(&dedans[i + 1..]) else { return rien };
        let avant = &dedans[..i];
        return (sans_separateur(avant).unwrap_or(avant.trim_end()).to_string(), Some(e));
    }
    let Some(i) = s.char_indices().rev().take_while(|(_, c)| c.is_alphabetic()).last().map(|(i, _)| i) else {
        return rien;
    };
    let Some(e) = exigence_du_mot(&s[i..]) else { return rien };
    let avant = &s[..i];
    if avant.trim().is_empty() {
        return (String::new(), Some(e));
    }
    if let Some(r) = sans_separateur(avant) {
        return (r.to_string(), Some(e));
    }
    if !avant.ends_with(char::is_whitespace) {
        return rien;
    }
    let avant = avant.trim_end();
    let dernier = avant.split_whitespace().last().map(normaliser).unwrap_or_default();
    if mots(avant) > 8 || AVANT_PHRASE.contains(&dernier.as_str()) {
        return rien;
    }
    (avant.to_string(), Some(e))
}

#[cfg(test)]
mod tests;
