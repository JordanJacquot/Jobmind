//! La lecture d'une offre (étape 6 du plan) : le diplôme et l'expérience demandés, trouvés dans le texte par des
//! règles simples ([`lire`]), puis par le petit modèle de Claude pour les offres que les règles n'ont pas su classer
//! (jobmind-llm, seulement quand Jordan a donné sa clé). Le permis et le CACES sont relevés au passage.
//!
//! Chaque résultat garde sa preuve : le bout de phrase de l'offre où il a été lu, recopié tel quel (jamais réécrit), et
//! son niveau de confiance (« forte » : la phrase le dit en clair, « moyenne » : il se déduit d'une tournure comme
//! « formation assurée »). Rien d'inventé : une offre qui ne dit rien reste « non dit ».
//!
//! Comment les règles lisent : le titre puis chaque phrase du texte, remis au propre (sans accents ni majuscules, des
//! mots séparés par une espace, [`normaliser`]). Les tournures connues sont cherchées mot à mot ; un niveau de diplôme
//! ambigu (« bac », « CAP », « licence ») ne compte que dans une phrase qui parle de diplôme ou d'exigence (« bac à
//! sable », « cap sur l'avenir », « licence IV » ne comptent pas) ; une durée ne compte que près du mot « expérience »
//! et jamais après « CDD », « contrat », « période d'essai », « avoir » (l'âge) ou « il y a ».

use crate::offre::normaliser;

/// La version des règles : une offre lue par une version plus ancienne est relue (la base le note).
pub const VERSION_REGLES: i64 = 1;

/// Le diplôme demandé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Diplome {
    /// Aucun diplôme demandé (« sans diplôme », « diplôme non exigé »...)
    Aucun,
    CapBep,
    Bac,
    Bac2,
    Bac3,
    /// Bac+5 et plus
    Bac5,
}

impl Diplome {
    pub const TOUS: [Diplome; 6] =
        [Diplome::Aucun, Diplome::CapBep, Diplome::Bac, Diplome::Bac2, Diplome::Bac3, Diplome::Bac5];

    pub fn code(self) -> &'static str {
        match self {
            Diplome::Aucun => "aucun",
            Diplome::CapBep => "cap_bep",
            Diplome::Bac => "bac",
            Diplome::Bac2 => "bac2",
            Diplome::Bac3 => "bac3",
            Diplome::Bac5 => "bac5",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Diplome> {
        Diplome::TOUS.into_iter().find(|d| d.code() == c)
    }

    /// « Aucun diplôme demandé », « CAP ou BEP », « Bac+2 ».
    pub fn nom(self) -> &'static str {
        match self {
            Diplome::Aucun => "Aucun diplôme demandé",
            Diplome::CapBep => "CAP ou BEP",
            Diplome::Bac => "Bac",
            Diplome::Bac2 => "Bac+2",
            Diplome::Bac3 => "Bac+3",
            Diplome::Bac5 => "Bac+5 et plus",
        }
    }

    /// Pour une raison : « demande un CAP ou un BEP », « demande un Bac+2 ».
    pub fn demande(self) -> &'static str {
        match self {
            Diplome::Aucun => "aucun diplôme demandé",
            Diplome::CapBep => "demande un CAP ou un BEP",
            Diplome::Bac => "demande le bac",
            Diplome::Bac2 => "demande un Bac+2",
            Diplome::Bac3 => "demande un Bac+3",
            Diplome::Bac5 => "demande un Bac+5",
        }
    }
}

/// L'expérience demandée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Experience {
    /// Débutant accepté (« sans expérience », « expérience exigée : non »...)
    Debutant,
    /// Au moins cette durée, en mois (« 2 ans d'expérience » : 24)
    Duree(u16),
    /// Une expérience exigée, sans durée (« profil expérimenté », « débutants s'abstenir »)
    Demandee,
    /// Une expérience appréciée, pas exigée (« une première expérience serait un plus »)
    Souhaitee,
}

impl Experience {
    /// « debutant », « duree:24 », « demandee », « souhaitee ».
    pub fn code(self) -> String {
        match self {
            Experience::Debutant => "debutant".into(),
            Experience::Duree(m) => format!("duree:{m}"),
            Experience::Demandee => "demandee".into(),
            Experience::Souhaitee => "souhaitee".into(),
        }
    }

    pub fn depuis_code(c: &str) -> Option<Experience> {
        match c {
            "debutant" => Some(Experience::Debutant),
            "demandee" => Some(Experience::Demandee),
            "souhaitee" => Some(Experience::Souhaitee),
            _ => c
                .strip_prefix("duree:")
                .and_then(|m| m.parse::<u16>().ok())
                .filter(|m| (1..=600).contains(m))
                .map(Experience::Duree),
        }
    }

    /// « Débutant accepté », « 2 ans d'expérience », « Expérience demandée ».
    pub fn nom(self) -> String {
        match self {
            Experience::Debutant => "Débutant accepté".into(),
            Experience::Duree(m) => format!("{} d'expérience", duree_texte(m)),
            Experience::Demandee => "Expérience demandée".into(),
            Experience::Souhaitee => "Expérience appréciée, pas exigée".into(),
        }
    }

    /// Pour une raison : « débutant accepté », « demande 2 ans d'expérience ».
    pub fn demande(self) -> String {
        match self {
            Experience::Debutant => "débutant accepté".into(),
            Experience::Duree(m) => format!("demande {} d'expérience", duree_texte(m)),
            Experience::Demandee => "expérience demandée".into(),
            Experience::Souhaitee => "expérience appréciée".into(),
        }
    }
}

/// « 6 mois », « 1 an », « 1 an et demi », « 2 ans ».
pub fn duree_texte(mois: u16) -> String {
    match (mois / 12, mois % 12) {
        (0, m) => format!("{m} mois"),
        (1, 0) => "1 an".into(),
        (1, 6) => "1 an et demi".into(),
        (a, 0) => format!("{a} ans"),
        (a, 6) => format!("{a} ans et demi"),
        (a, m) => format!("{a} an{} et {m} mois", if a > 1 { "s" } else { "" }),
    }
}

