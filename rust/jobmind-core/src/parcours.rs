//! Le parcours lu dans un CV (0.2.0, demande de Jordan du 25/11/2026 : « Est-ce qu'on peut pas faire une analyse du CV
//! d'abord ? Pour que ça corresponde à quelque chose. ») : [`lire`] tire du texte d'un CV une fiche de quatre listes
//! ([`Parcours`] : les métiers exercés et leur durée, les compétences, les diplômes, les permis et habilitations), et
//! [`comparer`] dit si une offre est proche de cette fiche ([`Proximite`]), avec sa raison en une phrase simple. Tout
//! est calculé ici, en Rust, jamais par l'IA : le même résultat à chaque fois, chez tout le monde.
//!
//! La fiche est montrée à la personne (Mon profil, « Ce que Jobmind a compris de ton CV »), qui la corrige : une
//! ligne retirée, une ligne ajoutée ([`Correction`], [`Parcours::corrige`]). La comparaison repose donc sur ce qu'elle
//! a pu vérifier.
//!
//! Comment le CV est lu, ligne par ligne (mieux vaut une fiche courte et juste qu'une fiche longue et fausse) :
//! - les rubriques (« Expériences », « Formation », « Compétences », « Permis », « Loisirs »...) sont reconnues à
//!   leur titre, seul sur sa ligne ou suivi de « : », dans n'importe quel ordre, avec une faute de frappe au plus ;
//! - un métier : un nom de poste connu (la liste des métiers courants, [`TETES_PRECISES`] et [`TETES_LARGES`]) en
//!   début de ligne ou de morceau de ligne, avec une date ou une durée sur la même ligne ou la ligne voisine ; dans
//!   la rubrique des expériences, un nom de poste seul sur sa ligne compte aussi. Jamais dans « Formation » ni dans
//!   « Loisirs ». Le nom s'arrête au nom de l'entreprise ou de la ville (« chez », « à Metz », un tiret, un mot inconnu
//!   qui commence par une majuscule) ;
//! - une durée : « 2019 - 2022 » (36 mois), « mars 2020 à juin 2020 » (4 mois, les deux bords comptés), « depuis
//!   2021 » ou « à aujourd'hui » (un poste en cours : son début est gardé, la durée se calcule le jour venu),
//!   « 3 ans », « 6 mois ». Une année seule ou une saison ne donnent pas de durée. Le même métier exercé deux fois :
//!   une ligne, durées ajoutées ;
//! - un diplôme : un mot de diplôme (CAP, BEP, Bac, BTS, licence, master, titre professionnel, diplôme d'État...) dans
//!   la rubrique « Formation », ou en début de morceau de ligne ailleurs ; « en cours » est noté, « non obtenu » est
//!   laissé ;
//! - un permis, un CACES (la règle des offres, crate::lecture), une habilitation (électrique, SST, FIMO, HACCP...),
//!   partout dans le CV ; jamais sur une ligne qui dit « en cours », « prévu » ou « pas de permis » ;
//! - une compétence : chaque élément de la rubrique « Compétences », et les courtes lignes à puce des expériences qui
//!   commencent par un nom d'action (« Conduite de chariot », « Mise en rayon ») ; les qualités personnelles
//!   (« sérieux », « ponctuel ») n'en sont pas.
//!
//! Comment une offre est comparée (la distance n'entre pas dans ce calcul, elle a son filtre et son étiquette) :
//! - le poste : le titre de l'offre et chaque métier de la fiche, remis au propre (sans « H/F », au masculin
//!   singulier). Le même métier : le même nom de métier précis (« vendeuse » et « vendeur en boulangerie »), deux
//!   noms de la même famille ([`GROUPES`] : « cariste » et « conducteur de chariot élévateur »), ou, pour un nom
//!   large (« agent », « chef », « technicien »...), le même nom et un mot en commun après lui (« chef de rayon » et
//!   « chef de projet » ne sont pas le même métier). Un métier voisin : le même domaine ([`DOMAINES`] : la logistique,
//!   la vente, le soin...) ;
//! - les compétences : celles de la fiche dont les mots sont dans une même phrase de l'offre ;
//! - les permis et habilitations demandés par l'offre : la personne les a, ou il lui en manque un ;
//! - le diplôme demandé : à sa portée (son plus haut diplôme l'atteint) ou non ;
//! - l'expérience demandée : comparée à la durée du métier exercé.

use crate::Jour;
use crate::candidature::une_ligne;
use crate::lecture::{self, Diplome, Experience, LectureOffre};
use crate::offre::normaliser;

/// La version des règles : elle entre dans ce qui dit si une fiche est à refaire.
pub const VERSION: u32 = 1;
/// Une ligne de la fiche (en caractères).
pub const LIGNE_MAX: usize = 60;
/// Au plus ce nombre de lignes ajoutées à la main dans une liste.
pub const AJOUTS_MAX: usize = 30;
const METIERS_MAX: usize = 12;
const COMPETENCES_MAX: usize = 24;

// ---------------------------------------------------------------------------------------------------------------
// La fiche
// ---------------------------------------------------------------------------------------------------------------

/// Les quatre listes de la fiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Liste {
    Metiers,
    Competences,
    Diplomes,
    Permis,
}

impl Liste {
    pub const TOUTES: [Liste; 4] = [Liste::Metiers, Liste::Competences, Liste::Diplomes, Liste::Permis];

    /// Le code de la base et de la page.
    pub fn code(self) -> &'static str {
        match self {
            Liste::Metiers => "metiers",
            Liste::Competences => "savoirs",
            Liste::Diplomes => "diplomes",
            Liste::Permis => "permis",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Liste> {
        Liste::TOUTES.into_iter().find(|l| l.code() == c)
    }

    pub fn nom(self) -> &'static str {
        match self {
            Liste::Metiers => "Métiers exercés",
            Liste::Competences => "Compétences",
            Liste::Diplomes => "Diplômes",
            Liste::Permis => "Permis et habilitations",
        }
    }
}

/// Un métier exercé.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metier {
    /// Le nom du poste, comme dans le CV (« Cariste », « Vendeuse en boulangerie »)
    pub nom: String,
    /// Les mois des périodes finies (rien : le CV ne dit pas de quoi les compter)
    pub mois: Option<u16>,
    /// Le début (année, mois) d'un poste encore en cours (« depuis 2021 », « mars 2020 à aujourd'hui »)
    pub depuis: Option<(i32, u32)>,
}

impl Metier {
    /// La durée connue à ce jour, en mois (rien : pas connue).
    pub fn duree(&self, aujourdhui: Jour) -> Option<u16> {
        let en_cours = self.depuis.map(|(a, m)| {
            let n = (i64::from(aujourdhui.annee) - i64::from(a)) * 12 + i64::from(aujourdhui.mois) - i64::from(m) + 1;
            n.clamp(1, 720) as u16
        });
        match (self.mois, en_cours) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0).saturating_add(b.unwrap_or(0))),
        }
    }

    /// « 3 ans », « depuis 5 ans », « 8 mois » ; vide si la durée n'est pas connue.
    pub fn duree_texte(&self, aujourdhui: Jour) -> String {
        match self.duree(aujourdhui) {
            None => String::new(),
            Some(m) if self.depuis.is_some() => format!("depuis {}", duree_en_mots(m)),
            Some(m) => duree_en_mots(m),
        }
    }
}

/// « 8 mois », « 1 an », « 1 an et demi », « 3 ans » (au-delà de deux ans, les années entières).
pub fn duree_en_mots(mois: u16) -> String {
    match mois {
        0..=11 => format!("{} mois", mois.max(1)),
        12..=17 => "1 an".to_string(),
        18..=23 => "1 an et demi".to_string(),
        m => format!("{} ans", m / 12),
    }
}

/// La fiche tirée d'un CV.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Parcours {
    pub metiers: Vec<Metier>,
    pub competences: Vec<String>,
    /// « CAP Cuisine », « Bac pro Logistique », « BTS MCO (en cours) »
    pub diplomes: Vec<String>,
    /// « Permis B », « CACES 1, 3, 5 », « Habilitation électrique », « SST »
    pub permis: Vec<String>,
}

impl Parcours {
    /// La fiche dit assez pour comparer une offre : un métier, une compétence ou un permis au moins.
    pub fn utilisable(&self) -> bool {
        !self.metiers.is_empty() || !self.competences.is_empty() || !self.permis.is_empty()
    }

    pub fn vide(&self) -> bool {
        !self.utilisable() && self.diplomes.is_empty()
    }

    /// Le plus haut diplôme obtenu de la fiche (rien : aucun dont le niveau est connu).
    pub fn diplome_le_plus_haut(&self) -> Option<Diplome> {
        self.diplomes.iter().filter_map(|d| niveau_diplome(d)).max()
    }

    /// Les clés des lignes d'une liste (voir [`cle_ligne`]).
    pub fn cles(&self, l: Liste) -> Vec<String> {
        match l {
            Liste::Metiers => self.metiers.iter().map(|m| cle_ligne(l, &m.nom)).collect(),
            Liste::Competences => self.competences.iter().map(|x| cle_ligne(l, x)).collect(),
            Liste::Diplomes => self.diplomes.iter().map(|x| cle_ligne(l, x)).collect(),
            Liste::Permis => self.permis.iter().map(|x| cle_ligne(l, x)).collect(),
        }
    }

    /// La fiche corrigée par la personne : les lignes retirées s'en vont, les lignes ajoutées s'y ajoutent (sauf
    /// celles que la fiche a déjà).
    pub fn corrige(&self, corrections: &[Correction]) -> Parcours {
        let mut p = self.clone();
        let retire = |l: Liste, cle: &str| corrections.iter().any(|c| !c.ajout && c.liste == l && c.cle == cle);
        p.metiers.retain(|m| !retire(Liste::Metiers, &cle_ligne(Liste::Metiers, &m.nom)));
        p.competences.retain(|x| !retire(Liste::Competences, &cle_ligne(Liste::Competences, x)));
        p.diplomes.retain(|x| !retire(Liste::Diplomes, &cle_ligne(Liste::Diplomes, x)));
        p.permis.retain(|x| !retire(Liste::Permis, &cle_ligne(Liste::Permis, x)));
        for c in corrections.iter().filter(|c| c.ajout) {
            if p.cles(c.liste).contains(&c.cle) {
                continue;
            }
            match c.liste {
                Liste::Metiers => p.metiers.push(metier_saisi(&c.texte)),
                Liste::Competences => p.competences.push(c.texte.clone()),
                Liste::Diplomes => p.diplomes.push(c.texte.clone()),
                Liste::Permis => p.permis.push(c.texte.clone()),
            }
        }
        p
    }
}

/// Une correction de la personne : une ligne ajoutée ou retirée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    pub liste: Liste,
    /// Vrai : une ligne ajoutée ; faux : une ligne retirée
    pub ajout: bool,
    /// La clé de la ligne ([`cle_ligne`])
    pub cle: String,
    /// Le texte de la ligne ajoutée (vide pour un retrait)
    pub texte: String,
}

/// La clé d'une ligne : ce qui fait que deux lignes sont la même (« Vendeuse » et « vendeur », « Permis B » et
/// « permis b »).
pub fn cle_ligne(l: Liste, texte: &str) -> String {
    let k = match l {
        Liste::Metiers => {
            // (« Apprenti électricien » et « Électricien » : deux lignes, même si la comparaison les prend pour le même
            // métier)
            let nom = metier_saisi(texte).nom;
            let avant = mots(&nom).first().map(|m| racine(m)).filter(|m| AVANT_LE_POSTE.contains(&m.as_str()));
            avant.into_iter().chain(mots_metier(&nom)).collect::<Vec<_>>().join(" ")
        }
        Liste::Competences => mots_competence(texte).join(" "),
        Liste::Diplomes | Liste::Permis => String::new(),
    };
    if k.is_empty() { normaliser(texte) } else { k }
}

/// Une ligne tapée par la personne, vérifiée : son texte remis au propre.
pub fn verifier_ajout(l: Liste, texte: &str) -> Result<String, String> {
    let mut t = une_ligne(texte);
    if t.chars().filter(|c| c.is_alphanumeric()).count() < 2 && !(l == Liste::Permis && !t.is_empty()) {
        return Err(match l {
            Liste::Metiers => "Écris le nom du métier, par exemple : Cariste, 3 ans.",
            Liste::Competences => "Écris la compétence, par exemple : Conduite de chariot.",
            Liste::Diplomes => "Écris le diplôme, par exemple : CAP Cuisine.",
            Liste::Permis => "Écris le permis ou l'habilitation, par exemple : Permis B.",
        }
        .into());
    }
    if t.chars().count() > LIGNE_MAX {
        return Err(format!("C'est trop long ({LIGNE_MAX} caractères au plus). Écris-le en quelques mots."));
    }
    // (« B », « CE » tapés seuls dans les permis : « Permis B »)
    if l == Liste::Permis
        && ["b", "c", "ce", "c1", "c1e", "d", "be", "a", "a2", "am"].contains(&normaliser(&t).as_str())
    {
        t = format!("Permis {}", t.to_uppercase());
    }
    if cle_ligne(l, &t).is_empty() {
        return Err("Écris-le avec des lettres.".into());
    }
    Ok(majuscule(&t))
}

/// Un métier tapé par la personne : « Cariste, 3 ans », « Vendeuse depuis 2021 », « Boulanger ».
pub fn metier_saisi(texte: &str) -> Metier {
    let t = une_ligne(texte);
    let mut j = jetons(&t);
    let d = dates_de(&t, &mut j);
    let fin = j.iter().position(|x| x.pris).map_or(t.len(), |i| j[i].debut);
    let mut nom = t[..fin].trim_end_matches(|c: char| !c.is_alphanumeric()).to_string();
    for mot in [" depuis", " pendant", " durant", " de", " du"] {
        let n = nom.len().saturating_sub(mot.len());
        if nom.is_char_boundary(n) && nom[n..].eq_ignore_ascii_case(mot) {
            nom.truncate(n);
        }
    }
    let nom = nom.trim_end_matches(|c: char| !c.is_alphanumeric()).to_string();
    if nom.chars().filter(|c| c.is_alphanumeric()).count() < 2 {
        return Metier { nom: t, mois: None, depuis: None };
    }
    Metier { nom, mois: d.mois, depuis: d.depuis }
}

