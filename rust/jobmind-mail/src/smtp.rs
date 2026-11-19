//! L'envoi d'une candidature par Gmail (étape 10 du plan) : le mail composé ici ([`composer`] : RFC 5322 et MIME,
//! UTF-8, objet encodé, lettre et pièce jointe en base64, un Message-ID choisi par Jobmind) puis remis à Gmail par un
//! client SMTP écrit ici, réduit au strict nécessaire : EHLO, AUTH PLAIN, MAIL FROM, RCPT TO (UN SEUL destinataire),
//! DATA, QUIT. Aucune boucle, aucun envoi groupé : une fonction [`envoyer`] = un mail à une adresse.
//!
//! La connexion est chiffrée dès le départ (smtp.gmail.com, port 465, TLS par SChannel, comme l'IMAP) ; sans TLS,
//! seulement vers 127.0.0.1 (le faux serveur des tests et de l'essai).
//!
//! L'honnêteté sur ce qui est parti ([`ErreurEnvoi::rien_parti`]) : tant que la fin du message (la ligne « . ») n'a
//! pas été écrite, rien n'est parti ; si Gmail répond 250 après elle, le mail est parti ; si la connexion se coupe
//! juste après elle, personne ne peut savoir : [`ErreurEnvoi::Incertain`] (Jordan regarde dans Gmail, Messages
//! envoyés, au lieu de renvoyer).
//!
//! Le mot de passe n'est jamais écrit dans une erreur, le journal ou un `Debug`.

use std::io::{ErrorKind, Read, Write};

use crate::{Acces, DateMail, ErreurMail};

/// Le serveur SMTP de Gmail.
pub const HOTE_SMTP_GMAIL: &str = "smtp.gmail.com";
/// Le port SMTP chiffré dès la connexion (TLS implicite).
pub const PORT_SMTP_GMAIL: u16 = 465;
/// La plus grosse pièce jointe acceptée (Gmail refuse au-delà de 25 Mo, base64 compris).
pub const PIECE_MAX: usize = 10 * 1024 * 1024;
/// La plus longue lettre acceptée (en caractères).
pub const TEXTE_MAX: usize = 20_000;
/// Le plus long objet accepté (en caractères).
pub const OBJET_MAX: usize = 200;
/// La plus longue ligne de réponse lue.
const LIGNE_MAX: usize = 4_000;
/// Le plus grand nombre de lignes d'une réponse.
const LIGNES_MAX: usize = 200;

impl Acces {
    /// L'accès au serveur d'envoi de Gmail (smtp.gmail.com, port 465, TLS), avec un délai de 30 s.
    pub fn gmail_envoi(adresse: &str, mdp: &str) -> Acces {
        Acces {
            hote: HOTE_SMTP_GMAIL.into(),
            port: PORT_SMTP_GMAIL,
            tls: true,
            adresse: adresse.into(),
            mdp: mdp.into(),
            delai: crate::DELAI_DEFAUT,
        }
    }
}

/// Pourquoi le mail n'est pas parti (ou peut-être parti : [`ErreurEnvoi::Incertain`]). Aucun texte ne contient le mot
/// de passe.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurEnvoi {
    /// Le mail ne peut pas être composé (une adresse impossible, un objet avec un retour à la ligne, une pièce jointe
    /// trop grosse ou vide) : rien n'a été tenté
    #[error("mail impossible : {0}")]
    Brouillon(String),
    /// La connexion n'a pas pu s'ouvrir, s'est coupée ou a trop attendu AVANT la fin du message : rien n'est parti
    #[error("connexion : {0}")]
    Connexion(ErreurMail),
    /// AUTH refusé (535, 534) : mauvais mot de passe d'application
    #[error("identifiants refusés")]
    Identifiants,
    /// L'adresse de Jordan refusée par MAIL FROM
    #[error("expéditeur refusé : {0}")]
    ExpediteurRefuse(String),
    /// Le destinataire refusé par RCPT TO (adresse inconnue, domaine qui n'existe pas)
    #[error("destinataire refusé : {0}")]
    DestinataireRefuse(String),
    /// Une autre réponse 4xx ou 5xx avant la fin du message (trop d'envois, service fermé...) : rien n'est parti
    #[error("refus : {0}")]
    Refus(String),
    /// Gmail a lu tout le message puis l'a refusé (réponse 4xx ou 5xx après « . ») : rien n'est parti
    #[error("message refusé : {0}")]
    MessageRefuse(String),
    /// La connexion s'est coupée (ou n'a plus répondu) juste après la fin du message : le mail est PEUT-ÊTRE parti
    #[error("envoi incertain : {0}")]
    Incertain(String),
    /// Une réponse du serveur qui ne se lit pas, avant la fin du message : rien n'est parti
    #[error("réponse illisible : {0}")]
    Illisible(String),
}

