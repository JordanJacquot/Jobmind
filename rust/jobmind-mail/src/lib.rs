//! Le Gmail de Jordan dans Jobmind (étapes 7 et 10 du plan) : IMAP pour lire les alertes d'offres (LinkedIn, Indeed,
//! Welcome to the Jungle, France Travail) et les réponses des recruteurs, SMTP (étape 10, [`smtp`]) pour envoyer une
//! candidature, toujours sur un clic de Jordan. Branchement par un mot de passe d'application Google, gardé chiffré sur
//! le PC (DPAPI), jamais dans la base.
//!
//! Étape 7 : la lecture, EN LECTURE SEULE. La boîte est ouverte par EXAMINE (jamais SELECT) et chaque mail est lu par
//! BODY.PEEK (rien n'est marqué lu) ; aucune commande qui change la boîte n'existe dans ce crate (voir
//! [`imap::Commande`]). Sans bibliothèque de mail : seulement les commandes utiles.
//!
//! - [`imap`] : le client IMAP, sur n'importe quel flux (TLS vers Gmail, TCP simple vers le faux serveur local) ;
//! - [`mime`] : la lecture d'un mail (RFC 5322 et MIME) en texte, qui ne panique et ne refuse jamais ;
//! - [`smtp`] (étape 10) : le mail composé (MIME, UTF-8, pièce jointe) et envoyé à UN destinataire, sur un clic ;
//!   « Essayer » vérifie la connexion et le mot de passe sans rien envoyer ;
//! - `faux` (les tests, et l'option « faux » de la construction d'essai) : un faux serveur IMAP local qui sert une
//!   boîte de mails inventés, et note chaque commande reçue (les tests vérifient que rien n'a été écrit).
//! - `faux_smtp` : un faux serveur SMTP local qui garde ce qu'il reçoit (aucun mail ne part) et imite les pannes.
//!
//! Le mot de passe n'est jamais écrit dans une erreur, le journal ou un `Debug` ; il est effacé de la mémoire dès que
//! l'accès est lâché.

pub mod imap;
pub mod mime;
pub mod smtp;

#[cfg(any(test, feature = "faux"))]
pub mod faux;
#[cfg(any(test, feature = "faux"))]
pub mod faux_smtp;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Le serveur IMAP de Gmail.
pub const HOTE_GMAIL: &str = "imap.gmail.com";
/// Le port IMAP chiffré (TLS dès la connexion).
pub const PORT_GMAIL: u16 = 993;

/// Au-delà de cette taille, seul le début d'un mail est lu (l'en-tête et les 300 000 premiers octets du corps).
pub const TAILLE_ENTIERE: u32 = 3 * 1024 * 1024;
/// Ce qui est lu du corps d'un mail trop gros.
pub const DEBUT_LU: u32 = 300_000;
/// Le délai pris quand l'accès n'en donne pas.
pub const DELAI_DEFAUT: Duration = Duration::from_secs(30);

/// Comment joindre la boîte. `tls: false` n'est accepté que pour l'hôte 127.0.0.1 (les tests, l'essai) : sinon
/// [`ErreurMail::TlsObligatoire`], sans rien envoyer.
#[derive(Clone)]
pub struct Acces {
    pub hote: String,
    pub port: u16,
    pub tls: bool,
    pub adresse: String,
    pub mdp: String,
    /// Le délai d'attente de chaque lecture et écriture sur la connexion (zéro : 30 s)
    pub delai: Duration,
}

impl Acces {
    /// L'accès à Gmail (imap.gmail.com, port 993, TLS), avec un délai de 30 s.
    pub fn gmail(adresse: &str, mdp: &str) -> Acces {
        Acces {
            hote: HOTE_GMAIL.into(),
            port: PORT_GMAIL,
            tls: true,
            adresse: adresse.into(),
            mdp: mdp.into(),
            delai: DELAI_DEFAUT,
        }
    }
}

impl std::fmt::Debug for Acces {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Acces")
            .field("hote", &self.hote)
            .field("port", &self.port)
            .field("tls", &self.tls)
            .field("adresse", &self.adresse)
            .field("mdp", &"*****")
            .field("delai", &self.delai)
            .finish()
    }
}

impl Drop for Acces {
    fn drop(&mut self) {
        effacer(&mut self.mdp);
    }
}

/// Écrase un texte secret par des zéros avant de le vider (comme `pont::effacer_valeur` de jobmind-app).
pub(crate) fn effacer(s: &mut String) {
    let n = s.len();
    s.clear();
    s.extend(std::iter::repeat_n('\0', n));
    std::hint::black_box(s.as_str());
    s.clear();
}

/// Où en était la dernière relève.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reperes {
    pub uidvalidity: Option<u32>,
    pub dernier_uid: Option<u32>,
    /// (année, mois, jour) : la première fois, ou si UIDVALIDITY a changé, seulement les mails reçus depuis ce jour
    /// (UID SEARCH SINCE). Une date impossible est ramenée dans les bornes ((0, 0, 0) : le 1er janvier 1970).
    pub depuis: (i32, u32, u32),
    /// Au plus tant de mails lus par relève (les plus anciens d'abord ; le reste au tour suivant). Zéro : un seul.
    pub max: usize,
}

