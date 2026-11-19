//! La candidature préparée par Claude (étape 8 du plan) : ce que Jobmind vérifie dans ce que Claude rend, sans écran,
//! sans fichier et sans réseau.
//!
//! Rien d'inventé : Claude rend la lettre et les réponses phrase par phrase ; pour chaque phrase qui dit quelque chose
//! de Jordan (son expérience, ses diplômes, ses compétences, sa disponibilité...), il recopie les passages du CV ou de
//! « Ce que je cherche » qui la soutiennent. [`verifier_phrase`] cherche chaque passage, mot pour mot, dans ces textes
//! ([`crate::lecture::citation_presente`]) ; une phrase sans passage trouvé est « à vérifier » (surlignée à l'écran,
//! Jordan la garde ou la retire). En plus, sans rien attendre de Claude :
//! - une phrase qui parle de Jordan (« j'ai travaillé », « titulaire », « mon expérience »...) alors que Claude dit le
//!   contraire est à vérifier aussi ;
//! - chaque nombre d'une phrase (« 3 ans », « deux ans », « 2019 ») doit se trouver dans le CV, le profil ou l'offre.
//!
//! Les conseils pour le CV ne gardent que ce qui s'appuie sur le CV (ou, pour « l'offre demande ce que ton CV ne dit
//! pas », sur l'offre) ; une question de l'offre n'est gardée que si elle est vraiment dans l'offre.
//!
//! La lettre que Jordan modifie à la main, la consigne d'une nouvelle version : vérifiées ici ([`verifier_lettre`],
//! [`verifier_consigne`]).

use crate::lecture::citation_presente;
use crate::offre::normaliser;

/// La consigne d'une nouvelle version (« plus court »), au plus (en caractères).
pub const CONSIGNE_MAX: usize = 120;
/// La lettre modifiée par Jordan, au plus (en caractères).
pub const LETTRE_MAX: usize = 8000;
/// Une phrase rendue par Claude, au plus (en caractères ; coupée au-delà).
pub const PHRASE_MAX: usize = 700;
/// Les passages cités pour une phrase, au plus.
pub const EXTRAITS_MAX: usize = 6;

/// Une phrase telle que Claude la rend : le texte, s'il parle de Jordan, et les passages cités.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhraseRendue {
    pub texte: String,
    pub sur_moi: bool,
    pub extraits: Vec<String>,
}

/// Pourquoi une phrase est à vérifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Raison {
    /// Elle parle de Jordan, sans aucun passage cité
    SansExtrait,
    /// Les passages cités ne sont ni dans le CV ni dans le profil
    ExtraitAbsent,
    /// Ce nombre n'est ni dans le CV, ni dans le profil, ni dans l'offre
    NombreAbsent(String),
    /// Claude dit qu'elle ne parle pas de Jordan, mais elle en parle
    FaitNonDeclare,
    /// Elle parle de Jordan avec ce mot, qui n'est ni dans son CV, ni dans son profil, ni dans l'offre
    MotAbsent(String),
    /// Étape 9 : la réécriture ajoute ce nom (une entreprise, un logiciel, un lieu), absent du CV, de l'offre et de la
    /// lettre de départ
    NomAbsent(String),
}