impl ErreurEnvoi {
    /// Le code de l'erreur, pour le journal et la page.
    pub fn code(&self) -> &'static str {
        match self {
            ErreurEnvoi::Brouillon(_) => "brouillon",
            ErreurEnvoi::Connexion(e) => e.code(),
            ErreurEnvoi::Identifiants => "identifiants",
            ErreurEnvoi::ExpediteurRefuse(_) => "expediteur",
            ErreurEnvoi::DestinataireRefuse(_) => "destinataire",
            ErreurEnvoi::Refus(_) => "refus",
            ErreurEnvoi::MessageRefuse(_) => "message_refuse",
            ErreurEnvoi::Incertain(_) => "incertain",
            ErreurEnvoi::Illisible(_) => "illisible",
        }
    }

    /// Vrai si l'on SAIT que rien n'est parti (tout sauf [`ErreurEnvoi::Incertain`]).
    pub fn rien_parti(&self) -> bool {
        !matches!(self, ErreurEnvoi::Incertain(_))
    }

    /// Une phrase simple pour Jordan, en français, sans jargon.
    pub fn message(&self) -> String {
        match self {
            ErreurEnvoi::Brouillon(_) => {
                "Ce mail ne peut pas partir tel quel : vérifie l'adresse, l'objet et ton CV. Rien n'a été envoyé."
                    .into()
            }
            ErreurEnvoi::Connexion(e) => format!("{} Rien n'a été envoyé.", e.message()),
            ErreurEnvoi::Identifiants => "Gmail refuse le mot de passe d'application pour envoyer. Vérifie ta boîte \
                                          dans Mon profil, rubrique Ma boîte mail. Rien n'a été envoyé."
                .into(),
            ErreurEnvoi::ExpediteurRefuse(_) => {
                "Gmail refuse d'envoyer depuis ton adresse. Vérifie ta boîte dans Mon profil. Rien n'a été envoyé."
                    .into()
            }
            ErreurEnvoi::DestinataireRefuse(_) => {
                "Gmail refuse cette adresse de destination. Vérifie l'adresse dans l'offre. Rien n'a été envoyé.".into()
            }
            ErreurEnvoi::Refus(_) => {
                "Gmail refuse d'envoyer pour l'instant. Réessaie plus tard. Rien n'a été envoyé.".into()
            }
            ErreurEnvoi::MessageRefuse(_) => {
                "Gmail a refusé ce mail. Vérifie ton CV joint, puis réessaie. Rien n'a été envoyé.".into()
            }
            ErreurEnvoi::Incertain(_) => "La connexion s'est coupée au moment de l'envoi : Jobmind ne sait pas si le \
                                          mail est parti. Regarde dans Gmail, dans Messages envoyés, avant de renvoyer."
                .into(),
            ErreurEnvoi::Illisible(_) => {
                "La réponse de Gmail n'a pas pu être lue. Réessaie plus tard. Rien n'a été envoyé.".into()
            }
        }
    }
}

/// Le texte du serveur, montrable : 200 caractères au plus, sans le mot de passe.
fn propre(t: &str, mdp: &str) -> String {
    let mut t: String = t.chars().filter(|c| !c.is_control()).take(300).collect();
    if !mdp.is_empty() {
        t = t.replace(mdp, "*****");
    }
    t.chars().take(200).collect::<String>().trim().to_string()
}

