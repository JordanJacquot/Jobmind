//! Un faux serveur IMAP local (127.0.0.1, un port libre, TCP simple), pour les tests du client et pour la
//! construction d'essai de l'appli (option « faux », jamais livrée) : aucune connexion ne part sur internet.
//!
//! Il sert une boîte INBOX faite de mails bruts donnés (inventés), avec une adresse et un mot de passe attendus
//! (inventés eux aussi), et imite ce que fait Gmail : UIDVALIDITY, UIDNEXT, « n:* » qui rend toujours le dernier
//! mail, les refus de LOGIN. Son mode change pendant qu'il tourne ([`ModeImap`]) et on peut lui ajouter des mails
//! (la relève suivante).
//!
//! Il NOTE chaque commande reçue (le mot de passe remplacé par *****) et REFUSE (BAD) tout ce qui écrirait dans la
//! boîte (STORE, COPY, MOVE, EXPUNGE, SELECT, APPEND, DELETE, CREATE, RENAME...), toute lecture qui marquerait un
//! mail lu (BODY[ sans PEEK, RFC822) et toute commande inconnue, en les notant à part : les tests vérifient que
//! cette liste reste vide.

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Ce que fait le serveur (réglable pendant qu'il tourne).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeImap {
    /// Comme Gmail
    #[default]
    Normal,
    /// LOGIN toujours refusé (NO [AUTHENTICATIONFAILED])
    MauvaisMotDePasse,
    /// LOGIN refusé par NO [ALERT] (IMAP désactivé)
    ImapFerme,
    /// L'accueil est « * BYE » (le serveur ne veut pas de la connexion)
    Bye,
    /// La connexion est fermée au milieu d'une réponse
    Coupe,
    /// Le serveur ne répond plus
    Lent,
}

/// La boîte de départ.
#[derive(Debug, Clone)]
pub struct ConfigImap {
    pub adresse: String,
    pub mdp: String,
    pub uidvalidity: u32,
    pub mails: Vec<Vec<u8>>,
    /// Les UID des mails (vide : 1, 2, 3...)
    pub uids: Vec<u32>,
}

impl ConfigImap {
    pub fn new(adresse: &str, mdp: &str, mails: Vec<Vec<u8>>) -> ConfigImap {
        ConfigImap { adresse: adresse.into(), mdp: mdp.into(), uidvalidity: 1, mails, uids: Vec::new() }
    }
}

struct MailFaux {
    uid: u32,
    octets: Vec<u8>,
    /// Le jour de réception (pris dans l'en-tête Date ; sans date : toujours « récent »)
    jour: Option<(i32, u32, u32)>,
}

struct Boite {
    uidvalidity: u32,
    mails: Vec<MailFaux>,
    prochain: u32,
    /// Les UID qui ne rendent plus rien au FETCH (un mail effacé entre la recherche et la lecture)
    disparus: Vec<u32>,
}

struct Partage {
    adresse: String,
    mdp: String,
    boite: Mutex<Boite>,
    mode: Arc<Mutex<ModeImap>>,
    commandes: Mutex<Vec<String>>,
    refusees: Mutex<Vec<String>>,
    fin: AtomicBool,
}

fn verrou<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

fn mail_faux(uid: u32, octets: Vec<u8>) -> MailFaux {
    let jour = crate::mime::lire(uid, &octets).date.map(|d| (d.annee, d.mois, d.jour));
    MailFaux { uid, octets, jour }
}

/// Le serveur, arrêté quand il est lâché.
pub struct FauxImap {
    port: u16,
    partage: Arc<Partage>,
}