// ---------------------------------------------------------------------------------------------------------------
// Les mots
// ---------------------------------------------------------------------------------------------------------------

fn mots(t: &str) -> Vec<String> {
    normaliser(t).split(' ').filter(|m| !m.is_empty()).map(str::to_string).collect()
}

/// Les mots qui finissent par « s » ou « x » au singulier.
const INVARIABLES: &[&str] = &[
    "poids", "frais", "bois", "repas", "colis", "temps", "cours", "mois", "secours", "parcours", "divers", "devis",
    "avis", "anglais", "francais", "tapis", "bras", "relais", "acces", "succes", "pays", "gros", "bas", "pas", "tres",
    "plus", "sous", "sans", "vers", "fois", "cas", "jus", "dos", "bus", "ras", "puis", "commis", "permis", "caces",
    "travaux", "prix", "choix", "voix", "process", "corps", "engrais",
];

/// Un mot ramené au masculin singulier (« vendeuses » : « vendeur », « conductrice » : « conducteur », « employée » :
/// « employe »). Une règle simple, la même des deux côtés d'une comparaison : elle n'a pas à être belle.
fn racine(m: &str) -> String {
    let mut s = m.to_string();
    if s == "hotesse" || s == "hotesses" {
        return "hote".into();
    }
    if INVARIABLES.contains(&m) {
        return s;
    }
    if s.len() > 5 && s.ends_with("eaux") {
        s.pop();
    } else if s.len() > 5 && s.ends_with("aux") {
        s.truncate(s.len() - 3);
        s.push_str("al");
    } else if s.len() > 3 && (s.ends_with('s') || s.ends_with('x')) && !s.ends_with("ss") {
        s.pop();
    }
    if s.len() < 6 || !s.is_ascii() {
        return s;
    }
    const FEMININS: &[(&str, &str)] = &[
        ("trice", "teur"),
        ("ienne", "ien"),
        ("euse", "eur"),
        ("iere", "ier"),
        ("onne", "on"),
        ("elle", "el"),
        ("ointe", "oint"),
        ("ante", "ant"),
        ("ente", "ent"),
        ("ive", "if"),
        ("ale", "al"),
        ("ere", "er"),
        ("ee", "e"),
    ];
    for (fin, par) in FEMININS {
        if let Some(debut) = s.strip_suffix(fin) {
            return format!("{debut}{par}");
        }
    }
    if s == "avocate" || s == "candidate" {
        s.pop();
    }
    s
}

/// La distance entre deux mots (lettres changées, ajoutées ou retirées), arrêtée à 3.
fn ecart(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 2 {
        return 3;
    }
    let mut avant: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut ici = vec![i; b.len() + 1];
        for k in 1..=b.len() {
            let cout = usize::from(a[i - 1] != b[k - 1]);
            ici[k] = (avant[k] + 1).min(ici[k - 1] + 1).min(avant[k - 1] + cout);
        }
        avant = ici;
    }
    avant[b.len()].min(3)
}

/// Le même mot, à une faute de frappe près (deux pour un long mot).
fn a_une_faute_pres(a: &str, b: &str) -> bool {
    a == b || (a.len() >= 5 && b.len() >= 5 && ecart(a, b) <= 1) || (a.len() >= 9 && b.len() >= 9 && ecart(a, b) <= 2)
}