impl Raison {
    /// Un code court (la base, le journal).
    pub fn code(&self) -> &'static str {
        match self {
            Raison::SansExtrait => "sans_extrait",
            Raison::ExtraitAbsent => "extrait_absent",
            Raison::NombreAbsent(_) => "nombre_absent",
            Raison::FaitNonDeclare => "fait_non_declare",
            Raison::MotAbsent(_) => "mot_absent",
            Raison::NomAbsent(_) => "nom_absent",
        }
    }

    /// Ce que la phrase ajoute, en quelques mots (« le nombre « quatre » », « « Excel » »), pour dire pourquoi une
    /// phrase réécrite est remise comme avant.
    pub fn ajout(&self) -> String {
        match self {
            Raison::NombreAbsent(n) => format!("le nombre « {n} »"),
            Raison::MotAbsent(m) | Raison::NomAbsent(m) => format!("« {m} »"),
            Raison::SansExtrait | Raison::ExtraitAbsent | Raison::FaitNonDeclare => {
                "un fait que ton CV ne dit pas".into()
            }
        }
    }

    /// La phrase pour Jordan, en mots simples.
    pub fn texte(&self) -> String {
        match self {
            Raison::SansExtrait => "Cette phrase parle de toi, mais rien dans ton CV ne la soutient.".into(),
            Raison::ExtraitAbsent => "Le passage que Claude cite n'est pas dans ton CV.".into(),
            Raison::NombreAbsent(n) => format!("Le nombre « {n} » n'est ni dans ton CV ni dans l'offre."),
            Raison::FaitNonDeclare => "Cette phrase parle de toi sans s'appuyer sur ton CV.".into(),
            Raison::MotAbsent(m) => format!("Cette phrase parle de toi avec « {m} », que ton CV ne dit pas."),
            Raison::NomAbsent(m) => format!("« {m} » n'est ni dans ton CV, ni dans l'offre, ni dans ta lettre."),
        }
    }
}

/// Une phrase vérifiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phrase {
    pub texte: String,
    /// Les passages trouvés dans le CV ou le profil
    pub appuis: Vec<String>,
    /// Rien : la phrase est sûre ; sinon, pourquoi elle est à vérifier
    pub a_verifier: Option<Raison>,
}

/// Les sources de la vérification : ce que Jordan a donné (le CV, « Ce que je cherche ») et l'offre.
#[derive(Debug, Clone, Copy)]
pub struct Sources<'a> {
    pub profil: &'a str,
    pub offre: &'a str,
}

/// Les mots qui disent qu'une phrase parle du parcours de Jordan (sur le texte normalisé : sans accents ni majuscules).
const MARQUES: [&str; 33] = [
    "j ai travaille",
    "j ai ete",
    "j ai occupe",
    "j ai exerce",
    "j ai acquis",
    "j ai appris",
    "j ai suivi",
    "j ai obtenu",
    "j ai gere",
    "j ai effectue",
    "j ai participe",
    "j ai deja",
    "j ai encadre",
    "j ai conduit",
    "j ai manage",
    "j ai realise",
    "j ai developpe",
    "titulaire",
    "diplome",
    "diplomee",
    "mon experience",
    "mes experiences",
    "une experience de",
    "ans d experience",
    "annees d experience",
    "caces",
    "permis",
    "ma formation",
    "je maitrise",
    "je parle couramment",
    "certifie",
    "certification",
    "competences en",
];

/// Les nombres écrits en lettres que la vérification reconnaît (ceux qu'une lettre emploie pour une durée).
const NOMBRES_LETTRES: [(&str, &str); 14] = [
    ("deux", "2"),
    ("trois", "3"),
    ("quatre", "4"),
    ("cinq", "5"),
    ("six", "6"),
    ("sept", "7"),
    ("huit", "8"),
    ("neuf", "9"),
    ("dix", "10"),
    ("onze", "11"),
    ("douze", "12"),
    ("quinze", "15"),
    ("vingt", "20"),
    ("trente", "30"),
];

/// La phrase parle du parcours de Jordan (une des [`MARQUES`]).
pub fn parle_de_moi(texte: &str) -> bool {
    let n = format!(" {} ", espaces(&normaliser(texte)));
    MARQUES.iter().any(|m| n.contains(&format!(" {m} ")) || n.contains(&format!(" {m}")))
}