impl FauxImap {
    /// Démarre sur un port libre de 127.0.0.1, sur un fil à part.
    pub fn demarrer(config: ConfigImap) -> std::io::Result<FauxImap> {
        let ecoute = TcpListener::bind("127.0.0.1:0")?;
        let port = ecoute.local_addr()?.port();
        let mut mails: Vec<MailFaux> = config
            .mails
            .into_iter()
            .enumerate()
            .map(|(i, o)| mail_faux(config.uids.get(i).copied().unwrap_or(i as u32 + 1), o))
            .collect();
        mails.sort_by_key(|m| m.uid);
        let prochain = mails.last().map(|m| m.uid + 1).unwrap_or(1);
        let partage = Arc::new(Partage {
            adresse: config.adresse,
            mdp: config.mdp,
            boite: Mutex::new(Boite { uidvalidity: config.uidvalidity, mails, prochain, disparus: Vec::new() }),
            mode: Arc::new(Mutex::new(ModeImap::Normal)),
            commandes: Mutex::new(Vec::new()),
            refusees: Mutex::new(Vec::new()),
            fin: AtomicBool::new(false),
        });
        let p = Arc::clone(&partage);
        std::thread::Builder::new().name("faux-imap".into()).spawn(move || {
            for c in ecoute.incoming() {
                if p.fin.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(c) = c else { continue };
                let p2 = Arc::clone(&p);
                let _ = std::thread::Builder::new().name("faux-imap-client".into()).spawn(move || servir(c, &p2));
            }
        })?;
        Ok(FauxImap { port, partage })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// « 127.0.0.1 »
    pub fn adresse(&self) -> String {
        "127.0.0.1".into()
    }

    /// L'accès à ce serveur (sans TLS, délai de 5 s).
    pub fn acces(&self, adresse: &str, mdp: &str) -> crate::Acces {
        crate::Acces {
            hote: self.adresse(),
            port: self.port,
            tls: false,
            adresse: adresse.into(),
            mdp: mdp.into(),
            delai: Duration::from_secs(5),
        }
    }

    /// Les commandes reçues jusqu'ici, sans étiquette (le mot de passe remplacé par *****).
    pub fn commandes(&self) -> Vec<String> {
        verrou(&self.partage.commandes).clone()
    }

    /// Les commandes refusées (écriture, lecture qui marque lu, inconnue).
    pub fn refusees(&self) -> Vec<String> {
        verrou(&self.partage.refusees).clone()
    }

    /// Ajoute un mail à la boîte ; rend son UID.
    pub fn ajouter(&self, octets: Vec<u8>) -> u32 {
        let mut b = verrou(&self.partage.boite);
        let uid = b.prochain;
        b.prochain += 1;
        b.mails.push(mail_faux(uid, octets));
        uid
    }

    /// Change UIDVALIDITY (la boîte a été recréée : les UID connus ne valent plus rien).
    pub fn regler_uidvalidity(&self, v: u32) {
        verrou(&self.partage.boite).uidvalidity = v;
    }

    /// Ce mail est toujours trouvé par SEARCH, mais FETCH ne rend plus rien (effacé entre les deux).
    pub fn retirer_au_fetch(&self, uid: u32) {
        verrou(&self.partage.boite).disparus.push(uid);
    }

    /// Le mode, réglable pendant que le serveur tourne.
    pub fn mode(&self) -> Arc<Mutex<ModeImap>> {
        Arc::clone(&self.partage.mode)
    }
}

impl Drop for FauxImap {
    fn drop(&mut self) {
        self.partage.fin.store(true, Ordering::SeqCst);
        // (une connexion pour débloquer l'attente)
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.port).into(), Duration::from_millis(200));
    }
}

/// Lit une ligne, en surveillant l'arrêt du serveur ; rien : connexion finie (fermée, arrêt, 10 s sans rien).
fn lire_ligne(l: &mut BufReader<TcpStream>, p: &Partage) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    let debut = Instant::now();
    loop {
        if p.fin.load(Ordering::SeqCst) || debut.elapsed() > Duration::from_secs(10) || v.len() > 1_000_000 {
            return None;
        }
        match l.read_until(b'\n', &mut v) {
            Ok(0) => return None,
            Ok(_) if v.ends_with(b"\n") => {
                while v.last().is_some_and(|c| *c == b'\n' || *c == b'\r') {
                    v.pop();
                }
                return Some(v);
            }
            Ok(_) => {}
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
            Err(_) => return None,
        }
    }
}