/// La première lettre en majuscule.
fn majuscule(t: &str) -> String {
    let mut c = t.chars();
    match c.next() {
        Some(x) => x.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Les sigles gardés en capitales quand une ligne tout en majuscules est remise en minuscules.
const SIGLES: &[&str] = &[
    "cap", "bep", "bts", "dut", "bp", "cqp", "pl", "spl", "vl", "caces", "haccp", "sst", "fimo", "fco", "adr", "ash",
    "atsem", "aesh", "deas", "deaes", "rh", "btp", "sav", "pme", "it", "bpjeps", "dcg", "dscg", "mba", "bafa",
];

/// Un texte écrit tout en majuscules (un CV en capitales) remis en minuscules, sa première lettre et ses sigles
/// gardés ; un texte qui a des minuscules reste tel quel.
fn casse(t: &str) -> String {
    if t.chars().any(char::is_lowercase) || !t.chars().any(char::is_uppercase) {
        return majuscule(t);
    }
    let bas = t.to_lowercase();
    let mut s = String::with_capacity(bas.len());
    let mut debut: Option<usize> = None;
    for (i, c) in bas.char_indices().chain(std::iter::once((bas.len(), ' '))) {
        if c.is_alphanumeric() {
            debut.get_or_insert(i);
        } else {
            if let Some(d) = debut.take() {
                let mot = &bas[d..i];
                if SIGLES.contains(&normaliser(mot).as_str()) {
                    s.push_str(&mot.to_uppercase());
                } else {
                    s.push_str(mot);
                }
            }
            if i < bas.len() {
                s.push(c);
            }
        }
    }
    majuscule(&s)
}

// ---------------------------------------------------------------------------------------------------------------
// Les noms de métier
// ---------------------------------------------------------------------------------------------------------------

/// Les noms qui disent un métier à eux seuls (au masculin singulier, sans accent).
pub const TETES_PRECISES: &[&str] = &[
    "cariste",
    "magasinier",
    "manutentionnaire",
    "vendeur",
    "caissier",
    "comptable",
    "developpeur",
    "programmeur",
    "developer",
    "chauffeur",
    "livreur",
    "routier",
    "serveur",
    "cuisinier",
    "boulanger",
    "patissier",
    "boucher",
    "charcutier",
    "poissonnier",
    "coiffeur",
    "estheticien",
    "infirmier",
    "medecin",
    "pharmacien",
    "secretaire",
    "receptionniste",
    "standardiste",
    "electricien",
    "plombier",
    "macon",
    "peintre",
    "menuisier",
    "charpentier",
    "couvreur",
    "carreleur",
    "soudeur",
    "chaudronnier",
    "mecanicien",
    "carrossier",
    "tourneur",
    "fraiseur",
    "usineur",
    "ajusteur",
    "jardinier",
    "paysagiste",
    "agriculteur",
    "eleveur",
    "ambulancier",
    "brancardier",
    "facteur",
    "coursier",
    "demenageur",
    "grutier",
    "plongeur",
    "barman",
    "barmaid",
    "sommelier",
    "traiteur",
    "fleuriste",
    "libraire",
    "bijoutier",
    "opticien",
    "photographe",
    "graphiste",
    "infographiste",
    "webmaster",
    "webdesigner",
    "designer",
    "journaliste",
    "traducteur",
    "interprete",
    "juriste",
    "avocat",
    "notaire",
    "architecte",
    "geometre",
    "dessinateur",
    "acheteur",
    "commercial",
    "televendeur",
    "teleconseiller",
    "teleoperateur",
    "negociateur",
    "courtier",
    "guichetier",
    "tresorier",
    "auditeur",
    "informaticien",
    "electromecanicien",
    "electrotechnicien",
    "automaticien",
    "frigoriste",
    "chauffagiste",
    "plaquiste",
    "platrier",
    "facadier",
    "etancheur",
    "vitrier",
    "serrurier",
    "metallier",
    "tuyauteur",
    "cableur",
    "conditionneur",
    "emballeur",
    "trieur",
    "receptionnaire",
    "approvisionneur",
    "logisticien",
    "affreteur",
    "eboueur",
    "ripeur",
    "balayeur",
    "laveur",
    "nettoyeur",
    "repasseur",
    "linger",
    "couturier",
    "retoucheur",
    "cordonnier",
    "tapissier",
    "ebeniste",
    "imprimeur",
    "bucheron",
    "elagueur",
    "viticulteur",
    "maraicher",
    "horticulteur",
    "pecheur",
    "marin",
    "matelot",
    "steward",
    "concierge",
    "veilleur",
    "vigile",
    "pompier",
    "policier",
    "gendarme",
    "militaire",
    "douanier",
    "enseignant",
    "professeur",
    "instituteur",
    "educateur",
    "formateur",
    "moniteur",
    "nounou",
    "puericulteur",
    "kinesitherapeute",
    "dentiste",
    "veterinaire",
    "orthophoniste",
    "psychologue",
    "dieteticien",
    "ergotherapeute",
    "ash",
    "atsem",
    "aesh",
    "docker",
    "manoeuvre",
    "terrassier",
    "coffreur",
    "ferrailleur",
    "canalisateur",
    "solier",
    "cuisiniste",
    "depanneur",
    "garagiste",
    "pompiste",
    "caviste",
    "barista",
    "crepier",
    "pizzaiolo",
    "fromager",
    "epicier",
    "buraliste",
    "commercant",
    "artisan",
    "tolier",
    "ouvreur",
    "bagagiste",
    "magasinier",
    "inventoriste",
    "merchandiseur",
    "etalagiste",
    "animalier",
    "toiletteur",
    "palefrenier",
    "berger",
    "apiculteur",
    "sauveteur",
    "croupier",
    "projectionniste",
    "monteur",
    "regleur",
    "decolleteur",
    "plasturgiste",
    "verrier",
    "ceramiste",
    "bobinier",
    "ascensoriste",
    "antenniste",
    "ramoneur",
    "vendangeur",
    "cueilleur",
    "saisonnier",
];

/// Les noms larges : ils ne disent un métier qu'avec ce qui les suit (« agent de quai », « chef de rayon »).
pub const TETES_LARGES: &[&str] = &[
    "agent",
    "employe",
    "technicien",
    "responsable",
    "chef",
    "assistant",
    "charge",
    "conseiller",
    "operateur",
    "ouvrier",
    "aide",
    "conducteur",
    "gestionnaire",
    "directeur",
    "ingenieur",
    "animateur",
    "controleur",
    "adjoint",
    "expert",
    "specialiste",
    "coordinateur",
    "coordonnateur",
    "manager",
    "consultant",
    "preparateur",
    "auxiliaire",
    "equipier",
    "hote",
    "attache",
    "delegue",
    "cadre",
    "collaborateur",
    "superviseur",
    "administrateur",
    "analyste",
    "concepteur",
    "inspecteur",
    "redacteur",
    "gerant",
    "garde",
    "gardien",
    "pilote",
    "poseur",
    "installateur",
    "reparateur",
    "commis",
    "second",
    "maitre",
    "intervenant",
    "accompagnant",
    "accompagnateur",
    "surveillant",
    "officier",
    "exploitant",
    "encadrant",
    "referent",
    "femme",
    "homme",
    "garcon",
];

/// Les noms larges qui commencent aussi une ligne de mission (« Responsable de la caisse », « Aide aux clients ») :
/// seuls sur leur ligne sans date, ils ne font pas un métier.
const AMBIGUES: &[&str] =
    &["responsable", "charge", "aide", "garde", "second", "homme", "femme", "garcon", "pilote", "cadre", "expert"];

/// Les mots qui précèdent le nom du poste et restent dans son nom (« Apprenti boulanger », « Stagiaire vendeur »).
const AVANT_LE_POSTE: &[&str] = &["apprenti", "stagiaire", "alternant"];

/// Les mots d'un titre de poste ou d'offre qui ne disent rien du métier.
const VIDES: &[&str] = &[
    "h",
    "f",
    "hf",
    "fh",
    "m",
    "x",
    "e",
    "es",
    "se",
    "ne",
    "euse",
    "trice",
    "ere",
    "iere",
    "rice",
    "nne",
    "cdi",
    "cdd",
    "interim",
    "interimaire",
    "urgent",
    "nouveau",
    "de",
    "du",
    "des",
    "d",
    "la",
    "le",
    "les",
    "l",
    "en",
    "et",
    "a",
    "au",
    "aux",
    "un",
    "une",
    "pour",
    "sur",
    "avec",
    "sans",
    "ou",
    "temps",
    "plein",
    "partiel",
    "junior",
    "senior",
    "confirme",
    "debutant",
    "experimente",
    "qualifie",
    "apprenti",
    "stagiaire",
    "alternant",
    "alternance",
    "stage",
    "apprentissage",
    "poste",
    "recherche",
    "recrute",
    "accepte",
    "offre",
];

/// Les mots après un nom large qui ne suffisent pas à dire que deux postes sont le même.
const QUALIFS_VIDES: &[&str] =
    &["polyvalent", "principal", "adjoint", "general", "niveau", "equipe", "service", "technique"];

/// Les mots de liaison dans un nom de poste.
const LIAISONS: &[&str] = &["de", "d", "du", "des", "la", "le", "l", "en", "et", "a", "au", "aux"];

/// Les mots où le nom du poste s'arrête.
const COUPES: &[&str] = &[
    "chez",
    "pour",
    "dans",
    "aupres",
    "depuis",
    "pendant",
    "durant",
    "sein",
    "societe",
    "entreprise",
    "ste",
    "sarl",
    "sas",
    "groupe",
    "caces",
    "permis",
    "h",
    "f",
    "hf",
    "cdi",
    "cdd",
    "interim",
    "avec",
    "puis",
    "avant",
    "mission",
];

/// Les mots qui suivent un nom de poste sans en faire partie (« Cariste expérimenté », « Aide-soignant diplômé »).
const FINS_DE_POSTE: &[&str] =
    &["experimente", "confirme", "qualifie", "debutant", "junior", "senior", "passionne", "diplome", "certifie"];

/// Les lieux de travail après « à », « au » : le nom du poste est fini (« aide à domicile à l'association... »).
const LIEUX: &[&str] = &[
    "association",
    "mairie",
    "hopital",
    "clinique",
    "usine",
    "ecole",
    "college",
    "lycee",
    "entreprise",
    "societe",
    "agence",
    "cabinet",
    "centre",
    "maison",
    "ehpad",
    "restaurant",
    "hotel",
    "garage",
    "supermarche",
    "hypermarche",
    "magasin",
    "boutique",
    "creche",
    "camping",
    "bar",
    "brasserie",
];

/// Les mots sautés avant le nom du poste (« J'ai été cariste », « Poste de vendeur », « en tant que »).
const AVANT: &[&str] = &[
    "poste",
    "de",
    "d",
    "en",
    "tant",
    "que",
    "qu",
    "comme",
    "emploi",
    "j",
    "je",
    "ai",
    "ete",
    "suis",
    "travaille",
    "occupe",
    "exerce",
    "fonction",
    "metier",
    "actuellement",
    "anciennement",
    "ancien",
    "experience",
    "experiences",
    "exp",
    "an",
    "ans",
    "un",
    "une",
    "le",
    "la",
    "l",
    "job",
    "contrat",
    "depuis",
    "pendant",
    "durant",
    "avant",
    "puis",
    "ensuite",
    "auparavant",
    "et",
    "aussi",
    "egalement",
    "deja",
];

/// Les familles de noms qui disent le même métier (mots au masculin singulier, sans mot de liaison).
pub const GROUPES: &[&[&str]] = &[
    &["cariste", "conducteur chariot", "chauffeur chariot", "conducteur engin manutention"],
    &[
        "chauffeur poids lourd",
        "conducteur poids lourd",
        "conducteur routier",
        "chauffeur routier",
        "chauffeur pl",
        "chauffeur spl",
        "conducteur pl",
        "conducteur spl",
        "chauffeur super lourd",
        "conducteur super lourd",
        "routier",
    ],
    &["livreur", "chauffeur livreur", "conducteur livreur", "chauffeur vl"],
    &["vendeur", "conseiller vente", "employe vente", "conseiller vendeur"],
    &["caissier", "hote caisse", "employe caisse"],
    &["employe libre service", "employe rayon", "employe commercial", "employe mise rayon", "els"],
    &["preparateur commande", "agent logistique", "operateur logistique", "employe logistique"],
    &["auxiliaire vie", "aide domicile", "assistant vie", "aide menager"],
    &[
        "agent entretien",
        "agent proprete",
        "agent nettoyage",
        "technicien surface",
        "femme menage",
        "homme menage",
        "employe menage",
    ],
    &[
        "developpeur",
        "programmeur",
        "developer",
        "ingenieur logiciel",
        "ingenieur developpement",
        "analyste programmeur",
    ],
    &["secretaire", "assistant administratif", "agent administratif", "employe administratif"],
    &["agent securite", "agent surete", "vigile", "agent prevention securite", "agent surveillance"],
    &["manutentionnaire", "agent manutention"],
    &["aide comptable", "assistant comptable"],
    &["teleconseiller", "teleoperateur", "conseiller clientele distance", "conseiller client distance"],
    &["equipier polyvalent", "employe polyvalent restauration", "equipier restauration", "employe restauration"],
    &["agent service hospitalier", "ash"],
];

/// Les domaines : les mots d'un nom de poste qui disent où il se range.
pub const DOMAINES: &[(&str, &[&str])] = &[
    (
        "la logistique",
        &[
            "cariste",
            "magasinier",
            "manutentionnaire",
            "logistique",
            "entrepot",
            "quai",
            "commande",
            "receptionnaire",
            "logisticien",
            "stock",
            "expedition",
            "chariot",
            "manutention",
            "magasinage",
        ],
    ),
    ("la vente", &["vendeur", "caissier", "caisse", "rayon", "magasin", "vente", "commerce", "boutique", "els"]),
    (
        "le soin",
        &[
            "soignant",
            "infirmier",
            "hospitalier",
            "brancardier",
            "ehpad",
            "ash",
            "domicile",
            "vie",
            "menager",
            "medico",
        ],
    ),
    ("le transport", &["chauffeur", "routier", "livreur", "coursier", "bus", "car", "taxi", "convoyeur", "pl", "spl"]),
    (
        "la restauration",
        &[
            "serveur",
            "cuisinier",
            "cuisine",
            "plongeur",
            "barman",
            "restauration",
            "restaurant",
            "crepier",
            "pizzaiolo",
        ],
    ),
    ("le nettoyage", &["entretien", "proprete", "nettoyage", "menage", "surface"]),
    (
        "le bâtiment",
        &[
            "macon",
            "peintre",
            "plombier",
            "electricien",
            "carreleur",
            "couvreur",
            "charpentier",
            "menuisier",
            "plaquiste",
            "coffreur",
            "batiment",
            "chantier",
            "travaux",
            "manoeuvre",
            "solier",
            "moquettiste",
            "platrier",
            "facadier",
            "etancheur",
            "vitrier",
            "serrurier",
            "metallier",
            "chauffagiste",
        ],
    ),
    ("le bureau", &["secretaire", "administratif", "accueil", "standardiste", "receptionniste", "direction"]),
    ("la comptabilité", &["comptable", "comptabilite", "paie", "tresorier", "audit"]),
    (
        "l'informatique",
        &[
            "developpeur",
            "programmeur",
            "informatique",
            "web",
            "logiciel",
            "reseau",
            "systeme",
            "data",
            "devops",
            "developer",
            "informaticien",
        ],
    ),
    (
        "l'industrie",
        &[
            "production",
            "ligne",
            "usinage",
            "soudeur",
            "chaudronnier",
            "assemblage",
            "conditionnement",
            "regleur",
            "fabrication",
            "monteur",
        ],
    ),
    ("la sécurité", &["securite", "surete", "vigile", "surveillance", "incendie"]),
    ("l'enfance", &["enfant", "creche", "atsem", "periscolaire", "maternel", "nounou", "puericulteur"]),
];

/// D'autres mots qui font partie d'un nom de poste (« Chauffeur Poids Lourd », « Développeur Web ») : un mot qui
/// commence par une majuscule et n'est connu nulle part est pris pour le nom de l'entreprise ou de la ville.
const MOTS_DE_POSTE: &[&str] = &[
    "poids",
    "lourd",
    "libre",
    "service",
    "polyvalent",
    "commercial",
    "administratif",
    "industriel",
    "technique",
    "general",
    "principal",
    "adjoint",
    "mobile",
    "fullstack",
    "full",
    "stack",
    "front",
    "back",
    "end",
    "java",
    "php",
    "python",
    "sanitaire",
    "social",
    "medical",
    "scolaire",
    "familial",
    "automobile",
    "agricole",
    "alimentaire",
    "textile",
    "bancaire",
    "immobilier",
    "juridique",
    "financier",
    "marketing",
    "digital",
    "numerique",
    "support",
    "export",
    "import",
    "international",
    "regional",
    "national",
    "salle",
    "etage",
    "nuit",
    "jour",
    "equipe",
    "maintenance",
    "qualite",
    "espace",
    "vert",
    "engin",
    "machine",
    "clientele",
    "client",
    "gestion",
    "recrutement",
    "communication",
    "formation",
    "projet",
    "etude",
    "developpement",
    "boulangerie",
    "patisserie",
    "boucherie",
    "pret",
    "porter",
    "super",
    "frais",
    "sec",
    "fruit",
    "legume",
    "chambre",
    "mise",
    "web",
    "cuisine",
    "production",
    "quai",
    "informatique",
    "reseau",
    "systeme",
    "data",
];

/// Ce que dit un nom en tête de poste : rien, un nom large, un nom précis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tete {
    Large,
    Precise,
}

fn tete_de(r: &str) -> Option<Tete> {
    if TETES_PRECISES.contains(&r) {
        Some(Tete::Precise)
    } else if TETES_LARGES.contains(&r) {
        Some(Tete::Large)
    } else {
        None
    }
}

/// Un nom de métier écrit avec une faute de frappe (« chaufeur », « manutentionaire ») ramené au nom connu ; tout
/// autre mot reste tel quel. Seulement pour un mot d'au moins sept lettres qui commence comme le nom connu, à une
/// lettre près, et qui n'est ni un verbe (« contrôler ») ni un nom d'action (« peinture »).
fn canonique(r: &str) -> String {
    if tete_de(r).is_some() || r.len() < 7 || !r.is_ascii() || r.ends_with("er") || r.ends_with("ir") || nom_d_action(r)
    {
        return r.to_string();
    }
    TETES_PRECISES
        .iter()
        .chain(TETES_LARGES)
        .find(|t| t.len() >= 7 && t.as_bytes()[0] == r.as_bytes()[0] && ecart(t, r) <= 1)
        .map_or_else(|| r.to_string(), |t| (*t).to_string())
}

/// Un mot qui suit un nom de poste sans en faire partie.
fn fin_de_poste(m: &str) -> bool {
    FINS_DE_POSTE.contains(&m) || FINS_DE_POSTE.contains(&racine(m).as_str())
}

/// Un mot de qualité personnelle (« sérieux », « motivée »).
fn qualite(m: &str) -> bool {
    QUALITES.contains(&m) || QUALITES.contains(&racine(m).as_str())
}

/// Un mot qui fait partie d'un nom de poste sans doute possible (pas un simple mot de domaine).
fn mot_de_poste_sur(r: &str) -> bool {
    tete_de(r).is_some()
        || MOTS_DE_POSTE.contains(&r)
        || GROUPES.iter().any(|g| g.iter().any(|p| p.split(' ').any(|m| m == r)))
}

/// Les mots d'un nom de poste ou d'un titre d'offre qui disent le métier, au masculin singulier.
fn mots_metier(t: &str) -> Vec<String> {
    mots(t).iter().filter(|m| !VIDES.contains(&m.as_str())).map(|m| canonique(&racine(m))).collect()
}

/// Les mots d'une compétence qui comptent pour la retrouver dans une offre.
fn mots_competence(t: &str) -> Vec<String> {
    const PETITS: &[&str] = &[
        "de", "d", "du", "des", "la", "le", "les", "l", "en", "et", "a", "au", "aux", "un", "une", "sur", "avec",
        "pour", "par", "ou", "son", "ses",
    ];
    mots(t).iter().filter(|m| !PETITS.contains(&m.as_str())).map(|m| racine(m)).collect()
}

// ---------------------------------------------------------------------------------------------------------------
// Les jetons d'une ligne, les dates et les durées
// ---------------------------------------------------------------------------------------------------------------

/// Un mot d'une ligne : remis au propre, sa place dans la ligne, pris par une date ou non, et s'il vient après une
/// coupure nette (une virgule, un tiret entouré d'espaces, deux-points, une parenthèse...).
#[derive(Debug, Clone)]
struct Jeton {
    mot: String,
    debut: usize,
    fin: usize,
    pris: bool,
    coupe: bool,
    /// Il commence une phrase (après un point)
    phrase: bool,
}

fn coupure(entre: &str) -> bool {
    entre.contains([',', ';', ':', '|', '(', ')', '[', ']', '•', '/', '"', '«', '»', '\t', '=', '>', '*'])
        || entre.contains(" - ")
        || entre.starts_with("- ")
        || entre.ends_with(" -")
        || entre.contains(". ")
}

fn jetons(l: &str) -> Vec<Jeton> {
    let mut v: Vec<Jeton> = Vec::new();
    let mut debut: Option<usize> = None;
    let mut fin_avant = 0;
    for (i, c) in l.char_indices().chain(std::iter::once((l.len(), ' '))) {
        if c.is_alphanumeric() {
            debut.get_or_insert(i);
        } else if let Some(d) = debut.take() {
            // (les fautes de frappe courantes des petits mots qui comptent)
            let mot = match normaliser(&l[d..i]).as_str() {
                "depui" | "depuit" | "depuits" => "depuis".to_string(),
                "che" | "chz" => "chez".to_string(),
                "pendan" => "pendant".to_string(),
                "aujourdui" | "aujourdhui" => "aujourdhui".to_string(),
                autre => autre.to_string(),
            };
            if !mot.is_empty() && !mot.contains(' ') {
                let entre = &l[fin_avant..d];
                let coupe = v.is_empty() || coupure(entre);
                let phrase = v.is_empty() || entre.contains(". ") || entre.contains('!') || entre.contains('?');
                v.push(Jeton { mot, debut: d, fin: i, pris: false, coupe, phrase });
            }
            fin_avant = i;
        }
    }
    v
}

/// Une date ou une durée lue dans une ligne.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Temps {
    /// Une période finie, en mois (0 : pas de quoi la compter) ; vrai : comptée d'après les années seules
    Finie(u16, bool),
    /// Un poste en cours depuis (l'année, le mois)
    EnCours(i32, u32),
    /// « 3 ans », « 6 mois »
    Duree(u16),
    /// Une date seule
    Seule,
}

/// Ce qu'une ligne, ou un bout de ligne, dit de dates et de durées.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Dates {
    /// Une date ou une durée y est écrite
    vue: bool,
    mois: Option<u16>,
    depuis: Option<(i32, u32)>,
}