/// La date d'un mail, telle qu'écrite dans son en-tête Date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateMail {
    pub annee: i32,
    pub mois: u32,
    pub jour: u32,
    pub heure: u32,
    pub minute: u32,
    pub seconde: u32,
    /// Le décalage avec l'heure universelle, en minutes (+0100 : 60)
    pub decalage_minutes: i32,
}

impl DateMail {
    /// « 2026-11-19T08:30:00 » : l'heure telle qu'écrite dans le mail (sans tenir compte du décalage)
    pub fn iso(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.annee, self.mois, self.jour, self.heure, self.minute, self.seconde
        )
    }

    /// L'instant du mail en secondes depuis le 1er janvier 1970 en heure universelle (le décalage retiré : « 08:30
    /// +0100 » est 07:30 en heure universelle). Rien : une date impossible (mois 13, 31 avril, 25 h...).
    pub fn secondes_utc(&self) -> Option<i64> {
        let bissextile = |a: i64| (a % 4 == 0 && a % 100 != 0) || a % 400 == 0;
        let a = i64::from(self.annee);
        let jours_du_mois = match self.mois {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if bissextile(a) => 29,
            2 => 28,
            _ => return None,
        };
        let valide = (1..=jours_du_mois).contains(&self.jour)
            && self.heure <= 23
            && self.minute <= 59
            && self.seconde <= 60
            && self.decalage_minutes.abs() <= 24 * 60;
        if !valide {
            return None;
        }
        // (les jours depuis le 1er janvier 1970, d'après l'algorithme « days from civil »)
        let (m, j) = (i64::from(self.mois), i64::from(self.jour));
        let y = if m <= 2 { a - 1 } else { a };
        let ere = y.div_euclid(400);
        let ya = y - ere * 400;
        let jour_annee = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + j - 1;
        let jour_ere = ya * 365 + ya / 4 - ya / 100 + jour_annee;
        let jours = ere * 146_097 + jour_ere - 719_468;
        let local =
            jours * 86_400 + i64::from(self.heure) * 3600 + i64::from(self.minute) * 60 + i64::from(self.seconde);
        Some(local - i64::from(self.decalage_minutes) * 60)
    }

    /// La même date en heure universelle (décalage 0), à convertir en heure locale par l'appli. Rien : une date
    /// impossible. `iso()` reste l'heure telle qu'écrite dans le mail.
    pub fn en_utc(&self) -> Option<DateMail> {
        let s = self.secondes_utc()?;
        let (jours, reste) = (s.div_euclid(86_400), s.rem_euclid(86_400));
        // (« civil from days », l'inverse)
        let z = jours + 719_468;
        let ere = z.div_euclid(146_097);
        let je = z - ere * 146_097;
        let ae = (je - je / 1460 + je / 36_524 - je / 146_096) / 365;
        let ja = je - (365 * ae + ae / 4 - ae / 100);
        let mp = (5 * ja + 2) / 153;
        let jour = ja - (153 * mp + 2) / 5 + 1;
        let mois = if mp < 10 { mp + 3 } else { mp - 9 };
        let annee = ae + ere * 400 + i64::from(mois <= 2);
        Some(DateMail {
            annee: i32::try_from(annee).ok()?,
            mois: mois as u32,
            jour: jour as u32,
            heure: (reste / 3600) as u32,
            minute: (reste % 3600 / 60) as u32,
            seconde: (reste % 60) as u32,
            decalage_minutes: 0,
        })
    }
}

/// Un mail lu.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Mail {
    pub uid: u32,
    /// Le Message-ID sans < > (absent : vide)
    pub message_id: String,
    /// L'adresse de l'expéditeur, en minuscules
    pub de_adresse: String,
    /// Le nom de l'expéditeur (décodé)
    pub de_nom: String,
    /// Reply-To s'il y en a un (minuscules), sinon vide
    pub repondre_a: String,
    /// Les adresses de To (minuscules)
    pub a: Vec<String>,
    /// L'objet, décodé (=?UTF-8?B?...?= etc.), sur une ligne
    pub objet: String,
    pub date: Option<DateMail>,
    /// In-Reply-To puis References : les Message-ID cités (sans < >), 20 au plus
    pub en_reponse_a: Vec<String>,
    /// Le texte du mail : la partie text/plain si elle existe, sinon le texte tiré de la partie HTML (balises,
    /// scripts et styles retirés, entités lues), remis au propre, 20 000 caractères au plus
    pub texte: String,
    /// La partie HTML décodée telle quelle (2 000 000 d'octets au plus), seulement pour lire les liens des alertes ;
    /// JAMAIS montrée à l'écran
    pub html: String,
    /// Le mail était trop gros : seul son début a été lu
    pub coupe: bool,
    /// Le nombre de pièces jointes (jamais gardées ni lues)
    pub pieces_jointes: usize,
    /// L'en-tête List-Unsubscribe ou Precedence: bulk existe (un envoi de masse)
    pub masse: bool,
}

/// Ce qu'une relève a rapporté.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Releve {
    pub uidvalidity: u32,
    /// Le plus grand UID lu (à garder dans les repères pour la relève suivante)
    pub dernier_uid: u32,
    pub mails: Vec<Mail>,
    /// Les mails nouveaux pas encore lus (au-delà de max) : lus au tour suivant
    pub restants: usize,
    /// Les mails qui n'ont pas pu être lus (abîmés) : passés
    pub illisibles: usize,
}