/// Un texte normalisé, ses espaces resserrées.
fn espaces(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Les nombres d'un texte : les suites de chiffres, et les nombres écrits en lettres (rendus en chiffres et en lettres).
fn nombres(t: &str) -> Vec<(String, String)> {
    let n = normaliser(t);
    let mut l: Vec<(String, String)> = Vec::new();
    for mot in n.split_whitespace() {
        if mot.chars().all(|c| c.is_ascii_digit()) {
            let x = mot.trim_start_matches('0');
            let x = if x.is_empty() { "0" } else { x };
            if !l.iter().any(|(a, _)| a == x) {
                l.push((x.to_string(), mot.to_string()));
            }
        } else if let Some((_, chiffre)) = NOMBRES_LETTRES.iter().find(|(lettres, _)| *lettres == mot)
            && !l.iter().any(|(a, _)| a == chiffre)
        {
            l.push(((*chiffre).to_string(), mot.to_string()));
        }
    }
    l
}

/// Le premier nombre de la phrase qui n'est dans aucune source (ni en chiffres ni en lettres).
fn nombre_absent(texte: &str, s: Sources) -> Option<String> {
    let connus: Vec<String> = nombres(&format!("{}\n{}", s.profil, s.offre)).into_iter().map(|(n, _)| n).collect();
    nombres(texte).into_iter().find(|(n, _)| !connus.contains(n)).map(|(_, mot)| mot)
}

/// Les mots de toute lettre, qui ne disent rien de précis sur Jordan (sur le texte normalisé, les 5 premières lettres
/// suffisent : « travaill » couvre travaillé, travaillais, travailler).
const MOTS_DE_LETTRE: &[&str] = &[
    "avec",
    "dans",
    "pour",
    "sans",
    "sous",
    "chez",
    "vers",
    "mais",
    "donc",
    "comme",
    "aussi",
    "tres",
    "bien",
    "plus",
    "moins",
    "tout",
    "toute",
    "tous",
    "toutes",
    "deja",
    "encore",
    "ainsi",
    "alors",
    "cette",
    "celui",
    "celle",
    "ceux",
    "leur",
    "leurs",
    "votre",
    "vous",
    "notre",
    "nous",
    "elle",
    "elles",
    "etre",
    "avoir",
    "suis",
    "sommes",
    "etes",
    "sont",
    "etait",
    "etais",
    "avais",
    "avait",
    "aurai",
    "serai",
    "serais",
    "serait",
    "peux",
    "peut",
    "pouvoir",
    "sais",
    "savoir",
    "saurai",
    "faire",
    "fait",
    "fais",
    "cela",
    "ceci",
    "quand",
    "depuis",
    "pendant",
    "avant",
    "apres",
    "entre",
    "chaque",
    "autre",
    "autres",
    "meme",
    "quelque",
    "plusieurs",
    "beaucoup",
    "souvent",
    "toujours",
    "jamais",
    "aujourd",
    "candidature",
    "poste",
    "offre",
    "emploi",
    "entreprise",
    "equipe",
    "travail",
    "metier",
    "experience",
    "parcours",
    "entretien",
    "rencontrer",
    "heureux",
    "heureuse",
    "envie",
    "souhaite",
    "interesse",
    "attention",
    "motivation",
    "occasion",
    "permettre",
    "rejoindre",
    "integrer",
    "convient",
    "correspond",
    "retenu",
    "propose",
    "actuellement",
    "recemment",
    "precedent",
    "dernier",
    "premier",
    "nouveau",
    "nouvelle",
    "mettre",
    "apporter",
    "contribuer",
    "presenter",
    "habitude",
    "occupe",
    "exerce",
    "acquis",
    "appris",
    "titulaire",
    "partir",
    "ensemble",
    "egalement",
    "demande",
    "chercher",
    "voulez",
    "aimerais",
    "plait",
    "volontiers",
    "rapidement",
    "maintenant",
    "merci",
    "remercie",
    "salutations",
    "agreer",
    "madame",
    "monsieur",
    "lettre",
    "reponse",
    "retour",
    "suite",
    "puisque",
    "connais",
    "connaitre",
    "assure",
    "assurer",
    "gere",
    "gerer",
    "obtenu",
    "tombe",
    "type",
    "bases",
    "forme",
    "font",
    "annee",
    "exigence",
    "puis",
    "semaine",
    "heure",
    "jour",
    "fois",
    "profil",
    "besoin",
    "partie",
    "utile",
    "possible",
    "pourvoir",
    "parfaitement",
];

/// Les 5 premières lettres d'un mot normalisé.
fn racine(m: &str) -> String {
    m.chars().take(5).collect()
}

/// Le premier mot de la phrase (de 4 lettres au moins, hors [`MOTS_DE_LETTRE`]) que ni le CV ni le profil ne
/// contiennent. L'offre ne compte pas : une phrase sur Jordan qui reprend les mots de l'offre (« la liaison froide »,
/// « autolaveuse ») sans qu'ils soient dans son CV est justement ce qu'il faut lui faire vérifier.
fn mot_absent(texte: &str, s: Sources) -> Option<String> {
    let connus: std::collections::HashSet<String> = normaliser(s.profil).split_whitespace().map(racine).collect();
    let communs: std::collections::HashSet<String> = MOTS_DE_LETTRE.iter().map(|m| racine(m)).collect();
    normaliser(texte)
        .split_whitespace()
        .filter(|m| m.chars().count() >= 4 && !m.chars().all(|c| c.is_ascii_digit()))
        // (les nombres en lettres : la vérification des nombres s'en charge)
        .filter(|m| !NOMBRES_LETTRES.iter().any(|(l, _)| l == m))
        .find(|m| !connus.contains(&racine(m)) && !communs.contains(&racine(m)))
        .map(str::to_string)
}

/// Remet une phrase de Claude au propre : sans caractères de contrôle, les espaces resserrées, sans tiret long, coupée
/// à [`PHRASE_MAX`].
pub fn propre(t: &str, max: usize) -> String {
    let t: String = t.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let t = sans_tiret_long(&espaces(&t));
    if t.chars().count() <= max { t } else { t.chars().take(max).collect::<String>().trim().to_string() }
}

/// Les tirets longs (cadratin et demi-cadratin) remplacés comme le fait le plancher du style (jobmind-humanizer,
/// `plancher::corriger`) : entre deux espaces, une virgule ; collé, un trait d'union.
fn sans_tiret_long(t: &str) -> String {
    if !t.contains(['\u{2014}', '\u{2013}']) {
        return t.to_string();
    }
    let c: Vec<char> = t.chars().collect();
    let mut s = String::with_capacity(t.len());
    for (i, &x) in c.iter().enumerate() {
        if x == '\u{2014}' || x == '\u{2013}' {
            if s.ends_with(' ') && c.get(i + 1) == Some(&' ') {
                s.pop();
                s.push(',');
            } else {
                s.push('-');
            }
        } else {
            s.push(x);
        }
    }
    // (« , , » laissé par deux tirets, ou une virgule en tête)
    let s = s.replace(" ,", ",").replace(",,", ",");
    espaces(s.trim_start_matches([',', ' ']))
}

/// Vérifie une phrase rendue par Claude (rien : la phrase est vide).
pub fn verifier_phrase(p: &PhraseRendue, s: Sources) -> Option<Phrase> {
    let texte = propre(&p.texte, PHRASE_MAX);
    if texte.is_empty() {
        return None;
    }
    let cites: Vec<String> =
        p.extraits.iter().take(EXTRAITS_MAX).map(|e| propre(e, PHRASE_MAX)).filter(|e| !e.is_empty()).collect();
    let appuis: Vec<String> = cites.iter().filter(|e| citation_presente(s.profil, e)).cloned().collect();
    let mut a_verifier = if p.sur_moi {
        match (appuis.is_empty(), cites.is_empty()) {
            (false, _) => None,
            (true, true) => Some(Raison::SansExtrait),
            (true, false) => Some(Raison::ExtraitAbsent),
        }
    } else if parle_de_moi(&texte) && appuis.is_empty() {
        Some(Raison::FaitNonDeclare)
    } else {
        None
    };
    if a_verifier.is_none()
        && let Some(n) = nombre_absent(&texte, s)
    {
        a_verifier = Some(Raison::NombreAbsent(n));
    }
    // (une phrase sur Jordan n'emploie que des mots de son CV, de son profil ou de l'offre, et ceux d'une lettre)
    if a_verifier.is_none()
        && (p.sur_moi || parle_de_moi(&texte))
        && let Some(m) = mot_absent(&texte, s)
    {
        a_verifier = Some(Raison::MotAbsent(m));
    }
    Some(Phrase { texte, appuis, a_verifier })
}

/// Vérifie une suite de phrases (les vides sont laissées).
pub fn verifier_phrases(l: &[PhraseRendue], s: Sources) -> Vec<Phrase> {
    l.iter().filter_map(|p| verifier_phrase(p, s)).collect()
}

/// Étape 9 : les sources d'une phrase réécrite dans le style de Jordan. La réécriture ne fait que reformuler : tout ce
/// qu'elle dit doit déjà être dans le CV et le profil (`profil`), l'offre, ou la lettre de départ (`depart`) ; les
/// textes de Jordan (`vocabulaire`) ne donnent que des mots, jamais des faits.
#[derive(Debug, Clone, Copy)]
pub struct SourcesReecriture<'a> {
    pub profil: &'a str,
    pub offre: &'a str,
    pub depart: &'a str,
    pub vocabulaire: &'a str,
}