fn masquer(e: ErreurEnvoi, mdp: &str) -> ErreurEnvoi {
    match e {
        ErreurEnvoi::Connexion(c) => ErreurEnvoi::Connexion(crate::masquer(c, mdp)),
        ErreurEnvoi::ExpediteurRefuse(t) => ErreurEnvoi::ExpediteurRefuse(propre(&t, mdp)),
        ErreurEnvoi::DestinataireRefuse(t) => ErreurEnvoi::DestinataireRefuse(propre(&t, mdp)),
        ErreurEnvoi::Refus(t) => ErreurEnvoi::Refus(propre(&t, mdp)),
        ErreurEnvoi::MessageRefuse(t) => ErreurEnvoi::MessageRefuse(propre(&t, mdp)),
        ErreurEnvoi::Incertain(t) => ErreurEnvoi::Incertain(propre(&t, mdp)),
        ErreurEnvoi::Illisible(t) => ErreurEnvoi::Illisible(propre(&t, mdp)),
        autre => autre,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Le mail composé
// ---------------------------------------------------------------------------------------------------------------

/// Une pièce jointe : son nom (montré au recruteur), son type et ses octets.
#[derive(Clone, PartialEq, Eq)]
pub struct PieceJointe {
    pub nom: String,
    /// « application/pdf », « application/vnd.openxmlformats-officedocument.wordprocessingml.document »
    pub type_mime: String,
    pub octets: Vec<u8>,
}

impl std::fmt::Debug for PieceJointe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PieceJointe {{ nom: {:?}, type_mime: {:?}, octets: {} }}",
            self.nom,
            self.type_mime,
            self.octets.len()
        )
    }
}

/// Le type d'un fichier d'après son extension (PDF, Word ; sinon « application/octet-stream »).
pub fn type_mime(nom: &str) -> &'static str {
    let n = nom.to_ascii_lowercase();
    if n.ends_with(".pdf") {
        "application/pdf"
    } else if n.ends_with(".docx") {
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    } else if n.ends_with(".doc") {
        "application/msword"
    } else if n.ends_with(".odt") {
        "application/vnd.oasis.opendocument.text"
    } else {
        "application/octet-stream"
    }
}

/// Un mail à envoyer : un seul destinataire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brouillon {
    /// L'adresse de Jordan (son Gmail)
    pub de: String,
    /// Le nom de Jordan (vide : seulement l'adresse)
    pub de_nom: String,
    /// L'adresse du recruteur
    pub a: String,
    pub objet: String,
    /// La lettre, telle que Jordan l'a gardée (des \n ou des \r\n)
    pub texte: String,
    pub piece: Option<PieceJointe>,
    /// Le Message-ID, sans < > (voir [`nouveau_message_id`])
    pub message_id: String,
    pub date: DateMail,
}