impl Dates {
    /// Le bilan des dates placées entre ces deux jetons (le second non compris). Une seule durée compte, la
    /// première (« vendeur 4 ans, puis 3 ans de manutention ») ; elle passe avant une période comptée d'après les
    /// années seules (« 2018 (6 mois) »).
    fn de(temps: &[(usize, Temps)], de: usize, a: usize) -> Dates {
        let mut d = Dates::default();
        let mut duree: Option<u16> = None;
        let mut annees_seules = false;
        for (_, t) in temps.iter().filter(|(p, _)| (de..a).contains(p)) {
            d.vue = true;
            match *t {
                Temps::Finie(m, seules) => {
                    annees_seules |= seules || m == 0;
                    if m > 0 {
                        d.mois = Some(d.mois.unwrap_or(0).saturating_add(m));
                    }
                }
                Temps::EnCours(a, m) => d.depuis = Some(d.depuis.map_or((a, m), |x| x.min((a, m)))),
                Temps::Duree(m) => duree = duree.or(Some(m)),
                Temps::Seule => annees_seules = true,
            }
        }
        if let Some(m) = duree
            && d.depuis.is_none()
            && (d.mois.is_none() || annees_seules)
        {
            d.mois = Some(m);
        }
        d
    }
}

fn annee(m: &str) -> Option<i32> {
    (m.len() == 4 && m.bytes().all(|b| b.is_ascii_digit()))
        .then(|| m.parse::<i32>().ok())
        .flatten()
        .filter(|a| (1950..=2100).contains(a))
}

fn petit_nombre(m: &str, max: u32) -> Option<u32> {
    (!m.is_empty() && m.len() <= 2 && m.bytes().all(|b| b.is_ascii_digit()))
        .then(|| m.parse::<u32>().ok())
        .flatten()
        .filter(|n| (1..=max).contains(n))
}

fn mois_nomme(m: &str) -> Option<u32> {
    Some(match m {
        "janvier" | "janv" | "jan" => 1,
        "fevrier" | "fev" | "fevr" => 2,
        "mars" => 3,
        "avril" | "avr" => 4,
        "mai" => 5,
        "juin" => 6,
        "juillet" | "juil" => 7,
        "aout" => 8,
        "septembre" | "sept" => 9,
        "octobre" | "oct" => 10,
        "novembre" | "nov" => 11,
        "decembre" | "dec" => 12,
        _ => return None,
    })
}

fn nombre_en_mots(m: &str) -> Option<u32> {
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
        _ => return petit_nombre(m, 60),
    })
}

/// Une date à cette place : (l'année, le mois ou 0, le nombre de jetons). « mars 2020 », « 03/2020 », « 12 mars
/// 2020 », « 12/03/2020 », « 2020 ».
fn date_ici(l: &str, j: &[Jeton], i: usize) -> Option<(i32, u32, usize)> {
    let mot = |k: usize| j.get(k).map_or("", |x| x.mot.as_str());
    let colle = |a: usize| {
        j.get(a + 1).is_some_and(|b| matches!(l[j[a].fin..b.debut].trim(), "/" | "." | "-") && !j[a + 1].mot.is_empty())
    };
    if let (Some(m), Some(a)) = (mois_nomme(mot(i)), annee(mot(i + 1))) {
        return Some((a, m, 2));
    }
    if let (Some(_), Some(m), Some(a)) = (petit_nombre(mot(i), 31), mois_nomme(mot(i + 1)), annee(mot(i + 2))) {
        return Some((a, m, 3));
    }
    if let (Some(_), Some(m), Some(a)) = (petit_nombre(mot(i), 31), petit_nombre(mot(i + 1), 12), annee(mot(i + 2)))
        && colle(i)
        && colle(i + 1)
    {
        return Some((a, m, 3));
    }
    if let (Some(m), Some(a)) = (petit_nombre(mot(i), 12), annee(mot(i + 1)))
        && colle(i)
    {
        return Some((a, m, 2));
    }
    annee(mot(i)).map(|a| (a, 0, 1))
}

/// « aujourd'hui », « ce jour », « présent », « en cours » à cette place : le nombre de jetons.
fn maintenant_ici(j: &[Jeton], i: usize) -> Option<usize> {
    let mot = |k: usize| j.get(k).map_or("", |x| x.mot.as_str());
    match (mot(i), mot(i + 1)) {
        ("aujourd", "hui") | ("ce", "jour") | ("en", "cours") | ("en", "poste") => Some(2),
        ("aujourdhui" | "maintenant" | "present" | "actuel" | "actuellement" | "now" | "today", _) => Some(1),
        _ => None,
    }
}

/// Les dates et les durées de toute une ligne (leurs jetons marqués).
fn dates_de(l: &str, j: &mut [Jeton]) -> Dates {
    Dates::de(&lire_dates(l, j), 0, usize::MAX)
}

/// Lit les dates et les durées d'une ligne, chacune à la place de son premier jeton, et marque leurs jetons.
fn lire_dates(l: &str, j: &mut [Jeton]) -> Vec<(usize, Temps)> {
    let mut d: Vec<(usize, Temps)> = Vec::new();
    let parle_d_experience =
        j.iter().any(|x| ["experience", "experiences", "exp", "pendant", "depuis", "durant"].contains(&x.mot.as_str()));
    let age = j.iter().any(|x| ["age", "ne", "nee"].contains(&x.mot.as_str()));
    let mut i = 0;
    while i < j.len() {
        if let Some((a, m, n)) = date_ici(l, j, i) {
            j[i..i + n].iter_mut().for_each(|x| x.pris = true);
            // (« été 2023 », « hiver 2023-2024 » : une saison, pas de quoi compter des mois)
            let saison =
                i >= 1 && ["ete", "hiver", "saison", "printemps", "automne", "noel"].contains(&j[i - 1].mot.as_str());
            let apres_depuis = i >= 1 && j[i - 1].mot == "depuis"
                || i >= 2 && j[i - 2].mot == "depuis" && ["le", "l"].contains(&j[i - 1].mot.as_str());
            // (« à », « au », « jusqu'à » entre les deux dates)
            let mut k = i + n;
            while k < j.len() && k < i + n + 2 && ["a", "au", "jusqu", "jusque"].contains(&j[k].mot.as_str()) {
                k += 1;
            }
            if let Some((a2, m2, n2)) = date_ici(l, j, k) {
                j[i + n..k + n2].iter_mut().for_each(|x| x.pris = true);
                let mois = if m > 0 && m2 > 0 {
                    (i64::from(a2) - i64::from(a)) * 12 + i64::from(m2) - i64::from(m) + 1
                } else {
                    (i64::from(a2) - i64::from(a)) * 12
                };
                let mois = if (1..=720).contains(&mois) && !saison { mois as u16 } else { 0 };
                d.push((i, Temps::Finie(mois, m == 0 || m2 == 0)));
                i = k + n2;
                continue;
            }
            if let Some(n2) = maintenant_ici(j, k) {
                j[i + n..k + n2].iter_mut().for_each(|x| x.pris = true);
                d.push((i, Temps::EnCours(a, m.max(1))));
                i = k + n2;
                continue;
            }
            d.push((i, if apres_depuis { Temps::EnCours(a, m.max(1)) } else { Temps::Seule }));
            i += n;
            continue;
        }
        // Une durée : « 3 ans », « six mois », « 1 an et demi », « 2 ans et 3 mois »
        let unite = j.get(i + 1).map_or("", |x| x.mot.as_str());
        if let Some(n) = nombre_en_mots(&j[i].mot)
            && matches!(unite, "an" | "ans" | "annee" | "annees" | "mois")
        {
            let en_ans = unite != "mois";
            // (« 25 ans » sans parler d'expérience : un âge)
            let un_age = en_ans && (age || (n >= 16 && !parle_d_experience));
            let plausible = if en_ans { n <= 45 } else { n <= 60 };
            let mut fin = i + 2;
            if !un_age && plausible {
                let mut mois = if en_ans { n * 12 } else { n };
                let mot = |k: usize| j.get(k).map_or("", |x| x.mot.as_str());
                if en_ans && mot(fin) == "et" && mot(fin + 1) == "demi" {
                    mois += 6;
                    fin += 2;
                } else if en_ans
                    && let Some(m2) = nombre_en_mots(mot(fin + usize::from(mot(fin) == "et")))
                    && mot(fin + usize::from(mot(fin) == "et") + 1) == "mois"
                    && m2 < 12
                {
                    mois += m2;
                    fin += usize::from(mot(fin) == "et") + 2;
                }
                d.push((i, Temps::Duree(mois.min(720) as u16)));
            }
            let fin = fin.min(j.len());
            j[i..fin].iter_mut().for_each(|x| x.pris = true);
            i = fin;
            continue;
        }
        i += 1;
    }
    d
}

// ---------------------------------------------------------------------------------------------------------------
// Le nom du poste d'une ligne
// ---------------------------------------------------------------------------------------------------------------

/// Un nom de poste trouvé dans une ligne.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Poste {
    nom: String,
    /// Son premier et son dernier jeton
    debut: usize,
    fin: usize,
    /// Il commence la ligne (rien avant lui que des dates et des petits mots)
    premier: bool,
    /// Son nom ne peut pas être une ligne de mission
    fort: bool,
}

fn commence_par_une_majuscule(t: &str) -> bool {
    t.chars().next().is_some_and(char::is_uppercase)
}

/// Les noms de poste d'une ligne (ses dates déjà marquées) : chaque morceau de la ligne qui commence par un nom de
/// métier connu.
fn postes_de(l: &str, j: &[Jeton]) -> Vec<Poste> {
    let mut v: Vec<Poste> = Vec::new();
    let mut s = 0;
    while s < j.len() {
        // (« comme », « en tant que » : le poste suit, où que ce soit dans la ligne)
        let annonce = j[s].mot == "comme" || (s >= 2 && j[s - 1].mot == "que" && j[s - 2].mot == "tant");
        let libre = !j[s].pris && (s == 0 || j[s].coupe || j[s - 1].pris || annonce);
        match libre.then(|| poste_a(l, j, s)).flatten() {
            Some(p) => {
                s = p.fin + 1;
                v.push(p);
            }
            None => s += 1,
        }
    }
    v
}

/// Le nom de poste qui commence au jeton `s` (après les petits mots qui le précèdent), s'il y en a un.
fn poste_a(l: &str, j: &[Jeton], s: usize) -> Option<Poste> {
    let n = j.len();
    // Les petits mots d'avant (« en tant que », « poste de »), sans passer une coupure
    let mut k = s;
    while k < n && (j[k].pris || AVANT.contains(&j[k].mot.as_str())) && (k == s || !j[k].coupe) {
        k += 1;
    }
    if k >= n || (k > s && j[k].coupe) {
        return None;
    }
    let debut = k;
    if AVANT_LE_POSTE.contains(&racine(&j[k].mot).as_str()) && k + 1 < n && !j[k + 1].coupe && !j[k + 1].pris {
        k += 1;
    }
    let r = canonique(&racine(&j[k].mot));
    let tete = tete_de(&r)?;
    // Le nom s'étend jusqu'à l'entreprise, la ville ou la coupure
    let mut fin = k;
    let mut qualifs = 0;
    for t in k + 1..n {
        let m = j[t].mot.as_str();
        if j[t].pris || j[t].coupe || COUPES.contains(&m) || t - k >= 6 {
            break;
        }
        let brut = &l[j[t].debut..j[t].fin];
        let suivant = j.get(t + 1).filter(|x| !x.coupe && !x.pris);
        // (« à Metz », « à l'Hôpital Fictif » : le lieu ; « aide à domicile », « vendeur à la criée » : le poste)
        if ["a", "au", "aux"].contains(&m) {
            let lieu = j[t + 1..]
                .iter()
                .take_while(|x| !x.coupe && !x.pris)
                .find(|x| !LIAISONS.contains(&x.mot.as_str()))
                .is_none_or(|x| {
                    commence_par_une_majuscule(&l[x.debut..x.fin]) || LIEUX.contains(&racine(&x.mot).as_str())
                });
            // (après un nom de métier précis, « au », « aux » : « vendeur au rayon chaussures »)
            if lieu || (tete == Tete::Precise && m != "a") {
                break;
            }
        }
        if m == "en"
            && suivant.is_none_or(|x| {
                ["cdi", "cdd", "interim", "alternance", "apprentissage", "stage", "contrat", "mission"]
                    .contains(&x.mot.as_str())
            })
        {
            break;
        }
        if !LIAISONS.contains(&m) {
            let apres_liaison = LIAISONS.contains(&j[t - 1].mot.as_str());
            let rt = racine(m);
            let premier_qualif = tete == Tete::Large && qualifs == 0;
            // (après un nom déjà complet, un mot de domaine seul, « Entrepôts », « Magasin », est celui de
            // l'entreprise)
            let connu = if premier_qualif || apres_liaison { true } else { mot_de_poste_sur(&rt) };
            if commence_par_une_majuscule(brut) && !connu {
                break;
            }
            if !QUALIFS_VIDES.contains(&rt.as_str()) && !m.bytes().all(|b| b.is_ascii_digit()) {
                qualifs += 1;
            }
        }
        fin = t;
    }
    while fin > k && (LIAISONS.contains(&j[fin].mot.as_str()) || fin_de_poste(&j[fin].mot) || qualite(&j[fin].mot)) {
        fin -= 1;
    }
    if tete == Tete::Large {
        let reste = j[k + 1..=fin]
            .iter()
            .filter(|x| !LIAISONS.contains(&x.mot.as_str()) && !QUALIFS_VIDES.contains(&racine(&x.mot).as_str()))
            .count();
        // (« Employé polyvalent » se dit ; « Agent », « Responsable » seuls ne disent rien)
        if reste == 0 && fin == k {
            return None;
        }
    }
    let premier = j[..debut].iter().all(|x| x.pris || AVANT.contains(&x.mot.as_str()));
    let fort = tete == Tete::Precise || !AMBIGUES.contains(&r.as_str());
    Some(Poste { nom: casse(&l[j[debut].debut..j[fin].fin]), debut, fin, premier, fort })
}

// ---------------------------------------------------------------------------------------------------------------
// Les rubriques d'un CV
// ---------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rubrique {
    /// Avant toute rubrique, ou « Profil », « À propos »
    Aucune,
    Experience,
    Formation,
    Competences,
    Permis,
    /// « Loisirs », « Langues », « Qualités »... : rien n'y est un métier ni une compétence
    Autre,
    /// Un CV à deux colonnes dont les lignes ont été mélangées : lu case par case
    Melange,
}