/// Les mots qu'une réécriture emploie souvent pour dire autrement la fin d'une lettre ou l'intérêt pour le poste, sans
/// rien dire de nouveau sur Jordan (en plus de ceux de toute lettre).
const MOTS_DE_REECRITURE: [&str; 26] = [
    "parler",
    "echanger",
    "discuter",
    "disponible",
    "disponibilite",
    "disposition",
    "attente",
    "rencontre",
    "interesse",
    "plait",
    "attire",
    "motive",
    "envoie",
    "adresse",
    "joindre",
    "contacter",
    "rappeler",
    "postuler",
    "voudrais",
    "souhaiterais",
    "pourrais",
    "convenir",
    "volontiers",
    "ravi",
    "content",
    "pense",
];

/// Le premier nom propre de la phrase (un mot qui commence par une majuscule, ailleurs qu'en tête de phrase) qui n'est
/// dans aucun de ces textes : une entreprise, un logiciel, un lieu ajoutés.
fn nom_absent(texte: &str, connus: &str) -> Option<String> {
    let connus: std::collections::HashSet<String> = normaliser(connus).split_whitespace().map(str::to_string).collect();
    let mut debut = true;
    for brut in texte.split_whitespace() {
        let fin_de_phrase = brut.ends_with(['.', '!', '?', ':']);
        let mot = brut.rsplit(['\'', '\u{2019}']).next().unwrap_or(brut);
        let mot: String = mot.trim_matches(|c: char| !c.is_alphanumeric()).to_string();
        let majuscule = mot.chars().next().is_some_and(char::is_uppercase);
        if !debut && majuscule && mot.chars().count() >= 2 {
            let n = normaliser(&mot);
            let n = espaces(&n);
            if !n.is_empty()
                && !["madame", "monsieur", "je"].contains(&n.as_str())
                && !n.split(' ').all(|x| connus.contains(x))
            {
                return Some(mot);
            }
        }
        debut = fin_de_phrase;
    }
    None
}