/// Une adresse qui peut aller dans un en-tête et dans MAIL FROM / RCPT TO : ASCII, une seule @, ni espace, ni
/// caractère de contrôle, ni < > " ( ) , ; : \ [ ].
pub fn adresse_sure(a: &str) -> bool {
    a.len() <= 254
        && a.is_ascii()
        && a.matches('@').count() == 1
        && !a.chars().any(|c| c.is_whitespace() || c.is_control() || "<>\"(),;:\\[]".contains(c))
        && a.split_once('@').is_some_and(|(l, d)| {
            !l.is_empty()
                && l.len() <= 64
                && !l.starts_with('.')
                && !l.ends_with('.')
                && !l.contains("..")
                && d.contains('.')
                && !d.starts_with(['.', '-'])
                && !d.ends_with(['.', '-'])
                && !d.contains("..")
                && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        })
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Le base64 d'octets, sur une seule ligne.
pub fn base64(o: &[u8]) -> String {
    let mut r = String::with_capacity(o.len().div_ceil(3) * 4);
    for c in o.chunks(3) {
        let n =
            (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        r.push(B64[(n >> 18) as usize & 63] as char);
        r.push(B64[(n >> 12) as usize & 63] as char);
        r.push(if c.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        r.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    r
}

/// Le base64 en lignes de 76 caractères, chacune finie par CRLF.
fn base64_lignes(o: &[u8]) -> String {
    let b = base64(o);
    let mut r = String::with_capacity(b.len() + b.len() / 76 * 2 + 2);
    for l in b.as_bytes().chunks(76) {
        r.push_str(std::str::from_utf8(l).unwrap_or_default());
        r.push_str("\r\n");
    }
    r
}

/// Un texte qui peut aller tel quel dans un en-tête : ASCII imprimable, sans « =? ».
fn ascii_simple(t: &str) -> bool {
    t.bytes().all(|b| (0x20..0x7f).contains(&b)) && !t.contains("=?")
}

/// Un texte d'en-tête en mots encodés (RFC 2047, « =?UTF-8?B?...?= »), chacun de 75 caractères au plus, jamais
/// coupé au milieu d'un caractère ; les mots sont séparés par CRLF et une espace (le pliage d'un en-tête long).
pub fn mot_encode(t: &str) -> String {
    // (12 caractères d'enrobage et 48 de base64, soit 36 octets au plus par mot : avec « Subject: » devant, la
    // ligne garde moins de 78 caractères)
    let mut mots = Vec::new();
    let mut courant = String::new();
    for c in t.chars() {
        if courant.len() + c.len_utf8() > 36 {
            mots.push(format!("=?UTF-8?B?{}?=", base64(courant.as_bytes())));
            courant.clear();
        }
        courant.push(c);
    }
    if !courant.is_empty() || mots.is_empty() {
        mots.push(format!("=?UTF-8?B?{}?=", base64(courant.as_bytes())));
    }
    mots.join("\r\n ")
}

/// Un en-tête d'un texte libre : tel quel s'il est en ASCII simple et court, sinon en mots encodés.
fn texte_entete(t: &str) -> String {
    if ascii_simple(t) && t.len() <= 70 { t.to_string() } else { mot_encode(t) }
}

/// Le nom d'un expéditeur dans From : entre guillemets s'il est en ASCII, sinon encodé.
fn nom_entete(n: &str) -> String {
    if ascii_simple(n) && n.len() <= 60 { format!("\"{}\"", n.replace(['\\', '"'], "")) } else { mot_encode(n) }
}

/// Le nom d'une pièce jointe sans rien de dangereux : ni chemin, ni caractère de contrôle, ni guillemet ; 120
/// caractères au plus ; vide : « CV.pdf ».
pub fn nom_piece_sur(n: &str) -> String {
    let base = n.rsplit(['\\', '/']).next().unwrap_or_default();
    let t: String = base
        .chars()
        .filter(|c| !c.is_control() && !matches!(c, '"' | '\\' | '/' | ':' | '*' | '?' | '<' | '>' | '|'))
        .collect::<String>()
        .trim()
        .trim_start_matches('.')
        .to_string();
    if t.is_empty() {
        return "CV.pdf".into();
    }
    if t.chars().count() <= 120 {
        return t;
    }
    // (trop long : coupé, l'extension gardée)
    match t.rsplit_once('.') {
        Some((b, e)) if e.chars().count() <= 5 => {
            format!("{}.{e}", b.chars().take(119 - e.chars().count()).collect::<String>().trim_end())
        }
        _ => t.chars().take(120).collect(),
    }
}

/// Un nom en ASCII simple : les lettres accentuées sans leur accent, le reste qui n'est pas ASCII en « _ ».
fn ascii_repli(n: &str) -> String {
    n.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'À' | 'Â' | 'Ä' | 'Á' => 'A',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'î' | 'ï' | 'í' => 'i',
            'Î' | 'Ï' => 'I',
            'ô' | 'ö' | 'ó' => 'o',
            'Ô' | 'Ö' => 'O',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'Ù' | 'Û' | 'Ü' => 'U',
            'ç' => 'c',
            'Ç' => 'C',
            'ÿ' => 'y',
            c if (' '..='~').contains(&c) && c != '"' && c != '\\' && c != '=' && c != '?' => c,
            _ => '_',
        })
        .collect()
}

/// Le nom de fichier en RFC 2231, en lignes pliées (« filename*=UTF-8''CV%20J%C3%A9r%C3%B4me.pdf ») ; un nom long
/// est découpé en morceaux numérotés (« filename*0*=...; filename*1*=... »), jamais au milieu d'un « %XX ».
fn nom_2231(n: &str) -> String {
    let mut jetons: Vec<String> = Vec::new();
    for b in n.bytes() {
        if b.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&b) {
            jetons.push((b as char).to_string());
        } else {
            jetons.push(format!("%{b:02X}"));
        }
    }
    let tout: String = jetons.concat();
    if tout.len() <= 55 {
        return format!(" filename*=UTF-8''{tout}");
    }
    let mut morceaux: Vec<String> = vec![String::new()];
    for j in jetons {
        if morceaux.last().is_some_and(|m| m.len() + j.len() > 55) {
            morceaux.push(String::new());
        }
        if let Some(m) = morceaux.last_mut() {
            m.push_str(&j);
        }
    }
    morceaux
        .iter()
        .enumerate()
        .map(|(i, m)| if i == 0 { format!(" filename*0*=UTF-8''{m}") } else { format!(" filename*{i}*={m}") })
        .collect::<Vec<_>>()
        .join(";\r\n")
}

/// Le nom en ASCII pour les en-têtes simples : 60 caractères au plus, l'extension gardée.
fn nom_court(n: &str) -> String {
    let a = ascii_repli(n);
    if a.chars().count() <= 60 {
        return a;
    }
    let (base, ext) = match a.rsplit_once('.') {
        Some((b, e)) if e.len() <= 5 => (b.to_string(), format!(".{e}")),
        _ => (a.clone(), String::new()),
    };
    format!("{}{ext}", base.chars().take(60 - ext.len()).collect::<String>())
}

const JOURS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MOIS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Le jour de la semaine (0 : lundi), par le nombre de jours depuis le 1er janvier 1970 (un jeudi).
fn jour_semaine(a: i32, m: u32, j: u32) -> usize {
    // (l'algorithme des jours civils de Howard Hinnant)
    let a = i64::from(if m <= 2 { a - 1 } else { a });
    let ere = if a >= 0 { a } else { a - 399 } / 400;
    let aoe = a - ere * 400;
    let m = i64::from(m);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(j) - 1;
    let doe = aoe * 365 + aoe / 4 - aoe / 100 + doy;
    let jours = ere * 146_097 + doe - 719_468;
    // (le 1er janvier 1970 était un jeudi : 3)
    (jours + 3).rem_euclid(7) as usize
}

/// La date d'un mail (« Wed, 19 Nov 2026 14:05:00 +0100 »).
pub fn date_rfc5322(d: &DateMail) -> String {
    let m = d.mois.clamp(1, 12);
    let s = if d.decalage_minutes < 0 { '-' } else { '+' };
    let dm = d.decalage_minutes.unsigned_abs().min(23 * 60 + 59);
    format!(
        "{}, {} {} {:04} {:02}:{:02}:{:02} {s}{:02}{:02}",
        JOURS[jour_semaine(d.annee, m, d.jour)],
        d.jour,
        MOIS[(m - 1) as usize],
        d.annee,
        d.heure.min(23),
        d.minute.min(59),
        d.seconde.min(59),
        dm / 60,
        dm % 60
    )
}

/// Un Message-ID neuf, sans < > : 32 chiffres hexadécimaux tirés au hasard (horloge, numéro de l'appel, hachage à clé
/// aléatoire de la bibliothèque standard), puis « .jobmind@ » et le domaine de l'adresse de Jordan.
pub fn nouveau_message_id(de: &str) -> String {
    use std::hash::{BuildHasher, Hasher};
    static COMPTE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COMPTE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut parts = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(t);
        h.write_u64(n);
        h.write_u64(i);
        h.write_u32(std::process::id());
        parts.push_str(&format!("{:016x}", h.finish()));
    }
    let domaine = de.rsplit_once('@').map(|(_, d)| d).filter(|d| adresse_sure(&format!("x@{d}"))).unwrap_or("jobmind");
    format!("{parts}.jobmind@{domaine}")
}