/// Les titres de rubrique (mots au singulier, sans petit mot).
const RUBRIQUES: &[(&str, Rubrique)] = &[
    ("experience", Rubrique::Experience),
    ("experience professionnel", Rubrique::Experience),
    ("experience pro", Rubrique::Experience),
    ("parcours professionnel", Rubrique::Experience),
    ("parcours", Rubrique::Experience),
    ("emploi", Rubrique::Experience),
    ("emploi occupe", Rubrique::Experience),
    ("poste occupe", Rubrique::Experience),
    ("historique professionnel", Rubrique::Experience),
    ("vie professionnel", Rubrique::Experience),
    ("experience travail", Rubrique::Experience),
    ("stage", Rubrique::Experience),
    ("stage emploi", Rubrique::Experience),
    ("emploi stage", Rubrique::Experience),
    ("experience stage", Rubrique::Experience),
    ("stage experience", Rubrique::Experience),
    ("job", Rubrique::Experience),
    ("job ete", Rubrique::Experience),
    ("emploi saisonnier", Rubrique::Experience),
    ("mission", Rubrique::Experience),
    ("mission interim", Rubrique::Experience),
    ("formation", Rubrique::Formation),
    ("diplome", Rubrique::Formation),
    ("etude", Rubrique::Formation),
    ("formation diplome", Rubrique::Formation),
    ("diplome formation", Rubrique::Formation),
    ("scolarite", Rubrique::Formation),
    ("cursus", Rubrique::Formation),
    ("parcours scolaire", Rubrique::Formation),
    ("formation initial", Rubrique::Formation),
    ("formation scolaire", Rubrique::Formation),
    ("formation professionnel", Rubrique::Formation),
    ("education", Rubrique::Formation),
    ("competence", Rubrique::Competences),
    ("competence cle", Rubrique::Competences),
    ("competence professionnel", Rubrique::Competences),
    ("competence technique", Rubrique::Competences),
    ("savoir faire", Rubrique::Competences),
    ("domaine competence", Rubrique::Competences),
    ("connaissance", Rubrique::Competences),
    ("informatique", Rubrique::Competences),
    ("outil", Rubrique::Competences),
    ("logiciel", Rubrique::Competences),
    ("competence informatique", Rubrique::Competences),
    ("outil informatique", Rubrique::Competences),
    ("connaissance informatique", Rubrique::Competences),
    ("permis", Rubrique::Permis),
    ("permis habilitation", Rubrique::Permis),
    ("habilitation permis", Rubrique::Permis),
    ("habilitation", Rubrique::Permis),
    ("certification", Rubrique::Permis),
    ("permis certification", Rubrique::Permis),
    ("certificat", Rubrique::Permis),
    ("caces", Rubrique::Permis),
    ("permis caces", Rubrique::Permis),
    ("caces permis", Rubrique::Permis),
    ("profil", Rubrique::Aucune),
    ("a propos", Rubrique::Aucune),
    ("a propos moi", Rubrique::Aucune),
    ("presentation", Rubrique::Aucune),
    ("objectif", Rubrique::Aucune),
    ("objectif professionnel", Rubrique::Aucune),
    ("projet professionnel", Rubrique::Aucune),
    ("resume", Rubrique::Aucune),
    ("loisir", Rubrique::Autre),
    ("centre interet", Rubrique::Autre),
    ("interet", Rubrique::Autre),
    ("langue", Rubrique::Autre),
    ("contact", Rubrique::Autre),
    ("coordonnee", Rubrique::Autre),
    ("divers", Rubrique::Autre),
    ("reference", Rubrique::Autre),
    ("qualite", Rubrique::Autre),
    ("qualite personnel", Rubrique::Autre),
    ("savoir etre", Rubrique::Autre),
    ("information", Rubrique::Autre),
    ("information personnel", Rubrique::Autre),
    ("information complementaire", Rubrique::Autre),
    ("etat civil", Rubrique::Autre),
    ("hobby", Rubrique::Autre),
    ("activite", Rubrique::Autre),
    ("benevolat", Rubrique::Autre),
    ("atout", Rubrique::Autre),
    ("passion", Rubrique::Autre),
    ("vie associatif", Rubrique::Autre),
    ("autre", Rubrique::Autre),
    ("autre information", Rubrique::Autre),
];

const PUCES: &[char] = &['•', '-', '*', '·', '▪', '●', '○', '>', '→', '✓', '■', '◦'];

/// La ligne sans sa puce, et si elle en avait une.
fn sans_puce(l: &str) -> (&str, bool) {
    let t = l.trim();
    match t.strip_prefix(PUCES) {
        Some(reste) => (reste.trim_start_matches(PUCES).trim(), true),
        None => (t, false),
    }
}

/// Le titre de rubrique d'une ligne, et ce qui le suit après « : » sur la même ligne.
fn titre_rubrique(l: &str) -> Option<(Rubrique, &str)> {
    let (l, _) = sans_puce(l);
    let (tete, reste) = match l.split_once(':') {
        Some((a, b)) => (a, b.trim()),
        None => (l, ""),
    };
    const PETITS: &[&str] = &["mes", "mon", "ma", "et", "de", "d", "des", "les", "l", "la", "le"];
    let m: Vec<String> = mots(tete).iter().filter(|x| !PETITS.contains(&x.as_str())).map(|x| racine(x)).collect();
    if m.is_empty() || m.len() > 4 || tete.chars().count() > 45 {
        return None;
    }
    RUBRIQUES
        .iter()
        .find(|(titre, _)| {
            let t: Vec<String> = titre.split(' ').map(racine).collect();
            t.len() == m.len() && t.iter().zip(&m).all(|(a, b)| a_une_faute_pres(a, b))
        })
        .map(|(_, r)| (*r, reste))
}

// ---------------------------------------------------------------------------------------------------------------
// Les permis et les habilitations
// ---------------------------------------------------------------------------------------------------------------

/// Les habilitations et formations obligatoires reconnues : (le nom montré, les suites de mots qui le disent).
const HABILITATIONS: &[(&str, &[&str])] = &[
    ("Habilitation électrique", &["habilitation electrique", "habilitations electriques", "habilite electrique"]),
    ("SST", &["sst", "sauveteur secouriste"]),
    ("FIMO", &["fimo"]),
    ("FCO", &["fco"]),
    ("ADR", &["adr"]),
    ("HACCP", &["haccp"]),
    ("AIPR", &["aipr"]),
    ("SSIAP", &["ssiap"]),
    ("PSC1", &["psc1", "psc 1"]),
    ("AFGSU", &["afgsu"]),
    ("BAFA", &["bafa"]),
    ("Travail en hauteur", &["travail en hauteur", "travaux en hauteur"]),
];

/// Les habilitations citées dans un texte.
pub fn habilitations(t: &str) -> Vec<String> {
    let n = format!(" {} ", normaliser(t));
    let mut l: Vec<String> = Vec::new();
    for (nom, motifs) in HABILITATIONS {
        if motifs.iter().any(|m| n.contains(&format!(" {m} "))) {
            l.push((*nom).to_string());
        }
    }
    // (« habilitation B0 H0 », « habilitations BR, B1V »)
    let m: Vec<&str> = n.split(' ').collect();
    const CODES: &[&str] = &["b0", "h0", "b1", "b1v", "b2", "b2v", "br", "bc", "h1", "h2", "h0v", "bs", "be", "h1v"];
    let electrique = m.iter().enumerate().any(|(i, x)| {
        x.starts_with("habilit") && m[i + 1..].iter().take(4).any(|y| CODES.contains(y) || y.starts_with("elec"))
    });
    if electrique && !l.iter().any(|x| x == "Habilitation électrique") {
        l.insert(0, "Habilitation électrique".to_string());
    }
    l
}

/// Un permis, un CACES ou une habilitation, pour savoir si la personne a celui que l'offre demande.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Droit {
    /// « b », « c », « ce »...
    Permis(String),
    PoidsLourd,
    /// « Permis de conduire », sans lettre
    Conduire,
    /// Ses catégories (aucune : le CACES sans précision)
    Caces(Vec<String>),
    /// Une habilitation (son nom), ou une ligne tapée que rien ne reconnaît (ses mots)
    Autre(String),
}

/// Les droits que dit un texte (une ligne de la fiche, ou ce que l'offre demande).
fn droits(t: &str) -> Vec<Droit> {
    // (« permi B » : la faute de frappe la plus courante)
    let corrige: String;
    let t = if normaliser(t).split(' ').any(|m| m == "permi") {
        corrige =
            t.split(' ').map(|m| if normaliser(m) == "permi" { "permis" } else { m }).collect::<Vec<_>>().join(" ");
        corrige.as_str()
    } else {
        t
    };
    let mut l = Vec::new();
    for x in lecture::permis_et_caces(t) {
        if let Some(reste) = x.strip_prefix("Permis ") {
            l.push(match reste {
                "poids lourd" => Droit::PoidsLourd,
                "de conduire" => Droit::Conduire,
                lettre => Droit::Permis(lettre.to_lowercase()),
            });
        } else if let Some(reste) = x.strip_prefix("CACES") {
            let cats = reste
                .split([' ', ','])
                .filter(|c| !c.is_empty() && c.len() <= 2 && c.starts_with(|x: char| x.is_ascii_digit()))
                .map(str::to_lowercase)
                .collect();
            l.push(Droit::Caces(cats));
        }
    }
    // (« Permis B, C et CE » : les lettres qui suivent la première)
    for phrase in t.split(['.', ';', '\n', '!', '?']) {
        let m = mots(phrase);
        let Some(i) = m.iter().position(|x| x == "permis") else { continue };
        let mut k = i + 1;
        let mut vu = false;
        while k < m.len() && k < i + 10 {
            // (« le permis, c'est un plus » : pas le permis C)
            let lettre = ["b", "c", "ce", "ec", "c1", "c1e", "be", "d1"].contains(&m[k].as_str())
                && m.get(k + 1).is_none_or(|x| x != "est");
            if lettre {
                let d = Droit::Permis(m[k].clone());
                if !l.contains(&d) {
                    l.push(d);
                }
                vu = true;
            } else if !(vu && ["et", "ou"].contains(&m[k].as_str()))
                && !(k == i + 1 && m[k] == "de")
                && m[k] != "conduire"
            {
                break;
            }
            k += 1;
        }
    }
    l.extend(habilitations(t).into_iter().map(Droit::Autre));
    l
}

/// La personne a ce droit, d'après ceux de sa fiche.
fn possede(fiche: &[Droit], demande: &Droit) -> bool {
    let a = |lettres: &[&str]| {
        fiche.iter().any(|d| matches!(d, Droit::Permis(x) if lettres.contains(&x.as_str())))
            || (lettres.contains(&"pl") && fiche.contains(&Droit::PoidsLourd))
    };
    match demande {
        Droit::Conduire => fiche.iter().any(|d| matches!(d, Droit::Permis(_) | Droit::PoidsLourd | Droit::Conduire)),
        Droit::PoidsLourd => a(&["c", "ce", "ec", "c1", "c1e", "pl"]),
        Droit::Permis(x) => match x.as_str() {
            "b" => a(&["b", "be", "c", "ce", "ec", "c1", "c1e", "d", "de", "pl"]) || fiche.contains(&Droit::Conduire),
            "c" => a(&["c", "ce", "ec", "pl"]),
            "c1" => a(&["c1", "c1e", "c", "ce", "ec", "pl"]),
            "ce" | "ec" => a(&["ce", "ec"]),
            "c1e" => a(&["c1e", "ce", "ec"]),
            "d" => a(&["d", "de"]),
            autre => a(&[autre]),
        },
        Droit::Caces(cats) => fiche.iter().any(|d| match d {
            Droit::Caces(les_miens) => {
                cats.is_empty() || les_miens.is_empty() || cats.iter().any(|c| les_miens.contains(c))
            }
            _ => false,
        }),
        // (la FCO renouvelle la FIMO : qui a l'une répond à une offre qui demande « FIMO ou FCO »)
        Droit::Autre(nom) if nom == "FIMO" || nom == "FCO" => {
            fiche.iter().any(|d| matches!(d, Droit::Autre(x) if x == "FIMO" || x == "FCO"))
        }
        Droit::Autre(nom) => fiche.iter().any(|d| matches!(d, Droit::Autre(x) if normaliser(x) == normaliser(nom))),
    }
}

/// Le droit dans une phrase : « le CACES », « le permis C », « l'habilitation électrique ».
fn nom_droit(d: &Droit) -> String {
    match d {
        Droit::Permis(x) => format!("le permis {}", x.to_uppercase()),
        Droit::PoidsLourd => "le permis poids lourd".into(),
        Droit::Conduire => "le permis".into(),
        Droit::Caces(c) if c.is_empty() => "le CACES".into(),
        Droit::Caces(c) => format!("le CACES {}", c.join(", ").to_uppercase()),
        Droit::Autre(x) if x == "Habilitation électrique" => "l'habilitation électrique".into(),
        Droit::Autre(x) if x == "Travail en hauteur" => "la formation au travail en hauteur".into(),
        Droit::Autre(x) if x == "FIMO" || x == "FCO" => format!("la {x}"),
        Droit::Autre(x) if x == "HACCP" => "la formation HACCP".into(),
        Droit::Autre(x) => format!("le {x}"),
    }
}

/// Un droit que beaucoup d'offres demandent sans qu'il dise rien du métier (le permis B).
fn banal(d: &Droit) -> bool {
    matches!(d, Droit::Conduire) || matches!(d, Droit::Permis(x) if x == "b")
}

