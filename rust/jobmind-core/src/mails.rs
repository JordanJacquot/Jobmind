//! Les mails de la boîte Gmail de Jordan (étape 7 du plan), sans réseau ni fichier : ce qui se vérifie quand Jordan
//! branche sa boîte (l'adresse, le mot de passe d'application), le rattachement d'un mail reçu à une candidature du
//! suivi ([`rattacher`]) et le statut que Jobmind propose d'après ce que dit le mail ([`proposer`]).
//!
//! Le rattachement se fait par des indices, chacun avec son poids : le mail répond à un mail envoyé par Jobmind (son
//! Message-ID cité : sûr), l'adresse de l'expéditeur est du domaine de l'entreprise, le nom de l'entreprise est dans
//! l'objet, le nom de l'expéditeur ou le texte, le poste est cité. Les boîtes de tout le monde (Gmail, Outlook,
//! Orange...) et les outils de recrutement (Teamtailor, Workday...) ne comptent jamais comme domaine de l'entreprise.
//! Sûr : rangé tout seul dans le dossier ; douteux : « Mails à ranger », Jordan choisit ; rien : le mail n'est pas
//! gardé.
//!
//! Le statut proposé vient de règles (les tournures des réponses : « nous ne pouvons pas donner une suite
//! favorable », « nous souhaiterions vous rencontrer »...), phrase par phrase ; la phrase qui le dit est gardée telle
//! quelle comme preuve. Une phrase au conditionnel (« si votre candidature est retenue, nous vous proposerons un
//! entretien ») ne compte pas. Jobmind ne change jamais le statut seul : Jordan confirme d'un clic.

use crate::Statut;
use crate::candidature::Candidature;
use crate::jour::Jour;
use crate::offre::normaliser;

// ---------------------------------------------------------------------------------------------------------------
// Le branchement
// ---------------------------------------------------------------------------------------------------------------

/// Vérifie l'adresse Gmail tapée : remise en minuscules, une seule « @ », un domaine avec un point, pas d'espace.
pub fn verifier_adresse(t: &str) -> Result<String, String> {
    let a = t.trim().trim_matches(|c: char| matches!(c, '<' | '>' | '"' | '\'')).to_lowercase();
    if a.is_empty() {
        return Err("Écris ton adresse Gmail.".into());
    }
    let ok = a.len() <= 254
        && a.is_ascii()
        && !a.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '"' | '\\' | ',' | ';'))
        && a.matches('@').count() == 1
        && a.split_once('@').is_some_and(|(l, d)| {
            !l.is_empty() && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !d.contains("..")
        });
    if !ok {
        return Err("Cette adresse ne semble pas juste. Elle s'écrit comme prenom.nom@gmail.com.".into());
    }
    Ok(a)
}

/// Vérifie le mot de passe d'application collé : Google le montre en 4 groupes de 4 lettres (« abcd efgh ijkl mnop ») ;
/// les espaces sont retirées, il reste 16 lettres. L'erreur ne le répète jamais.
pub fn verifier_mot_de_passe(t: &str) -> Result<String, String> {
    let m: String = t.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase();
    if m.is_empty() {
        return Err("Colle ici le mot de passe d'application que Google t'a donné.".into());
    }
    if !m.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(
            "Le mot de passe d'application n'a que des lettres : copie-le de nouveau depuis la page de Google. \
                    Ce n'est pas le mot de passe de ton compte."
                .into(),
        );
    }
    if m.chars().count() != 16 {
        return Err("Le mot de passe d'application a 16 lettres : copie-le de nouveau, en entier.".into());
    }
    Ok(m)
}

/// Le texte d'un mot de passe d'application à l'écran : seulement ses 4 dernières lettres.
pub fn apercu_mot_de_passe(m: &str) -> String {
    let n = m.chars().count();
    let fin: String = m.chars().skip(n.saturating_sub(4)).collect();
    format!("se termine par {fin}")
}

// ---------------------------------------------------------------------------------------------------------------
// Le rattachement
// ---------------------------------------------------------------------------------------------------------------