/// Ce qui empêche de lire la boîte. Aucun texte d'erreur ne contient le mot de passe.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurMail {
    /// Pas de connexion, nom introuvable, connexion coupée
    #[error("réseau : {0}")]
    Reseau(String),
    /// Le serveur met trop de temps à répondre
    #[error("délai dépassé")]
    Delai,
    /// Une connexion non chiffrée vers autre chose que 127.0.0.1 : rien n'est envoyé
    #[error("connexion non chiffrée refusée")]
    TlsObligatoire,
    /// La poignée de main TLS a échoué
    #[error("TLS : {0}")]
    Tls(String),
    /// NO [AUTHENTICATIONFAILED], mauvais mot de passe d'application, ou un identifiant impossible à envoyer
    #[error("identifiants refusés")]
    Identifiants,
    /// Le serveur refuse l'accès IMAP (IMAP désactivé, « Web login required », [ALERT], BYE)
    #[error("accès IMAP fermé : {0}")]
    ImapFerme(String),
    /// Une autre réponse NO ou BAD
    #[error("refus : {0}")]
    Refus(String),
    /// Une réponse du serveur qui ne se lit pas
    #[error("réponse illisible : {0}")]
    Illisible(String),
}

impl ErreurMail {
    /// Le code de l'erreur, pour la page et le journal.
    pub fn code(&self) -> &'static str {
        match self {
            ErreurMail::Reseau(_) => "reseau",
            ErreurMail::Delai => "delai",
            ErreurMail::TlsObligatoire => "tls_obligatoire",
            ErreurMail::Tls(_) => "tls",
            ErreurMail::Identifiants => "identifiants",
            ErreurMail::ImapFerme(_) => "imap_ferme",
            ErreurMail::Refus(_) => "refus",
            ErreurMail::Illisible(_) => "illisible",
        }
    }

    /// Une phrase simple pour Jordan, en français, sans jargon.
    pub fn message(&self) -> String {
        match self {
            ErreurMail::Reseau(_) => {
                "Jobmind n'arrive pas à joindre Gmail. Vérifie ta connexion à internet, puis réessaie.".into()
            }
            ErreurMail::Delai => "Gmail met trop de temps à répondre. Réessaie dans un moment.".into(),
            ErreurMail::TlsObligatoire => "La connexion à Gmail doit être protégée. Rien n'a été envoyé.".into(),
            ErreurMail::Tls(_) => "La connexion protégée avec Gmail n'a pas pu s'ouvrir. Vérifie la date et l'heure \
                                   de l'ordinateur, puis réessaie."
                .into(),
            ErreurMail::Identifiants => {
                "Gmail refuse ce mot de passe d'application. Vérifie l'adresse et recopie le mot de passe.".into()
            }
            ErreurMail::ImapFerme(_) => "Gmail ne laisse pas Jobmind lire ta boîte pour l'instant. Ouvre Gmail dans \
                                         ton navigateur pour voir s'il te demande quelque chose, puis réessaie."
                .into(),
            ErreurMail::Refus(_) => "Gmail a refusé une demande de Jobmind. Réessaie plus tard.".into(),
            ErreurMail::Illisible(_) => "La réponse de Gmail n'a pas pu être lue. Réessaie plus tard.".into(),
        }
    }
}

/// Retire le mot de passe (et sa forme sans espaces) du texte d'une erreur, et garde ce texte court.
fn masquer(e: ErreurMail, mdp: &str) -> ErreurMail {
    let m = |t: String| -> String {
        let mut t = t;
        if !mdp.is_empty() {
            t = t.replace(mdp, "*****");
            let colle: String = mdp.chars().filter(|c| !c.is_whitespace()).collect();
            if colle.len() >= 8 {
                t = t.replace(&colle, "*****");
            }
        }
        t.chars().take(200).collect()
    };
    match e {
        ErreurMail::Reseau(t) => ErreurMail::Reseau(m(t)),
        ErreurMail::Tls(t) => ErreurMail::Tls(m(t)),
        ErreurMail::ImapFerme(t) => ErreurMail::ImapFerme(m(t)),
        ErreurMail::Refus(t) => ErreurMail::Refus(m(t)),
        ErreurMail::Illisible(t) => ErreurMail::Illisible(m(t)),
        autre => autre,
    }
}

/// Un flux de la connexion (TLS vers Gmail, TCP simple vers 127.0.0.1).
trait Flux: Read + Write {}
impl<T: Read + Write> Flux for T {}