/// La ligne dit que le permis n'est pas encore là (« en cours », « prévu », « pas de permis »).
fn pas_encore(l: &str) -> bool {
    let n = format!(" {} ", normaliser(l));
    [
        " en cours ",
        " en cour ",
        " prevu ",
        " prevue ",
        " en preparation ",
        " sans permis ",
        " pas de permis ",
        " pas le permis ",
        " non titulaire ",
        " a passer ",
    ]
    .iter()
    .any(|m| n.contains(m))
}

// ---------------------------------------------------------------------------------------------------------------
// Les diplômes
// ---------------------------------------------------------------------------------------------------------------

/// Le jeton `i` commence un morceau de la ligne : avant lui, jusqu'à la coupure, seulement des dates et ces petits
/// mots.
fn debute_un_morceau(j: &[Jeton], i: usize, petits: &[&str]) -> bool {
    let mut k = i;
    loop {
        if k == 0 || j[k].coupe {
            return true;
        }
        k -= 1;
        // (juste après une date : un nouveau morceau, « pêche 2015 Bac pro Logistique »)
        if j[k].pris {
            return true;
        }
        if !petits.contains(&j[k].mot.as_str()) {
            return false;
        }
    }
}

/// Les mots qui commencent un diplôme.
const MOTS_DIPLOME: &[&str] = &[
    "cap",
    "bep",
    "bac",
    "baccalaureat",
    "bts",
    "dut",
    "deug",
    "deust",
    "licence",
    "master",
    "mastere",
    "bachelor",
    "doctorat",
    "mba",
    "dess",
    "bpjeps",
    "cqp",
    "deas",
    "deaes",
    "deavs",
    "deamp",
    "dees",
    "deje",
    "dcg",
    "dscg",
    "brevet",
];

/// Un diplôme commence à ce jeton.
fn diplome_ici(l: &str, j: &[Jeton], i: usize) -> bool {
    let mot = |k: usize| j.get(k).map_or("", |x| x.mot.as_str());
    let m = mot(i);
    MOTS_DIPLOME.contains(&m)
        || (m.len() > 3 && m.starts_with("bac") && m[3..].bytes().all(|b| b.is_ascii_digit()))
        || (m == "bp" && &l[j[i].debut..j[i].fin] == "BP")
        || (m == "titre" && ["professionnel", "pro"].contains(&mot(i + 1)))
        || (m == "diplome" && !mot(i + 1).is_empty())
        || (m == "niveau" && (MOTS_DIPLOME.contains(&mot(i + 1)) || mot(i + 1).starts_with("bac")))
        || (m == "certificat" && ["d", "de"].contains(&mot(i + 1)))
}

/// Le diplôme d'une ligne, en court (« CAP Cuisine »), s'il y en a un. Dans la rubrique « Formation », le mot de
/// diplôme peut être n'importe où dans la ligne ; ailleurs, il doit commencer un morceau de la ligne.
fn diplome_de(l: &str, dans_formation: bool) -> Option<String> {
    let n = format!(" {} ", normaliser(l));
    if [" non obtenu ", " pas obtenu ", " pas eu ", " echec ", " non valide ", " abandon "]
        .iter()
        .any(|m| n.contains(m))
    {
        return None;
    }
    let mut j = jetons(l);
    lire_dates(l, &mut j);
    const AVANT_DIPLOME: &[&str] =
        &["titulaire", "d", "un", "une", "du", "de", "la", "obtention", "obtenu", "j", "ai", "mon"];
    let debut = (0..j.len()).find(|&i| {
        if j[i].pris || !diplome_ici(l, &j, i) {
            return false;
        }
        // (ce qui est avant, jusqu'à la coupure : seulement des dates et des petits mots)
        dans_formation || debute_un_morceau(&j, i, AVANT_DIPLOME)
    })?;
    const FINS: &[&str] = &[
        "obtenu",
        "obtention",
        "mention",
        "lycee",
        "universite",
        "ecole",
        "cfa",
        "greta",
        "afpa",
        "iut",
        "centre",
        "institut",
        "academie",
        "session",
        "valide",
        "avec",
        "non",
        "pas",
        "chez",
        "organisme",
    ];
    let mut fin = debut;
    for t in debut + 1..j.len() {
        let m = j[t].mot.as_str();
        if j[t].pris || j[t].coupe || FINS.contains(&m) || t - debut >= 9 {
            break;
        }
        // (« Brevet des collèges » garde son collège ; ailleurs, le nom de l'école finit le diplôme)
        if ["college", "colleges"].contains(&m) && j[debut].mot != "brevet" {
            break;
        }
        let suivant = j.get(t + 1).filter(|x| !x.coupe && !x.pris);
        if ["a", "au"].contains(&m) && suivant.is_none_or(|x| commence_par_une_majuscule(&l[x.debut..x.fin])) {
            break;
        }
        if m == "en" && suivant.is_none_or(|x| ["cours", "alternance", "apprentissage"].contains(&x.mot.as_str())) {
            break;
        }
        fin = t;
    }
    while fin > debut && LIAISONS.contains(&j[fin].mot.as_str()) {
        fin -= 1;
    }
    if fin == debut && ["diplome", "niveau", "certificat", "titre"].contains(&j[debut].mot.as_str()) {
        return None;
    }
    let mut nom = casse(&l[j[debut].debut..j[fin].fin]);
    if [" en cours ", " en cour ", " actuellement ", " en preparation ", " je prepare "].iter().any(|m| n.contains(m)) {
        nom.push_str(" (en cours)");
    }
    Some(nom)
}

/// Le niveau d'un diplôme de la fiche (rien : pas obtenu, ou un diplôme dont le niveau n'est pas connu).
pub fn niveau_diplome(ligne: &str) -> Option<Diplome> {
    let n = normaliser(ligne);
    let sans_niveau = n.starts_with("niveau ") && !n.split(' ').nth(1).is_some_and(|x| x.len() == 1);
    if n.contains("en cours") || sans_niveau {
        return None;
    }
    lecture::diplome_le_plus_haut(&format!("Diplôme : {ligne}")).filter(|d| *d != Diplome::Aucun)
}

// ---------------------------------------------------------------------------------------------------------------
// Les compétences
// ---------------------------------------------------------------------------------------------------------------

/// Les mots des qualités personnelles : une ligne qui n'a qu'eux n'est pas une compétence.
const QUALITES: &[&str] = &[
    "ponctuel",
    "serieux",
    "motive",
    "dynamique",
    "rigoureux",
    "autonome",
    "autonomie",
    "rigueur",
    "ponctualite",
    "sociable",
    "souriant",
    "organise",
    "polyvalent",
    "polyvalence",
    "adaptable",
    "adaptabilite",
    "esprit",
    "equipe",
    "travail",
    "sens",
    "relationnel",
    "contact",
    "bon",
    "capacite",
    "ecoute",
    "courageux",
    "volontaire",
    "discret",
    "discretion",
    "patient",
    "patience",
    "honnete",
    "minutieux",
    "curieux",
    "creatif",
    "reactif",
    "reactivite",
    "flexible",
    "disponible",
    "disponibilite",
    "assidu",
    "motivation",
    "resistance",
    "stress",
    "initiative",
    "responsabilite",
    "aisance",
    "presentation",
    "excellent",
    "tres",
    "grand",
    "facilite",
    "organisation",
    "humain",
    "aimable",
    "poli",
    "calme",
    "fiable",
    "investi",
    "implique",
    "goût",
    "gout",
];

/// Les mots trop larges pour retrouver une compétence dans une offre à eux seuls.
const MOTS_LARGES: &[&str] = &[
    "gestion",
    "travail",
    "service",
    "equipe",
    "organisation",
    "communication",
    "client",
    "qualite",
    "connaissance",
    "maitrise",
    "notion",
    "base",
    "utilisation",
    "pratique",
    "experience",
    "logiciel",
    "outil",
    "bonne",
    "bon",
    "respect",
    "regle",
    "poste",
];

/// Les premiers mots d'une ligne de mission qui dit un savoir-faire (« Conduite de chariot », « Mise en rayon »).
const ACTIONS: &[&str] = &[
    "accueil",
    "vente",
    "conduite",
    "tenue",
    "suivi",
    "gestion",
    "mise",
    "saisie",
    "entretien",
    "controle",
    "conseil",
    "soin",
    "aide",
    "pose",
    "tri",
    "port",
    "prise",
    "inventaire",
    "caisse",
    "reassort",
    "facing",
    "picking",
    "releve",
    "plonge",
    "decoupe",
    "service",
    "toilette",
    "livraison",
    "cuisson",
    "frappe",
    "diagnostic",
    "repas",
    "change",
    "sieste",
    "activite",
    "course",
    "menage",
    "repassage",
    "soudure",
    "peinture",
    "maconnerie",
    "plomberie",
    "electricite",
    "comptabilite",
    "facturation",
    "paie",
    "secretariat",
    "standard",
    "cuisine",
    "patisserie",
    "tenu",
    "travaux",
];

fn nom_d_action(premier: &str) -> bool {
    let r = racine(premier);
    ACTIONS.contains(&r.as_str())
        || (r.len() >= 6
            && ["tion", "sion", "ment", "age", "ance", "ence", "ure"].iter().any(|f| r.ends_with(f))
            && tete_de(&r).is_none())
}

/// Une compétence remise au propre, ou rien si ce n'en est pas une (une qualité, un permis, un diplôme, trop long).
fn competence(brut: &str) -> Option<String> {
    let t = brut.trim().trim_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | '-' | '•')).trim();
    let t = t.strip_prefix("et ").unwrap_or(t).trim();
    let m = mots(t);
    if m.is_empty()
        || m.len() > 7
        || t.chars().count() > LIGNE_MAX
        || t.chars().filter(|c| c.is_alphabetic()).count() < 3
    {
        return None;
    }
    if m.iter().any(|x| annee(x).is_some()) || !droits(t).is_empty() {
        return None;
    }
    if mots_competence(t).is_empty() || m.iter().all(|x| qualite(x) || LIAISONS.contains(&x.as_str())) {
        return None;
    }
    // (un seul mot : un sigle ou un nom d'outil, gardé tel quel, « PHP », « SQL »)
    Some(if m.len() == 1 { majuscule(t) } else { casse(t) })
}

// ---------------------------------------------------------------------------------------------------------------
// La lecture d'un CV
// ---------------------------------------------------------------------------------------------------------------

struct Ligne<'a> {
    rubrique: Rubrique,
    texte: &'a str,
    puce: bool,
    jetons: Vec<Jeton>,
    temps: Vec<(usize, Temps)>,
    dates: Dates,
    postes: Vec<Poste>,
    prise: bool,
}

