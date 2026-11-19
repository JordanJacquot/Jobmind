//! Un faux serveur SMTP local (127.0.0.1, un port libre, TCP simple), pour les tests de l'envoi et pour la
//! construction d'essai de l'appli (option « faux », jamais livrée) : AUCUN mail ne part, il ne fait que GARDER ce
//! qu'il reçoit, pour que les tests le relisent octet par octet.
//!
//! Il imite Gmail (accueil 220, EHLO avec AUTH PLAIN LOGIN, AUTH PLAIN vérifié contre une adresse et un mot de passe
//! inventés, MAIL FROM, RCPT TO, DATA, « . », QUIT) et ses pannes ([`ModeSmtp`]) : mot de passe refusé, destinataire
//! refusé, message refusé après la fin, coupure juste après la fin (l'envoi incertain), coupure avant, serveur
//! muet. Il note chaque commande (AUTH remplacé par « AUTH PLAIN ***** ») et compte les RCPT d'une même session : un
//! client qui en enverrait deux serait vu.

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Ce que fait le serveur (réglable pendant qu'il tourne).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeSmtp {
    /// Comme Gmail : tout est accepté (et gardé ici)
    #[default]
    Normal,
    /// AUTH toujours refusé (535 5.7.8)
    MauvaisMotDePasse,
    /// RCPT TO refusé (550 5.1.1)
    DestinataireRefuse,
    /// Le message entier lu, puis refusé (552 5.3.4)
    MessageRefuse,
    /// Le message entier lu, puis la connexion fermée sans réponse : l'envoi est incertain (le message est gardé)
    CoupeApresFin,
    /// La connexion fermée juste après EHLO
    CoupeAvant,
    /// Ne dit jamais bonjour
    Lent,
}

/// Un mail reçu : l'enveloppe et les octets (le doublement des points retiré).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recu {
    pub de: String,
    pub a: Vec<String>,
    pub octets: Vec<u8>,
}

struct Partage {
    adresse: String,
    mdp: String,
    mode: Arc<Mutex<ModeSmtp>>,
    commandes: Mutex<Vec<String>>,
    recus: Mutex<Vec<Recu>>,
    sessions: Mutex<u32>,
    fin: AtomicBool,
}

fn verrou<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Le serveur ; il s'arrête quand il est lâché.
pub struct FauxSmtp {
    port: u16,
    partage: Arc<Partage>,
}

impl FauxSmtp {
    /// Démarre sur un port libre de 127.0.0.1, sur un fil à part, avec l'adresse et le mot de passe attendus.
    pub fn demarrer(adresse: &str, mdp: &str) -> std::io::Result<FauxSmtp> {
        let ecoute = TcpListener::bind("127.0.0.1:0")?;
        let port = ecoute.local_addr()?.port();
        let partage = Arc::new(Partage {
            adresse: adresse.to_ascii_lowercase(),
            mdp: mdp.into(),
            mode: Arc::new(Mutex::new(ModeSmtp::Normal)),
            commandes: Mutex::new(Vec::new()),
            recus: Mutex::new(Vec::new()),
            sessions: Mutex::new(0),
            fin: AtomicBool::new(false),
        });
        let p = Arc::clone(&partage);
        std::thread::Builder::new().name("faux-smtp".into()).spawn(move || {
            for c in ecoute.incoming() {
                if p.fin.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(c) = c else { continue };
                let p2 = Arc::clone(&p);
                let _ = std::thread::Builder::new().name("faux-smtp-client".into()).spawn(move || servir(c, &p2));
            }
        })?;
        Ok(FauxSmtp { port, partage })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// L'accès à ce serveur (sans TLS, délai de 5 s).
    pub fn acces(&self, adresse: &str, mdp: &str) -> crate::Acces {
        crate::Acces {
            hote: "127.0.0.1".into(),
            port: self.port,
            tls: false,
            adresse: adresse.into(),
            mdp: mdp.into(),
            delai: Duration::from_secs(5),
        }
    }

    /// Les commandes reçues (AUTH masqué ; les lignes du message ne sont pas notées).
    pub fn commandes(&self) -> Vec<String> {
        verrou(&self.partage.commandes).clone()
    }

    /// Les mails reçus (acceptés, ou lus entiers avant une coupure).
    pub fn recus(&self) -> Vec<Recu> {
        verrou(&self.partage.recus).clone()
    }

    /// Le nombre de connexions reçues.
    pub fn sessions(&self) -> u32 {
        *verrou(&self.partage.sessions)
    }

    pub fn mode(&self) -> Arc<Mutex<ModeSmtp>> {
        Arc::clone(&self.partage.mode)
    }

    pub fn regler(&self, m: ModeSmtp) {
        *verrou(&self.partage.mode) = m;
    }
}

impl Drop for FauxSmtp {
    fn drop(&mut self) {
        self.partage.fin.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.port).into(), Duration::from_millis(200));
    }
}