/// La confiance dans ce qui a été lu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confiance {
    /// Déduit d'une tournure (« formation assurée », « ouvert à tous »), ou lu par Claude
    Moyenne,
    /// Dit en clair
    Forte,
}

impl Confiance {
    pub fn code(self) -> &'static str {
        match self {
            Confiance::Forte => "forte",
            Confiance::Moyenne => "moyenne",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Confiance> {
        match c {
            "forte" => Some(Confiance::Forte),
            "moyenne" => Some(Confiance::Moyenne),
            _ => None,
        }
    }
}

/// Qui a lu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuPar {
    Regles,
    Ia,
}

impl LuPar {
    /// Le code de la base (`diplome_lu_par`, `experience_lue_par`).
    pub fn code(self) -> &'static str {
        match self {
            LuPar::Regles => "regles",
            LuPar::Ia => "ia",
        }
    }

    pub fn depuis_code(c: &str) -> Option<LuPar> {
        match c {
            "regles" => Some(LuPar::Regles),
            "ia" => Some(LuPar::Ia),
            _ => None,
        }
    }

    /// « lu par Jobmind », « lu par Claude ».
    pub fn nom(self) -> &'static str {
        match self {
            LuPar::Regles => "lu par Jobmind",
            LuPar::Ia => "lu par Claude",
        }
    }
}

/// Une chose trouvée, avec sa preuve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trouve<T> {
    pub valeur: T,
    /// Le bout de phrase de l'offre, tel quel
    pub preuve: String,
    pub confiance: Confiance,
}

/// Ce que les règles ont lu dans une offre.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LectureOffre {
    /// Rien : l'offre ne le dit pas (ou les règles n'ont pas su)
    pub diplome: Option<Trouve<Diplome>>,
    pub experience: Option<Trouve<Experience>>,
    /// Le permis et le CACES demandés (« Permis B », « CACES 1, 3, 5 »)
    pub autres: Vec<String>,
}

impl LectureOffre {
    /// Les règles ont trouvé le diplôme et l'expérience : pas besoin de Claude.
    pub fn classee(&self) -> bool {
        self.diplome.is_some() && self.experience.is_some()
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Les phrases
// ---------------------------------------------------------------------------------------------------------------

/// Une phrase de l'offre : son texte tel quel, ses mots remis au propre, et la place de chaque mot dans le texte.
struct Phrase<'a> {
    brut: &'a str,
    mots: Vec<String>,
    places: Vec<(usize, usize)>,
}

/// Les abréviations après lesquelles un point ne finit pas la phrase.
const ABREVIATIONS: [&str; 12] = ["exp", "dipl", "min", "mini", "max", "env", "cf", "ex", "etc", "niv", "cat", "h"];

/// Coupe un texte en phrases : aux fins de ligne, aux puces, à « ; », « ! », « ? », et aux points qui finissent une
/// phrase (pas celui d'une abréviation comme « exp. », ni « 2.5 »).
fn phrases(t: &str) -> Vec<&str> {
    let mut l = Vec::new();
    let mut debut = 0;
    let cs: Vec<(usize, char)> = t.char_indices().collect();
    for (k, (i, c)) in cs.iter().enumerate() {
        let coupe = match c {
            '\n' | '\r' | ';' | '!' | '?' | '•' | '|' | '·' | '*' | '\u{25cf}' | '\u{25aa}' | '\u{2023}' => true,
            '.' => {
                let suivant = cs.get(k + 1).map(|x| x.1);
                let mot_avant: String =
                    t[debut..*i].rsplit(|x: char| !x.is_alphanumeric()).next().unwrap_or_default().to_lowercase();
                let chiffres = suivant.is_some_and(|x| x.is_ascii_digit())
                    && mot_avant.chars().last().is_some_and(|x| x.is_ascii_digit());
                !chiffres && !ABREVIATIONS.contains(&mot_avant.as_str())
            }
            _ => false,
        };
        if coupe {
            l.push(&t[debut..*i]);
            debut = i + c.len_utf8();
        }
    }
    l.push(&t[debut..]);
    l.into_iter().filter(|p| p.chars().any(char::is_alphanumeric)).collect()
}

fn decouper(brut: &str) -> Phrase<'_> {
    let mut mots = Vec::new();
    let mut places = Vec::new();
    let mut debut: Option<usize> = None;
    for (i, c) in brut.char_indices().chain(std::iter::once((brut.len(), ' '))) {
        if c.is_alphanumeric() {
            debut.get_or_insert(i);
        } else if let Some(d) = debut.take() {
            let m = normaliser(&brut[d..i]);
            // (les fautes de frappe courantes des annonces)
            let m = match m.as_str() {
                "dipolme" | "diplme" | "dilpome" | "diplom" => "diplome".to_string(),
                "experiance" | "experence" | "exprience" | "experince" | "experiene" => "experience".to_string(),
                "experiances" | "experences" => "experiences".to_string(),
                _ => m,
            };
            if !m.is_empty() {
                mots.push(m);
                places.push((d, i));
            }
        }
    }
    Phrase { brut, mots, places }
}