/// Le texte de la lettre, lignes finies par CRLF (un \r ou \n seul devient CRLF ; les caractères de contrôle autres que
/// la tabulation sont retirés).
fn texte_crlf(t: &str) -> String {
    let t = t.replace("\r\n", "\n").replace('\r', "\n");
    let mut r = String::with_capacity(t.len() + 64);
    for c in t.chars() {
        if c == '\n' {
            r.push_str("\r\n");
        } else if c == '\t' || !c.is_control() {
            r.push(c);
        }
    }
    r
}

/// Vérifie le brouillon avant de le composer : rien d'impossible ou de dangereux (un retour à la ligne dans un en-tête
/// ferait un en-tête de plus).
pub fn verifier(b: &Brouillon) -> Result<(), ErreurEnvoi> {
    let e = |t: &str| Err(ErreurEnvoi::Brouillon(t.into()));
    if !adresse_sure(&b.de) {
        return e("adresse de l'expéditeur");
    }
    if !adresse_sure(&b.a) {
        return e("adresse du destinataire");
    }
    if b.objet.trim().is_empty() || b.objet.chars().count() > OBJET_MAX || b.objet.chars().any(char::is_control) {
        return e("objet");
    }
    if b.de_nom.chars().count() > 120 || b.de_nom.chars().any(char::is_control) {
        return e("nom de l'expéditeur");
    }
    if b.texte.trim().is_empty() || b.texte.chars().count() > TEXTE_MAX {
        return e("lettre");
    }
    if !adresse_sure(&format!("x@{}", b.message_id.rsplit_once('@').map(|(_, d)| d).unwrap_or("")))
        || b.message_id.split_once('@').is_none_or(|(l, _)| {
            l.is_empty() || !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        })
    {
        return e("Message-ID");
    }
    if let Some(p) = &b.piece {
        if p.octets.is_empty() {
            return e("pièce jointe vide");
        }
        if p.octets.len() > PIECE_MAX {
            return e("pièce jointe trop grosse");
        }
        if !ascii_simple(&p.type_mime) || p.type_mime.contains([';', '"', ' ']) || !p.type_mime.contains('/') {
            return e("type de la pièce jointe");
        }
    }
    Ok(())
}