impl<'a> Ligne<'a> {
    fn nouvelle(rubrique: Rubrique, texte: &'a str, puce: bool) -> Ligne<'a> {
        let mut jetons = jetons(texte);
        let temps = lire_dates(texte, &mut jetons);
        let dates = Dates::de(&temps, 0, usize::MAX);
        let postes = postes_de(texte, &jetons);
        Ligne { rubrique, texte, puce, jetons, temps, dates, postes, prise: false }
    }

    /// Les dates de chaque poste de la ligne. Un seul poste : celles de toute la ligne. Plusieurs : chacun prend
    /// celles de sa phrase ; et quand une phrase a plusieurs postes (« aide à domicile depuis 2021, avant agent
    /// d'entretien 3 ans »), celles écrites après lui, jusqu'au poste suivant (ou avant lui, quand la phrase commence
    /// par une date).
    fn dates_des_postes(&self) -> Vec<Dates> {
        let n = self.postes.len();
        if n <= 1 {
            return vec![self.dates.clone(); n];
        }
        let j = &self.jetons;
        (0..n)
            .map(|i| {
                let p = &self.postes[i];
                let d = (0..=p.debut).rev().find(|&k| j[k].phrase).unwrap_or(0);
                let f = (p.fin + 1..j.len()).find(|&k| j[k].phrase).unwrap_or(usize::MAX);
                // (les postes de la même phrase, et la place de celui-ci parmi eux)
                let dans: Vec<usize> = (0..n).filter(|&k| (d..f).contains(&self.postes[k].debut)).collect();
                let rang = dans.iter().position(|&k| k == i).unwrap_or(0);
                if dans.len() <= 1 {
                    return Dates::de(&self.temps, d, f);
                }
                let dernier = rang + 1 == dans.len();
                let premier_poste = self.postes[dans[0]].debut;
                let avant =
                    self.temps.iter().find(|(x, _)| (d..f).contains(x)).is_some_and(|(x, _)| *x < premier_poste);
                let (de, a) = match (avant, rang) {
                    (true, 0) => (d, p.debut),
                    (true, _) => (self.postes[dans[rang - 1]].fin + 1, if dernier { f } else { p.debut }),
                    (false, 0) => (d, self.postes[dans[1]].debut),
                    (false, _) => (p.fin + 1, if dernier { f } else { self.postes[dans[rang + 1]].debut }),
                };
                Dates::de(&self.temps, de, a)
            })
            .collect()
    }
}

/// Une ligne faite de deux titres de rubrique collés (« COMPETENCES EXPERIENCES PROFESSIONNELLES », « LOISIRS
/// FORMATION ») : le signe d'un CV à deux colonnes dont les lignes ont été mélangées.
fn deux_titres(l: &str) -> bool {
    let m: Vec<&str> = l.split_whitespace().collect();
    if m.len() < 2 || m.len() > 7 || l.contains(':') {
        return false;
    }
    (1..m.len()).any(|k| match (titre_rubrique(&m[..k].join(" ")), titre_rubrique(&m[k..].join(" "))) {
        (Some((a, _)), Some((b, _))) => a != b,
        _ => false,
    })
}

/// Un titre de rubrique en capitales suivi d'autre chose sur la même ligne, sans « : » (« LANGUES Cariste -
/// Logi-Bidule ») : le même signe. Rend ce qui suit le titre.
fn titre_puis_texte(l: &str) -> Option<&str> {
    let debuts: Vec<usize> = l
        .char_indices()
        .filter(|(i, c)| !c.is_whitespace() && (*i == 0 || l[..*i].ends_with(char::is_whitespace)))
        .map(|(i, _)| i)
        .collect();
    (1..=3usize).rev().filter(|k| *k < debuts.len()).find_map(|k| {
        let (titre, suite) = l.split_at(debuts[k]);
        let en_capitales = titre.chars().any(char::is_uppercase) && !titre.chars().any(char::is_lowercase);
        let mot_suivant = suite.split_whitespace().next().unwrap_or_default();
        (en_capitales
            && !titre.contains(':')
            && mot_suivant.chars().any(char::is_lowercase)
            && titre_rubrique(titre.trim()).is_some())
        .then_some(suite)
    })
}

/// Dans un CV aux colonnes mélangées, une case commence à ce jeton : après une coupure ou une date, ou un mot qui
/// commence par une majuscule après un mot en minuscules, un nombre ou un mot tout en capitales (« de camions
/// Cariste magasinier », « HABILITATIONS Préparateur de commandes »).
fn debut_de_case(l: &str, j: &[Jeton], s: usize) -> bool {
    if s == 0 || j[s].coupe || j[s - 1].pris {
        return true;
    }
    let brut = |k: usize| &l[j[k].debut..j[k].fin];
    let (ici, avant) = (brut(s), brut(s - 1));
    let capitale = commence_par_une_majuscule(ici) && ici.chars().any(char::is_lowercase);
    let avant_autre = !commence_par_une_majuscule(avant) || !avant.chars().any(char::is_lowercase);
    capitale && avant_autre && !LIAISONS.contains(&j[s - 1].mot.as_str())
}

/// Les compétences d'une ligne de missions (« Encaissement, accueil des clients, mise en rayon ») : la ligne doit
/// commencer par un nom d'action ; ses morceaux courts suivent.
fn missions(t: &str, puce: bool) -> Vec<String> {
    let commence_bien = |x: &str| mots(x).first().is_some_and(|m| nom_d_action(m));
    // (une longue ligne s'arrête à « sur », « avec », « pour »... : « Prise de rendez-vous sur le logiciel Machin »)
    let court = |x: &'_ str| -> String {
        let x = x.split('(').next().unwrap_or(x).trim();
        if mots(x).len() <= 7 {
            return x.to_string();
        }
        [" sur ", " avec ", " pour ", " dans ", " chez ", " auprès ", " à l'aide "]
            .iter()
            .filter_map(|m| x.find(m))
            .min()
            .map_or_else(|| x.to_string(), |i| x[..i].trim().to_string())
    };
    let morceaux: Vec<String> = t.split([',', ';']).map(court).collect();
    // (une ligne à puce sous un poste est une mission, même sans nom d'action, sauf ce qui parle des horaires ou du
    // contrat, ou d'un diplôme)
    const PAS_UNE_MISSION: &[&str] =
        &["travail", "horaire", "horaires", "poste", "contrat", "cdi", "cdd", "salaire", "astreinte", "astreintes"];
    let premier = morceaux.first().map(String::as_str).unwrap_or_default();
    let mission = commence_bien(premier)
        || (puce
            && mots(premier).len() >= 2
            && !mots(premier).first().is_some_and(|m| PAS_UNE_MISSION.contains(&m.as_str()))
            && diplome_de(premier, false).is_none());
    if !mission {
        return Vec::new();
    }
    morceaux
        .iter()
        .enumerate()
        .filter(|(i, x)| {
            let n = mots(x).len();
            let autre_chose = mots(x).first().is_some_and(|m| PAS_UNE_MISSION.contains(&m.as_str()));
            !autre_chose && if *i == 0 { n <= 7 } else { n <= 4 || (n <= 6 && commence_bien(x)) }
        })
        .filter_map(|(_, x)| competence(x))
        .collect()
}

/// Lit un CV et en tire sa fiche. Un texte vide ou illisible : une fiche vide, jamais une erreur.
pub fn lire(texte: &str) -> Parcours {
    let mut p = Parcours::default();
    let mut rubrique = Rubrique::Aucune;
    let mut melange = false;
    let mut lignes: Vec<Ligne> = Vec::new();
    let mut fiche_droits: Vec<Droit> = Vec::new();
    for brute in texte.lines().take(2000) {
        let brute = brute.trim();
        if !brute.chars().any(char::is_alphanumeric) {
            continue;
        }
        // Les permis et habilitations : partout, sauf sur une ligne qui dit « en cours »
        if !pas_encore(brute) {
            for d in droits(brute) {
                if !fiche_droits.contains(&d) {
                    fiche_droits.push(d);
                }
            }
        }
        let (texte, puce) = if let Some((r, reste)) = titre_rubrique(brute) {
            rubrique = r;
            if reste.is_empty() {
                continue;
            }
            (reste, false)
        } else if deux_titres(brute) {
            melange = true;
            continue;
        } else if let Some(reste) = titre_puis_texte(brute) {
            melange = true;
            (reste, false)
        } else {
            sans_puce(brute)
        };
        lignes.push(Ligne::nouvelle(if melange { Rubrique::Melange } else { rubrique }, texte, puce));
    }
    p.permis = lignes_de_droits(&fiche_droits);

    let ajouter_diplome = |d: String, p: &mut Parcours| {
        let k = normaliser(&d);
        if !p.diplomes.iter().any(|x| normaliser(x) == k) && p.diplomes.len() < 12 {
            p.diplomes.push(d);
        }
    };
    let ajouter = |c: Option<String>, p: &mut Parcours| {
        if let Some(c) = c {
            let k = cle_ligne(Liste::Competences, &c);
            if p.competences.len() < COMPETENCES_MAX
                && !p.competences.iter().any(|x| cle_ligne(Liste::Competences, x) == k)
            {
                p.competences.push(c);
            }
        }
    };

    // Les diplômes
    for l in &lignes {
        let dans_formation = match l.rubrique {
            Rubrique::Formation => true,
            Rubrique::Autre | Rubrique::Melange => continue,
            _ => false,
        };
        if let Some(d) = diplome_de(l.texte, dans_formation) {
            ajouter_diplome(d, &mut p);
        }
    }

    // Les métiers : (le nom, les dates)
    // (vrai : lu hors de toute rubrique, dans une phrase de présentation)
    let mut trouves: Vec<(String, Dates, bool)> = Vec::new();
    let permise =
        |r: Rubrique| matches!(r, Rubrique::Experience | Rubrique::Aucune | Rubrique::Competences | Rubrique::Permis);
    // 1. Le poste et sa date sur la même ligne
    for l in lignes.iter_mut().filter(|l| permise(l.rubrique)) {
        for (poste, d) in l.postes.iter().zip(l.dates_des_postes()) {
            if d.vue {
                trouves.push((poste.nom.clone(), d, l.rubrique == Rubrique::Aucune));
                l.prise = true;
            }
        }
    }
    // 2. La date seule sur sa ligne, le poste sur la ligne d'après (ou d'avant)
    for i in 0..lignes.len() {
        let l = &lignes[i];
        let seule = l.jetons.iter().filter(|x| !x.pris).count() <= 6;
        let ici = matches!(l.rubrique, Rubrique::Experience | Rubrique::Aucune);
        if l.prise || !l.dates.vue || !l.postes.is_empty() || !seule || !ici {
            continue;
        }
        let libre = |k: usize| {
            lignes.get(k).is_some_and(|x| {
                x.rubrique == l.rubrique && !x.prise && !x.dates.vue && x.postes.first().is_some_and(|p| p.premier)
            })
        };
        let voisine = if libre(i + 1) {
            Some(i + 1)
        } else if i > 0 && libre(i - 1) {
            Some(i - 1)
        } else {
            None
        };
        if let Some(k) = voisine {
            let nom = lignes[k].postes[0].nom.clone();
            trouves.push((nom, lignes[i].dates.clone(), false));
            lignes[i].prise = true;
            lignes[k].prise = true;
        }
    }
    // 3. Dans la rubrique des expériences, un nom de poste seul sur sa ligne
    for l in lignes.iter_mut().filter(|l| l.rubrique == Rubrique::Experience && !l.prise && !l.dates.vue) {
        if let Some(poste) = l.postes.first()
            && poste.premier
            && poste.fort
            && !l.puce
            && l.jetons.len() <= 8
        {
            trouves.push((poste.nom.clone(), Dates::default(), false));
            l.prise = true;
        }
    }

    // Les compétences : la rubrique, puis les lignes de missions des expériences
    for l in lignes.iter().filter(|l| l.rubrique == Rubrique::Competences && !l.prise) {
        for morceau in l.texte.split([',', ';', '•', '|', '·']).flat_map(|x| x.split(" - ")) {
            let morceau = morceau.rsplit(':').next().unwrap_or(morceau);
            if diplome_de(morceau, false).is_none() {
                ajouter(competence(morceau), &mut p);
            }
        }
    }
    for l in lignes.iter().filter(|l| l.rubrique == Rubrique::Experience && !l.prise && !l.dates.vue) {
        for c in missions(l.texte, l.puce) {
            ajouter(Some(c), &mut p);
        }
    }

    // Un CV aux colonnes mélangées : case par case. Un poste prend la date de sa ligne, sinon celle de la ligne
    // d'avant quand elle n'avait pas de poste ; sans date, il n'est pas gardé.
    let mut en_attente: Option<Dates> = None;
    for l in lignes.iter().filter(|l| l.rubrique == Rubrique::Melange) {
        let j = &l.jetons;
        let mut poste_vu = false;
        let mut s = 0;
        while s < j.len() {
            if j[s].pris || !debut_de_case(l.texte, j, s) {
                s += 1;
                continue;
            }
            if let Some(poste) = poste_a(l.texte, j, s) {
                let d = if l.dates.vue { l.dates.clone() } else { en_attente.take().unwrap_or_default() };
                if d.vue {
                    trouves.push((poste.nom.clone(), d, false));
                }
                poste_vu = true;
                s = poste.fin + 1;
                continue;
            }
            let fin = (s + 1..j.len()).find(|&k| j[k].pris || debut_de_case(l.texte, j, k)).unwrap_or(j.len());
            if diplome_ici(l.texte, j, s) {
                if let Some(d) = diplome_de(&l.texte[j[s].debut..], false) {
                    ajouter_diplome(d, &mut p);
                }
            } else if nom_d_action(&j[s].mot) && fin - s <= 7 {
                ajouter(competence(&l.texte[j[s].debut..j[fin - 1].fin]), &mut p);
            }
            s = fin;
        }
        en_attente = if poste_vu || !l.dates.vue { None } else { Some(l.dates.clone()) };
    }

    // (un métier dit dans une phrase de présentation, « Technicien de maintenance depuis 14 ans », et détaillé dans
    // les expériences : seules les expériences comptent, sinon ses années seraient comptées deux fois)
    let detailles: Vec<String> = trouves.iter().filter(|x| !x.2).map(|x| cle_ligne(Liste::Metiers, &x.0)).collect();
    for (nom, d, presentation) in trouves {
        let cle = cle_ligne(Liste::Metiers, &nom);
        if cle.is_empty() || (presentation && detailles.contains(&cle)) {
            continue;
        }
        match p.metiers.iter().position(|m| cle_ligne(Liste::Metiers, &m.nom) == cle) {
            Some(i) => {
                let m = &mut p.metiers[i];
                if let Some(x) = d.mois {
                    m.mois = Some(m.mois.unwrap_or(0).saturating_add(x));
                }
                if let Some(x) = d.depuis {
                    m.depuis = Some(m.depuis.map_or(x, |y| y.min(x)));
                }
            }
            None if p.metiers.len() < METIERS_MAX => p.metiers.push(Metier { nom, mois: d.mois, depuis: d.depuis }),
            None => {}
        }
    }
    p
}

/// Les lignes de la fiche pour ces droits : les permis, le CACES et toutes ses catégories sur une ligne, puis les
/// habilitations.
fn lignes_de_droits(d: &[Droit]) -> Vec<String> {
    let mut l: Vec<String> = Vec::new();
    for x in d {
        if let Droit::Permis(lettre) = x {
            l.push(format!("Permis {}", lettre.to_uppercase()));
        }
    }
    // (« permis poids lourd » ne dit rien de plus que le permis C ou CE déjà écrit)
    let lourd = d.iter().any(|x| matches!(x, Droit::Permis(l) if ["c", "ce", "ec", "c1", "c1e"].contains(&l.as_str())));
    if d.contains(&Droit::PoidsLourd) && !lourd {
        l.push("Permis poids lourd".into());
    }
    if l.is_empty() && d.contains(&Droit::Conduire) {
        l.push("Permis de conduire".into());
    }
    let mut cats: Vec<String> = Vec::new();
    let mut caces = false;
    for x in d {
        if let Droit::Caces(c) = x {
            caces = true;
            for k in c {
                if !cats.contains(k) {
                    cats.push(k.clone());
                }
            }
        }
    }
    if caces {
        cats.sort_by_key(|c| (c.trim_end_matches(char::is_alphabetic).parse::<u32>().unwrap_or(0), c.clone()));
        let c = cats.join(", ").to_uppercase();
        l.push(if c.is_empty() { "CACES".into() } else { format!("CACES {c}") });
    }
    for x in d {
        if let Droit::Autre(nom) = x {
            l.push(nom.clone());
        }
    }
    l
}

// ---------------------------------------------------------------------------------------------------------------
// La comparaison d'une offre à la fiche
// ---------------------------------------------------------------------------------------------------------------

/// À quel point une offre est proche du parcours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Proximite {
    Loin,
    UnPeu,
    Proche,
    TresProche,
}