/// Un mail reçu, tel que le rattachement le voit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MailVu {
    /// L'adresse de l'expéditeur, en minuscules
    pub de_adresse: String,
    pub de_nom: String,
    /// Reply-To (en minuscules), sinon vide
    pub repondre_a: String,
    pub objet: String,
    pub texte: String,
    /// Les Message-ID cités (In-Reply-To, References)
    pub en_reponse_a: Vec<String>,
    /// Le jour du mail (rien : inconnu)
    pub jour: Option<Jour>,
    /// Un envoi de masse (List-Unsubscribe, Precedence: bulk)
    pub masse: bool,
}

/// Un mail envoyé par Jobmind pour une candidature (étape 10) : son Message-ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envoye {
    pub candidature: i64,
    pub message_id: String,
}

/// Ce que donne le rattachement d'un mail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rattachement {
    /// Sûr : rangé tout seul dans le dossier de cette candidature (la raison, en mots simples)
    Sur { candidature: i64, raison: String },
    /// Pas sûr : « Mails à ranger », avec la candidature la plus probable s'il y en a une
    ARanger { suggestion: Option<i64>, raison: String },
    /// Rien à voir avec une candidature : le mail n'est pas gardé
    Rien,
}

/// Le score à partir duquel un mail est rangé tout seul, et celui à partir duquel il est proposé à Jordan.
pub const SEUIL_SUR: u32 = 70;
pub const SEUIL_DOUTE: u32 = 35;

/// Les domaines de boîtes ouvertes à tous : jamais le domaine d'une entreprise.
const DOMAINES_PUBLICS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "outlook.fr",
    "hotmail.com",
    "hotmail.fr",
    "live.com",
    "live.fr",
    "msn.com",
    "yahoo.com",
    "yahoo.fr",
    "orange.fr",
    "wanadoo.fr",
    "free.fr",
    "sfr.fr",
    "neuf.fr",
    "laposte.net",
    "bbox.fr",
    "icloud.com",
    "me.com",
    "gmx.fr",
    "gmx.com",
    "aol.com",
    "protonmail.com",
    "proton.me",
];

/// Les sites d'offres et les outils de recrutement : le mail peut venir d'une entreprise, mais leur domaine n'est pas
/// le sien (leur nom d'expéditeur, l'objet ou le texte le disent alors).
const DOMAINES_OUTILS: &[&str] = &[
    "linkedin",
    "indeed",
    "indeedemail",
    "welcometothejungle",
    "francetravail",
    "pole-emploi",
    "hellowork",
    "apec",
    "teamtailor",
    "smartrecruiters",
    "lever",
    "greenhouse",
    "workday",
    "myworkday",
    "myworkdayjobs",
    "recruitee",
    "taleo",
    "successfactors",
    "flatchr",
    "jobaffinity",
    "beetween",
    "talentsoft",
    "cornerstoneondemand",
    "icims",
    "jobvite",
    "softy",
    "digitalrecruiters",
    "werecruit",
    "sendgrid",
    "mailchimp",
    "mailjet",
    "sendinblue",
    "brevo",
    "jobteaser",
    "meteojob",
    "monster",
    "cadremploi",
    "regionsjob",
    "adzuna",
];

/// Les mots retirés d'un nom d'entreprise pour le reconnaître (formes de société, mots de liaison).
const MOTS_VIDES_ENTREPRISE: &[&str] = &[
    "sas",
    "sasu",
    "sarl",
    "sa",
    "eurl",
    "snc",
    "sci",
    "scop",
    "sca",
    "groupe",
    "group",
    "societe",
    "ste",
    "cie",
    "compagnie",
    "et",
    "de",
    "des",
    "du",
    "la",
    "le",
    "les",
    "l",
    "d",
    "the",
    "france",
    "fr",
];

/// Les mots d'un poste qui ne disent rien (« H/F », le contrat, les liaisons).
const MOTS_VIDES_POSTE: &[&str] = &[
    "h", "f", "hf", "fh", "m", "x", "cdi", "cdd", "interim", "de", "des", "du", "d", "la", "le", "les", "l", "en",
    "et", "a", "au", "aux", "pour",
];