/// Étape 9 : vérifie une phrase réécrite. Comme [`verifier_phrase`] (les passages cités cherchés dans le CV et le
/// profil, une phrase sur Jordan sans appui), mais : un nombre doit être dans le CV, le profil, l'offre ou la lettre de
/// départ ; un nom propre aussi ; et une phrase sur Jordan n'emploie que des mots de son CV, de son profil, de la lettre
/// de départ, de ses propres textes ou d'une lettre. Rien : la phrase est vide.
pub fn verifier_reecrite(p: &PhraseRendue, s: SourcesReecriture) -> Option<Phrase> {
    let texte = propre(&p.texte, PHRASE_MAX);
    if texte.is_empty() {
        return None;
    }
    let cites: Vec<String> =
        p.extraits.iter().take(EXTRAITS_MAX).map(|e| propre(e, PHRASE_MAX)).filter(|e| !e.is_empty()).collect();
    let appuis: Vec<String> = cites.iter().filter(|e| citation_presente(s.profil, e)).cloned().collect();
    let sur_moi = p.sur_moi || parle_de_moi(&texte);
    let mut a_verifier = match (sur_moi, appuis.is_empty(), cites.is_empty()) {
        (false, _, _) | (true, false, _) => None,
        (true, true, true) if p.sur_moi => Some(Raison::SansExtrait),
        (true, true, true) => Some(Raison::FaitNonDeclare),
        (true, true, false) => Some(Raison::ExtraitAbsent),
    };
    let tout = format!("{}\n{}\n{}", s.profil, s.offre, s.depart);
    if a_verifier.is_none()
        && let Some(n) = nombre_absent(&texte, Sources { profil: &tout, offre: "" })
    {
        a_verifier = Some(Raison::NombreAbsent(n));
    }
    if a_verifier.is_none()
        && let Some(n) = nom_absent(&texte, &tout)
    {
        a_verifier = Some(Raison::NomAbsent(n));
    }
    // (une phrase à la première personne aussi : « J'ai dirigé un entrepôt » n'a pas besoin d'être déclarée)
    let n = format!(" {} ", espaces(&normaliser(&texte)));
    let premiere_personne = [" je ", " j ", " mon ", " ma ", " mes "].iter().any(|m| n.contains(m));
    if a_verifier.is_none() && (sur_moi || premiere_personne) {
        let connus = format!("{}\n{}\n{}\n{}", s.profil, s.depart, s.vocabulaire, MOTS_DE_REECRITURE.join(" "));
        if let Some(m) = mot_absent(&texte, Sources { profil: &connus, offre: "" }) {
            a_verifier = Some(Raison::MotAbsent(m));
        }
    }
    Some(Phrase { texte, appuis, a_verifier })
}