/// Ouvre la connexion : TCP avec délai sur chaque adresse trouvée, puis TLS (vérification normale des certificats).
fn ouvrir(acces: &Acces) -> Result<Box<dyn Flux>, ErreurMail> {
    if !acces.tls && acces.hote != "127.0.0.1" {
        return Err(ErreurMail::TlsObligatoire);
    }
    let hote = acces.hote.trim();
    if hote.is_empty() || hote.len() > 255 || hote.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ErreurMail::Reseau("adresse du serveur introuvable".into()));
    }
    let delai = if acces.delai.is_zero() { DELAI_DEFAUT } else { acces.delai };
    let mut adresses: Vec<SocketAddr> = (hote, acces.port)
        .to_socket_addrs()
        .map_err(|_| ErreurMail::Reseau("adresse du serveur introuvable".into()))?
        .collect();
    if !acces.tls {
        adresses.retain(|a| a.ip().is_loopback());
    }
    let mut derniere = ErreurMail::Reseau("adresse du serveur introuvable".into());
    let mut tcp = None;
    for a in &adresses {
        match TcpStream::connect_timeout(a, delai) {
            Ok(t) => {
                tcp = Some(t);
                break;
            }
            Err(e) => derniere = ErreurMail::Reseau(e.to_string()),
        }
    }
    let tcp = tcp.ok_or(derniere)?;
    let reglage = tcp
        .set_read_timeout(Some(delai))
        .and_then(|_| tcp.set_write_timeout(Some(delai)))
        .and_then(|_| tcp.set_nodelay(true));
    reglage.map_err(|e| ErreurMail::Reseau(e.to_string()))?;
    if !acces.tls {
        return Ok(Box::new(tcp));
    }
    let connecteur = native_tls::TlsConnector::builder()
        .min_protocol_version(Some(native_tls::Protocol::Tlsv12))
        .build()
        .map_err(|e| ErreurMail::Tls(e.to_string()))?;
    match connecteur.connect(hote, tcp) {
        Ok(s) => Ok(Box::new(s)),
        Err(native_tls::HandshakeError::Failure(e)) => Err(ErreurMail::Tls(e.to_string())),
        Err(native_tls::HandshakeError::WouldBlock(_)) => Err(ErreurMail::Delai),
    }
}

/// Une session : connexion, accueil, LOGIN, EXAMINE INBOX, le travail demandé, puis LOGOUT (seulement si tout s'est
/// bien passé : après une erreur, la connexion est simplement fermée).
fn session<R>(
    acces: &Acces,
    travail: impl FnOnce(&mut imap::Client<Box<dyn Flux>>, &imap::Boite) -> Result<R, ErreurMail>,
) -> Result<R, ErreurMail> {
    fn tout<R>(
        acces: &Acces,
        travail: impl FnOnce(&mut imap::Client<Box<dyn Flux>>, &imap::Boite) -> Result<R, ErreurMail>,
    ) -> Result<R, ErreurMail> {
        let flux = ouvrir(acces)?;
        let mut client = imap::Client::new(flux);
        client.accueil()?;
        client.capacites()?;
        client.connecter(&acces.adresse, &acces.mdp)?;
        let boite = client.examiner()?;
        let r = travail(&mut client, &boite)?;
        client.deconnecter();
        Ok(r)
    }
    tout(acces, travail).map_err(|e| masquer(e, &acces.mdp))
}

/// Se connecte, ouvre INBOX en lecture seule (EXAMINE), et rend le nombre de mails de la boîte ; puis LOGOUT (le
/// bouton « Essayer »).
pub fn essayer(acces: &Acces) -> Result<u32, ErreurMail> {
    session(acces, |_, boite| Ok(boite.existe))
}

/// La relève : les mails arrivés depuis la dernière fois (ou depuis `reperes.depuis` la première fois, ou si
/// UIDVALIDITY a changé), les plus anciens d'abord, `reperes.max` au plus ; le reste est compté dans `restants` et lu
/// au tour suivant (dernier_uid rendu = le plus grand UID lu, donc rien n'est perdu).
pub fn relever(acces: &Acces, reperes: &Reperes) -> Result<Releve, ErreurMail> {
    session(acces, |c, boite| {
        let max = reperes.max.max(1);
        let meme = reperes.uidvalidity == Some(boite.uidvalidity);
        let suite = match reperes.dernier_uid {
            Some(d) if meme && d > 0 => Some(d),
            _ => None,
        };
        let mut uids = match suite {
            // (« n:* » rend toujours au moins le dernier mail, même si son UID est plus petit que n : filtré)
            Some(d) if d == u32::MAX => Vec::new(),
            Some(d) => c.chercher_depuis_uid(d + 1)?.into_iter().filter(|&u| u > d).collect(),
            None => {
                let (a, m, j) = reperes.depuis;
                c.chercher_depuis_jour(a, m, j)?
            }
        };
        uids.sort_unstable();
        uids.dedup();
        let mut restants = uids.len().saturating_sub(max);
        uids.truncate(max);
        let mut mails = Vec::new();
        let mut illisibles = 0;
        // (au plus OCTETS_PAR_RELEVE lus : une relève finit toujours et dernier_uid avance ; le reste au tour suivant)
        let mut octets: u64 = 0;
        let mut lus = 0;
        for &uid in &uids {
            if lus > 0 && octets >= OCTETS_PAR_RELEVE {
                break;
            }
            let (m, n) = lire_un_compte(c, uid)?;
            octets += n;
            lus += 1;
            match m {
                Some(m) => mails.push(m),
                None => illisibles += 1,
            }
        }
        restants += uids.len() - lus;
        uids.truncate(lus);
        let dernier_uid = match (uids.last(), suite) {
            (Some(&u), _) => u,
            (None, Some(d)) => d,
            // (rien depuis le jour donné : tout ce qui est déjà dans la boîte est plus ancien)
            (None, None) => boite.uidnext.map(|n| n.saturating_sub(1)).unwrap_or(0),
        };
        Ok(Releve { uidvalidity: boite.uidvalidity, dernier_uid, mails, restants, illisibles })
    })
}

/// Une relève s'arrête après avoir lu environ autant d'octets (le mail en cours est fini) : avec des mails de 3 Mo, une
/// relève de 200 mails ferait 600 Mo et pourrait ne jamais finir avant une coupure.
pub const OCTETS_PAR_RELEVE: u64 = 20 * 1024 * 1024;

