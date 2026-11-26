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
//!
//! Avant les phrases, la rubrique « Profil souhaité » d'une offre France Travail (« Expérience », « Formation »,
//! « Permis », une ligne par élément, avec « Exigé » ou « Souhaité ») est lue ligne à ligne (`profil`) : c'est la
//! référence, elle passe avant ce que dit le texte libre.

use crate::offre::normaliser;

/// La version des règles : une offre lue par une version plus ancienne est relue (la base le note).
/// (2 : la rubrique « Profil souhaité » des offres France Travail, 20/11/2026 ; 3 : un âge n'est plus une durée
/// d'expérience, les lettres et les listes de permis, 26/11/2026 ; 4 : les avantages, l'expérience de l'entreprise, le
/// fourgon « Master », les négations de « débutant accepté » et les durées seulement souhaitées ne trompent plus,
/// 26/11/2026)
pub const VERSION_REGLES: i64 = 4;

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

    /// Ce qui est écrit entre le mot `x` et le suivant, tel quel (« , », « + », « : »).
    fn joint(&self, x: usize) -> &str {
        match (self.places.get(x), self.places.get(x + 1)) {
            (Some(a), Some(b)) => &self.brut[a.1..b.0],
            _ => "",
        }
    }

    /// Rien que des espaces entre le mot `x` et le suivant.
    fn colle(&self, x: usize) -> bool {
        self.joint(x).chars().all(char::is_whitespace)
    }

    /// Le mot `x`, tel qu'écrit dans l'offre.
    fn ecrit(&self, x: usize) -> &str {
        self.places.get(x).map_or("", |(d, f)| &self.brut[*d..*f])
    }

    /// Le morceau de phrase où se trouve le mot `i` (entre deux virgules, deux parenthèses ou deux tirets isolés) :
    /// son premier et son dernier mot.
    fn morceau(&self, i: usize) -> (usize, usize) {
        let coupe = |x: usize| {
            let s = self.joint(x);
            let seul = s.trim();
            s.contains([',', '(', ')']) || (matches!(seul, "-" | "\u{2013}" | "\u{2014}") && seul.len() < s.len())
        };
        let n = self.mots.len();
        let (mut a, mut b) = (i.min(n.saturating_sub(1)), i.min(n.saturating_sub(1)));
        while a > 0 && !coupe(a - 1) {
            a -= 1;
        }
        while b + 1 < n && !coupe(b) {
            b += 1;
        }
        (a, b)
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

/// Le chiffre d'un « Bac+2 » écrit en deux mots (« Bac+2 », « Bac + 2 », « bac 2 ») : entre « bac » et le chiffre, rien
/// qu'un « + » ou des espaces (pas « Bac, 3 ans d'expérience »), et pas une durée d'expérience (« Bac + 2 ans
/// d'expérience » : le bac, et deux ans).
fn chiffre_du_bac(p: &Phrase, i: usize) -> Option<u32> {
    if p.mot(i) != "bac" {
        return None;
    }
    let n = chiffre(p.mot(i + 1))?;
    let colle = p.joint(i).chars().all(|c| c.is_whitespace() || c == '+');
    let duree = UNITES.contains(&p.mot(i + 2))
        && (MOTS_EXPERIENCE.contains(&p.mot(i + 3))
            || (["d", "de"].contains(&p.mot(i + 3)) && MOTS_EXPERIENCE.contains(&p.mot(i + 4))));
    (colle && !duree).then_some(n)
}

/// Les mots qui disent qu'un « Master » est un fourgon, à coup sûr.
const VEHICULES: [&str; 22] = [
    "renault",
    "fourgon",
    "fourgons",
    "fourgonnette",
    "camion",
    "camions",
    "camionnette",
    "camionnettes",
    "utilitaire",
    "utilitaires",
    "vehicule",
    "vehicules",
    "vul",
    "trafic",
    "kangoo",
    "iveco",
    "jumper",
    "boxer",
    "sprinter",
    "ducato",
    "crafter",
    "volant",
];

/// Les mots qui le laissent penser (sauf si la phrase parle de diplôme).
const VEHICULES_PEUT_ETRE: [&str; 11] = [
    "livraison",
    "livraisons",
    "livreur",
    "livreuse",
    "chauffeur",
    "chauffeuse",
    "conduite",
    "conduire",
    "conduisez",
    "tournee",
    "tournees",
];

/// Ce qui suit « master » quand ce n'est pas le diplôme (« master data », « master class », un fourgon « Master L2H2 »).
const MASTER_AUTRE_CHOSE: [&str; 20] = [
    "data",
    "class",
    "classe",
    "classes",
    "planner",
    "scheduler",
    "franchise",
    "franchisee",
    "black",
    "plan",
    "batch",
    "l1h1",
    "l2h2",
    "l3h2",
    "l3h3",
    "benne",
    "frigo",
    "frigorifique",
    "dci",
    "chef",
];

/// Le mot « master » de place `i` est le diplôme : sûr (la phrase parle de diplôme, « Master 2 », « un master en... »)
/// ou seulement probable ; rien : un fourgon (« Renault Master »), un « scrum master », une « master class ».
fn master(p: &Phrase, i: usize, contexte: bool) -> Option<bool> {
    let (suiv, prec) = (p.mot(i + 1), if i > 0 { p.mot(i - 1) } else { "" });
    if ["scrum", "web", "quiz", "grand", "dock", "brew"].contains(&prec) || MASTER_AUTRE_CHOSE.contains(&suiv) {
        return None;
    }
    // (« Master 20 m3 », « Master 3T5 » : un fourgon ; « Master 1 », « Master 2 » : le diplôme)
    let annee = matches!(chiffre(suiv), Some(1 | 2));
    if suiv.starts_with(|c: char| c.is_ascii_digit()) && !annee {
        return None;
    }
    let avant = |mots: &[&str]| p.entre(i as isize - 5, i as isize - 1, mots);
    if avant(&VEHICULES) || p.a(&["renault"]) {
        return None;
    }
    let diplome = contexte
        || annee
        || [
            "en",
            "ou",
            "pro",
            "professionnel",
            "specialise",
            "specialisee",
            "mention",
            "meef",
            "recherche",
            "universitaire",
            "of",
            "degree",
            "d",
            "de",
        ]
        .contains(&suiv)
        || ["un", "du", "niveau", "ou", "bac", "titulaire", "le", "son", "votre"].contains(&prec)
        || p.a(&["bac", "ingenieur", "ingenieurs", "ecole", "universite", "licence", "doctorat", "bac5"]);
    if avant(&VEHICULES_PEUT_ETRE) && !(contexte || annee) {
        return None;
    }
    Some(diplome)
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
    niveaux_avec(p, false)
}

/// Les niveaux d'une phrase ; `force` : la phrase parle de diplôme quoi qu'il arrive (une ligne de la rubrique
/// « Formation » d'une offre France Travail).
fn niveaux_avec(p: &Phrase, force: bool) -> Vec<Niveau> {
    let contexte = force || p.a(&CONTEXTE);
    let mut l = Vec::new();
    for i in 0..p.mots.len() {
        let w = p.mot(i);
        let (suiv, prec) = (p.mot(i + 1), if i > 0 { p.mot(i - 1) } else { "" });
        let trouve: Option<(Diplome, usize, bool)> = match w {
            "bac" => {
                if let Some(n) = chiffre_du_bac(p, i) {
                    Some((niveau_bac(n), i + 1, true))
                } else if chiffre(suiv).is_some() {
                    // (« Niveau Bac, 3 ans d'expérience » : le bac, puis une durée)
                    contexte.then_some((Diplome::Bac, i, false))
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
            "master" | "mastere" => master(p, i, contexte).map(|sur| (Diplome::Bac5, i, sur)),
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
    "non ouvert aux debutants",
    "pas accessible aux debutants",
    "non accessible aux debutants",
    "pas adapte aux debutants",
    "pas un poste pour debutant",
    "pas un poste de debutant",
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
    "justifiez d experience",
    "justifiez d experiences",
    "experience similaire exigee",
    "experimente",
    "experimentee",
    "experimentes",
    "experimentees",
    "experimente e",
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

/// (« ancienneté » n'en est pas : elle ne compte que dans une phrase qui exige, voir `durees`)
const MOTS_EXPERIENCE: [&str; 8] =
    ["experience", "experiences", "exp", "experimente", "experimentee", "experimentes", "experimentees", "expertise"];

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

/// Ceux dont on dit l'âge : « enfants de 3 à 6 ans », « jeunes de 16 à 25 ans », « âgé de 18 ans » ne sont pas des
/// durées d'expérience.
const AGE_DE: [&str; 28] = [
    "enfant",
    "enfants",
    "bebe",
    "bebes",
    "nourrisson",
    "nourrissons",
    "petits",
    "jeune",
    "jeunes",
    "adolescent",
    "adolescents",
    "ado",
    "ados",
    "eleves",
    "mineur",
    "mineurs",
    "garcons",
    "filles",
    "adulte",
    "adultes",
    "personnes",
    "public",
    "publics",
    "residents",
    "age",
    "agee",
    "ages",
    "agees",
];
/// Ceux dont l'âge se dit aussi sans « de » : « enfants (3-6 ans) », « âgés 16 à 25 ans ».
const AGE_COLLE: [&str; 10] =
    ["enfant", "enfants", "bebe", "bebes", "nourrisson", "nourrissons", "age", "agee", "ages", "agees"];

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

/// Les unités d'une durée.
const UNITES: [&str; 5] = ["an", "ans", "annee", "annees", "mois"];

/// Les mots d'un avantage : dans leur morceau de phrase, une durée n'est pas une expérience demandée (« rémunération
/// sur 13 mois », « prime après 6 mois », « mutuelle au bout de 3 mois »).
const AVANTAGES: [&str; 28] = [
    "salaire",
    "salaires",
    "remuneration",
    "remunerations",
    "prime",
    "primes",
    "mutuelle",
    "interessement",
    "participation",
    "paie",
    "paye",
    "brut",
    "bruts",
    "brute",
    "net",
    "nets",
    "euro",
    "euros",
    "smic",
    "treizieme",
    "13e",
    "13eme",
    "conges",
    "rtt",
    "ticket",
    "tickets",
    "avantage",
    "avantages",
];

/// « Salaire selon expérience », « en fonction de votre expérience ».
const SELON: [&str; 3] = ["selon", "fonction", "suivant"];

/// Les mots qui disent qu'une durée n'est que souhaitée (« 2 ans d'expérience souhaités », « idéalement 2 ans »).
const SOUHAITS_DUREE: [&str; 19] = [
    "souhaite",
    "souhaitee",
    "souhaites",
    "souhaitees",
    "souhaitable",
    "apprecie",
    "appreciee",
    "apprecies",
    "appreciees",
    "appreciable",
    "idealement",
    "ideal",
    "ideale",
    "atout",
    "bienvenu",
    "bienvenue",
    "preference",
    "serait",
    "seraient",
];

/// Ceux qui parlent de l'entreprise (« notre agence », « le groupe »), et ceux qui parlent du candidat.
const ENTREPRISE: [&str; 8] = ["notre", "nos", "nous", "groupe", "societe", "entreprise", "enseigne", "cabinet"];
const CANDIDAT: [&str; 9] =
    ["vous", "votre", "vos", "candidat", "candidate", "candidats", "profil", "collaborateur", "collaboratrice"];
/// « Notre agence compte 15 ans d'expérience », « nous avons 20 ans d'expérience ».
const POSSEDE: [&str; 24] = [
    "a",
    "ont",
    "avons",
    "compte",
    "comptons",
    "comptent",
    "cumule",
    "cumulons",
    "cumulent",
    "possede",
    "possedons",
    "possedent",
    "totalise",
    "totalisons",
    "totalisent",
    "affiche",
    "beneficie",
    "beneficions",
    "capitalise",
    "capitalisons",
    "dispose",
    "disposons",
    "fete",
    "fetons",
];
/// « Nous recherchons un vendeur qui a 2 ans d'expérience » : là, c'est le candidat.
const RECHERCHE: [&str; 12] = [
    "recherche",
    "recherchons",
    "recherchent",
    "recrute",
    "recrutons",
    "recrutent",
    "cherche",
    "cherchons",
    "souhaitons",
    "attendons",
    "poste",
    "pour",
];

/// Le nombre de place `i` ouvre une fourchette (« 2 à 3 ans », « 2 ou 3 ans », « 2-3 ans », « 2/3 ans », « entre 2 et
/// 5 ans ») : la place du second nombre. Entre les deux, tel qu'écrit, rien d'autre : « Bac+2, 5 ans » et « Bac+2 et
/// 3 ans » ne sont pas des fourchettes.
fn fourchette(p: &Phrase, i: usize) -> Option<usize> {
    nombre(p.mot(i))?;
    let m = p.mot(i + 1);
    let lie = ["a", "ou"].contains(&m) || (m == "et" && i > 0 && p.mot(i - 1) == "entre");
    if lie && nombre(p.mot(i + 2)).is_some() && p.colle(i) && p.colle(i + 1) {
        return Some(i + 2);
    }
    let signe = matches!(p.joint(i).trim(), "-" | "/" | "\u{2013}" | "\u{2014}");
    (signe && nombre(m).is_some() && !["un", "une"].contains(&p.mot(i))).then_some(i + 1)
}

/// La durée qui commence au mot `tete` et finit au mot `k` est l'expérience de l'entreprise, pas celle demandée :
/// « Forte de 15 ans d'expérience, notre agence recrute », « 20 ans d'existence », « le groupe compte 30 ans
/// d'expérience ».
fn experience_de_l_entreprise(p: &Phrase, tete: usize, k: usize) -> bool {
    if p.mot(k + 1) == "existence" || (["d", "de"].contains(&p.mot(k + 1)) && p.mot(k + 2) == "existence") {
        return true;
    }
    let dernier = |mots: &[&str]| (0..tete).rev().find(|&x| mots.contains(&p.mot(x)));
    let (entreprise, candidat) = (dernier(&ENTREPRISE), dernier(&CANDIDAT));
    // « Fort(e) de 15 ans d'expérience », « riche d'une expérience de 15 ans » : celui dont on parle ensuite
    let fort_de = (tete.saturating_sub(6)..tete).any(|x| {
        ["fort", "forte", "forts", "fortes", "riche", "riches"].contains(&p.mot(x))
            && ["de", "d"].contains(&p.mot(x + 1))
    });
    if fort_de {
        if entreprise.is_some() && candidat.is_none() && !p.entre(0, tete as isize - 1, &RECHERCHE) {
            return true;
        }
        for x in k + 1..p.mots.len() {
            if CANDIDAT.contains(&p.mot(x)) || EXIGENCES.contains(&p.mot(x)) {
                return false;
            }
            if ENTREPRISE.contains(&p.mot(x)) {
                return true;
            }
        }
        return candidat.is_none();
    }
    // « Notre agence compte plus de 15 ans d'expérience » : l'entreprise, puis un verbe juste avant la durée
    let Some(e) = entreprise else { return false };
    if p.entre(e as isize, tete as isize - 1, &CANDIDAT) || p.entre(e as isize, tete as isize - 1, &RECHERCHE) {
        return false;
    }
    let mut x = tete;
    while x > e + 1 && ["plus", "de", "d", "pres", "deja", "environ", "presque"].contains(&p.mot(x - 1)) {
        x -= 1;
    }
    // (« a », pas « à »)
    x > e && POSSEDE.contains(&p.mot(x - 1)) && p.ecrit(x - 1).is_ascii()
}

/// Une durée d'expérience : en mois, ses premier et dernier mots, et si elle n'est que souhaitée.
type Duree = (u16, usize, usize, bool);

/// Les durées d'une phrase : celles d'expérience (la durée en mois, ses mots, et si elle n'est que souhaitée, « une
/// expérience de 2 ans serait un plus »), et les mots écartés parce qu'ils parlent de l'entreprise ou d'un avantage
/// (« Forte de 15 ans d'expérience, notre agence recrute » : ce mot « expérience » n'en demande pas une).
fn durees(p: &Phrase) -> (Vec<Duree>, Vec<(usize, usize)>) {
    let mut l = Vec::new();
    let mut ecartes = Vec::new();
    let n = p.mots.len();
    for i in 0..n {
        let Some(n1) = nombre(p.mot(i)) else { continue };
        // (le chiffre d'un « Bac+2 » ou d'un « Bac+2/3 » : un niveau, pas une durée ; « Bac+5 - 3 ans » : une durée)
        if (i >= 1 && chiffre_du_bac(p, i - 1).is_some())
            || (i >= 2 && chiffre_du_bac(p, i - 2).is_some() && matches!(p.joint(i - 1), "/" | "-"))
        {
            continue;
        }
        // « 2 à 3 ans », « 2 ou 3 ans », « 2-3 ans »
        let k = fourchette(p, i).map_or(i + 1, |f| f + 1);
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
        // (un âge, pas une durée : « enfants de 3 à 6 ans », « enfants (0-3 ans) », « jeunes de moins de 26 ans »,
        // « âgé de 16 à 25 ans », « 18 ans révolus », « 18 ans minimum » ; le second nombre d'une tranche, « 0 à
        // 3 ans », se juge d'après ce qui précède le premier)
        let tete = if i >= 2 && fourchette(p, i - 2) == Some(i) {
            i - 2
        } else if i >= 1 && fourchette(p, i - 1) == Some(i) {
            i - 1
        } else {
            i
        };
        let devant = |x: usize| if tete >= x { p.mot(tete - x) } else { "" };
        let de_qui = |x: usize| AGE_DE.contains(&devant(x));
        let age = AGE_COLLE.contains(&devant(1))
            || (["de", "d", "des", "entre"].contains(&devant(1)) && de_qui(2))
            || (devant(1) == "de" && ["moins", "plus"].contains(&devant(2)) && devant(3) == "de" && de_qui(4))
            || ["revolu", "revolus"].contains(&p.mot(k + 1))
            || (n1 >= 16 && mois >= 192);
        // (sauf quand le mot « expérience » est collé à la durée : « 20 ans d'expérience », « expérience de 20 ans »)
        let suite = |x: usize| {
            let colle = |c: char| c.is_alphanumeric() || matches!(c, ' ' | '\'' | '\u{2019}');
            MOTS_EXPERIENCE.contains(&p.mot(x)) && p.brut[p.places[k].1..p.places[x].0].chars().all(colle)
        };
        let collee = suite(k + 1)
            || (["d", "de"].contains(&p.mot(k + 1)) && suite(k + 2))
            || p.entre(tete as isize - 2, tete as isize - 1, &MOTS_EXPERIENCE);
        let (ma, mb) = (p.morceau(tete).0 as isize, p.morceau(k).1 as isize);
        // (les mots écartés avec la durée : jusqu'à 3 mots devant et 6 derrière, sans sortir de son morceau de phrase)
        let ecart = (tete.saturating_sub(3).max(ma as usize), (k + 6).min(mb as usize));
        // (« 25 ans d'existence et d'expérience » : l'âge de l'entreprise)
        let existence =
            p.mot(k + 1) == "existence" || (["d", "de"].contains(&p.mot(k + 1)) && p.mot(k + 2) == "existence");
        if existence {
            ecartes.push(ecart);
            continue;
        }
        if age && !collee {
            continue;
        }
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
        // (un avantage, pas une exigence : « rémunération sur 13 mois », « prime après 6 mois d'ancienneté », « au bout
        // d'un an », ou une durée dans le morceau de phrase d'un salaire, d'une prime, d'une mutuelle)
        let de = ["de", "d"].contains(&devant(1));
        if ["sur", "apres"].contains(&devant(1)) || (de && ["bout", "issue"].contains(&devant(2))) {
            ecartes.push(ecart);
            continue;
        }
        if p.entre(ma, mb, &AVANTAGES) && (!collee || p.entre(ma, mb, &SELON)) {
            ecartes.push(ecart);
            continue;
        }
        // (l'expérience de l'entreprise : « Forte de 15 ans d'expérience, notre agence recrute »)
        if experience_de_l_entreprise(p, tete, k) {
            ecartes.push(ecart);
            continue;
        }
        // (sans le mot « expérience » : seulement « 2 ans minimum », « au moins 6 mois », dans une phrase sans contrat ;
        // « ancienneté » ne vaut « expérience » que dans une phrase qui exige, sans avantage)
        let minimum = ["mini", "minimum", "min"].contains(&p.mot(k + 1))
            || (avant(1) == "moins" && avant(2) == "au")
            || ["minimum", "mini"].contains(&avant(1));
        let sans_contrat = !p.a(&PAS_UNE_EXPERIENCE) && mois <= 180;
        let proche = |mots: &[&str]| p.entre(i as isize - 8, k as isize + 5, mots);
        // (« salaire selon expérience » ne parle pas d'une expérience demandée)
        let experience_proche = (i.saturating_sub(8)..(k + 6).min(n))
            .any(|x| MOTS_EXPERIENCE.contains(&p.mot(x)) && !p.entre(x as isize - 3, x as isize - 1, &SELON));
        let anciennete = proche(&["anciennete"]) && p.a(&EXIGENCES) && !p.a(&AVANTAGES);
        if !(experience_proche || anciennete) && !(minimum && sans_contrat) {
            continue;
        }
        // (seulement souhaitée : un mot de souhait dans le même morceau de phrase, sans mot d'exigence)
        let un_plus = (ma..mb).any(|x| {
            let (a, b) = (p.mot(x as usize), p.mot(x as usize + 1));
            (a == "un" && b == "plus") || (a == "si" && b == "possible")
        });
        let souhaitee = (p.entre(ma, mb, &SOUHAITS_DUREE) || un_plus) && !p.entre(ma, mb, &EXIGENCES[..12]);
        l.push((mois, i, k, souhaitee));
    }
    (l, ecartes)
}

/// Un « débutant accepté » dit en clair : sa place, sa longueur en mots, et s'il est nié (« débutant accepté : non »,
/// « poste non ouvert aux débutants », « pas accessible aux débutants », « sans expérience significative s'abstenir »).
fn debutant_dit(p: &Phrase) -> Option<(usize, usize, bool)> {
    let (i, n) = p.premier(DEBUTANT)?;
    // (« non », « pas » juste devant, sans signe entre les deux ; pas « Diplôme : non Débutant accepté »)
    let nie_avant = i > 0 && {
        let nouvelle = p.ecrit(i).starts_with(char::is_uppercase) && p.ecrit(i - 1).chars().any(char::is_lowercase);
        ["non", "pas", "ne", "jamais"].contains(&p.mot(i - 1)) && p.colle(i - 1) && !nouvelle
    };
    // (après le motif et ses « (e)(s) » : « non », ou « s'abstenir » un peu plus loin)
    let mut f = i + n;
    while ["e", "s", "es"].contains(&p.mot(f)) && p.mot(f + 1) != "abstenir" {
        f += 1;
    }
    let signes = p.joint(f - 1).chars().all(|c| c.is_whitespace() || matches!(c, ':' | '(' | ')' | '=' | '?'));
    let autre = ["fumeur", "fumeuse", "fumeurs", "exhaustif", "exhaustive", "contractuel", "negociable", "seulement"];
    let non = p.mot(f) == "non" && signes && !autre.contains(&p.mot(f + 1));
    let abstenir = p.entre(f as isize, f as isize + 5, &["abstenir", "abstenant"]);
    Some((i, n, nie_avant || non || abstenir))
}

/// Un profil « confirmé » (« Cariste confirmé », « profil confirmé(e) ») : la place du mot. Il doit être écrit avec
/// son accent (ou tout en capitales), et ne pas être le verbe ni parler d'autre chose (« le groupe confirme sa
/// croissance », « votre rendez-vous sera confirmé par téléphone », « CDI confirmé »).
fn profil_confirme(p: &Phrase) -> Option<usize> {
    (0..p.mots.len()).find(|&i| {
        if !["confirme", "confirmee", "confirmes", "confirmees"].contains(&p.mot(i)) {
            return false;
        }
        let ecrit = p.ecrit(i);
        if !ecrit.contains(['é', 'É']) && ecrit.chars().any(char::is_lowercase) {
            return false;
        }
        let avant = |x: usize| if i >= x { p.mot(i - x) } else { "" };
        let verbe = [
            "est", "sera", "ete", "sont", "seront", "soit", "etre", "ont", "avons", "avez", "ai", "fut", "serait",
            "seraient", "ayant", "se", "nous", "je", "il", "elle", "on", "qui", "ne", "n", "non", "pas", "vous",
            "bien", "deja",
        ];
        let a = avant(1) == "a" && i >= 1 && p.ecrit(i - 1).is_ascii();
        let chose = [
            "cdi",
            "cdd",
            "contrat",
            "poste",
            "embauche",
            "rdv",
            "commande",
            "commandes",
            "date",
            "dates",
            "candidature",
            "inscription",
            "reservation",
            "croissance",
            "succes",
            "engagement",
            "interet",
            "cas",
            "diagnostic",
            "depart",
            "entretien",
            "essai",
            "mission",
            "groupe",
            "societe",
            "entreprise",
            "direction",
        ];
        let suite = [
            "par", "que", "qu", "sa", "son", "ses", "la", "le", "les", "l", "un", "une", "notre", "nos", "votre",
            "vos", "leur",
        ];
        !(verbe.contains(&avant(1)) || a || chose.contains(&avant(1)) || suite.contains(&p.mot(i + 1)))
    })
}

/// Le mot « expérience » d'une phrase dit qu'une expérience est voulue (« une expérience dans la vente »), et pas
/// « salaire selon expérience » ni « expérience client », ni un mot `ecarte` (l'expérience de l'entreprise).
fn experience_voulue(p: &Phrase, ecarte: impl Fn(usize) -> bool) -> Option<usize> {
    for i in 0..p.mots.len() {
        if !["experience", "experiences"].contains(&p.mot(i)) || ecarte(i) {
            continue;
        }
        let avant = |x: usize| if i >= x { p.mot(i - x) } else { "" };
        if ["selon", "fonction", "votre"].contains(&avant(1))
            || ["selon", "fonction"].contains(&avant(2))
            || ["selon", "fonction"].contains(&avant(3))
        {
            continue;
        }
        // (l'expérience que l'entreprise offre : « Nous offrons une expérience », « vivez une expérience »)
        if p.entre(i as isize - 3, i as isize - 1, &["offrons", "offre", "proposons", "propose", "vivez", "vivre"]) {
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
                "aupres",
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

/// L'expérience demandée, d'après toutes les phrases. Débutant accepté en clair l'emporte, puis la plus petite durée
/// exigée, puis une expérience exigée, puis appréciée, puis une tournure comme « formation assurée ».
fn experience(ps: &[Phrase]) -> Option<Trouve<Experience>> {
    let mut debutant: Option<Trouve<Experience>> = None;
    let mut pas_debutant: Option<Trouve<Experience>> = None;
    let mut plus_courte: Option<Trouve<Experience>> = None;
    let mut demandee: Option<Trouve<Experience>> = None;
    let mut souhaitee: Option<Trouve<Experience>> = None;
    let mut debutant_moyen: Option<Trouve<Experience>> = None;
    let t = |v, p: &Phrase, i, j, c| Trouve { valeur: v, preuve: p.preuve(i, j), confiance: c };
    for p in ps {
        // (un « débutant accepté » nié, « débutant accepté : non », demande une expérience ; la durée dite dans la même
        // phrase, « débutants s'abstenir, 3 ans minimum », est lue quand même)
        if let Some((i, n)) = p.premier(PAS_DE_DEBUTANT) {
            pas_debutant.get_or_insert(t(Experience::Demandee, p, i, i + n - 1, Confiance::Forte));
        } else if let Some((i, n, nie)) = debutant_dit(p) {
            if nie {
                pas_debutant.get_or_insert(t(Experience::Demandee, p, i, i + n - 1, Confiance::Forte));
            } else {
                debutant.get_or_insert(t(Experience::Debutant, p, i, i + n - 1, Confiance::Forte));
            }
        } else if let Some((i, n)) = rubrique_non(p, "experience") {
            debutant.get_or_insert(t(Experience::Debutant, p, i, i + n - 1, Confiance::Forte));
        }
        // (une durée seulement souhaitée, « une expérience de 2 ans serait un plus », n'est pas exigée)
        let (ds, ecartes) = durees(p);
        let ecarte = |x: usize| ecartes.iter().any(|&(a, b)| (a..=b).contains(&x));
        for &(m, i, j, souhait) in &ds {
            if souhait {
                souhaitee.get_or_insert(t(Experience::Souhaitee, p, i, j, Confiance::Forte));
            } else if plus_courte.as_ref().is_none_or(|x| matches!(x.valeur, Experience::Duree(y) if m < y)) {
                plus_courte = Some(t(Experience::Duree(m), p, i, j, Confiance::Forte));
            }
        }
        // (une phrase dont toutes les durées sont seulement souhaitées, « 6 mois d'expérience souhaités », ne demande
        // pas non plus d'expérience par ses autres mots)
        // (sauf si elle exige ailleurs, « Expérience exigée, 5 ans souhaités » ; pas « souhaitée mais pas obligatoire »)
        let exige = (0..p.mots.len()).any(|x| {
            let nie = |d: usize| x >= d && ["non", "pas"].contains(&p.mot(x - d));
            EXIGENCES[..12].contains(&p.mot(x)) && !nie(1) && !nie(2)
        });
        let que_souhaits = !ds.is_empty() && ds.iter().all(|d| d.3) && !exige;
        let doux = p.a(&SOUHAITS) || p.trouver("un plus").is_some();
        let demande = p.premier(DEMANDEE).filter(|&(i, _)| !ecarte(i));
        if let Some((i, n)) = p.premier(SOUHAITEE) {
            souhaitee.get_or_insert(t(Experience::Souhaitee, p, i, i + n - 1, Confiance::Forte));
        } else if !que_souhaits && let Some((i, n)) = demande.or_else(|| profil_confirme(p).map(|i| (i, 1))) {
            let fin = (i + n + 2).min(p.mots.len());
            let fort = ["exigee", "obligatoire", "requise", "indispensable", "imperative", "necessaire", "oui"]
                .iter()
                .any(|x| p.mots[i..fin].iter().any(|m| m == x));
            if doux && !fort {
                souhaitee.get_or_insert(t(Experience::Souhaitee, p, i, i + n - 1, Confiance::Moyenne));
            } else {
                demandee.get_or_insert(t(Experience::Demandee, p, i, i + n - 1, Confiance::Forte));
            }
        } else if !que_souhaits && let Some(i) = experience_voulue(p, ecarte) {
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

/// Les catégories de permis reconnues après le mot « permis ».
const LETTRES_PERMIS: [&str; 16] =
    ["b", "c", "ce", "c1", "c1e", "d", "de", "d1", "d1e", "be", "a", "a1", "a2", "am", "ec", "eb"];
/// Celles qui sont aussi un mot courant (« à », « ce », « de ») : elles ne comptent que suivies d'un signe ou de la
/// fin de la phrase (« permis B et CE, exigés »), ou écrites en majuscules dans un texte qui n'est pas tout en
/// majuscules (« Permis CE exigé »).
const LETTRES_AMBIGUES: [&str; 3] = ["a", "ce", "de"];

/// Le mot `k` est la lettre d'un permis : une lettre isolée, telle qu'écrite dans le texte (pas « c'est », « d'un »,
/// ni « à »), après le mot « permis » de place `i`.
fn lettre_permis(p: &Phrase, i: usize, k: usize) -> bool {
    let m = p.mot(k);
    if !LETTRES_PERMIS.contains(&m) {
        return false;
    }
    let brut = |x: usize| p.places.get(x).map_or("", |(d, f)| &p.brut[*d..*f]);
    let ecrit = brut(k);
    // (« à » : pas une lettre ; « c'est », « d'un » : le début d'un autre mot)
    let apostrophe = p.places.get(k).is_some_and(|(_, f)| p.brut[*f..].starts_with(['\'', '\u{2019}', '\u{02bc}']));
    if !ecrit.is_ascii()
        || apostrophe
        || (ecrit == "c" && p.mot(k + 1) == "est")
        || (m == "a" && p.mot(k + 1) == "jour")
    {
        return false;
    }
    if LETTRES_AMBIGUES.contains(&m) {
        // (« permis de chasse », « Permis de\nconduire » : jamais le permis DE)
        if m == "de" && k == i + 1 {
            return false;
        }
        let majuscules = |x: &str| x.chars().any(char::is_alphabetic) && !x.chars().any(char::is_lowercase);
        let suite = p.places.get(k).map_or("", |(_, f)| p.brut[*f..].trim_start());
        let signe = !suite.starts_with(char::is_alphanumeric);
        return signe || (majuscules(ecrit) && !majuscules(brut(i)));
    }
    true
}

/// Les lettres du permis qui commencent au mot `k`, après le mot « permis » de place `i` : « B », ou une liste
/// (« B, C et CE », « B/C », « B + C », « B ou C1 »). Entre deux lettres, seulement une virgule, une barre, un plus,
/// un tiret, une esperluette, « et » ou « ou ».
fn lettres_permis<'a>(p: &'a Phrase, i: usize, k: usize) -> Vec<&'a str> {
    let mut l = Vec::new();
    if !lettre_permis(p, i, k) {
        return l;
    }
    l.push(p.mot(k));
    let colles = |x: usize| {
        let (Some(a), Some(b)) = (p.places.get(x), p.places.get(x + 1)) else { return false };
        p.brut[a.1..b.0].chars().all(|c| c.is_whitespace() || matches!(c, ',' | '/' | '+' | '-' | '&'))
    };
    let mut j = k;
    while l.len() < 6 && colles(j) {
        let mut n = j + 1;
        if ["et", "ou"].contains(&p.mot(n)) && colles(n) {
            n += 1;
        }
        if !lettre_permis(p, i, n) {
            break;
        }
        if !l.contains(&p.mot(n)) {
            l.push(p.mot(n));
        }
        j = n;
    }
    l
}

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
                    let lettres = lettres_permis(p, i, k);
                    if !lettres.is_empty() {
                        for x in lettres {
                            ajouter(format!("Permis {}", x.to_uppercase()));
                        }
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

// ---------------------------------------------------------------------------------------------------------------
// La rubrique « Profil souhaité » des offres France Travail
// ---------------------------------------------------------------------------------------------------------------
//
// Sous le texte libre, une offre France Travail donne son profil en rubriques, une ligne par élément :
//
//     Profil souhaité
//     Expérience
//     2 An(s)
//     Exigé
//     Formation
//     CAP, BEP et équivalents Cuisine Exigé
//     Permis
//     B - Véhicule léger Souhaité
//
// Le titre de la rubrique est seul sur sa ligne (ou suivi de « : » et de la valeur), la valeur sur la ligne suivante,
// « Exigé » ou « Souhaité » au bout de la valeur ou seul sur une des deux lignes d'après. C'est la référence : ce
// qu'elle dit passe avant le texte libre. Seules les formes connues de France Travail comptent (« Débutant accepté »,
// « 2 An(s) », « Bac+2 ou équivalents »...) : une ligne « Formation » d'une annonce ordinaire suivie d'une phrase
// (« Formation assurée en interne », « Vous serez formé ») ne donne rien ici, les phrases sont lues comme avant.
//
// La preuve est le bout du texte qui va du début de la valeur à la fin de « Exigé » ou « Souhaité » (tel quel, retours
// à la ligne compris : un bout contigu du texte), et commence au titre de la rubrique si la valeur seule est trop courte
// pour prouver quoi que ce soit (« Formation BTS »).

/// Les rubriques du profil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rubrique {
    Experience,
    Formation,
    Permis,
    /// Une rubrique qui ne dit ni diplôme ni expérience (« Savoir-être professionnels », « Qualification »...)
    Autre,
}

/// Les titres des rubriques, en mots remis au propre.
const RUBRIQUES: &[(&str, Rubrique)] = &[
    ("experience", Rubrique::Experience),
    ("experiences", Rubrique::Experience),
    ("formation", Rubrique::Formation),
    ("formations", Rubrique::Formation),
    ("permis", Rubrique::Permis),
    ("permis de conduire", Rubrique::Permis),
    ("profil souhaite", Rubrique::Autre),
    ("profil recherche", Rubrique::Autre),
    ("savoir etre professionnels", Rubrique::Autre),
    ("savoir etre professionnel", Rubrique::Autre),
    ("savoir etre", Rubrique::Autre),
    ("savoir faire", Rubrique::Autre),
    ("competences", Rubrique::Autre),
    ("competence", Rubrique::Autre),
    ("qualification", Rubrique::Autre),
    ("qualifications", Rubrique::Autre),
    ("langue", Rubrique::Autre),
    ("langues", Rubrique::Autre),
    ("informations complementaires", Rubrique::Autre),
    ("salaire", Rubrique::Autre),
    ("deplacements", Rubrique::Autre),
    ("secteur d activite", Rubrique::Autre),
    ("duree du travail", Rubrique::Autre),
    ("type de contrat", Rubrique::Autre),
];

/// « Exigé » ou « Souhaité ».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Marque {
    Exige,
    Souhaite,
}

fn marque(m: &str) -> Option<Marque> {
    match m {
        "exige" | "exigee" | "exiges" | "exigees" | "indispensable" | "obligatoire" | "requis" | "requise" => {
            Some(Marque::Exige)
        }
        "souhaite" | "souhaitee" | "souhaites" | "souhaitees" | "souhaitable" | "apprecie" | "appreciee" => {
            Some(Marque::Souhaite)
        }
        _ => None,
    }
}

/// Une ligne du texte, sans la puce du début ni les espaces autour : sa place dans le texte et ses mots.
struct Ligne<'a> {
    debut: usize,
    fin: usize,
    p: Phrase<'a>,
    /// Le titre d'une rubrique
    rubrique: Option<Rubrique>,
}

/// La ligne `t[debut..fin]`, resserrée (sans puce ni espaces autour) ; rien si elle n'a pas de mot.
fn ligne_du_texte(t: &str, debut: usize, fin: usize) -> Option<Ligne<'_>> {
    let x = &t[debut..fin];
    let a = debut + x.find(char::is_alphanumeric)?;
    let b = debut + x.trim_end().len();
    let p = decouper(&t[a..b]);
    (!p.mots.is_empty()).then_some(Ligne { debut: a, fin: b, p, rubrique: None })
}

/// La rubrique dont ces mots sont le titre.
fn titre_rubrique(mots: &[String]) -> Option<Rubrique> {
    let m = mots.join(" ");
    RUBRIQUES.iter().find(|(r, _)| *r == m).map(|x| x.1)
}

/// Les lignes du texte ; un titre suivi de « : » (« Formation : Bac+2 Exigé ») fait deux lignes, le titre et la valeur.
fn lignes(t: &str) -> Vec<Ligne<'_>> {
    let mut l = Vec::new();
    let mut debut = 0;
    for x in t.split_inclusive('\n') {
        let (d, f) = (debut, debut + x.trim_end_matches(['\n', '\r']).len());
        debut += x.len();
        let Some(mut ligne) = ligne_du_texte(t, d, f) else { continue };
        ligne.rubrique = titre_rubrique(&ligne.p.mots);
        // (« Qualification : Employé qualifié »)
        if ligne.rubrique.is_none()
            && let Some(k) = t[ligne.debut..ligne.fin].find(':')
            && let Some(r) = titre_rubrique(&decouper(&t[ligne.debut..ligne.debut + k]).mots)
            && let Some(valeur) = ligne_du_texte(t, ligne.debut + k + 1, ligne.fin)
        {
            let titre = &t[ligne.debut..ligne.debut + k];
            let fin = ligne.debut + titre.trim_end().len();
            l.push(Ligne { debut: ligne.debut, fin, p: decouper(titre), rubrique: Some(r) });
            l.push(valeur);
            continue;
        }
        l.push(ligne);
    }
    l
}

/// Une ligne qui n'est que « Exigé » ou « Souhaité » (ou « Cette expérience est indispensable »).
fn ligne_marque(l: &Ligne) -> Option<Marque> {
    let m = &l.p.mots;
    match m.len() {
        1 => marque(&m[0]),
        4 if m[0] == "cette" && m[2] == "est" => marque(&m[3]),
        _ => None,
    }
}

/// Une durée à la façon de France Travail : « 2 An(s) », « 6 Mois », « 3 ans minimum » (rien d'autre sur la ligne).
fn duree_ft(m: &[&str]) -> Option<u16> {
    let n = nombre(m.first()?)?;
    let mois = match *m.get(1)? {
        "an" | "ans" | "annee" | "annees" => n.checked_mul(12)?,
        "mois" => n,
        _ => return None,
    };
    let reste_permis = m[2..].iter().all(|x| ["s", "minimum", "mini", "min"].contains(x));
    (reste_permis && (1..=600).contains(&mois)).then_some(mois)
}

/// Ce que dit une ligne d'expérience (exigée ou souhaitée : sa marque).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValeurExperience {
    Debutant,
    Mois(u16),
    /// Une expérience, sans durée (« Expérience exigée »)
    Demandee,
}

/// La forme courte « Expérience exigée de 2 An(s) », « Expérience souhaitée de 6 Mois », seule sur sa ligne : la durée
/// et la marque.
fn experience_courte(m: &[&str]) -> Option<(u16, Marque)> {
    if m.first() != Some(&"experience") || m.get(2) != Some(&"de") {
        return None;
    }
    let mq = marque(m.get(1)?).or_else(|| (m[1] == "demandee").then_some(Marque::Exige))?;
    Some((duree_ft(&m[3..])?, mq))
}

/// Une ligne de la rubrique « Expérience » (sans sa marque au bout).
fn valeur_experience(m: &[&str]) -> Option<ValeurExperience> {
    let debutant = ["debutant", "debutants", "debutante", "debutantes"].contains(m.first()?)
        && m.len() <= 5
        && m[1..].iter().any(|x| x.starts_with("accepte") || x.starts_with("bienvenu"));
    if debutant {
        return Some(ValeurExperience::Debutant);
    }
    if let Some(mois) = duree_ft(m) {
        return Some(ValeurExperience::Mois(mois));
    }
    // (« Expérience exigée », « Expérience souhaitée » : la marque est lue à part)
    match m {
        ["experience"] | ["experience", "demandee"] => Some(ValeurExperience::Demandee),
        _ => None,
    }
}

/// Une ligne de la rubrique « Formation » (sans sa marque au bout) : le diplôme et la confiance.
fn valeur_formation(l: &Ligne, n: usize) -> Option<(Diplome, Confiance)> {
    let m: Vec<&str> = l.p.mots[..n].iter().map(String::as_str).collect();
    let debut = |x: &[&str]| m.len() >= x.len() && m[..x.len()] == *x;
    if debut(&["aucune", "formation"]) || debut(&["sans", "diplome"]) || debut(&["aucun", "diplome"]) {
        return Some((Diplome::Aucun, Confiance::Forte));
    }
    // (« Primaire à 4e », « 3ème achevée ou Brevet » : aucun diplôme de la liste, déduit)
    if ["primaire", "4eme", "3eme", "2nd", "2nde", "seconde", "1ere"].contains(m.first()?) {
        return Some((Diplome::Aucun, Confiance::Moyenne));
    }
    let p = Phrase { brut: l.p.brut, mots: l.p.mots[..n].to_vec(), places: l.p.places[..n].to_vec() };
    let niv = niveaux_avec(&p, true).into_iter().find(|x| !x.nie)?;
    let au_debut = niv.i == 0 || (niv.i == 1 && ["niveau", "diplome", "titre"].contains(&m[0]));
    au_debut.then_some((niv.diplome, if niv.sur { Confiance::Forte } else { Confiance::Moyenne }))
}

/// Une ligne de la rubrique « Permis » : « B - Véhicule léger » donne « Permis B ».
fn valeur_permis(m: &[&str]) -> Option<String> {
    let lettre = if m.first() == Some(&"permis") { m.get(1)? } else { m.first()? };
    ["b", "c", "ce", "c1", "c1e", "d", "de", "be", "a", "a1", "a2", "am", "ec"]
        .contains(lettre)
        .then(|| format!("Permis {}", lettre.to_uppercase()))
}

/// Ce que dit la rubrique « Profil souhaité ».
#[derive(Debug, Default)]
struct Profil {
    diplome: Option<Trouve<Diplome>>,
    experience: Option<Trouve<Experience>>,
    permis: Vec<String>,
}

/// Ce que dit une ligne de valeur.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Valeur {
    /// Une expérience, et la marque dite dans la ligne même (« Expérience souhaitée de 1 An(s) »)
    Experience(ValeurExperience, Option<Marque>),
    Formation(Diplome, Confiance),
    Permis(String),
}

/// La valeur d'une ligne dans cette rubrique, d'après ses `n` premiers mots (sans la marque du bout).
fn valeur(r: Option<Rubrique>, l: &Ligne, n: usize) -> Option<Valeur> {
    let mots: Vec<&str> = l.p.mots[..n].iter().map(String::as_str).collect();
    // (la forme courte se dit d'elle-même, dans une rubrique ou non)
    if let Some((mois, m)) = experience_courte(&mots) {
        return Some(Valeur::Experience(ValeurExperience::Mois(mois), Some(m)));
    }
    match r? {
        Rubrique::Experience => valeur_experience(&mots).map(|v| Valeur::Experience(v, None)),
        Rubrique::Formation => valeur_formation(l, n).map(|(d, c)| Valeur::Formation(d, c)),
        Rubrique::Permis => valeur_permis(&mots).map(Valeur::Permis),
        Rubrique::Autre => None,
    }
}

/// Le nombre de mots d'une ligne sans la marque du bout, et cette marque (« CAP, BEP et équivalents Exigé »).
fn sans_marque(l: &Ligne) -> (usize, Option<Marque>) {
    let n = l.p.mots.len();
    match marque(l.p.mot(n - 1)) {
        Some(m) if n > 1 => (n - 1, Some(m)),
        _ => (n, None),
    }
}

/// Lit la rubrique « Profil souhaité » d'une offre France Travail (rien pour une autre offre).
fn profil(t: &str) -> Profil {
    let ls = lignes(t);
    // (la rubrique en cours, et le début de son titre)
    let mut courante: Option<(Rubrique, usize)> = None;
    // (les expériences et les formations lues : valeur, marque, preuve)
    let mut exps: Vec<(ValeurExperience, Option<Marque>, String)> = Vec::new();
    let mut forms: Vec<(Diplome, Confiance, Option<Marque>, String)> = Vec::new();
    let mut permis = Vec::new();
    let mut k = 0;
    while k < ls.len() {
        let l = &ls[k];
        if let Some(r) = l.rubrique {
            courante = Some((r, l.debut));
            k += 1;
            continue;
        }
        let r = courante.map(|x| x.0);
        let (n, mut mq) = sans_marque(l);
        let Some(v) = valeur(r, l, n) else {
            // (une ligne d'une autre forme : la rubrique est finie, le reste est du texte libre)
            if r.is_some_and(|r| r != Rubrique::Autre) {
                courante = None;
            }
            k += 1;
            continue;
        };
        // La marque au bout de la ligne, sinon seule sur l'une des deux lignes suivantes (avec, entre les deux, une
        // ligne qui n'est ni un titre ni une autre valeur : la spécialité)
        let (mut fin, mut suite) = (l.fin, k + 1);
        if mq.is_none() {
            for (j, x) in ls.iter().enumerate().skip(k + 1).take(2) {
                if let Some(m) = ligne_marque(x) {
                    (mq, fin, suite) = (Some(m), x.fin, j + 1);
                    break;
                }
                if x.rubrique.is_some() || valeur(r, x, sans_marque(x).0).is_some() {
                    break;
                }
            }
        }
        // La preuve, assez longue pour prouver (sinon depuis le titre de la rubrique)
        let p = &t[l.debut..fin];
        let preuve = match courante {
            Some((_, d)) if !citation_presente(t, p) => t[d..fin].to_string(),
            _ => p.to_string(),
        };
        match v {
            Valeur::Experience(x, m) => exps.push((x, m.or(mq), preuve)),
            Valeur::Formation(d, c) => forms.push((d, c, mq, preuve)),
            Valeur::Permis(x) => {
                if !permis.contains(&x) {
                    permis.push(x);
                }
            }
        }
        k = suite;
    }
    // L'expérience : débutant accepté, puis la plus petite durée exigée, puis une expérience exigée, puis souhaitée
    // (une durée seulement souhaitée, « 2 An(s) Souhaité », n'est pas exigée : souhaitée, la durée reste dans la preuve)
    let forte = |valeur, preuve: &String| Trouve { valeur, preuve: preuve.clone(), confiance: Confiance::Forte };
    let souhaitee = |m: Option<Marque>| m == Some(Marque::Souhaite);
    let experience = exps
        .iter()
        .find(|x| x.0 == ValeurExperience::Debutant)
        .map(|x| forte(Experience::Debutant, &x.2))
        .or_else(|| {
            exps.iter()
                .filter_map(|x| match x.0 {
                    ValeurExperience::Mois(m) if !souhaitee(x.1) => Some((m, &x.2)),
                    _ => None,
                })
                .min_by_key(|x| x.0)
                .map(|(m, p)| forte(Experience::Duree(m), p))
        })
        .or_else(|| {
            exps.iter()
                .find(|x| x.0 == ValeurExperience::Demandee && !souhaitee(x.1))
                .map(|x| forte(Experience::Demandee, &x.2))
        })
        .or_else(|| {
            exps.iter()
                .find(|x| x.0 != ValeurExperience::Debutant && souhaitee(x.1))
                .map(|x| forte(Experience::Souhaitee, &x.2))
        });
    // Le diplôme : le plus bas exigé, sinon le plus bas souhaité
    let plus_bas = |exige: bool| {
        forms.iter().filter(|x| souhaitee(x.2) != exige).min_by_key(|x| x.0).map(|x| Trouve {
            valeur: x.0,
            preuve: x.3.clone(),
            confiance: x.1,
        })
    };
    let diplome = plus_bas(true).or_else(|| plus_bas(false));
    Profil { diplome, experience, permis }
}

/// Ce que les règles lisent dans une offre : son titre et son texte. La rubrique « Profil souhaité » d'une offre
/// France Travail (`profil`) passe avant ce que disent les phrases.
pub fn lire(titre: &str, texte: &str) -> LectureOffre {
    let mut morceaux = phrases(titre);
    morceaux.extend(phrases(texte));
    let ps: Vec<Phrase> = morceaux.into_iter().map(decouper).filter(|p| !p.mots.is_empty()).collect();
    let fiche = profil(texte);
    let experience = fiche.experience.or_else(|| experience(&ps));
    let mut diplome = fiche.diplome.or_else(|| diplome(&ps));
    // « Débutants acceptés, formation assurée » : aucun diplôme demandé, déduit. Seulement quand « débutant accepté » est
    // dit en clair : « Infirmier H/F. Vous serez formé à nos protocoles. » ne dit rien du diplôme.
    let debutant_sur =
        experience.as_ref().is_some_and(|e| e.valeur == Experience::Debutant && e.confiance == Confiance::Forte);
    if diplome.is_none() && debutant_sur {
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
    // (le permis de la rubrique « Permis », sans doublon)
    let mut autres = autres(&ps);
    for x in fiche.permis {
        if !autres.contains(&x) {
            autres.push(x);
        }
    }
    LectureOffre { diplome, experience, autres }
}

/// Le plus haut diplôme cité dans un texte (un CV) : les niveaux trouvés, sauf ceux qui sont niés et ceux qui n'ont
/// pas été obtenus (« BTS MCO (non obtenu) », « échec au BTS »).
pub fn diplome_le_plus_haut(t: &str) -> Option<Diplome> {
    let mut haut = None;
    for p in phrases(t).into_iter().map(decouper) {
        let ns = niveaux(&p);
        for (x, n) in ns.iter().enumerate() {
            // (ce qui suit ce diplôme, jusqu'au diplôme suivant)
            let fin = ns.get(x + 1).map_or(p.mots.len(), |s| s.i).min(n.j + 7);
            let rate = ["echec", "echoue", "echouee", "abandon", "abandonne", "abandonnee", "interrompu", "inacheve"];
            let pas_obtenu = (n.j + 1..fin).any(|k| {
                rate.contains(&p.mot(k))
                    || (["non", "pas"].contains(&p.mot(k))
                        && ["obtenu", "obtenue", "valide", "validee", "acquis", "eu"].contains(&p.mot(k + 1)))
            }) || p.entre(n.i as isize - 3, n.i as isize - 1, &["echec", "abandon"]);
            if !n.nie && !pas_obtenu {
                haut = haut.max(Some(n.diplome));
            }
        }
    }
    haut
}

/// 0.2.0 : les permis et les CACES cités dans un texte (« Permis B », « CACES 1, 3, 5 »), par la même règle que pour
/// une offre : sert à lire ceux d'un CV (crate::parcours).
pub fn permis_et_caces(t: &str) -> Vec<String> {
    let ps: Vec<Phrase> = phrases(t).into_iter().map(decouper).filter(|p| !p.mots.is_empty()).collect();
    autres(&ps)
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