impl Phrase<'_> {
    fn mot(&self, i: usize) -> &str {
        self.mots.get(i).map_or("", String::as_str)
    }

    /// La place (le premier mot) de cette suite de mots (« pas de diplome »).
    fn trouver(&self, motif: &str) -> Option<usize> {
        let m: Vec<&str> = motif.split(' ').collect();
        if m.len() > self.mots.len() {
            return None;
        }
        (0..=self.mots.len() - m.len()).find(|&i| m.iter().enumerate().all(|(k, x)| self.mots[i + k] == *x))
    }

    /// Le premier de ces motifs trouvé : sa place et sa longueur en mots.
    fn premier(&self, motifs: &[&str]) -> Option<(usize, usize)> {
        motifs.iter().find_map(|m| self.trouver(m).map(|i| (i, m.split(' ').count())))
    }

    fn a(&self, mots: &[&str]) -> bool {
        self.mots.iter().any(|m| mots.contains(&m.as_str()))
    }

    /// Un mot de cette liste entre ces places (bornes comprises, coupées au texte).
    fn entre(&self, de: isize, a: isize, mots: &[&str]) -> bool {
        let de = de.max(0) as usize;
        let a = a.min(self.mots.len() as isize - 1);
        a >= 0 && (de..=a as usize).any(|i| mots.contains(&self.mots[i].as_str()))
    }

    /// Le bout de phrase autour des mots `i` à `j`, tel qu'écrit dans l'offre (8 mots de chaque côté au plus).
    fn preuve(&self, i: usize, j: usize) -> String {
        if self.places.is_empty() {
            return String::new();
        }
        let a = i.saturating_sub(8);
        let b = (j + 8).min(self.places.len() - 1);
        let (debut, fin) = (self.places[a].0, self.places[b].1);
        // (la parenthèse ou le signe collé au dernier mot : « Bac+2 », « (H/F) »)
        let fin = self.brut[fin..]
            .char_indices()
            .find(|(_, c)| c.is_whitespace() || matches!(c, ',' | ':' | '.'))
            .map_or(self.brut.len(), |(k, _)| fin + k);
        let p = self.brut[debut..fin].split_whitespace().collect::<Vec<_>>().join(" ");
        if a == 0 && b == self.places.len() - 1 {
            let tout = self.brut.split_whitespace().collect::<Vec<_>>().join(" ");
            let tout = tout.trim_matches(|c: char| c == '-' || c == ':' || c.is_whitespace()).to_string();
            if tout.chars().count() <= 180 {
                return tout;
            }
        }
        p
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Le diplôme
// ---------------------------------------------------------------------------------------------------------------

/// Les tournures qui disent qu'aucun diplôme n'est demandé, en clair.
const AUCUN_DIPLOME: &[&str] = &[
    "sans diplome",
    "aucun diplome",
    "pas de diplome",
    "pas besoin de diplome",
    "pas besoin d un diplome",
    "pas besoin d avoir de diplome",
    "pas besoin d avoir un diplome",
    "nul besoin de diplome",
    "diplome non requis",
    "diplome non exige",
    "diplome non obligatoire",
    "diplome non necessaire",
    "diplome non demande",
    "diplome pas exige",
    "diplome pas obligatoire",
    "diplome pas necessaire",
    "diplome pas requis",
    "diplome n est pas exige",
    "diplome n est pas requis",
    "diplome n est pas obligatoire",
    "diplome n est pas necessaire",
    "diplome n est pas demande",
    "diplome n est pas indispensable",
    "diplome exige non",
    "diplome requis non",
    "diplome obligatoire non",
    "diplome demande non",
    "diplome aucun",
    "diplomes non exiges",
    "diplomes non requis",
    "diplome ou non",
    "ni de diplome",
    "ni diplome",
    "sans condition de diplome",
    "sans qualification",
    "aucune qualification",
    "pas de qualification",
    "aucune formation requise",
    "aucune formation exigee",
    "pas de formation requise",
    "pas de formation exigee",
    "sans formation prealable",
    "aucune formation prealable",
    "niveau d etudes aucun",
    "aucun niveau d etudes",
    "aucun niveau requis",
    "aucun niveau exige",
];

/// Les tournures d'où se déduit qu'aucun diplôme n'est demandé.
const AUCUN_DIPLOME_MOYEN: &[&str] = &[
    "aucun prerequis",
    "aucun pre requis",
    "sans prerequis",
    "sans pre requis",
    "pas de prerequis",
    "ouvert a tous",
    "ouverte a tous",
    "accessible a tous",
    "ouvert a toutes et a tous",
    "ouvert a toutes et tous",
];

/// Les spécialités qui suivent « CAP » ou « BP » (« CAP cuisine », « BP coiffure ») : là, c'est un diplôme.
const SPECIALITES_CAP: [&str; 44] = [
    "cuisine",
    "cuisinier",
    "patissier",
    "patisserie",
    "boulanger",
    "boulangerie",
    "boucher",
    "boucherie",
    "charcutier",
    "poissonnier",
    "coiffure",
    "esthetique",
    "petite",
    "aepe",
    "accompagnant",
    "electricien",
    "electricite",
    "maconnerie",
    "macon",
    "plombier",
    "plomberie",
    "menuisier",
    "menuiserie",
    "carreleur",
    "peintre",
    "vente",
    "commerce",
    "mecanique",
    "logistique",
    "agent",
    "employe",
    "serveur",
    "restaurant",
    "fleuriste",
    "couture",
    "chaudronnerie",
    "soudure",
    "maintenance",
    "conducteur",
    "operateur",
    "proprete",
    "hygiene",
    "charpente",
    "sanitaire",
];

/// Les mots qui disent qu'une phrase parle de diplôme ou d'exigence (un « bac » ou un « CAP » y est un diplôme).
const CONTEXTE: [&str; 36] = [
    "demande",
    "demandee",
    "diplome",
    "diplomes",
    "diplomee",
    "diplomees",
    "dipl",
    "niveau",
    "niv",
    "titulaire",
    "exige",
    "exigee",
    "exiges",
    "requis",
    "requise",
    "minimum",
    "mini",
    "obligatoire",
    "souhaite",
    "souhaitee",
    "apprecie",
    "appreciee",
    "equivalent",
    "equivalence",
    "profil",
    "detenteur",
    "detentrice",
    "formation",
    "type",
    "avez",
    "possedez",
    "obtenu",
    "qualification",
    "etudes",
    "indispensable",
    "imperatif",
];

const EXIGENCES: [&str; 14] = [
    "exige",
    "exigee",
    "exiges",
    "exigees",
    "requis",
    "requise",
    "obligatoire",
    "necessaire",
    "indispensable",
    "imperatif",
    "demande",
    "demandee",
    "minimum",
    "mini",
];

const SOUHAITS: [&str; 11] = [
    "souhaite",
    "souhaitee",
    "souhaitable",
    "apprecie",
    "appreciee",
    "appreciable",
    "plus",
    "idealement",
    "atout",
    "bienvenu",
    "preference",
];

/// Les mots devant « cap » quand c'est le mot (« passer un cap », « garder le cap », « un nouveau cap »), pas le diplôme.
const CAP_LE_MOT: [&str; 23] = [
    "passer",
    "passe",
    "franchir",
    "franchi",
    "franchit",
    "garder",
    "maintenir",
    "changer",
    "change",
    "mettre",
    "mettez",
    "mettons",
    "nouveau",
    "grand",
    "prochain",
    "dernier",
    "premier",
    "tenir",
    "tenez",
    "fixer",
    "donner",
    "donnez",
    "prendre",
];

fn chiffre(m: &str) -> Option<u32> {
    (!m.is_empty() && m.len() <= 2 && m.bytes().all(|b| b.is_ascii_digit())).then(|| m.parse().ok()).flatten()
}

fn niveau_bac(n: u32) -> Diplome {
    match n {
        0 | 1 => Diplome::Bac,
        2 => Diplome::Bac2,
        3 | 4 => Diplome::Bac3,
        _ => Diplome::Bac5,
    }
}

/// Un niveau trouvé : le diplôme, ses mots (de `i` à `j`), sûr (un mot qui ne peut être que ça), nié (« bac non
/// exigé ») ou seulement souhaité.
struct Niveau {
    diplome: Diplome,
    i: usize,
    j: usize,
    sur: bool,
    nie: bool,
    souhaite: bool,
}

fn niveaux(p: &Phrase) -> Vec<Niveau> {
    let contexte = p.a(&CONTEXTE);
    let mut l = Vec::new();
    for i in 0..p.mots.len() {
        let w = p.mot(i);
        let (suiv, prec) = (p.mot(i + 1), if i > 0 { p.mot(i - 1) } else { "" });
        let trouve: Option<(Diplome, usize, bool)> = match w {
            "bac" => {
                if let Some(n) = chiffre(suiv) {
                    Some((niveau_bac(n), i + 1, true))
                } else if [
                    "pro",
                    "professionnel",
                    "general",
                    "technologique",
                    "stmg",
                    "sti2d",
                    "st2s",
                    "stav",
                    "es",
                    "s",
                    "l",
                    "ou",
                ]
                .contains(&suiv)
                {
                    Some((Diplome::Bac, i, true))
                } else if suiv == "a" && p.mot(i + 2).starts_with("bac") {
                    // (« Bac à Bac+2 »)
                    Some((Diplome::Bac, i, true))
                } else if ["a", "de", "d", "du", "des", "en", "plastique", "roulant", "sable"].contains(&suiv) {
                    None
                } else {
                    contexte.then_some((Diplome::Bac, i, false))
                }
            }
            w if w.len() > 3 && w.starts_with("bac") && chiffre(&w[3..]).is_some() => {
                chiffre(&w[3..]).map(|n| (niveau_bac(n), i, true))
            }
            "baccalaureat" => Some((Diplome::Bac, i, true)),
            "bts" | "dut" | "deug" | "deust" | "btsa" => Some((Diplome::Bac2, i, true)),
            "bachelor" => Some((Diplome::Bac3, i, true)),
            "licence" => {
                if ["iv", "4", "d", "de", "ffr", "sportive", "debit", "boisson", "boissons"].contains(&suiv) {
                    None
                } else if ["pro", "professionnelle", "generale"].contains(&suiv) {
                    Some((Diplome::Bac3, i, true))
                } else {
                    contexte.then_some((Diplome::Bac3, i, false))
                }
            }
            "master" | "mastere" if prec != "scrum" => Some((Diplome::Bac5, i, true)),
            "doctorat" | "phd" | "mba" | "dess" => Some((Diplome::Bac5, i, true)),
            "ingenieur" | "ingenieurs"
                if prec == "d"
                    && i >= 2
                    && ["diplome", "ecole", "formation", "titre", "cursus"].contains(&p.mot(i - 2)) =>
            {
                Some((Diplome::Bac5, i, true))
            }
            // (« passer un cap », « garder le cap », « changer de cap », « cap sur » : le mot, pas le diplôme. Un CAP
            // qui n'est ni suivi de BEP ni d'une spécialité ne compte que s'il est écrit en capitales, « CAP »)
            "cap" if p.entre(i as isize - 3, i as isize - 1, &CAP_LE_MOT) || suiv == "sur" => None,
            "cap" => {
                let capitales = p.places.get(i).is_some_and(|&(d, f)| &p.brut[d..f] == "CAP");
                if suiv == "bep" || prec == "bep" {
                    Some((Diplome::CapBep, if suiv == "bep" { i + 1 } else { i }, true))
                } else if suiv == "a" && p.mot(i + 2).starts_with("bac") {
                    // (« CAP à Bac+2 »)
                    Some((Diplome::CapBep, i, true))
                } else if SPECIALITES_CAP.contains(&suiv) {
                    Some((Diplome::CapBep, i + 1, true))
                } else if ["sur", "vers", "a", "horn", "ferret", "d", "est", "ouest", "nord", "sud"].contains(&suiv) {
                    None
                } else if capitales
                    && (contexte || ["un", "du", "le", "votre", "son", "en", "les", "au"].contains(&prec))
                {
                    Some((Diplome::CapBep, i, false))
                } else {
                    None
                }
            }
            "bep" if prec != "cap" => Some((Diplome::CapBep, i, true)),
            "brevet" if suiv == "professionnel" => Some((Diplome::Bac, i + 1, true)),
            "bpjeps" => Some((Diplome::Bac, i, true)),
            "bp" if suiv == "jeps" => Some((Diplome::Bac, i + 1, true)),
            "bp" if contexte || SPECIALITES_CAP.contains(&suiv) => Some((Diplome::Bac, i, false)),
            "niveau"
                if p.a(&["diplome", "rncp", "bac", "titre", "qualification", "etude", "etudes", "cap", "dipl"]) =>
            {
                match chiffre(suiv) {
                    Some(3) => Some((Diplome::CapBep, i + 1, false)),
                    Some(4) => Some((Diplome::Bac, i + 1, false)),
                    Some(5) => Some((Diplome::Bac2, i + 1, false)),
                    Some(6) => Some((Diplome::Bac3, i + 1, false)),
                    Some(7 | 8) => Some((Diplome::Bac5, i + 1, false)),
                    _ => None,
                }
            }
            _ => None,
        };
        let Some((diplome, j, sur)) = trouve else { continue };
        // Nié : « sans le bac », « pas besoin du bac », « bac non exigé », « le bac n'est pas obligatoire »
        let avant = |k: usize| if i >= k { p.mot(i - k) } else { "" };
        let nie_avant = avant(1) == "sans"
            || (avant(2) == "sans" && ["le", "un", "de"].contains(&avant(1)))
            || (["pas", "besoin"].contains(&avant(2)) && ["de", "d", "du"].contains(&avant(1)))
            || (avant(3) == "pas" && avant(2) == "besoin");
        let mut nie_apres = false;
        for k in j + 1..(j + 5).min(p.mots.len()) {
            let (x, y) = (p.mot(k), p.mot(k + 1));
            // (la suite parle d'autre chose : « CAP apprécié, expérience exigée : non »)
            if MOTS_EXPERIENCE.contains(&x) || ["permis", "caces", "debutant", "debutants"].contains(&x) {
                break;
            }
            if (["non", "pas"].contains(&x) && EXIGENCES[..9].contains(&y))
                || (EXIGENCES[..9].contains(&x) && y == "non")
            {
                nie_apres = true;
            }
        }
        let souhaite =
            p.entre(j as isize + 1, j as isize + 6, &SOUHAITS) || p.entre(i as isize - 3, i as isize - 1, &SOUHAITS);
        l.push(Niveau { diplome, i, j, sur, nie: nie_avant || nie_apres, souhaite });
    }
    l
}

/// Une rubrique d'annonce qui répond « non » (« Diplôme : non », « Expérience : non - Permis : B ») : le mot, puis
/// « non » en fin de phrase ou avant la rubrique suivante.
fn rubrique_non(p: &Phrase, mot: &str) -> Option<(usize, usize)> {
    let i = p.trouver(&format!("{mot} non"))?;
    let suite = p.mot(i + 2);
    let rubriques = [
        "diplome",
        "experience",
        "permis",
        "salaire",
        "contrat",
        "lieu",
        "horaires",
        "poste",
        "langue",
        "langues",
        "formation",
        "niveau",
        "caces",
        "teletravail",
        "duree",
    ];
    (suite.is_empty() || rubriques.contains(&suite)).then_some((i, 2))
}

/// Le diplôme demandé, d'après toutes les phrases.
fn diplome(ps: &[Phrase]) -> Option<Trouve<Diplome>> {
    let mut aucun: Option<Trouve<Diplome>> = None;
    let mut aucun_moyen: Option<Trouve<Diplome>> = None;
    // (le niveau le plus bas exigé, et le plus bas souhaité)
    let mut exige: Option<(Diplome, Trouve<Diplome>)> = None;
    let mut souhaite: Option<(Diplome, Trouve<Diplome>)> = None;
    for p in ps {
        if aucun.is_none()
            && let Some((i, n)) = p.premier(AUCUN_DIPLOME).or_else(|| rubrique_non(p, "diplome"))
        {
            aucun =
                Some(Trouve { valeur: Diplome::Aucun, preuve: p.preuve(i, i + n - 1), confiance: Confiance::Forte });
        }
        if aucun_moyen.is_none()
            && let Some((i, n)) = p.premier(AUCUN_DIPLOME_MOYEN)
        {
            aucun_moyen =
                Some(Trouve { valeur: Diplome::Aucun, preuve: p.preuve(i, i + n - 1), confiance: Confiance::Moyenne });
        }
        for n in niveaux(p) {
            if n.nie {
                if aucun.is_none() {
                    aucun = Some(Trouve {
                        valeur: Diplome::Aucun,
                        preuve: p.preuve(n.i, n.j),
                        confiance: Confiance::Moyenne,
                    });
                }
                continue;
            }
            let t = Trouve {
                valeur: n.diplome,
                preuve: p.preuve(n.i, n.j),
                confiance: if n.sur { Confiance::Forte } else { Confiance::Moyenne },
            };
            let place = if n.souhaite { &mut souhaite } else { &mut exige };
            if place.as_ref().is_none_or(|(d, _)| n.diplome < *d) {
                *place = Some((n.diplome, t));
            }
        }
    }
    // Aucun diplôme dit en clair l'emporte sur un niveau seulement souhaité ; un niveau exigé l'emporte sur une
    // tournure (« ouvert à tous ») ; deux choses contraires : la confiance baisse
    match (aucun, exige, souhaite, aucun_moyen) {
        (Some(a), None, _, _) => Some(a),
        (Some(_), Some((_, mut e)), _, _) => {
            e.confiance = Confiance::Moyenne;
            Some(e)
        }
        (None, Some((_, e)), _, _) => Some(e),
        (None, None, _, Some(a)) => Some(a),
        (None, None, Some((_, s)), None) => Some(s),
        (None, None, None, None) => None,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// L'expérience
// ---------------------------------------------------------------------------------------------------------------

/// « Pas de débutant » : une expérience est demandée.
const PAS_DE_DEBUTANT: &[&str] = &[
    "debutant s abstenir",
    "debutants s abstenir",
    "debutant e s abstenir",
    "debutants s abstenant",
    "sans experience s abstenir",
    "pas de debutant",
    "pas de debutants",
    "pas de debutant e",
    "debutant non accepte",
    "debutants non acceptes",
    "debutant e non accepte e",
    "debutants refuses",
    "debutants non admis",
    "pas ouvert aux debutants",
];

/// Débutant accepté, dit en clair.
const DEBUTANT: &[&str] = &[
    "debutant accepte",
    "debutants acceptes",
    "debutant acceptee",
    "debutante acceptee",
    "debutantes acceptees",
    "debutants acceptees",
    "debutant e accepte",
    "debutant e accepte e",
    "debutant e s accepte e s",
    "debutant e s acceptes",
    "debutants accepte",
    "debutant bienvenu",
    "debutants bienvenus",
    "debutant e bienvenu e",
    "debutant e s bienvenu e s",
    "debutants bienvenu",
    "debutants sont bienvenus",
    "debutants sont acceptes",
    "debutant possible",
    "debutants possible",
    "debutants possibles",
    "debutant ok",
    "debutants ok",
    "debutant accepte e",
    "ouvert aux debutants",
    "ouverte aux debutants",
    "ouvert aux debutant",
    "accessible aux debutants",
    "accessibles aux debutants",
    "accessible a un debutant",
    "accessible aux debutant e s",
    "debutant ou confirme",
    "debutant ou experimente",
    "debutant e ou confirme e",
    "debutant e ou experimente e",
    "debutants ou confirmes",
    "debutants ou experimentes",
    "debutant ou non",
    "debutant e ou non",
    "que vous soyez debutant",
    "que vous soyez debutant e",
    "vous etes debutant",
    "vous etes debutant e",
    "vous debutez",
    "si vous debutez",
    "meme debutant",
    "meme debutante",
    "meme debutant e",
    "meme sans experience",
    "ni d experience",
    "ni experience",
    "ni de l experience",
    "sans experience",
    "aucune experience",
    "pas d experience",
    "pas besoin d experience",
    "pas de condition d experience",
    "sans condition d experience",
    "pas besoin d avoir d experience",
    "pas besoin d avoir de l experience",
    "nul besoin d experience",
    "pas d experience exigee",
    "pas d experience requise",
    "pas d experience necessaire",
    "experience non requise",
    "experience non exigee",
    "experience non obligatoire",
    "experience non necessaire",
    "experience non indispensable",
    "experience pas exigee",
    "experience pas obligatoire",
    "experience pas necessaire",
    "experience pas requise",
    "experience n est pas exigee",
    "experience n est pas requise",
    "experience n est pas obligatoire",
    "experience n est pas necessaire",
    "experience n est pas indispensable",
    "experience exigee non",
    "experience requise non",
    "experience obligatoire non",
    "experience demandee non",
    "experience aucune",
    "experience debutant",
    "experience debutant accepte",
    "experience debutant e",
    "premiere experience non exigee",
    "premiere experience non requise",
    "premiere experience non obligatoire",
    "jeune diplome accepte",
    "jeunes diplomes acceptes",
    "jeune diplome e accepte e",
    "jeunes diplome e s acceptes",
];

/// Débutant accepté, déduit d'une tournure.
const DEBUTANT_MOYEN: &[&str] = &[
    "formation assuree",
    "formation interne assuree",
    "formation en interne assuree",
    "formation au poste assuree",
    "formation sur le terrain assuree",
    "formation assuree en interne",
    "formation sera assuree",
    "formation est assuree",
    "nous vous formons",
    "nous vous formerons",
    "on vous forme",
    "on vous formera",
    "vous serez forme",
    "vous serez formee",
    "vous serez forme e",
    "formation complete assuree",
    "formation prise en charge",
    "formation offerte",
    "formation en interne",
    "formation interne",
    "jeune diplome",
    "jeunes diplomes",
    "jeune diplomee",
    "jeune diplome e",
    "premier emploi",
    "premiere experience professionnelle",
];

/// Une expérience demandée, en clair.
const DEMANDEE: &[&str] = &[
    "experience exigee",
    "experience obligatoire",
    "experience requise",
    "experience indispensable",
    "experience imperative",
    "experience necessaire",
    "experience demandee",
    "experience exigee oui",
    "experience oui",
    "experience significative",
    "experience confirmee",
    "experience solide",
    "solide experience",
    "experience averee",
    "experience probante",
    "experience reussie",
    "forte experience",
    "experience exigee dans",
    "justifiez d une experience",
    "justifier d une experience",
    "justifiant d une experience",
    "vous justifiez",
    "experience similaire exigee",
    "experimente",
    "experimentee",
    "experimentes",
    "experimentees",
    "experimente e",
    "confirme",
    "confirmee",
    "confirmes",
    "confirme e",
];

/// Une expérience appréciée, pas exigée, en clair.
const SOUHAITEE: &[&str] = &[
    "experience appreciee",
    "experience souhaitee",
    "experience souhaitable",
    "experience appreciable",
    "experience serait un plus",
    "experience est un plus",
    "experience un plus",
    "experience bienvenue",
    "experience serait appreciee",
    "experience serait un atout",
    "experience est un atout",
    "experience idealement",
    "idealement une experience",
    "idealement une premiere experience",
    "premiere experience appreciee",
    "premiere experience souhaitee",
    "premiere experience serait",
    "premiere experience est un plus",
    "une premiere experience",
    "experience serait",
    "une experience serait",
];

const MOTS_EXPERIENCE: [&str; 9] = [
    "experience",
    "experiences",
    "exp",
    "experimente",
    "experimentee",
    "experimentes",
    "experimentees",
    "anciennete",
    "expertise",
];

/// Ce qui précède une durée qui n'est pas une expérience (un contrat, un essai, l'âge, l'âge de l'entreprise).
const PAS_UNE_EXPERIENCE: [&str; 27] = [
    "interim",
    "age",
    "agee",
    "ages",
    "agees",
    "avoir",
    "cdd",
    "contrat",
    "mission",
    "missions",
    "duree",
    "periode",
    "essai",
    "depuis",
    "cree",
    "creee",
    "fondee",
    "fonde",
    "existe",
    "formation",
    "stage",
    "alternance",
    "engagement",
    "pendant",
    "durant",
    "sous",
    "garantie",
];

fn nombre(m: &str) -> Option<u16> {
    if let Some(n) = chiffre(m) {
        return u16::try_from(n).ok();
    }
    Some(match m {
        "un" | "une" => 1,
        "deux" => 2,
        "trois" => 3,
        "quatre" => 4,
        "cinq" => 5,
        "six" => 6,
        "sept" => 7,
        "huit" => 8,
        "neuf" => 9,
        "dix" => 10,
        "douze" => 12,
        "quinze" => 15,
        "vingt" => 20,
        _ => return None,
    })
}

/// La plus petite durée d'expérience d'une phrase (en mois), avec ses mots.
fn duree(p: &Phrase) -> Option<(u16, usize, usize)> {
    let mut meilleure: Option<(u16, usize, usize)> = None;
    let n = p.mots.len();
    for i in 0..n {
        let Some(n1) = nombre(p.mot(i)) else { continue };
        // « 2 à 3 ans », « 2 ou 3 ans », « 2-3 ans »
        let mut k = i + 1;
        if ["a", "ou", "et"].contains(&p.mot(k)) && nombre(p.mot(k + 1)).is_some() {
            k += 2;
        } else if nombre(p.mot(k)).is_some() && p.mot(i) != "un" && p.mot(i) != "une" {
            k += 1;
        }
        let mois = match p.mot(k) {
            "an" | "ans" | "annee" | "annees" => {
                let demi = p.mot(k + 1) == "et" && p.mot(k + 2) == "demi";
                n1 * 12 + if demi { 6 } else { 0 }
            }
            "mois" => n1,
            _ => continue,
        };
        if mois == 0 || mois > 600 {
            continue;
        }
        let avant = |x: usize| if i >= x { p.mot(i - x) } else { "" };
        // (« il y a 20 ans », « avoir 18 ans », « CDD de 6 mois », « période d'essai de 2 mois »)
        if (avant(1) == "a" && avant(2) == "y")
            || (p.entre(i as isize - 4, i as isize - 1, &PAS_UNE_EXPERIENCE)
                && !p.entre(i as isize - 3, i as isize - 1, &MOTS_EXPERIENCE))
            || (["renouvelable", "renouvelables"].contains(&p.mot(k + 1)) && p.a(&["cdd", "contrat", "mission"]))
        {
            continue;
        }
        if n1 >= 16 && mois >= 192 && p.a(&["age", "agee", "majeur", "majeure", "avoir"]) {
            continue;
        }
        // (sans le mot « expérience » : seulement « 2 ans minimum », « au moins 6 mois », dans une phrase sans contrat)
        let minimum = ["mini", "minimum", "min"].contains(&p.mot(k + 1))
            || (avant(1) == "moins" && avant(2) == "au")
            || ["minimum", "mini"].contains(&avant(1));
        let sans_contrat = !p.a(&PAS_UNE_EXPERIENCE) && mois <= 180;
        if !p.entre(i as isize - 8, k as isize + 5, &MOTS_EXPERIENCE) && !(minimum && sans_contrat) {
            continue;
        }
        if meilleure.is_none_or(|(m, ..)| mois < m) {
            meilleure = Some((mois, i, k));
        }
    }
    meilleure
}

/// Le mot « expérience » d'une phrase dit qu'une expérience est voulue (« une expérience dans la vente »), et pas
/// « salaire selon expérience » ni « expérience client ».
fn experience_voulue(p: &Phrase) -> Option<usize> {
    for i in 0..p.mots.len() {
        if !["experience", "experiences"].contains(&p.mot(i)) {
            continue;
        }
        let avant = |x: usize| if i >= x { p.mot(i - x) } else { "" };
        if ["selon", "fonction", "votre"].contains(&avant(1))
            || ["selon", "fonction"].contains(&avant(2))
            || ["selon", "fonction"].contains(&avant(3))
        {
            continue;
        }
        let apres = p.mot(i + 1);
        if [
            "client",
            "clients",
            "utilisateur",
            "utilisateurs",
            "candidat",
            "collaborateur",
            "salarie",
            "enrichissante",
            "unique",
            "inoubliable",
            "humaine",
            "formidable",
            "patient",
            "employeur",
        ]
        .contains(&apres)
        {
            continue;
        }
        let voulue = [
            "une",
            "d",
            "avez",
            "possedez",
            "justifiez",
            "dote",
            "dotee",
            "avec",
            "forte",
            "solide",
            "bonne",
            "premiere",
            "significative",
            "votre",
            "exp",
        ]
        .contains(&avant(1))
            || [
                "significative",
                "solide",
                "confirmee",
                "reussie",
                "similaire",
                "professionnelle",
                "dans",
                "en",
                "sur",
                "terrain",
                "du",
                "de",
                "d",
                "exigee",
                "requise",
                "souhaitee",
                "appreciee",
                "minimum",
                "prealable",
                "sur",
                "au",
                "a",
                "comme",
                "en tant",
            ]
            .contains(&apres);
        // (« expérience avec les enfants obligatoire »)
        let exigee = p.entre(
            i as isize + 1,
            i as isize + 6,
            &["exigee", "obligatoire", "requise", "indispensable", "imperative", "necessaire", "demandee"],
        );
        if voulue || exigee {
            return Some(i);
        }
    }
    None
}

/// L'expérience demandée, d'après toutes les phrases. Débutant accepté en clair l'emporte, puis la plus petite durée,
/// puis une expérience exigée, puis appréciée, puis une tournure comme « formation assurée ».
fn experience(ps: &[Phrase]) -> Option<Trouve<Experience>> {
    let mut debutant: Option<Trouve<Experience>> = None;
    let mut pas_debutant: Option<Trouve<Experience>> = None;
    let mut plus_courte: Option<Trouve<Experience>> = None;
    let mut demandee: Option<Trouve<Experience>> = None;
    let mut souhaitee: Option<Trouve<Experience>> = None;
    let mut debutant_moyen: Option<Trouve<Experience>> = None;
    let t = |v, p: &Phrase, i, j, c| Trouve { valeur: v, preuve: p.preuve(i, j), confiance: c };
    for p in ps {
        if let Some((i, n)) = p.premier(PAS_DE_DEBUTANT) {
            pas_debutant.get_or_insert(t(Experience::Demandee, p, i, i + n - 1, Confiance::Forte));
            continue;
        }
        if let Some((i, n)) = p.premier(DEBUTANT).or_else(|| rubrique_non(p, "experience")) {
            debutant.get_or_insert(t(Experience::Debutant, p, i, i + n - 1, Confiance::Forte));
        }
        if let Some((m, i, j)) = duree(p)
            && plus_courte.as_ref().is_none_or(|x| matches!(x.valeur, Experience::Duree(y) if m < y))
        {
            plus_courte = Some(t(Experience::Duree(m), p, i, j, Confiance::Forte));
        }
        let doux = p.a(&SOUHAITS) || p.trouver("un plus").is_some();
        if let Some((i, n)) = p.premier(SOUHAITEE) {
            souhaitee.get_or_insert(t(Experience::Souhaitee, p, i, i + n - 1, Confiance::Forte));
        } else if let Some((i, n)) = p.premier(DEMANDEE) {
            let fin = (i + n + 2).min(p.mots.len());
            let fort = ["exigee", "obligatoire", "requise", "indispensable", "imperative", "necessaire", "oui"]
                .iter()
                .any(|x| p.mots[i..fin].iter().any(|m| m == x));
            if doux && !fort {
                souhaitee.get_or_insert(t(Experience::Souhaitee, p, i, i + n - 1, Confiance::Moyenne));
            } else {
                demandee.get_or_insert(t(Experience::Demandee, p, i, i + n - 1, Confiance::Forte));
            }
        } else if let Some(i) = experience_voulue(p) {
            if doux {
                souhaitee.get_or_insert(t(Experience::Souhaitee, p, i, i, Confiance::Moyenne));
            } else {
                demandee.get_or_insert(t(Experience::Demandee, p, i, i, Confiance::Moyenne));
            }
        }
        if let Some((i, n)) = p.premier(DEBUTANT_MOYEN) {
            debutant_moyen.get_or_insert(t(Experience::Debutant, p, i, i + n - 1, Confiance::Moyenne));
        }
    }
    debutant.or(plus_courte).or(pas_debutant).or(demandee).or(souhaitee).or(debutant_moyen)
}

// ---------------------------------------------------------------------------------------------------------------
// Le permis et le CACES
// ---------------------------------------------------------------------------------------------------------------

fn autres(ps: &[Phrase]) -> Vec<String> {
    let mut l: Vec<String> = Vec::new();
    let mut ajouter = |x: String| {
        if !l.contains(&x) {
            l.push(x);
        }
    };
    for p in ps {
        for i in 0..p.mots.len() {
            match p.mot(i) {
                "permis" => {
                    let mut k = i + 1;
                    if p.mot(k) == "de" && p.mot(k + 1) == "conduire" {
                        k += 2;
                    }
                    let lettre = p.mot(k);
                    if ["b", "c", "ce", "c1", "c1e", "d", "de", "be", "a", "a2", "am", "ec"].contains(&lettre)
                        && !(lettre == "de" && k == i + 1)
                    {
                        ajouter(format!("Permis {}", lettre.to_uppercase()));
                    } else if lettre == "poids" && p.mot(k + 1) == "lourd" || lettre == "pl" || lettre == "spl" {
                        ajouter("Permis poids lourd".into());
                    } else if k == i + 3 {
                        ajouter("Permis de conduire".into());
                    }
                }
                "caces" => {
                    let mut cats: Vec<String> = Vec::new();
                    let mut reco = String::new();
                    let mut k = i + 1;
                    while k < p.mots.len() && k < i + 12 {
                        let w = p.mot(k);
                        if let Some(n) = chiffre(w).filter(|n| (1..=10).contains(n)) {
                            if !cats.contains(&n.to_string()) {
                                cats.push(n.to_string());
                            }
                        } else if w.len() == 2 && w.starts_with(|c: char| c.is_ascii_digit()) && w.ends_with(['a', 'b'])
                        {
                            cats.push(w.to_uppercase());
                        } else if w.starts_with('r') && w.len() == 4 && w[1..].bytes().all(|b| b.is_ascii_digit()) {
                            reco = w.to_uppercase();
                        } else if w == "r"
                            && p.mot(k + 1).len() == 3
                            && p.mot(k + 1).bytes().all(|b| b.is_ascii_digit())
                        {
                            reco = format!("R{}", p.mot(k + 1));
                            k += 1;
                        } else if ![
                            "cat",
                            "categorie",
                            "categories",
                            "et",
                            "ou",
                            "n",
                            "no",
                            "de",
                            "en",
                            "valide",
                            "valides",
                            "cours",
                        ]
                        .contains(&w)
                        {
                            break;
                        }
                        k += 1;
                    }
                    let mut x = "CACES".to_string();
                    if !reco.is_empty() {
                        x.push(' ');
                        x.push_str(&reco);
                    }
                    if !cats.is_empty() {
                        x.push(' ');
                        x.push_str(&cats.join(", "));
                    }
                    ajouter(x);
                }
                _ => {}
            }
        }
    }
    l
}

/// Ce que les règles lisent dans une offre : son titre et son texte.
pub fn lire(titre: &str, texte: &str) -> LectureOffre {
    let mut morceaux = phrases(titre);
    morceaux.extend(phrases(texte));
    let ps: Vec<Phrase> = morceaux.into_iter().map(decouper).filter(|p| !p.mots.is_empty()).collect();
    let experience = experience(&ps);
    let mut diplome = diplome(&ps);
    // « Débutants acceptés, formation assurée » : aucun diplôme demandé, déduit
    if diplome.is_none() && experience.as_ref().is_some_and(|e| e.valeur == Experience::Debutant) {
        for p in &ps {
            if let Some((i, n)) = p.premier(&DEBUTANT_MOYEN[..20]) {
                diplome = Some(Trouve {
                    valeur: Diplome::Aucun,
                    preuve: p.preuve(i, i + n - 1),
                    confiance: Confiance::Moyenne,
                });
                break;
            }
        }
    }
    LectureOffre { diplome, experience, autres: autres(&ps) }
}

/// Le plus haut diplôme cité dans un texte (un CV) : les niveaux trouvés, sauf ceux qui sont niés.
pub fn diplome_le_plus_haut(t: &str) -> Option<Diplome> {
    phrases(t)
        .into_iter()
        .map(decouper)
        .flat_map(|p| niveaux(&p).into_iter().filter(|n| !n.nie).map(|n| n.diplome).collect::<Vec<_>>())
        .max()
}

/// Une citation se trouve vraiment dans le texte : mot pour mot (sans tenir compte des accents, des majuscules, de la
/// ponctuation ni des espaces), et au moins deux mots. Sert à vérifier ce que rend Claude (rien d'inventé).
pub fn citation_presente(texte: &str, citation: &str) -> bool {
    let c = normaliser(citation);
    if c.split(' ').filter(|m| !m.is_empty()).count() < 2 || c.chars().count() < 6 {
        return false;
    }
    format!(" {} ", normaliser(texte)).contains(&format!(" {c} "))
}

#[cfg(test)]
mod tests;