impl Proximite {
    pub fn code(self) -> &'static str {
        match self {
            Proximite::Loin => "loin",
            Proximite::UnPeu => "peu",
            Proximite::Proche => "proche",
            Proximite::TresProche => "tres",
        }
    }

    /// Ce que le bandeau de la carte écrit (rien quand l'offre est loin).
    pub fn nom(self) -> &'static str {
        match self {
            Proximite::Loin => "",
            Proximite::UnPeu => "Un peu proche de ton parcours",
            Proximite::Proche => "Proche de ton parcours",
            Proximite::TresProche => "Très proche de ton parcours",
        }
    }
}

/// Une offre rapprochée de la fiche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rapprochement {
    pub niveau: Proximite,
    /// Pourquoi, en une phrase simple (vide quand l'offre est loin)
    pub raison: String,
    /// Pour trier les cartes (les plus proches d'abord) ; jamais montré
    pub rang: u16,
}

/// Le lien entre un métier de la fiche et le poste d'une offre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Lien {
    Rien,
    Voisin,
    Meme,
}

/// La première tête de métier de ces mots : sa place, et si elle est précise.
fn tete_des_mots(m: &[String]) -> Option<(usize, Tete)> {
    m.iter().enumerate().find_map(|(i, x)| tete_de(x).map(|t| (i, t)))
}

/// Les familles ([`GROUPES`]) où ces mots se rangent.
fn groupes_de(m: &[String]) -> Vec<usize> {
    let suite = |motif: &str| {
        let x: Vec<&str> = motif.split(' ').collect();
        m.len() >= x.len() && (0..=m.len() - x.len()).any(|i| x.iter().enumerate().all(|(k, y)| m[i + k] == *y))
    };
    GROUPES.iter().enumerate().filter(|(_, g)| g.iter().any(|p| suite(p))).map(|(i, _)| i).collect()
}

fn domaine_de(m: &[String]) -> Option<&'static str> {
    DOMAINES.iter().find(|(_, l)| m.iter().any(|x| l.contains(&x.as_str()))).map(|(nom, _)| *nom)
}

fn lien(m: &[String], t: &[String]) -> Lien {
    if m.is_empty() || t.is_empty() {
        return Lien::Rien;
    }
    let (gm, gt) = (groupes_de(m), groupes_de(t));
    if gm.iter().any(|g| gt.contains(g)) {
        return Lien::Meme;
    }
    // (chacun dans sa famille, et pas la même : « chauffeur livreur » et « chauffeur poids lourd » sont voisins)
    let familles_differentes = !gm.is_empty() && !gt.is_empty();
    let qualifs = |x: &[String], i: usize| -> Vec<String> {
        x.iter()
            .enumerate()
            .filter(|(k, y)| *k != i && !QUALIFS_VIDES.contains(&y.as_str()) && !y.bytes().all(|b| b.is_ascii_digit()))
            .map(|(_, y)| y.clone())
            .collect()
    };
    let mut voisin = false;
    match (tete_des_mots(m), tete_des_mots(t)) {
        (Some((im, tm)), Some((it, tt))) => {
            if m[im] == t[it] {
                if tm == Tete::Precise && !familles_differentes {
                    return Lien::Meme;
                }
                if tm == Tete::Precise {
                    return Lien::Voisin;
                }
                let (qm, qt) = (qualifs(m, im), qualifs(t, it));
                if qm.is_empty() || qt.is_empty() {
                    voisin = true;
                } else if qm.iter().any(|x| qt.contains(x)) {
                    return Lien::Meme;
                }
            } else if (tm == Tete::Precise && t.contains(&m[im])) || (tt == Tete::Precise && m.contains(&t[it])) {
                return Lien::Meme;
            }
        }
        (Some((im, Tete::Precise)), None) if t.contains(&m[im]) => return Lien::Meme,
        _ => {}
    }
    match (domaine_de(m), domaine_de(t)) {
        (Some(a), Some(b)) if a == b => Lien::Voisin,
        _ if voisin => Lien::Voisin,
        _ => Lien::Rien,
    }
}

/// Les phrases d'une offre, en mots au masculin singulier.
fn phrases_offre(titre: &str, description: &str) -> Vec<Vec<String>> {
    std::iter::once(titre)
        .chain(description.split(['\n', '.', ';', '!', '?', '•']))
        .map(|p| mots(p).iter().map(|m| racine(m)).collect::<Vec<_>>())
        .filter(|p: &Vec<String>| !p.is_empty())
        .collect()
}

/// Le même mot dans une compétence et dans une offre : égaux, ou les mêmes six premières lettres (« encaissement » et
/// « encaissez », « accueil » et « accueillez »).
fn meme_mot(a: &str, b: &str) -> bool {
    a == b || (a.len() >= 6 && b.len() >= 6 && a.is_char_boundary(6) && b.is_char_boundary(6) && a[..6] == b[..6])
}

/// L'offre parle de cette compétence : ses mots (sauf un, à partir de trois) dans une même phrase.
fn competence_demandee(c: &str, phrases: &[Vec<String>]) -> bool {
    // (sans ce qui est entre parenthèses : « Excel (tableaux croisés) » se retrouve par « Excel »)
    let c = sans_parenthese(c);
    let m: Vec<String> = mots_competence(c).into_iter().filter(|x| !MOTS_LARGES.contains(&x.as_str())).collect();
    match m.len() {
        0 => false,
        1 => m[0].len() >= 3 && phrases.iter().any(|p| p.iter().any(|y| meme_mot(&m[0], y))),
        n => {
            let besoin = if n >= 3 { n - 1 } else { n };
            phrases.iter().any(|p| m.iter().filter(|x| p.iter().any(|y| meme_mot(x, y))).count() >= besoin)
        }
    }
}

/// Le nom d'un métier dans une phrase (« cariste », « aide-soignante », « ASH »).
fn nom_dans_phrase(nom: &str) -> String {
    let mut c = nom.chars();
    match (c.next(), c.next()) {
        (Some(a), Some(b)) if b.is_lowercase() || !b.is_alphabetic() => {
            a.to_lowercase().collect::<String>() + &nom[a.len_utf8()..]
        }
        _ => nom.to_string(),
    }
}

/// Une compétence sans ce qu'elle dit entre parenthèses.
fn sans_parenthese(c: &str) -> &str {
    match c.split('(').next().map(str::trim) {
        Some(x) if !x.is_empty() => x,
        _ => c,
    }
}

/// Une compétence dans une phrase : un nom commun en minuscules (« encaissement »), un nom d'outil tel quel
/// (« Excel »).
fn competence_dans_phrase(c: &str) -> String {
    let c = sans_parenthese(c);
    if mots(c).first().is_some_and(|m| nom_d_action(m)) { nom_dans_phrase(c) } else { c.to_string() }
}

/// Compare une offre (son titre, son texte, ce que les règles y ont lu) à la fiche.
pub fn comparer(p: &Parcours, titre: &str, description: &str, lu: &LectureOffre, aujourdhui: Jour) -> Rapprochement {
    let loin = Rapprochement { niveau: Proximite::Loin, raison: String::new(), rang: 0 };
    if !p.utilisable() {
        return loin;
    }
    // Le poste : le meilleur lien avec un métier de la fiche (à égalité, le métier le plus long)
    let t = mots_metier(titre);
    let mut meilleur: Option<(Lien, u16, &Metier)> = None;
    for m in &p.metiers {
        let l = lien(&mots_metier(&m.nom), &t);
        let d = m.duree(aujourdhui).unwrap_or(0);
        if l != Lien::Rien && meilleur.is_none_or(|(l2, d2, _)| (l, d) > (l2, d2)) {
            meilleur = Some((l, d, m));
        }
    }
    let le_lien = meilleur.map_or(Lien::Rien, |x| x.0);
    let duree = meilleur.and_then(|x| x.2.duree(aujourdhui));
    // Les compétences demandées que la personne a
    let phrases = phrases_offre(titre, description);
    let communes: Vec<&str> =
        p.competences.iter().filter(|c| competence_demandee(c, &phrases)).map(String::as_str).collect();
    // Les permis et habilitations demandés
    let les_miens: Vec<Droit> = p
        .permis
        .iter()
        .flat_map(|x| {
            let d = droits(x);
            if d.is_empty() { vec![Droit::Autre(x.clone())] } else { d }
        })
        .collect();
    let mut demandes: Vec<Droit> = lu.autres.iter().flat_map(|x| droits(x)).collect();
    for h in habilitations(&format!("{titre}\n{description}")) {
        if !demandes.contains(&Droit::Autre(h.clone())) {
            demandes.push(Droit::Autre(h));
        }
    }
    let precis = |d: &&Droit| !banal(d);
    let a_moi: Vec<&Droit> = demandes.iter().filter(|d| possede(&les_miens, d)).collect();
    let manque: Option<&Droit> = demandes.iter().filter(precis).find(|d| !possede(&les_miens, d));
    let permis_precis = a_moi.iter().copied().find(precis);
    // Le diplôme demandé : à portée ou non
    let hors_de_portee = lu.diplome.as_ref().map(|d| d.valeur).filter(|d| *d != Diplome::Aucun).filter(|d| {
        match p.diplome_le_plus_haut() {
            Some(h) => *d > h,
            None => *d >= Diplome::Bac2,
        }
    });
    // L'expérience demandée
    let trop_d_experience = match (lu.experience.as_ref().map(|e| e.valeur), le_lien) {
        (Some(Experience::Duree(x)), Lien::Meme) => duree.is_some_and(|d| u32::from(d) * 2 < u32::from(x)).then_some(x),
        (Some(Experience::Duree(x)), _) if x >= 24 => Some(x),
        (Some(Experience::Demandee), Lien::Voisin | Lien::Rien) => Some(0),
        _ => None,
    };

    let fort = communes.len() >= 2 || permis_precis.is_some();
    // (une offre ouverte aux débutants, qui demande une chose que la personne sait faire : un peu proche)
    let debutant = lu.experience.as_ref().is_some_and(|e| e.valeur == Experience::Debutant);
    let mut n: i32 = match le_lien {
        Lien::Meme if duree.is_some_and(|d| d >= 12) || fort => 3,
        Lien::Meme => 2,
        Lien::Voisin if fort => 2,
        Lien::Voisin => 1,
        Lien::Rien if fort || (debutant && !communes.is_empty()) => 1,
        Lien::Rien => 0,
    };
    if hors_de_portee.is_some() {
        n -= 2;
    }
    if manque.is_some() {
        n -= 1;
    }
    if trop_d_experience.is_some() {
        n -= 1;
    }
    // (un métier déjà fait reste au moins un peu proche, quoi que l'offre demande en plus ; un métier voisin aussi,
    // sauf si le diplôme demandé est hors de portée)
    let plancher = match le_lien {
        Lien::Meme => 1,
        Lien::Voisin if hors_de_portee.is_none() => 1,
        _ => 0,
    };
    let niveau = match n.max(plancher) {
        i32::MIN..=0 => Proximite::Loin,
        1 => Proximite::UnPeu,
        2 => Proximite::Proche,
        _ => Proximite::TresProche,
    };
    if niveau == Proximite::Loin {
        return loin;
    }

    // La raison, en une phrase
    let liste = |l: &[&str]| l.iter().take(2).map(|c| competence_dans_phrase(c)).collect::<Vec<_>>().join(", ");
    let mut raison = match (le_lien, meilleur) {
        (Lien::Meme, Some((_, _, m))) => {
            let nom = nom_dans_phrase(&m.nom);
            match (duree, m.depuis.is_some()) {
                (Some(d), true) => format!("Tu es {nom} depuis {}", duree_en_mots(d)),
                (Some(d), false) => format!("Tu as été {nom} {}", duree_en_mots(d)),
                (None, _) => format!("Tu as déjà été {nom}"),
            }
        }
        (Lien::Voisin, Some((_, _, m))) => {
            format!("Ce poste ressemble à un métier que tu as fait ({})", nom_dans_phrase(&m.nom))
        }
        _ if !communes.is_empty() => {
            format!(
                "Tu n'as pas fait ce métier, mais l'offre demande des choses que tu sais faire : {}",
                liste(&communes)
            )
        }
        _ => "Tu n'as pas fait ce métier".to_string(),
    };
    let dit_les_competences = le_lien == Lien::Rien && !communes.is_empty();
    if let Some(d) = permis_precis.or(a_moi.first().copied()) {
        let accord =
            if nom_droit(d).starts_with("la ") || nom_droit(d).starts_with("l'") { "demandée" } else { "demandé" };
        let lie = if le_lien == Lien::Rien && !dit_les_competences { ", mais" } else { ", et" };
        raison.push_str(&format!("{lie} tu as {} {accord}", nom_droit(d)));
    } else if !communes.is_empty() && !dit_les_competences {
        raison.push_str(&format!(", et l'offre demande des choses que tu sais faire : {}", liste(&communes)));
    }
    if let Some(d) = hors_de_portee {
        raison.push_str(&format!(", mais l'offre {}", d.demande()));
    } else if let Some(d) = manque {
        raison.push_str(&format!(", mais il te manque {}", nom_droit(d)));
    } else if let Some(x) = trop_d_experience {
        raison.push_str(&if x > 0 {
            format!(", mais l'offre demande {} d'expérience", lecture::duree_texte(x))
        } else {
            ", mais l'offre demande de l'expérience dans ce métier".to_string()
        });
    }
    raison.push('.');
    let rang = (n.max(plancher).clamp(0, 3) as u16) * 100
        + duree.map_or(0, |d| (d / 12).min(20))
        + (communes.len().min(5) as u16) * 5
        + if permis_precis.is_some() { 10 } else { 0 };
    Rapprochement { niveau, raison, rang }
}

/// L'empreinte d'un texte de CV et des règles : une autre empreinte, et la fiche est refaite.
pub fn empreinte(cv: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in VERSION.to_le_bytes().into_iter().chain(cv.bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests;