/// Le mail entier, prêt pour DATA (lignes en CRLF, sans le « . » final ni le doublement des points).
pub fn composer(b: &Brouillon) -> Result<Vec<u8>, ErreurEnvoi> {
    verifier(b)?;
    let de_nom = b.de_nom.trim();
    let from = if de_nom.is_empty() { format!("<{}>", b.de) } else { format!("{} <{}>", nom_entete(de_nom), b.de) };
    let mut m = String::new();
    m.push_str(&format!("Date: {}\r\n", date_rfc5322(&b.date)));
    m.push_str(&format!("From: {from}\r\n"));
    m.push_str(&format!("To: <{}>\r\n", b.a));
    m.push_str(&format!("Subject: {}\r\n", texte_entete(b.objet.trim())));
    m.push_str(&format!("Message-ID: <{}>\r\n", b.message_id));
    m.push_str("MIME-Version: 1.0\r\n");
    let corps = base64_lignes(texte_crlf(&b.texte).as_bytes());
    match &b.piece {
        None => {
            m.push_str("Content-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n");
            m.push_str(&corps);
        }
        Some(p) => {
            // (une frontière qui ne peut pas se trouver dans du base64 : elle contient « = » et « _ »)
            let frontiere = format!("=_jobmind_{}", b.message_id.split('.').next().unwrap_or("partie"));
            let nom = nom_piece_sur(&p.nom);
            // (le nom en ASCII pour les vieux lecteurs ; le vrai nom, accents compris, dans filename*)
            let nom_simple = nom_court(&nom);
            m.push_str(&format!("Content-Type: multipart/mixed;\r\n boundary=\"{frontiere}\"\r\n\r\n"));
            m.push_str("This is a multi-part message in MIME format.\r\n");
            m.push_str(&format!("--{frontiere}\r\n"));
            m.push_str("Content-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n");
            m.push_str(&corps);
            m.push_str(&format!("--{frontiere}\r\n"));
            m.push_str(&format!("Content-Type: {};\r\n name=\"{nom_simple}\"\r\n", p.type_mime));
            m.push_str("Content-Transfer-Encoding: base64\r\n");
            m.push_str(&format!(
                "Content-Disposition: attachment;\r\n filename=\"{nom_simple}\";\r\n{}\r\n\r\n",
                nom_2231(&nom)
            ));
            m.push_str(&base64_lignes(&p.octets));
            m.push_str(&format!("--{frontiere}--\r\n"));
        }
    }
    Ok(m.into_bytes())
}