/// Ne répond plus, jusqu'à ce que le client parte (ou l'arrêt, ou 30 s).
fn attendre(l: &mut BufReader<TcpStream>, p: &Partage) {
    let debut = Instant::now();
    let mut v = Vec::new();
    while !p.fin.load(Ordering::SeqCst) && debut.elapsed() < Duration::from_secs(30) {
        v.clear();
        match l.read_until(b'\n', &mut v) {
            Ok(0) => return,
            Ok(_) => {}
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted) => {}
            Err(_) => return,
        }
    }
}

/// Les mots d'une commande (atomes, chaînes entre guillemets sans leurs guillemets, groupes entre parenthèses
/// gardés entiers).
fn mots(t: &str) -> Vec<String> {
    let mut r = Vec::new();
    let mut ch = t.chars().peekable();
    while let Some(&c) = ch.peek() {
        if c == ' ' {
            ch.next();
            continue;
        }
        let mut m = String::new();
        if c == '"' {
            ch.next();
            while let Some(c) = ch.next() {
                match c {
                    '\\' => m.extend(ch.next()),
                    '"' => break,
                    _ => m.push(c),
                }
            }
        } else if c == '(' {
            let mut prof = 0;
            for c in ch.by_ref() {
                m.push(c);
                match c {
                    '(' => prof += 1,
                    ')' => {
                        prof -= 1;
                        if prof == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        } else {
            while let Some(&c) = ch.peek() {
                if c == ' ' {
                    break;
                }
                m.push(c);
                ch.next();
            }
        }
        r.push(m);
    }
    r
}

const MOIS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

fn jour_imap(t: &str) -> Option<(i32, u32, u32)> {
    let mut p = t.split('-');
    let j: u32 = p.next()?.parse().ok()?;
    let nom = p.next()?;
    let m = MOIS.iter().position(|x| x.eq_ignore_ascii_case(nom))? as u32 + 1;
    let a: i32 = p.next()?.parse().ok()?;
    Some((a, m, j))
}

/// Les éléments FETCH permis (tout le reste est refusé : en particulier BODY[ sans PEEK et RFC822).
fn element_permis(e: &str) -> bool {
    let e = e.to_ascii_uppercase();
    matches!(e.as_str(), "UID" | "RFC822.SIZE" | "FLAGS" | "INTERNALDATE" | "BODY.PEEK[]" | "BODY.PEEK[HEADER]")
        || (e.starts_with("BODY.PEEK[TEXT]")
            && (e == "BODY.PEEK[TEXT]"
                || e.strip_prefix("BODY.PEEK[TEXT]<0.")
                    .and_then(|r| r.strip_suffix('>'))
                    .is_some_and(|n| n.parse::<u32>().is_ok())))
}

fn servir(c: TcpStream, p: &Partage) {
    let _ = c.set_read_timeout(Some(Duration::from_millis(50)));
    let Ok(mut ecrit) = c.try_clone() else { return };
    let mut l = BufReader::new(c);
    let mode = || *verrou(&p.mode);
    match mode() {
        ModeImap::Lent => return attendre(&mut l, p),
        ModeImap::Bye => {
            let _ = ecrit.write_all(b"* BYE [UNAVAILABLE] Faux serveur indisponible\r\n");
            return;
        }
        _ => {}
    }
    if ecrit.write_all(b"* OK [CAPABILITY IMAP4rev1 AUTH=PLAIN] Faux IMAP pret\r\n").is_err() {
        return;
    }
    let mut connecte = false;
    while let Some(ligne) = lire_ligne(&mut l, p) {
        let texte = String::from_utf8_lossy(&ligne).into_owned();
        let (etiquette, reste) = texte.split_once(' ').unwrap_or((texte.as_str(), ""));
        let m = mots(reste);
        let verbe = m.first().map(|v| v.to_ascii_uppercase()).unwrap_or_default();
        // (noter la commande, sans le mot de passe)
        let note = if verbe == "LOGIN" {
            format!("LOGIN \"{}\" *****", m.get(1).map(String::as_str).unwrap_or(""))
        } else {
            reste.to_string()
        };
        verrou(&p.commandes).push(note.clone());
        match mode() {
            ModeImap::Lent => return attendre(&mut l, p),
            ModeImap::Coupe => {
                let _ = ecrit.write_all(b"* 1 EXI");
                let _ = ecrit.flush();
                return;
            }
            _ => {}
        }
        let r = repondre(p, etiquette, &verbe, &m, &note, &mut connecte);
        if ecrit.write_all(&r).and_then(|_| ecrit.flush()).is_err() || verbe == "LOGOUT" {
            return;
        }
    }
}

fn refuser(p: &Partage, etiquette: &str, note: &str) -> Vec<u8> {
    verrou(&p.refusees).push(note.to_string());
    format!("{etiquette} BAD Commande refusee par le faux serveur (lecture seule)\r\n").into_bytes()
}

fn repondre(p: &Partage, etiquette: &str, verbe: &str, m: &[String], note: &str, connecte: &mut bool) -> Vec<u8> {
    let ok = |t: &str| format!("{etiquette} OK {t}\r\n").into_bytes();
    match verbe {
        "CAPABILITY" => {
            let mut r =
                b"* CAPABILITY IMAP4rev1 UNSELECT IDLE NAMESPACE QUOTA ID XLIST CHILDREN AUTH=PLAIN\r\n".to_vec();
            r.extend(ok("Thats all she wrote!"));
            r
        }
        "NOOP" => ok("Success"),
        "LOGOUT" => {
            let mut r = b"* BYE LOGOUT Requested\r\n".to_vec();
            r.extend(ok("73 good day (Success)"));
            r
        }
        "LOGIN" => {
            let mode = *verrou(&p.mode);
            let bons = m.len() == 3 && m[1].eq_ignore_ascii_case(&p.adresse) && m[2] == p.mdp;
            match mode {
                ModeImap::ImapFerme => format!(
                    "{etiquette} NO [ALERT] Your account is not enabled for IMAP use. Please visit your Gmail settings \
                     page and enable your account for IMAP access. (Failure)\r\n"
                )
                .into_bytes(),
                _ if mode == ModeImap::MauvaisMotDePasse || !bons => {
                    format!("{etiquette} NO [AUTHENTICATIONFAILED] Invalid credentials (Failure)\r\n").into_bytes()
                }
                _ => {
                    *connecte = true;
                    let mut r = b"* CAPABILITY IMAP4rev1 UNSELECT IDLE NAMESPACE QUOTA ID XLIST CHILDREN\r\n".to_vec();
                    r.extend(ok("jordan authenticated (Success)"));
                    r
                }
            }
        }
        "EXAMINE" if !*connecte => format!("{etiquette} NO Not authenticated\r\n").into_bytes(),
        "EXAMINE" => {
            if !m.get(1).is_some_and(|b| b.eq_ignore_ascii_case("INBOX")) || m.len() != 2 {
                return format!("{etiquette} NO [NONEXISTENT] Unknown Mailbox (Failure)\r\n").into_bytes();
            }
            let b = verrou(&p.boite);
            let mut r = format!(
                "* FLAGS (\\Answered \\Flagged \\Draft \\Deleted \\Seen $NotPhishing $Phishing)\r\n\
                 * OK [PERMANENTFLAGS ()] Flags permitted.\r\n* OK [UIDVALIDITY {}] UIDs valid.\r\n* {} EXISTS\r\n\
                 * 0 RECENT\r\n* OK [UIDNEXT {}] Predicted next UID.\r\n* OK [HIGHESTMODSEQ 1000]\r\n",
                b.uidvalidity,
                b.mails.len(),
                b.prochain
            )
            .into_bytes();
            r.extend(ok("[READ-ONLY] INBOX selected. (Success)"));
            r
        }
        "UID" if *connecte => {
            let sous = m.get(1).map(|v| v.to_ascii_uppercase()).unwrap_or_default();
            match sous.as_str() {
                "SEARCH" => chercher(p, etiquette, &m[2..], note),
                "FETCH" => lire(p, etiquette, &m[2..], note),
                _ => refuser(p, etiquette, note),
            }
        }
        _ => refuser(p, etiquette, note),
    }
}

fn chercher(p: &Partage, etiquette: &str, m: &[String], note: &str) -> Vec<u8> {
    let b = verrou(&p.boite);
    let uids: Vec<u32> = match (m.first().map(|v| v.to_ascii_uppercase()).as_deref(), m.get(1), m.len()) {
        (Some("UID"), Some(plage), 2) => {
            let Some(n) = plage.strip_suffix(":*").and_then(|n| n.parse::<u32>().ok()) else {
                drop(b);
                return refuser(p, etiquette, note);
            };
            let mut u: Vec<u32> = b.mails.iter().map(|m| m.uid).filter(|&u| u >= n).collect();
            // (comme Gmail : « n:* » rend toujours le dernier mail)
            if u.is_empty() {
                u.extend(b.mails.last().map(|m| m.uid));
            }
            u
        }
        (Some("SINCE"), Some(jour), 2) => {
            let Some(j) = jour_imap(jour) else {
                drop(b);
                return refuser(p, etiquette, note);
            };
            b.mails.iter().filter(|m| m.jour.is_none_or(|d| d >= j)).map(|m| m.uid).collect()
        }
        _ => {
            drop(b);
            return refuser(p, etiquette, note);
        }
    };
    let liste: Vec<String> = uids.iter().map(u32::to_string).collect();
    let mut r = if liste.is_empty() { "* SEARCH\r\n".to_string() } else { format!("* SEARCH {}\r\n", liste.join(" ")) };
    r.push_str(&format!("{etiquette} OK SEARCH completed (Success)\r\n"));
    r.into_bytes()
}

fn lire(p: &Partage, etiquette: &str, m: &[String], note: &str) -> Vec<u8> {
    let (Some(ensemble), Some(elements), 2) = (m.first(), m.get(1), m.len()) else {
        return refuser(p, etiquette, note);
    };
    let elements: Vec<&str> =
        elements.trim_start_matches('(').trim_end_matches(')').split(' ').filter(|e| !e.is_empty()).collect();
    if elements.is_empty() || !elements.iter().all(|e| element_permis(e)) {
        return refuser(p, etiquette, note);
    }
    let Some(voulus) = ensemble.split(',').map(|u| u.parse::<u32>().ok()).collect::<Option<Vec<u32>>>() else {
        return refuser(p, etiquette, note);
    };
    let b = verrou(&p.boite);
    let mut r = Vec::new();
    for (i, mail) in b.mails.iter().enumerate() {
        if !voulus.contains(&mail.uid) || b.disparus.contains(&mail.uid) {
            continue;
        }
        let (tete, texte) = match mail.octets.windows(4).position(|w| w == b"\r\n\r\n") {
            Some(p) => mail.octets.split_at(p + 4),
            None => (mail.octets.as_slice(), &[][..]),
        };
        r.extend(format!("* {} FETCH (UID {}", i + 1, mail.uid).into_bytes());
        for e in elements.iter().map(|e| e.to_ascii_uppercase()) {
            let (nom, contenu): (String, &[u8]) = match e.as_str() {
                "UID" => continue,
                "RFC822.SIZE" => {
                    r.extend(format!(" RFC822.SIZE {}", mail.octets.len()).into_bytes());
                    continue;
                }
                "FLAGS" => {
                    r.extend(b" FLAGS ()");
                    continue;
                }
                "INTERNALDATE" => {
                    r.extend(b" INTERNALDATE \"19-Nov-2026 08:30:00 +0100\"");
                    continue;
                }
                "BODY.PEEK[]" => ("BODY[]".into(), &mail.octets),
                "BODY.PEEK[HEADER]" => ("BODY[HEADER]".into(), tete),
                _ => {
                    let n = e
                        .strip_prefix("BODY.PEEK[TEXT]<0.")
                        .and_then(|r| r.strip_suffix('>'))
                        .and_then(|n| n.parse::<usize>().ok());
                    match n {
                        Some(n) => ("BODY[TEXT]<0>".into(), &texte[..texte.len().min(n)]),
                        None => ("BODY[TEXT]".into(), texte),
                    }
                }
            };
            r.extend(format!(" {nom} {{{}}}\r\n", contenu.len()).into_bytes());
            r.extend_from_slice(contenu);
        }
        r.extend(b")\r\n");
    }
    r.extend(format!("{etiquette} OK Success\r\n").into_bytes());
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    /// Envoie des lignes brutes (comme un client qui voudrait écrire dans la boîte) et rend tout ce qui a été répondu.
    fn brut(f: &FauxImap, lignes: &[&str]) -> String {
        let mut c = TcpStream::connect(("127.0.0.1", f.port())).expect("connexion");
        let _ = c.set_read_timeout(Some(Duration::from_millis(500)));
        for l in lignes {
            c.write_all(format!("{l}\r\n").as_bytes()).expect("écriture");
            std::thread::sleep(Duration::from_millis(30));
        }
        let mut r = Vec::new();
        let mut tampon = [0u8; 4096];
        while let Ok(n) = c.read(&mut tampon) {
            if n == 0 {
                break;
            }
            r.extend_from_slice(&tampon[..n]);
        }
        String::from_utf8_lossy(&r).into_owned()
    }

    #[test]
    fn le_faux_serveur_refuse_et_note_toute_ecriture() {
        let f =
            FauxImap::demarrer(ConfigImap::new("a@exemple.fr", "mdp-invente", vec![b"Subject: x\r\n\r\ny".to_vec()]))
                .expect("démarrage");
        let interdites = [
            "A3 SELECT INBOX",
            "A4 STORE 1 +FLAGS (\\Seen)",
            "A5 UID STORE 1 +FLAGS (\\Deleted)",
            "A6 COPY 1 Corbeille",
            "A7 UID MOVE 1 Corbeille",
            "A8 EXPUNGE",
            "A9 APPEND INBOX {3}",
            "A10 DELETE INBOX",
            "A11 CREATE Dossier",
            "A12 RENAME INBOX Autre",
            "A13 UID FETCH 1 (BODY[])",
            "A14 UID FETCH 1 (RFC822)",
            "A15 FETCH 1 (BODY.PEEK[])",
            "A16 BONJOUR",
        ];
        let mut lignes = vec!["A1 LOGIN \"a@exemple.fr\" \"mdp-invente\"", "A2 EXAMINE INBOX"];
        lignes.extend(interdites);
        lignes.push("A17 LOGOUT");
        let r = brut(&f, &lignes);
        assert!(r.contains("A1 OK") && r.contains("A2 OK [READ-ONLY]"), "{r}");
        for i in 3..=16 {
            assert!(r.contains(&format!("A{i} BAD")), "A{i} : {r}");
        }
        assert_eq!(f.refusees().len(), interdites.len());
        assert!(f.commandes().iter().all(|c| !c.contains("mdp-invente")));
        assert_eq!(f.commandes()[0], "LOGIN \"a@exemple.fr\" *****");
    }

    #[test]
    fn sans_login_rien_n_est_servi() {
        let f = FauxImap::demarrer(ConfigImap::new("a@exemple.fr", "x", vec![])).expect("démarrage");
        let r = brut(&f, &["A1 EXAMINE INBOX", "A2 UID SEARCH SINCE 1-Nov-2026", "A3 LOGOUT"]);
        assert!(r.contains("A1 NO") && r.contains("A2 BAD"), "{r}");
        assert_eq!(f.adresse(), "127.0.0.1");
    }

    #[test]
    fn les_mots_et_les_jours() {
        assert_eq!(mots("LOGIN \"a\\\"b\" c (UID BODY.PEEK[])"), vec!["LOGIN", "a\"b", "c", "(UID BODY.PEEK[])"]);
        assert_eq!(jour_imap("5-Nov-2026"), Some((2026, 11, 5)));
        assert_eq!(jour_imap("5-Foo-2026"), None);
        assert!(element_permis("BODY.PEEK[TEXT]<0.300000>"));
        assert!(!element_permis("BODY[TEXT]") && !element_permis("RFC822") && !element_permis("RFC822.TEXT"));
    }
}