/// Une question de l'offre et sa réponse, telles que Claude les rend.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestionRendue {
    pub question: String,
    /// Le passage de l'offre qui pose la question
    pub extrait_offre: String,
    pub reponse: Vec<PhraseRendue>,
}

/// Une question gardée (elle est bien dans l'offre) et sa réponse vérifiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub question: String,
    pub reponse: Vec<Phrase>,
}

impl Question {
    /// La réponse en un texte.
    pub fn texte(&self) -> String {
        self.reponse.iter().map(|p| p.texte.as_str()).collect::<Vec<_>>().join(" ")
    }
}

/// Les questions vérifiées : seulement celles qui sont vraiment dans l'offre (une question inventée est laissée), au
/// plus 8. Rend aussi le nombre de questions laissées.
pub fn verifier_questions(l: &[QuestionRendue], s: Sources) -> (Vec<Question>, usize) {
    let mut gardees = Vec::new();
    let mut laissees = 0;
    for q in l {
        let question = propre(&q.question, 300);
        let extrait = propre(&q.extrait_offre, 400);
        let reponse = verifier_phrases(&q.reponse, s);
        if question.is_empty() || reponse.is_empty() || !citation_presente(s.offre, &extrait) || gardees.len() >= 8 {
            laissees += 1;
            continue;
        }
        gardees.push(Question { question, reponse });
    }
    (gardees, laissees)
}

/// Le genre d'un conseil pour le CV.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenreConseil {
    /// Ce que le CV dit déjà et qu'il faut mettre en avant pour cette offre
    MettreEnAvant,
    /// Ce que le CV dit déjà, à dire avec les mots de l'offre
    Reformuler,
    /// Ce que l'offre demande et que le CV ne dit pas (à ajouter seulement si c'est vrai)
    Absent,
}

impl GenreConseil {
    pub fn code(self) -> &'static str {
        match self {
            GenreConseil::MettreEnAvant => "mettre_en_avant",
            GenreConseil::Reformuler => "reformuler",
            GenreConseil::Absent => "absent",
        }
    }

    pub fn depuis_code(c: &str) -> Option<GenreConseil> {
        [GenreConseil::MettreEnAvant, GenreConseil::Reformuler, GenreConseil::Absent]
            .into_iter()
            .find(|g| g.code() == c)
    }

    /// Le petit titre à l'écran.
    pub fn titre(self) -> &'static str {
        match self {
            GenreConseil::MettreEnAvant => "À mettre en avant",
            GenreConseil::Reformuler => "À dire avec les mots de l'offre",
            GenreConseil::Absent => "L'offre le demande, ton CV n'en parle pas",
        }
    }
}

/// Un conseil pour le CV tel que Claude le rend.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConseilRendu {
    pub genre: String,
    pub texte: String,
    pub extrait_cv: String,
    pub extrait_offre: String,
}