/// Les mots d'un mail qui parle de recrutement (un mail sans aucun de ces mots n'est jamais proposé).
const MOTS_RECRUTEMENT: &[&str] = &[
    "candidature",
    "candidat",
    "candidate",
    "entretien",
    "recrutement",
    "recruteur",
    "recrutons",
    "votre profil",
    "votre cv",
    "poste",
    "embauche",
    "offre d emploi",
];

/// Les mots distinctifs d'un nom d'entreprise.
fn mots_entreprise(e: &str) -> Vec<String> {
    normaliser(e).split(' ').filter(|m| !m.is_empty() && !MOTS_VIDES_ENTREPRISE.contains(m)).map(String::from).collect()
}

/// Les mots distinctifs d'un poste.
fn mots_poste(p: &str) -> Vec<String> {
    normaliser(p)
        .split(' ')
        .filter(|m| m.chars().count() >= 3 && !MOTS_VIDES_POSTE.contains(m))
        .map(String::from)
        .collect()
}

/// Un texte normalisé contient ces mots à la suite (mots entiers).
fn contient_suite(texte_norm: &str, mots: &[String]) -> bool {
    if mots.is_empty() {
        return false;
    }
    format!(" {texte_norm} ").contains(&format!(" {} ", mots.join(" ")))
}

/// Un texte normalisé contient tous ces mots (n'importe où, mots entiers).
fn contient_tous(texte_norm: &str, mots: &[String]) -> bool {
    if mots.is_empty() {
        return false;
    }
    let t = format!(" {texte_norm} ");
    mots.iter().all(|m| t.contains(&format!(" {m} ")))
}

/// Le nom de l'entreprise est dans ce texte : tous ses mots distinctifs à la suite ; un seul mot distinctif doit
/// faire au moins 4 lettres (« Lidl » oui, « Ma » non).
fn nomme(texte_norm: &str, mots: &[String]) -> bool {
    match mots {
        [] => false,
        [un] if un.chars().count() < 4 => false,
        _ => contient_suite(texte_norm, mots),
    }
}

/// Le domaine d'une adresse (« recrutement@rh.transports-moselle-fret.fr » : « rh.transports-moselle-fret.fr »).
fn domaine(adresse: &str) -> &str {
    adresse.rsplit_once('@').map_or("", |(_, d)| d.trim_end_matches('>').trim())
}

/// Le domaine est celui de tout le monde (une boîte ouverte à tous, un site d'offres, un outil de recrutement).
fn domaine_commun(d: &str) -> bool {
    if d.is_empty() {
        return true;
    }
    if DOMAINES_PUBLICS.iter().any(|p| d == *p || d.ends_with(&format!(".{p}"))) {
        return true;
    }
    d.split('.').any(|l| DOMAINES_OUTILS.contains(&l))
}

/// Le domaine (hors boîtes de tout le monde) est celui de l'entreprise : ses étiquettes collées (sans l'extension)
/// contiennent le nom de l'entreprise collé (« transports-moselle-fret.fr » et « Transports Moselle Fret »), ou son
/// mot distinctif le plus long d'au moins 5 lettres.
fn domaine_de(d: &str, mots: &[String]) -> bool {
    if domaine_commun(d) || mots.is_empty() {
        return false;
    }
    let etiquettes: Vec<&str> = d.split('.').collect();
    let sans_ext = &etiquettes[..etiquettes.len().saturating_sub(1)];
    let colle: String = sans_ext.join("").chars().filter(char::is_ascii_alphanumeric).collect();
    let nom: String = mots.concat();
    if nom.chars().count() >= 4 && colle.contains(&nom) {
        return true;
    }
    let long = mots.iter().max_by_key(|m| m.chars().count());
    long.is_some_and(|m| m.chars().count() >= 5 && m.is_ascii() && colle.contains(m.as_str()))
}

/// Le mail parle de recrutement.
pub fn parle_de_recrutement(m: &MailVu) -> bool {
    let t = format!(" {} {} ", normaliser(&m.objet), normaliser(&m.texte));
    MOTS_RECRUTEMENT.iter().any(|w| t.contains(&format!(" {w}")))
}