/// Le message pour DATA : chaque ligne qui commence par « . » en reçoit un second (RFC 5321, 4.5.2), puis la fin
/// « CRLF . CRLF ».
pub fn pour_data(message: &[u8]) -> Vec<u8> {
    let mut r = Vec::with_capacity(message.len() + 64);
    let mut debut_ligne = true;
    for &o in message {
        if debut_ligne && o == b'.' {
            r.push(b'.');
        }
        r.push(o);
        debut_ligne = o == b'\n';
    }
    if !r.ends_with(b"\r\n") {
        r.extend_from_slice(b"\r\n");
    }
    r.extend_from_slice(b".\r\n");
    r
}

// ---------------------------------------------------------------------------------------------------------------
// Le client SMTP
// ---------------------------------------------------------------------------------------------------------------

/// Une réponse : son code (250, 535...) et son texte (les lignes jointes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reponse {
    pub code: u16,
    pub texte: String,
}

/// Le client, sur un flux déjà ouvert.
pub struct Client<S: Read + Write> {
    flux: S,
    tampon: Vec<u8>,
}

fn erreur_io(e: std::io::Error) -> ErreurMail {
    match e.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => ErreurMail::Delai,
        _ => ErreurMail::Reseau(e.to_string()),
    }
}

impl<S: Read + Write> Client<S> {
    pub fn new(flux: S) -> Client<S> {
        Client { flux, tampon: Vec::new() }
    }

    fn ligne(&mut self) -> Result<String, ErreurMail> {
        loop {
            if let Some(p) = self.tampon.iter().position(|&c| c == b'\n') {
                let mut l: Vec<u8> = self.tampon.drain(..=p).collect();
                l.pop();
                if l.last() == Some(&b'\r') {
                    l.pop();
                }
                return Ok(String::from_utf8_lossy(&l).into_owned());
            }
            if self.tampon.len() > LIGNE_MAX {
                return Err(ErreurMail::Illisible("ligne trop longue".into()));
            }
            let mut b = [0u8; 2048];
            match self.flux.read(&mut b) {
                Ok(0) => return Err(ErreurMail::Reseau("la connexion a été coupée".into())),
                Ok(n) => self.tampon.extend_from_slice(&b[..n]),
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(erreur_io(e)),
            }
        }
    }

    /// Une réponse entière (« 250-... » puis « 250 ... »).
    pub fn reponse(&mut self) -> Result<Reponse, ErreurMail> {
        let mut textes = Vec::new();
        for _ in 0..LIGNES_MAX {
            let l = self.ligne()?;
            let o = l.as_bytes();
            if o.len() < 3 || !o[..3].iter().all(u8::is_ascii_digit) || (o.len() > 3 && o[3] != b' ' && o[3] != b'-') {
                return Err(ErreurMail::Illisible("réponse sans code".into()));
            }
            let code: u16 = l[..3].parse().map_err(|_| ErreurMail::Illisible("code".into()))?;
            textes.push(l.get(4..).unwrap_or_default().to_string());
            if o.len() == 3 || o[3] == b' ' {
                return Ok(Reponse { code, texte: textes.join(" ") });
            }
        }
        Err(ErreurMail::Illisible("réponse trop longue".into()))
    }

    fn ecrire(&mut self, o: &[u8]) -> Result<(), ErreurMail> {
        self.flux.write_all(o).and_then(|_| self.flux.flush()).map_err(erreur_io)
    }

    /// Une commande (une ligne sans CRLF), puis sa réponse.
    pub fn commande(&mut self, c: &str) -> Result<Reponse, ErreurMail> {
        let mut l = Vec::with_capacity(c.len() + 2);
        l.extend_from_slice(c.as_bytes());
        l.extend_from_slice(b"\r\n");
        let r = self.ecrire(&l);
        effacer_octets(&mut l);
        r?;
        self.reponse()
    }
}

fn effacer_octets(o: &mut Vec<u8>) {
    o.iter_mut().for_each(|x| *x = 0);
    std::hint::black_box(o.as_slice());
    o.clear();
}

fn refus_ou(r: &Reponse, mdp: &str, f: fn(String) -> ErreurEnvoi) -> ErreurEnvoi {
    f(propre(&format!("{} {}", r.code, r.texte), mdp))
}