/// Le base64 lu (les tests seulement : pas de caractère inattendu).
pub fn base64_lu(t: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut n = 0;
    let mut r = Vec::new();
    for c in t.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(v);
        n += 6;
        if n >= 8 {
            n -= 8;
            r.push((bits >> n) as u8);
            bits &= (1 << n) - 1;
        }
    }
    Some(r)
}

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
                v.pop();
                if v.last() == Some(&b'\r') {
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

fn servir(c: TcpStream, p: &Partage) {
    *verrou(&p.sessions) += 1;
    let _ = c.set_read_timeout(Some(Duration::from_millis(200)));
    let Ok(mut sortie) = c.try_clone() else { return };
    let mut l = BufReader::new(c);
    let mode = *verrou(&p.mode);
    if mode == ModeSmtp::Lent {
        let debut = Instant::now();
        while !p.fin.load(Ordering::SeqCst) && debut.elapsed() < Duration::from_secs(30) {
            if lire_ligne(&mut l, p).is_none() {
                return;
            }
        }
        return;
    }
    let mut dire = |t: &str| sortie.write_all(format!("{t}\r\n").as_bytes()).is_ok();
    if !dire("220 smtp.faux.invalid ESMTP pret (faux serveur de Jobmind)") {
        return;
    }
    let mut connecte = false;
    let mut de: Option<String> = None;
    let mut a: Vec<String> = Vec::new();
    while let Some(o) = lire_ligne(&mut l, p) {
        let ligne = String::from_utf8_lossy(&o).into_owned();
        let u = ligne.to_ascii_uppercase();
        let note = if u.starts_with("AUTH ") { "AUTH PLAIN *****".to_string() } else { ligne.clone() };
        verrou(&p.commandes).push(note);
        let mode = *verrou(&p.mode);
        if u.starts_with("EHLO ") || u.starts_with("HELO ") {
            if mode == ModeSmtp::CoupeAvant {
                return;
            }
            dire("250-smtp.faux.invalid a votre service");
            dire("250-SIZE 35882577");
            dire("250-8BITMIME");
            dire("250-AUTH LOGIN PLAIN XOAUTH2");
            dire("250 SMTPUTF8");
        } else if let Some(b) = ligne.strip_prefix("AUTH PLAIN ") {
            let ok = base64_lu(b.trim()).and_then(|x| String::from_utf8(x).ok()).is_some_and(|x| {
                let parts: Vec<&str> = x.split('\0').collect();
                parts.len() == 3 && parts[1].eq_ignore_ascii_case(&p.adresse) && parts[2] == p.mdp
            });
            if ok && mode != ModeSmtp::MauvaisMotDePasse {
                connecte = true;
                dire("235 2.7.0 Accepted");
            } else {
                dire("535-5.7.8 Username and Password not accepted. For more information, go to");
                dire("535 5.7.8  https://support.google.com/mail/?p=BadCredentials (faux)");
            }
        } else if u.starts_with("MAIL FROM:") {
            if !connecte {
                dire("530 5.7.0 Authentication Required");
                continue;
            }
            de = Some(ligne[10..].trim().trim_matches(['<', '>']).to_string());
            a.clear();
            dire("250 2.1.0 OK");
        } else if u.starts_with("RCPT TO:") {
            if de.is_none() {
                dire("503 5.5.1 MAIL first");
                continue;
            }
            if mode == ModeSmtp::DestinataireRefuse {
                dire("550-5.1.1 The email account that you tried to reach does not exist.");
                dire("550 5.1.1  (faux)");
                continue;
            }
            a.push(ligne[8..].trim().trim_matches(['<', '>']).to_string());
            dire("250 2.1.5 OK");
        } else if u == "DATA" {
            if a.is_empty() {
                dire("503 5.5.1 RCPT first");
                continue;
            }
            dire("354 Go ahead");
            let mut octets = Vec::new();
            let mut fini = false;
            while let Some(x) = lire_ligne(&mut l, p) {
                if x == b"." {
                    fini = true;
                    break;
                }
                let x = if x.starts_with(b".") { &x[1..] } else { &x[..] };
                octets.extend_from_slice(x);
                octets.extend_from_slice(b"\r\n");
            }
            if !fini {
                return;
            }
            let recu = Recu { de: de.take().unwrap_or_default(), a: std::mem::take(&mut a), octets };
            match mode {
                ModeSmtp::MessageRefuse => {
                    dire("552 5.3.4 Your message exceeded Google's message size limits (faux)");
                }
                ModeSmtp::CoupeApresFin => {
                    verrou(&p.recus).push(recu);
                    return;
                }
                _ => {
                    verrou(&p.recus).push(recu);
                    dire("250 2.0.0 OK  1700000000 faux.1 - gsmtp");
                }
            }
        } else if u == "QUIT" {
            dire("221 2.0.0 closing connection");
            return;
        } else if u == "RSET" || u == "NOOP" {
            dire("250 2.0.0 OK");
        } else {
            dire("502 5.5.1 Unrecognized command");
        }
    }
}