/// Le score d'une candidature pour ce mail, et la raison du meilleur indice.
fn score(m: &MailVu, c: &Candidature, objet: &str, nom: &str, texte: &str) -> (u32, &'static str) {
    let ent = mots_entreprise(&c.entreprise);
    let poste = mots_poste(&c.poste);
    let mut s: u32 = 0;
    let mut raison = "";
    if domaine_de(domaine(&m.de_adresse), &ent) || domaine_de(domaine(&m.repondre_a), &ent) {
        s += 60;
        raison = "l'adresse de l'expéditeur est celle de l'entreprise";
    }
    let (dans_objet, dans_nom, dans_texte) = (nomme(objet, &ent), nomme(nom, &ent), nomme(texte, &ent));
    if dans_objet || dans_nom {
        s += if dans_nom { 55 } else { 50 };
        if raison.is_empty() {
            raison =
                if dans_objet { "le nom de l'entreprise est dans l'objet" } else { "le mail vient de l'entreprise" };
        }
        if dans_texte {
            s += 10;
        }
    } else if dans_texte {
        s += 30;
        if raison.is_empty() {
            raison = "le mail cite l'entreprise";
        }
    }
    if contient_tous(objet, &poste) {
        s += 25;
        if raison.is_empty() {
            raison = "le poste est dans l'objet";
        }
    } else if contient_tous(texte, &poste) {
        s += 15;
        if raison.is_empty() {
            raison = "le mail cite le poste";
        }
    }
    let re = objet.starts_with("re ") || objet.starts_with("tr re ") || objet.starts_with("re re ");
    if re && s > 0 {
        s += 10;
    }
    // (l'entreprise reconnue dans un mail qui parle de recrutement)
    if s > 0 && !raison.contains("poste") && parle_de_recrutement(m) {
        s += 20;
    }
    if c.statut.termine() {
        s = s.saturating_sub(10);
    }
    (s, raison)
}

/// Rattache un mail reçu à une candidature du suivi.
pub fn rattacher(m: &MailVu, liste: &[Candidature], envoyes: &[Envoye]) -> Rattachement {
    // Il répond à un mail envoyé par Jobmind : sûr
    for e in envoyes {
        if !e.message_id.is_empty()
            && m.en_reponse_a.iter().any(|x| x.trim_matches(['<', '>']).eq_ignore_ascii_case(&e.message_id))
            && liste.iter().any(|c| c.id == e.candidature)
        {
            return Rattachement::Sur {
                candidature: e.candidature,
                raison: "il répond au mail envoyé par Jobmind".into(),
            };
        }
    }
    let objet = normaliser(&m.objet);
    let nom = normaliser(&m.de_nom);
    let texte = normaliser(&m.texte);
    let mut notes: Vec<(u32, i64, &'static str)> = liste
        .iter()
        // (un mail plus ancien que la candidature ne lui répond pas ; un jour d'écart toléré pour les fuseaux)
        .filter(|c| m.jour.is_none_or(|j| j.plus(1) >= c.envoyee_le))
        .map(|c| {
            let (s, r) = score(m, c, &objet, &nom, &texte);
            (s, c.id, r)
        })
        .filter(|(s, ..)| *s > 0)
        .collect();
    notes.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    let recrutement = parle_de_recrutement(m);
    match notes.as_slice() {
        // (un mail de l'entreprise qui ne parle pas de recrutement, une publicité par exemple : pas une réponse)
        [(s, id, r), reste @ ..] if *s >= SEUIL_SUR && recrutement && reste.first().is_none_or(|x| x.0 + 20 <= *s) => {
            Rattachement::Sur { candidature: *id, raison: (*r).to_string() }
        }
        [(s, id, r), ..] if *s >= SEUIL_DOUTE && recrutement => {
            Rattachement::ARanger { suggestion: Some(*id), raison: format!("Peut-être : {r}.") }
        }
        [(s, id, r), ..] if *s >= 15 && recrutement && !m.masse => {
            Rattachement::ARanger { suggestion: Some(*id), raison: format!("Peut-être : {r}.") }
        }
        _ if recrutement && !m.masse && !liste.is_empty() && fort_recrutement(&objet, &texte) => {
            Rattachement::ARanger { suggestion: None, raison: "Ce mail parle d'une candidature.".into() }
        }
        _ => Rattachement::Rien,
    }
}

/// Le mail parle clairement d'une candidature de Jordan (pas seulement du mot « poste »).
fn fort_recrutement(objet: &str, texte: &str) -> bool {
    let t = format!(" {objet} {texte} ");
    ["votre candidature", "votre entretien", "un entretien", "votre profil", "votre cv"].iter().any(|w| t.contains(w))
}

// ---------------------------------------------------------------------------------------------------------------
// Le statut proposé
// ---------------------------------------------------------------------------------------------------------------

/// Ce qu'une réponse dit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proposition {
    /// Une réponse négative
    Refus,
    /// Un entretien proposé
    Entretien,
    /// Une réponse positive (une embauche proposée)
    Positive,
    /// Le recruteur demande des informations ou des pièces
    Infos,
    /// Un simple accusé de réception
    Accuse,
}