/// Un conseil gardé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conseil {
    pub genre: GenreConseil,
    pub texte: String,
    /// Le passage du CV (vide pour « absent »)
    pub extrait_cv: String,
    /// Le passage de l'offre (peut être vide pour « mettre en avant »)
    pub extrait_offre: String,
}

/// La phrase ajoutée à un conseil « absent » : rien ne s'ajoute au CV s'il n'est pas vrai.
pub const SEULEMENT_SI_VRAI: &str = "Ajoute-le seulement si c'est vrai.";

/// Les conseils vérifiés : « mettre en avant » et « reformuler » s'appuient sur un passage du CV (cherché dans le CV
/// seulement) ; « absent » sur un passage de l'offre ; un passage de l'offre donné est cherché dans l'offre. Au plus
/// 6 ; rend aussi le nombre de conseils laissés.
pub fn verifier_conseils(l: &[ConseilRendu], cv: &str, offre: &str) -> (Vec<Conseil>, usize) {
    let mut gardes = Vec::new();
    let mut laisses = 0;
    for c in l {
        let texte = propre(&c.texte, 500);
        let extrait_cv = propre(&c.extrait_cv, 400);
        let extrait_offre = propre(&c.extrait_offre, 400);
        let ok = match GenreConseil::depuis_code(c.genre.trim()) {
            Some(g @ (GenreConseil::MettreEnAvant | GenreConseil::Reformuler)) => citation_presente(cv, &extrait_cv)
                .then_some(g)
                .filter(|_| extrait_offre.is_empty() || citation_presente(offre, &extrait_offre)),
            Some(GenreConseil::Absent) => citation_presente(offre, &extrait_offre).then_some(GenreConseil::Absent),
            None => None,
        };
        match ok {
            Some(genre) if !texte.is_empty() && gardes.len() < 6 => {
                let extrait_cv = if genre == GenreConseil::Absent { String::new() } else { extrait_cv };
                let texte = if genre == GenreConseil::Absent && !texte.contains("seulement si") {
                    format!("{texte} {SEULEMENT_SI_VRAI}")
                } else {
                    texte
                };
                gardes.push(Conseil { genre, texte, extrait_cv, extrait_offre });
            }
            _ => laisses += 1,
        }
    }
    (gardes, laisses)
}

/// La lettre, rendue par Claude puis vérifiée.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lettre {
    /// « Madame, Monsieur, »
    pub appel: String,
    pub paragraphes: Vec<Vec<Phrase>>,
    /// La formule de politesse
    pub politesse: String,
    /// Le nom de Jordan, seulement s'il est dans son CV
    pub signature: String,
}

impl Lettre {
    /// Le texte entier de la lettre : l'appel, les paragraphes (une ligne vide entre deux), la politesse, la signature.
    pub fn texte(&self) -> String {
        let mut blocs: Vec<String> = Vec::new();
        if !self.appel.is_empty() {
            blocs.push(self.appel.clone());
        }
        for p in &self.paragraphes {
            let t = p.iter().map(|x| x.texte.as_str()).collect::<Vec<_>>().join(" ");
            if !t.is_empty() {
                blocs.push(t);
            }
        }
        let mut fin = self.politesse.clone();
        if !self.signature.is_empty() {
            if !fin.is_empty() {
                fin.push('\n');
            }
            fin.push_str(&self.signature);
        }
        if !fin.is_empty() {
            blocs.push(fin);
        }
        blocs.join("\n\n")
    }

    /// Les phrases à vérifier, dans l'ordre de la lettre.
    pub fn a_verifier(&self) -> Vec<&Phrase> {
        self.paragraphes.iter().flatten().filter(|p| p.a_verifier.is_some()).collect()
    }

    /// Le nombre de phrases.
    pub fn phrases(&self) -> usize {
        self.paragraphes.iter().map(Vec::len).sum()
    }
}

/// La signature n'est gardée que si elle est dans le CV (le nom de Jordan) ; sinon elle reste vide.
pub fn verifier_signature(signature: &str, cv: &str) -> String {
    let s = propre(signature, 80);
    if citation_presente(cv, &s) { s } else { String::new() }
}

/// Un morceau de texte à montrer : surligné « à vérifier » ou non.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Morceau {
    pub texte: String,
    /// Le numéro de la phrase à vérifier (dans la liste des phrases à vérifier), s'il l'est
    pub a_verifier: Option<usize>,
}