/// L'accueil (220), EHLO (250), puis AUTH PLAIN (235).
fn ouvrir_session(acces: &Acces) -> Result<Client<Box<dyn crate::Flux>>, ErreurEnvoi> {
    let flux = crate::ouvrir(acces).map_err(ErreurEnvoi::Connexion)?;
    let mut c = Client::new(flux);
    let r = c.reponse().map_err(ErreurEnvoi::Connexion)?;
    if r.code != 220 {
        return Err(refus_ou(&r, &acces.mdp, ErreurEnvoi::Refus));
    }
    let r = c.commande("EHLO [127.0.0.1]").map_err(ErreurEnvoi::Connexion)?;
    if r.code != 250 {
        return Err(refus_ou(&r, &acces.mdp, ErreurEnvoi::Refus));
    }
    // AUTH PLAIN : « \0adresse\0mot de passe » en base64 (jamais noté nulle part, effacé aussitôt)
    if acces.adresse.contains('\0') || acces.mdp.contains('\0') || acces.mdp.is_empty() {
        return Err(ErreurEnvoi::Identifiants);
    }
    let mut brut = format!("\0{}\0{}", acces.adresse, acces.mdp);
    let mut ligne = format!("AUTH PLAIN {}", base64(brut.as_bytes()));
    crate::effacer(&mut brut);
    let r = c.commande(&ligne);
    crate::effacer(&mut ligne);
    let r = r.map_err(ErreurEnvoi::Connexion)?;
    match r.code {
        235 => Ok(c),
        534 | 535 | 530 => Err(ErreurEnvoi::Identifiants),
        _ => Err(refus_ou(&r, &acces.mdp, ErreurEnvoi::Refus)),
    }
}

/// « Essayer » : se connecte et s'identifie, puis QUIT, SANS envoyer de mail (ni MAIL FROM, ni RCPT, ni DATA).
pub fn essayer_envoi(acces: &Acces) -> Result<(), ErreurEnvoi> {
    let r = ouvrir_session(acces).map(|mut c| {
        let _ = c.commande("QUIT");
    });
    r.map_err(|e| masquer(e, &acces.mdp))
}

/// Envoie UN mail à UN destinataire. Rend le Message-ID (sans < >) quand Gmail a répondu 250 après la fin du message.
pub fn envoyer(acces: &Acces, b: &Brouillon) -> Result<String, ErreurEnvoi> {
    let message = composer(b)?;
    if !b.de.eq_ignore_ascii_case(&acces.adresse) {
        return Err(ErreurEnvoi::Brouillon("l'expéditeur n'est pas la boîte branchée".into()));
    }
    let tout = || -> Result<String, ErreurEnvoi> {
        let mut c = ouvrir_session(acces)?;
        let r = c.commande(&format!("MAIL FROM:<{}>", b.de)).map_err(ErreurEnvoi::Connexion)?;
        if r.code != 250 {
            return Err(refus_ou(&r, &acces.mdp, ErreurEnvoi::ExpediteurRefuse));
        }
        let r = c.commande(&format!("RCPT TO:<{}>", b.a)).map_err(ErreurEnvoi::Connexion)?;
        if r.code != 250 && r.code != 251 {
            return Err(refus_ou(&r, &acces.mdp, ErreurEnvoi::DestinataireRefuse));
        }
        let r = c.commande("DATA").map_err(ErreurEnvoi::Connexion)?;
        if r.code != 354 {
            return Err(refus_ou(&r, &acces.mdp, ErreurEnvoi::Refus));
        }
        let data = pour_data(&message);
        // Le corps sans la fin : une coupure ici, le serveur n'a pas de message complet, rien ne part
        let (corps, fin) = data.split_at(data.len() - 3);
        c.ecrire(corps).map_err(ErreurEnvoi::Connexion)?;
        // La fin : à partir d'ici, une coupure laisse un doute
        c.ecrire(fin).map_err(|e| ErreurEnvoi::Incertain(e.to_string()))?;
        let r = c.reponse().map_err(|e| ErreurEnvoi::Incertain(e.to_string()))?;
        if r.code != 250 {
            return Err(refus_ou(&r, &acces.mdp, ErreurEnvoi::MessageRefuse));
        }
        let _ = c.commande("QUIT");
        Ok(b.message_id.clone())
    };
    tout().map_err(|e| masquer(e, &acces.mdp))
}

#[cfg(test)]
mod tests;