impl Proposition {
    pub const TOUTES: [Proposition; 5] =
        [Proposition::Refus, Proposition::Entretien, Proposition::Positive, Proposition::Infos, Proposition::Accuse];

    pub fn code(self) -> &'static str {
        match self {
            Proposition::Refus => "refus",
            Proposition::Entretien => "entretien",
            Proposition::Positive => "positive",
            Proposition::Infos => "infos",
            Proposition::Accuse => "accuse",
        }
    }

    pub fn depuis_code(c: &str) -> Option<Proposition> {
        Proposition::TOUTES.into_iter().find(|p| p.code() == c)
    }

    /// Le statut du suivi que Jordan peut confirmer (rien : la réponse ne change pas le statut).
    pub fn statut(self) -> Option<Statut> {
        match self {
            Proposition::Refus => Some(Statut::Negative),
            Proposition::Entretien => Some(Statut::Entretien),
            Proposition::Positive => Some(Statut::Positive),
            Proposition::Infos | Proposition::Accuse => None,
        }
    }

    /// Ce que dit la réponse, en mots simples (« Une réponse négative »).
    pub fn nom(self) -> &'static str {
        match self {
            Proposition::Refus => "Une réponse négative",
            Proposition::Entretien => "Un entretien proposé",
            Proposition::Positive => "Une réponse positive",
            Proposition::Infos => "Une demande d'informations",
            Proposition::Accuse => "Un accusé de réception",
        }
    }
}

/// Ce que les règles ont lu dans une réponse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lu {
    /// Ce que dit la réponse, et la phrase du mail qui le dit (telle quelle)
    pub proposition: Option<(Proposition, String)>,
    /// Les règles n'ont pas su (rien trouvé, ou des phrases qui se contredisent) : Claude peut lire le mail
    pub flou: bool,
}

const REFUS: &[&str] = &[
    "poursuivre avec d autres",
    "continuer avec d autres",
    "avec d autres candidat",
    "ne correspond pas",
    "ne correspondez pas",
    "pas en mesure de vous proposer",
    "regret de ne pouvoir",
    "regret de ne pas pouvoir",
    "ne pouvoir y donner",
    "ne pas pouvoir donner suite",
    "d autres candidatures correspond",
    "d autres profils correspond",
    "d autres candidats correspond",
    "correspondaient davantage",
    "correspondent davantage",
    "plus en adequation avec",
    "pas ete retenu",
    "pas ete retenue",
    "pas ete selectionne",
    "pas ete selectionnee",
    "ne pas donner suite",
    "ne pouvons pas donner suite",
    "ne pouvons donner suite",
    "pas donner une suite favorable",
    "pas y donner une suite favorable",
    "ne donnerons pas suite",
    "pas en mesure de donner suite",
    "pas en mesure de retenir",
    "pas pu retenir",
    "ne pouvons pas retenir",
    "ne pouvons retenir",
    "avons retenu un autre",
    "avons retenu une autre",
    "retenu d autres candidat",
    "porte notre choix sur un autre",
    "porte notre choix sur une autre",
    "regret de vous informer",
    "regrettons de vous informer",
    "regret de vous annoncer",
    "ne correspond pas a nos attentes",
    "ne correspond pas aux attentes",
    "ne correspond pas au profil",
    "ne correspond pas exactement",
    "reponse negative",
    "poste a ete pourvu",
    "poste est pourvu",
    "poste est deja pourvu",
    "decide de ne pas poursuivre",
    "ne pas poursuivre",
    "pas retenue pour la suite",
    "pas retenu pour la suite",
];