/// Découpe un texte en morceaux : chaque phrase à vérifier qui s'y trouve encore telle quelle est surlignée (une
/// phrase que Jordan a changée n'est plus surlignée : il l'a relue).
pub fn morceaux(texte: &str, a_verifier: &[String]) -> Vec<Morceau> {
    // Les places de chaque phrase à vérifier (la première fois qu'elle paraît), sans chevauchement
    let mut places: Vec<(usize, usize, usize)> = Vec::new();
    for (i, p) in a_verifier.iter().enumerate() {
        if p.is_empty() {
            continue;
        }
        let mut depart = 0;
        while let Some(x) = texte[depart..].find(p.as_str()) {
            let debut = depart + x;
            let fin = debut + p.len();
            if places.iter().all(|(d, f, _)| fin <= *d || debut >= *f) {
                places.push((debut, fin, i));
                break;
            }
            depart = fin;
        }
    }
    places.sort_unstable();
    let mut l = Vec::new();
    let mut ici = 0;
    for (d, f, i) in places {
        if d > ici {
            l.push(Morceau { texte: texte[ici..d].to_string(), a_verifier: None });
        }
        l.push(Morceau { texte: texte[d..f].to_string(), a_verifier: Some(i) });
        ici = f;
    }
    if ici < texte.len() || l.is_empty() {
        l.push(Morceau { texte: texte[ici..].to_string(), a_verifier: None });
    }
    l
}

/// Retire une phrase de la lettre (la première fois qu'elle paraît), avec l'espace qui la suivait ou la précédait ;
/// un paragraphe vidé disparaît. Rien : la phrase n'y est plus.
pub fn retirer_phrase(lettre: &str, phrase: &str) -> Option<String> {
    if phrase.is_empty() {
        return None;
    }
    let debut = lettre.find(phrase)?;
    let fin = debut + phrase.len();
    let (mut d, mut f) = (debut, fin);
    if lettre[f..].starts_with(' ') {
        f += 1;
    } else if d > 0 && lettre[..d].ends_with(' ') {
        d -= 1;
    }
    let t = format!("{}{}", &lettre[..d], &lettre[f..]);
    // (un paragraphe vidé : ses lignes vides resserrées)
    let mut propre = String::with_capacity(t.len());
    let mut vides = 0;
    for (i, ligne) in t.split('\n').enumerate() {
        let l = ligne.trim_end();
        if l.trim().is_empty() {
            vides += 1;
            if vides > 1 {
                continue;
            }
        } else {
            vides = 0;
        }
        if i > 0 {
            propre.push('\n');
        }
        propre.push_str(l);
    }
    Some(propre.trim().to_string())
}

/// Vérifie la lettre que Jordan a modifiée : sans caractères de contrôle (sauf les retours à la ligne), au plus
/// [`LETTRE_MAX`] caractères, pas vide.
pub fn verifier_lettre(t: &str) -> Result<String, String> {
    let t: String = t
        .replace("\r\n", "\n")
        .chars()
        .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
        .map(|c| if c == '\t' { ' ' } else { c })
        .collect();
    let t = t.trim().to_string();
    if t.is_empty() {
        return Err("La lettre est vide. Écris-la, ou garde la version de Claude.".into());
    }
    if t.chars().count() > LETTRE_MAX {
        return Err(format!("La lettre est trop longue : {LETTRE_MAX} caractères au plus."));
    }
    Ok(t)
}

/// Vérifie la consigne d'une nouvelle version (« plus court ») : facultative, une ligne, au plus [`CONSIGNE_MAX`]
/// caractères.
pub fn verifier_consigne(t: &str) -> Result<Option<String>, String> {
    let t = propre(t, usize::MAX);
    if t.is_empty() {
        return Ok(None);
    }
    if t.chars().count() > CONSIGNE_MAX {
        return Err(format!(
            "La consigne est trop longue : {CONSIGNE_MAX} caractères au plus, par exemple « plus court »."
        ));
    }
    Ok(Some(t))
}

#[cfg(test)]
mod tests;