/// Lit les octets d'un mail en texte, sans jamais arrêter la relève : un mail qui ferait paniquer la lecture est passé
/// (rien : compté parmi les illisibles).
fn lire_protege(uid: u32, octets: &[u8]) -> Option<Mail> {
    lire_protege_avec(uid, octets, mime::lire)
}

fn lire_protege_avec(uid: u32, octets: &[u8], lire: impl FnOnce(u32, &[u8]) -> Mail) -> Option<Mail> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| lire(uid, octets))).ok()
}

/// Lit un mail : sa taille, puis tout (3 Mo au plus) ou son en-tête et le début de son corps. Rien : il n'a pas pu
/// être lu (disparu, refusé par le serveur, ou sa lecture a échoué). Rend aussi les octets reçus.
fn lire_un_compte<S: Read + Write>(c: &mut imap::Client<S>, uid: u32) -> Result<(Option<Mail>, u64), ErreurMail> {
    let Some(taille) = c.taille(uid)? else { return Ok((None, 0)) };
    if taille <= TAILLE_ENTIERE {
        return Ok(match c.corps(uid)? {
            Some(o) => (lire_protege(uid, &o), o.len() as u64),
            None => (None, 0),
        });
    }
    let Some(mut octets) = c.en_tete(uid)? else { return Ok((None, 0)) };
    if let Some(debut) = c.debut(uid, DEBUT_LU)? {
        octets.extend_from_slice(&debut);
    }
    let n = octets.len() as u64;
    let m = lire_protege(uid, &octets).map(|mut m| {
        m.coupe = true;
        m
    });
    Ok((m, n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::faux::{ConfigImap, FauxImap, ModeImap};

    const ADRESSE: &str = "jordan.essai@exemple.fr";
    const MDP: &str = "zqvx kmtr pwla hfeo";

    fn mail(n: u32, date: &str) -> Vec<u8> {
        format!(
            "From: Recruteur {n} <rh{n}@exemple-entreprise.fr>\r\nTo: {ADRESSE}\r\nSubject: Offre {n}\r\n\
             Date: {date}\r\nMessage-ID: <m{n}@exemple-entreprise.fr>\r\nContent-Type: text/plain; charset=utf-8\r\n\
             \r\nBonjour, voici l'offre num\u{e9}ro {n}.\r\n"
        )
        .into_bytes()
    }

    fn serveur(mails: Vec<Vec<u8>>) -> FauxImap {
        let r = FauxImap::demarrer(ConfigImap::new(ADRESSE, MDP, mails));
        r.expect("faux serveur")
    }

    fn reperes(depuis: (i32, u32, u32), max: usize) -> Reperes {
        Reperes { uidvalidity: None, dernier_uid: None, depuis, max }
    }

    fn mode(f: &FauxImap, m: ModeImap) {
        *f.mode().lock().expect("mode") = m;
    }

    /// Le test clé : rien d'écrit dans la boîte, seulement des lectures sans marquer lu, et jamais le mot de passe.
    fn verifier_lecture_seule(f: &FauxImap, erreurs: &[ErreurMail]) {
        assert!(f.refusees().is_empty(), "commandes refusées : {:?}", f.refusees());
        let interdits = ["STORE", "COPY", "MOVE", "EXPUNGE", "SELECT", "APPEND", "DELETE", "CREATE", "RENAME"];
        let commandes = f.commandes();
        assert!(!commandes.is_empty());
        for c in &commandes {
            let u = c.to_ascii_uppercase();
            let sans_uid = u.strip_prefix("UID ").unwrap_or(&u);
            for i in interdits {
                assert!(!sans_uid.starts_with(i), "commande interdite envoyée : {c}");
            }
            if u.contains("FETCH") {
                let reste = u.replace("BODY.PEEK[", "").replace("RFC822.SIZE", "");
                assert!(!reste.contains("BODY[") && !reste.contains("RFC822"), "lecture qui marque lu : {c}");
                assert!(u.contains("BODY.PEEK[") || u.contains("RFC822.SIZE"), "{c}");
            }
            assert!(!c.contains(MDP) && !c.contains("zqvxkmtrpwlahfeo"), "mot de passe noté : {c}");
        }
        for e in erreurs {
            for t in [e.to_string(), format!("{e:?}"), e.message()] {
                assert!(!t.contains(MDP) && !t.contains("zqvx"), "mot de passe dans une erreur : {t}");
            }
        }
    }

    #[test]
    fn essayer_rend_le_nombre_de_mails() {
        let f = serveur(vec![mail(1, "Mon, 17 Nov 2026 09:00:00 +0100"), mail(2, "Tue, 18 Nov 2026 09:00:00 +0100")]);
        assert_eq!(essayer(&f.acces(ADRESSE, MDP)), Ok(2));
        let c = f.commandes();
        assert_eq!(c.first().map(String::as_str), Some("CAPABILITY"));
        assert!(c.iter().any(|l| l == "EXAMINE INBOX"));
        assert!(c.iter().any(|l| l.starts_with("LOGIN \"jordan.essai@exemple.fr\" *****")));
        assert_eq!(c.last().map(String::as_str), Some("LOGOUT"));
        verifier_lecture_seule(&f, &[]);
    }

    #[test]
    fn les_refus_et_les_pannes() {
        let f = serveur(vec![mail(1, "Mon, 17 Nov 2026 09:00:00 +0100")]);
        let mut erreurs = Vec::new();
        let e = essayer(&f.acces(ADRESSE, "pas le bon")).expect_err("refus");
        assert_eq!(e, ErreurMail::Identifiants);
        erreurs.push(e);
        mode(&f, ModeImap::MauvaisMotDePasse);
        let e = essayer(&f.acces(ADRESSE, MDP)).expect_err("refus");
        assert_eq!(e, ErreurMail::Identifiants);
        erreurs.push(e);
        mode(&f, ModeImap::ImapFerme);
        let e = essayer(&f.acces(ADRESSE, MDP)).expect_err("fermé");
        assert!(matches!(e, ErreurMail::ImapFerme(_)), "{e:?}");
        erreurs.push(e);
        mode(&f, ModeImap::Bye);
        let e = essayer(&f.acces(ADRESSE, MDP)).expect_err("bye");
        assert!(matches!(e, ErreurMail::ImapFerme(_)), "{e:?}");
        erreurs.push(e);
        mode(&f, ModeImap::Coupe);
        let e = relever(&f.acces(ADRESSE, MDP), &reperes((2026, 11, 1), 10)).expect_err("coupé");
        assert!(matches!(e, ErreurMail::Reseau(_)), "{e:?}");
        erreurs.push(e);
        mode(&f, ModeImap::Lent);
        let mut a = f.acces(ADRESSE, MDP);
        a.delai = Duration::from_millis(300);
        let debut = std::time::Instant::now();
        let e = essayer(&a).expect_err("lent");
        assert_eq!(e, ErreurMail::Delai);
        assert!(debut.elapsed() < Duration::from_secs(5));
        erreurs.push(e);
        mode(&f, ModeImap::Normal);
        assert_eq!(essayer(&f.acces(ADRESSE, MDP)), Ok(1));
        verifier_lecture_seule(&f, &erreurs);
    }

    #[test]
    fn sans_tls_seulement_vers_127_0_0_1() {
        for hote in ["localhost", "192.0.2.1", "imap.exemple.invalid", "127.0.0.2", ""] {
            let a = Acces {
                hote: hote.into(),
                port: 1,
                tls: false,
                adresse: ADRESSE.into(),
                mdp: MDP.into(),
                delai: Duration::from_millis(100),
            };
            let debut = std::time::Instant::now();
            assert_eq!(essayer(&a), Err(ErreurMail::TlsObligatoire), "{hote}");
            assert_eq!(relever(&a, &Reperes::default()), Err(ErreurMail::TlsObligatoire));
            assert!(debut.elapsed() < Duration::from_millis(50), "aucune connexion tentée");
        }
    }

    #[test]
    fn port_ferme_donne_reseau() {
        // (un port libéré tout de suite : personne n'écoute)
        let port = std::net::TcpListener::bind("127.0.0.1:0").and_then(|l| l.local_addr()).map(|a| a.port());
        let a = Acces {
            hote: "127.0.0.1".into(),
            port: port.expect("port"),
            tls: false,
            adresse: ADRESSE.into(),
            mdp: MDP.into(),
            delai: Duration::from_secs(2),
        };
        let e = essayer(&a).expect_err("personne");
        assert!(matches!(e, ErreurMail::Reseau(_)), "{e:?}");
    }

    #[test]
    fn la_releve_complete_sans_rien_perdre() {
        let f = serveur(vec![
            mail(1, "Sat, 1 Nov 2026 09:00:00 +0100"),
            mail(2, "Mon, 10 Nov 2026 09:00:00 +0100"),
            mail(3, "Tue, 18 Nov 2026 09:00:00 +0100"),
        ]);
        let a = f.acces(ADRESSE, MDP);
        // La première fois : depuis le 5 novembre
        let r = relever(&a, &reperes((2026, 11, 5), 50)).expect("relève");
        assert_eq!(r.uidvalidity, 1);
        assert_eq!(r.mails.iter().map(|m| m.uid).collect::<Vec<_>>(), vec![2, 3]);
        assert_eq!((r.dernier_uid, r.restants, r.illisibles), (3, 0, 0));
        assert_eq!(r.mails[0].objet, "Offre 2");
        assert_eq!(r.mails[1].texte, "Bonjour, voici l'offre num\u{e9}ro 3.");
        let mut rep = Reperes {
            uidvalidity: Some(r.uidvalidity),
            dernier_uid: Some(r.dernier_uid),
            depuis: (2026, 11, 5),
            max: 50,
        };
        // Rien de neuf (le serveur rend quand même le dernier mail pour « 4:* » : filtré)
        let r = relever(&a, &rep).expect("relève");
        assert!(r.mails.is_empty());
        assert_eq!((r.dernier_uid, r.restants), (3, 0));
        // Deux nouveaux : seuls eux
        f.ajouter(mail(4, "Wed, 19 Nov 2026 08:00:00 +0100"));
        f.ajouter(mail(5, "Wed, 19 Nov 2026 08:30:00 +0100"));
        let r = relever(&a, &rep).expect("relève");
        assert_eq!(r.mails.iter().map(|m| m.uid).collect::<Vec<_>>(), vec![4, 5]);
        rep.dernier_uid = Some(r.dernier_uid);
        // Cinq nouveaux, deux par relève : 2, puis 2, puis 1, sans en perdre
        for n in 6..=10 {
            f.ajouter(mail(n, "Wed, 19 Nov 2026 09:00:00 +0100"));
        }
        rep.max = 2;
        let mut lus = Vec::new();
        let mut restes = Vec::new();
        for _ in 0..4 {
            let r = relever(&a, &rep).expect("relève");
            lus.extend(r.mails.iter().map(|m| m.uid));
            restes.push(r.restants);
            rep.dernier_uid = Some(r.dernier_uid);
        }
        assert_eq!(lus, vec![6, 7, 8, 9, 10]);
        assert_eq!(restes, vec![3, 1, 0, 0]);
        // UIDVALIDITY change : tout est relu depuis la date
        f.regler_uidvalidity(7);
        rep.max = 50;
        let r = relever(&a, &rep).expect("relève");
        assert_eq!(r.uidvalidity, 7);
        assert_eq!(r.mails.len(), 9);
        assert_eq!(r.mails.first().map(|m| m.uid), Some(2));
        verifier_lecture_seule(&f, &[]);
    }

    #[test]
    fn rien_depuis_la_date_puis_la_suite_par_uid() {
        let f = serveur(vec![mail(1, "Sat, 1 Nov 2026 09:00:00 +0100"), mail(2, "Sun, 2 Nov 2026 09:00:00 +0100")]);
        let a = f.acces(ADRESSE, MDP);
        let r = relever(&a, &reperes((2026, 11, 19), 10)).expect("relève");
        assert!(r.mails.is_empty());
        // (UIDNEXT - 1 : la relève suivante ne relit pas la boîte entière)
        assert_eq!(r.dernier_uid, 2);
        f.ajouter(mail(3, "Sat, 1 Nov 2025 09:00:00 +0100"));
        let rep = Reperes { uidvalidity: Some(1), dernier_uid: Some(2), depuis: (2026, 11, 19), max: 10 };
        let r = relever(&a, &rep).expect("relève");
        assert_eq!(r.mails.iter().map(|m| m.uid).collect::<Vec<_>>(), vec![3]);
        verifier_lecture_seule(&f, &[]);
    }

    #[test]
    fn un_mail_trop_gros_est_lu_en_partie() {
        let mut gros = b"From: rh@exemple.fr\r\nSubject: Gros\r\nDate: Wed, 19 Nov 2026 08:00:00 +0100\r\n\
                         Content-Type: text/plain\r\n\r\nD\xc3\xa9but du mail.\r\n"
            .to_vec();
        while gros.len() < 3_600_000 {
            gros.extend_from_slice(b"Une ligne de remplissage pour un tr\xc3\xa8s gros mail.\r\n");
        }
        let f = serveur(vec![mail(1, "Wed, 19 Nov 2026 07:00:00 +0100"), gros]);
        let r = relever(&f.acces(ADRESSE, MDP), &reperes((2026, 11, 19), 10)).expect("relève");
        assert_eq!(r.mails.len(), 2);
        assert!(!r.mails[0].coupe);
        let m = &r.mails[1];
        assert!(m.coupe);
        assert_eq!(m.objet, "Gros");
        assert!(m.texte.starts_with("D\u{e9}but du mail."));
        assert!(m.texte.chars().count() <= 20_000);
        let c = f.commandes();
        assert!(c.iter().any(|l| l.contains("BODY.PEEK[HEADER]")));
        assert!(c.iter().any(|l| l.contains("BODY.PEEK[TEXT]<0.300000>")));
        verifier_lecture_seule(&f, &[]);
    }

    /// Des mails de près de 3 Mo : une relève s'arrête vers 20 Mo (avant : jusqu'à `max` mails, des centaines de Mo),
    /// dernier_uid avance, et la suite vient au tour suivant sans rien perdre.
    #[test]
    fn la_releve_est_plafonnee_en_octets() {
        let gros = |n: u32| {
            let mut m = mail(n, "Wed, 19 Nov 2026 08:00:00 +0100");
            while m.len() < 3_050_000 {
                m.extend_from_slice(b"Une ligne de remplissage pour un gros mail.\r\n");
            }
            m
        };
        let f = serveur((1..=10).map(gros).collect());
        let a = f.acces(ADRESSE, MDP);
        let r = relever(&a, &reperes((2026, 11, 19), 50)).expect("relève");
        assert_eq!(r.mails.len(), 7);
        assert_eq!((r.dernier_uid, r.restants, r.illisibles), (7, 3, 0));
        let rep = Reperes { uidvalidity: Some(1), dernier_uid: Some(r.dernier_uid), depuis: (2026, 11, 19), max: 50 };
        let r = relever(&a, &rep).expect("relève");
        assert_eq!(r.mails.iter().map(|m| m.uid).collect::<Vec<_>>(), vec![8, 9, 10]);
        assert_eq!((r.dernier_uid, r.restants), (10, 0));
        verifier_lecture_seule(&f, &[]);
    }

    /// L'instant d'un mail en heure universelle : le décalage est retiré (avant : seule l'heure écrite, décalage
    /// ignoré), jour, mois et année changés au besoin ; une date impossible ne donne rien.
    #[test]
    fn la_date_en_heure_universelle() {
        let d = |annee, mois, jour, heure, minute, decalage_minutes| DateMail {
            annee,
            mois,
            jour,
            heure,
            minute,
            seconde: 5,
            decalage_minutes,
        };
        let iso = |x: DateMail| x.en_utc().map(|u| u.iso());
        assert_eq!(iso(d(2026, 11, 19, 8, 30, 60)).as_deref(), Some("2026-11-19T07:30:05"));
        assert_eq!(iso(d(2026, 11, 19, 8, 30, -300)).as_deref(), Some("2026-11-19T13:30:05"));
        // (passage au jour, au mois et à l'année d'avant ou d'après, années bissextiles)
        assert_eq!(iso(d(2027, 1, 1, 0, 15, 60)).as_deref(), Some("2026-12-31T23:15:05"));
        assert_eq!(iso(d(2028, 2, 28, 23, 0, -120)).as_deref(), Some("2028-02-29T01:00:05"));
        assert_eq!(iso(d(2026, 2, 28, 23, 0, -120)).as_deref(), Some("2026-03-01T01:00:05"));
        assert_eq!(iso(d(2026, 3, 29, 1, 0, 0)).as_deref(), Some("2026-03-29T01:00:05"));
        assert_eq!(d(1970, 1, 1, 1, 0, 60).secondes_utc(), Some(5));
        assert_eq!(d(2026, 11, 19, 8, 30, 60).secondes_utc(), Some(1_795_073_405));
        // (l'heure écrite reste la même)
        assert_eq!(d(2026, 11, 19, 8, 30, 60).iso(), "2026-11-19T08:30:05");
        for faux in [d(2026, 13, 1, 0, 0, 0), d(2026, 4, 31, 0, 0, 0), d(2026, 2, 29, 0, 0, 0), d(2026, 1, 1, 24, 0, 0)]
        {
            assert_eq!(faux.en_utc(), None, "{faux:?}");
        }
        // Depuis un vrai en-tête Date
        let m = mime::lire(1, &mail(1, "Wed, 19 Nov 2026 08:30:05 +0100"));
        assert_eq!(m.date.and_then(|x| x.en_utc()).map(|u| u.iso()).as_deref(), Some("2026-11-19T07:30:05"));
    }

    /// Un mail dont la lecture ferait paniquer est passé (compté illisible) : la relève continue.
    #[test]
    fn un_mail_qui_fait_paniquer_est_passe() {
        let r = lire_protege_avec(3, b"x", |_, _| panic!("lecture impossible"));
        assert!(r.is_none());
        let r = lire_protege_avec(4, &mail(4, "Wed, 19 Nov 2026 08:00:00 +0100"), mime::lire);
        assert_eq!(r.map(|m| m.uid), Some(4));
    }

    #[test]
    fn un_mail_disparu_compte_illisible() {
        let f = serveur(vec![mail(1, "Wed, 19 Nov 2026 07:00:00 +0100"), mail(2, "Wed, 19 Nov 2026 07:00:00 +0100")]);
        f.retirer_au_fetch(2);
        let r = relever(&f.acces(ADRESSE, MDP), &reperes((2026, 11, 19), 10)).expect("relève");
        assert_eq!(r.mails.len(), 1);
        assert_eq!((r.illisibles, r.dernier_uid), (1, 2));
        verifier_lecture_seule(&f, &[]);
    }

    #[test]
    fn le_mot_de_passe_ne_s_imprime_jamais() {
        let a = Acces::gmail(ADRESSE, MDP);
        let d = format!("{a:?}");
        assert!(!d.contains(MDP) && d.contains("*****"), "{d}");
        assert_eq!((a.hote.as_str(), a.port, a.tls), (HOTE_GMAIL, 993, true));
        let e = masquer(ErreurMail::Refus(format!("echo {MDP} et zqvxkmtrpwlahfeo")), MDP);
        assert!(!e.to_string().contains("zqvx"), "{e}");
        let mut s = String::from("secret");
        effacer(&mut s);
        assert!(s.is_empty());
    }

    #[test]
    fn messages_simples_sans_tiret_long() {
        let toutes = [
            ErreurMail::Reseau("x".into()),
            ErreurMail::Delai,
            ErreurMail::TlsObligatoire,
            ErreurMail::Tls("x".into()),
            ErreurMail::Identifiants,
            ErreurMail::ImapFerme("x".into()),
            ErreurMail::Refus("x".into()),
            ErreurMail::Illisible("x".into()),
        ];
        let mut codes = std::collections::HashSet::new();
        for e in &toutes {
            assert!(codes.insert(e.code()));
            let m = e.message();
            assert!(m.ends_with('.') && !m.contains('\u{2014}') && !m.contains('\u{2013}'), "{m}");
            assert!(!m.contains("IMAP") && !m.contains("TLS"), "pas de jargon : {m}");
        }
    }

    #[test]
    fn aucun_tiret_long_dans_le_crate() {
        let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut fichiers = vec![dossier.join("Cargo.toml")];
        if let Ok(l) = std::fs::read_dir(dossier.join("src")) {
            for e in l.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if let Ok(s) = std::fs::read_dir(&p) {
                        fichiers.extend(s.flatten().map(|x| x.path()).filter(|x| x.is_file()));
                    }
                } else {
                    fichiers.push(p);
                }
            }
        }
        assert!(fichiers.len() >= 5);
        for f in fichiers {
            let t = std::fs::read_to_string(&f).expect("lecture");
            assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'), "tiret long dans {}", f.display());
        }
    }
}