const ENTRETIEN: &[&str] = &[
    "pour un entretien",
    "passer a l agence",
    "proposer un entretien",
    "vous proposer un rendez vous",
    "convier a un entretien",
    "vous convier",
    "inviter a un entretien",
    "vous inviter a un entretien",
    "planifier un entretien",
    "organiser un entretien",
    "fixer un entretien",
    "fixer un rendez vous",
    "prendre rendez vous",
    "vous rencontrer",
    "vous recevoir",
    "echange telephonique",
    "entretien telephonique",
    "entretien en visio",
    "entretien en visioconference",
    "seriez vous disponible",
    "etes vous disponible",
    "seriez vous disponibles",
    "aimerions vous rencontrer",
    "souhaiterions vous rencontrer",
    "souhaitons vous rencontrer",
];

const POSITIVE: &[&str] = &[
    "avez ete retenu",
    "avez ete retenue",
    "avez ete selectionne",
    "heureux de vous informer que vous",
    "heureuse de vous informer que vous",
    "confirmons votre embauche",
    "confirmer votre embauche",
    "avons le plaisir de vous annoncer",
    "avons le plaisir de vous informer que votre candidature",
    "heureux de vous annoncer",
    "heureuse de vous annoncer",
    "votre candidature a ete retenue",
    "vous proposer le poste",
    "proposition d embauche",
    "promesse d embauche",
    "bienvenue dans l equipe",
    "souhaitons vous embaucher",
    "avons retenu votre candidature",
    "offre de contrat",
];

const INFOS: &[&str] = &[
    "pourriez vous nous transmettre",
    "pourriez vous nous envoyer",
    "pourriez vous nous faire parvenir",
    "pouvez vous nous transmettre",
    "pouvez vous nous envoyer",
    "pouvez vous nous indiquer",
    "pourriez vous nous indiquer",
    "pourriez vous me transmettre",
    "pourriez vous m envoyer",
    "merci de nous transmettre",
    "merci de nous faire parvenir",
    "merci de nous envoyer",
    "merci de me transmettre",
    "merci de m envoyer",
    "vos pretentions salariales",
    "complement d information",
    "complements d information",
    "pieces justificatives",
    "remplir le questionnaire",
    "repondre au questionnaire",
    "nous aurions besoin",
];

const ACCUSE: &[&str] = &[
    "a ete enregistree",
    "a bien ete enregistree",
    "etudiera",
    "dans les meilleurs delais",
    "bien recu votre candidature",
    "accusons reception",
    "accuse de reception",
    "avons bien recu",
    "nous reviendrons vers vous",
    "votre candidature a bien ete",
    "sera etudie",
    "sera examine",
    "interet que vous portez",
    "interet porte a",
    "est en cours d etude",
    "etudions votre candidature",
    "merci pour votre candidature",
    "merci de votre candidature",
    "remercions pour votre candidature",
    "remercions de votre candidature",
    "remercions de l interet",
];

/// Les tournures au conditionnel : la phrase ne dit pas ce qui arrive (« si votre candidature est retenue »).
const CONDITIONS: &[&str] = &[
    "en cas de",
    "si vous etes selectionne",
    "si votre",
    "si vous etes retenu",
    "si vous etes retenue",
    "si nous",
    "dans le cas ou",
    "au cas ou",
    "sans reponse de notre part",
    "en l absence de reponse",
    "sans nouvelles de notre part",
    "a defaut de reponse",
    "le cas echeant",
    "eventuel entretien",
    "si tel est le cas",
];

/// Les phrases d'un texte (avec l'objet en premier), coupées aux points, points d'exclamation, d'interrogation et
/// retours à la ligne, sans les lignes citées d'un mail précédent (« > ... ») ni ce qui suit « Le ... a écrit : ».
fn phrases(objet: &str, texte: &str) -> Vec<String> {
    let mut l = vec![objet.trim().to_string()];
    for ligne in texte.lines() {
        let t = ligne.trim();
        if t.starts_with('>') {
            continue;
        }
        let n = normaliser(t);
        // (le mail cité dessous n'est pas la réponse)
        if (n.starts_with("le ") && n.ends_with(" a ecrit")) || n.starts_with("de ") && n.contains(" envoye ") {
            break;
        }
        if n.starts_with("message d origine") || n.starts_with("original message") {
            break;
        }
        let mut debut = 0;
        let o: Vec<(usize, char)> = t.char_indices().collect();
        for (k, (i, c)) in o.iter().enumerate() {
            let fin =
                matches!(c, '!' | '?' | ';') || (*c == '.' && o.get(k + 1).is_none_or(|(_, x)| x.is_whitespace()));
            if fin {
                let p = t[debut..i + c.len_utf8()].trim();
                if !p.is_empty() {
                    l.push(p.to_string());
                }
                debut = i + c.len_utf8();
            }
        }
        let p = t[debut..].trim();
        if !p.is_empty() {
            l.push(p.to_string());
        }
        if l.len() > 400 {
            break;
        }
    }
    l
}

fn trouve(n: &str, motifs: &[&str]) -> bool {
    let t = format!(" {n} ");
    motifs.iter().any(|m| t.contains(&format!(" {m}")))
}

/// Une phrase gardée comme preuve : telle quelle, sur une ligne, 300 caractères au plus.
fn preuve(p: &str) -> String {
    let p = p.split_whitespace().collect::<Vec<_>>().join(" ");
    if p.chars().count() <= 300 { p } else { p.chars().take(300).collect::<String>().trim_end().to_string() }
}

/// Ce que dit une réponse, d'après ses phrases.
pub fn proposer(objet: &str, texte: &str) -> Lu {
    let mut vus: Vec<(Proposition, String)> = Vec::new();
    let mut accuse: Option<String> = None;
    for p in &phrases(objet, texte) {
        let n = normaliser(p);
        if n.is_empty() {
            continue;
        }
        let condition = trouve(&n, CONDITIONS);
        if !condition {
            for (genre, motifs) in [
                (Proposition::Refus, REFUS),
                (Proposition::Positive, POSITIVE),
                (Proposition::Entretien, ENTRETIEN),
                (Proposition::Infos, INFOS),
            ] {
                // (« vos disponibilités » seul ne dit pas un entretien : il faut aussi un entretien, un rendez-vous, un appel)
                // (un rendez-vous ou un entretien avec un jour ou une heure : proposé)
                let quand = format!(" {n} ");
                let date = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "demain"]
                    .iter()
                    .any(|w| quand.contains(&format!(" {w} ")))
                    || quand
                        .split(' ')
                        .collect::<Vec<_>>()
                        .windows(2)
                        .any(|w| w[0].chars().all(|c| c.is_ascii_digit()) && !w[0].is_empty() && w[1] == "h");
                let dispo = genre == Proposition::Entretien
                    && ((n.contains("disponibilit")
                        && ["entretien", "rencontr", "rendez vous", "echange", "appel", "visio"]
                            .iter()
                            .any(|w| n.contains(w)))
                        || ((n.contains("rendez vous") || n.contains("entretien")) && date));
                if (trouve(&n, motifs) || dispo) && !vus.iter().any(|(g, _)| *g == genre) {
                    vus.push((genre, preuve(p)));
                }
            }
        }
        if accuse.is_none() && (trouve(&n, ACCUSE) || condition) {
            accuse = Some(preuve(p));
        }
    }
    let a = |g: Proposition| vus.iter().find(|(x, _)| *x == g).cloned();
    let (refus, positive, entretien, infos) =
        (a(Proposition::Refus), a(Proposition::Positive), a(Proposition::Entretien), a(Proposition::Infos));
    let proposition = match (refus, positive, entretien) {
        // (des phrases qui se contredisent : Claude, ou Jordan, tranchera)
        (Some(_), Some(_), _) | (Some(_), _, Some(_)) => return Lu { proposition: None, flou: true },
        (Some(r), None, None) => Some(r),
        (None, Some(p), _) => Some(p),
        (None, None, Some(e)) => Some(e),
        (None, None, None) => infos.or(accuse.map(|x| (Proposition::Accuse, x))),
    };
    let flou = proposition.is_none() && texte.trim().chars().count() >= 40;
    Lu { proposition, flou }
}

#[cfg(test)]
mod tests;
