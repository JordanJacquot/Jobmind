//! Le client IMAP de Jobmind (RFC 9051 et 3501), écrit ici et réduit aux seules commandes de LECTURE : il marche sur
//! n'importe quel flux `Read + Write` (TLS vers Gmail, TCP simple vers le faux serveur local, un tampon en mémoire
//! dans les tests).
//!
//! Les commandes possibles sont celles de [`Commande`], et SEULEMENT elles : aucun moyen d'envoyer STORE, COPY, MOVE,
//! EXPUNGE, SELECT, APPEND, DELETE, CREATE, RENAME, SUBSCRIBE ni du texte libre. La boîte est ouverte par EXAMINE
//! (lecture seule) et chaque mail est lu par BODY.PEEK (il reste « non lu » dans Gmail).
//!
//! Les réponses sont lues ligne par ligne avec leurs littéraux (`{n}` et `{n+}`), dans des bornes : une ligne de plus
//! de 100 000 octets ou une réponse de plus de 8 Mo donnent [`ErreurMail::Illisible`], sans tout garder en mémoire.

use std::io::{ErrorKind, Read, Write};

use crate::ErreurMail;

/// La plus longue ligne acceptée (hors littéraux).
pub const LIGNE_MAX: usize = 100_000;
/// La plus grosse réponse acceptée (lignes et littéraux).
pub const REPONSE_MAX: usize = 8 * 1024 * 1024;
/// Le plus grand nombre de lignes d'une réponse.
const LIGNES_MAX: usize = 200_000;
/// Le plus grand nombre de littéraux sur une ligne.
const LITTERAUX_MAX: usize = 100;

/// Les seules commandes que Jobmind peut envoyer. Aucune ne change la boîte.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Commande<'a> {
    /// CAPABILITY
    Capability,
    /// LOGIN "adresse" "mot de passe" (refusé avant d'envoyer si l'un des deux a un retour à la ligne ou un
    /// caractère non ASCII)
    Login { adresse: &'a str, mdp: &'a str },
    /// EXAMINE INBOX (la boîte de réception, en lecture seule)
    Examine,
    /// UID SEARCH UID n:*
    UidSearchDepuisUid(u32),
    /// UID SEARCH SINCE 19-Nov-2026 (année, mois, jour)
    UidSearchDepuisJour(i32, u32, u32),
    /// UID FETCH u (UID RFC822.SIZE)
    UidFetchTaille(u32),
    /// UID FETCH u (UID BODY.PEEK[])
    UidFetchCorps(u32),
    /// UID FETCH u (UID BODY.PEEK[HEADER])
    UidFetchEnTete(u32),
    /// UID FETCH u (UID BODY.PEEK[TEXT]<0.n>) : les n premiers octets du corps
    UidFetchDebut(u32, u32),
    /// LOGOUT
    Logout,
}

impl std::fmt::Debug for Commande<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Commande::Login { adresse, .. } => write!(f, "Login {{ adresse: {adresse:?}, mdp: \"*****\" }}"),
            Commande::Capability => write!(f, "Capability"),
            Commande::Examine => write!(f, "Examine"),
            Commande::UidSearchDepuisUid(u) => write!(f, "UidSearchDepuisUid({u})"),
            Commande::UidSearchDepuisJour(a, m, j) => write!(f, "UidSearchDepuisJour({a}, {m}, {j})"),
            Commande::UidFetchTaille(u) => write!(f, "UidFetchTaille({u})"),
            Commande::UidFetchCorps(u) => write!(f, "UidFetchCorps({u})"),
            Commande::UidFetchEnTete(u) => write!(f, "UidFetchEnTete({u})"),
            Commande::UidFetchDebut(u, n) => write!(f, "UidFetchDebut({u}, {n})"),
            Commande::Logout => write!(f, "Logout"),
        }
    }
}

const MOIS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// Un jour au format IMAP (« 19-Nov-2026 »), ramené dans les bornes.
pub fn jour_imap(annee: i32, mois: u32, jour: u32) -> String {
    let a = annee.clamp(1970, 9999);
    let m = mois.clamp(1, 12);
    let j = jour.clamp(1, 31);
    format!("{j}-{}-{a}", MOIS[(m - 1) as usize])
}

/// Une chaîne IMAP entre guillemets (\ et " échappés). Rien : elle ne peut pas être envoyée (vide, retour à la
/// ligne, caractère de contrôle ou non ASCII).
fn entre_guillemets(s: &str) -> Option<String> {
    if s.is_empty() || s.len() > 1000 || s.bytes().any(|b| !(0x20..0x7f).contains(&b)) {
        return None;
    }
    let mut r = String::with_capacity(s.len() + 2);
    r.push('"');
    for c in s.chars() {
        if c == '\\' || c == '"' {
            r.push('\\');
        }
        r.push(c);
    }
    r.push('"');
    Some(r)
}

impl Commande<'_> {
    /// Le texte de la commande, sans l'étiquette ni la fin de ligne.
    fn texte(&self) -> Result<String, ErreurMail> {
        Ok(match self {
            Commande::Capability => "CAPABILITY".into(),
            Commande::Login { adresse, mdp } => {
                let a = entre_guillemets(adresse).ok_or(ErreurMail::Identifiants)?;
                let m = entre_guillemets(mdp).ok_or(ErreurMail::Identifiants)?;
                format!("LOGIN {a} {m}")
            }
            Commande::Examine => "EXAMINE INBOX".into(),
            Commande::UidSearchDepuisUid(u) => format!("UID SEARCH UID {}:*", (*u).max(1)),
            Commande::UidSearchDepuisJour(a, m, j) => format!("UID SEARCH SINCE {}", jour_imap(*a, *m, *j)),
            Commande::UidFetchTaille(u) => format!("UID FETCH {u} (UID RFC822.SIZE)"),
            Commande::UidFetchCorps(u) => format!("UID FETCH {u} (UID BODY.PEEK[])"),
            Commande::UidFetchEnTete(u) => format!("UID FETCH {u} (UID BODY.PEEK[HEADER])"),
            Commande::UidFetchDebut(u, n) => format!("UID FETCH {u} (UID BODY.PEEK[TEXT]<0.{n}>)"),
            Commande::Logout => "LOGOUT".into(),
        })
    }
}

/// Une ligne de réponse, avec ses littéraux (le texte garde la marque « {n} » à leur place).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ligne {
    pub texte: Vec<u8>,
    pub litteraux: Vec<Vec<u8>>,
}

/// Le résultat d'une commande (la ligne étiquetée).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etat {
    Ok,
    No,
    Bad,
}

/// La réponse complète à une commande : les lignes non étiquetées (« * ... ») puis le résultat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reponse {
    pub lignes: Vec<Ligne>,
    pub etat: Etat,
    /// Le texte de la ligne étiquetée après OK, NO ou BAD (court, sans caractères de contrôle)
    pub texte: String,
}

/// La boîte de réception ouverte par EXAMINE.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Boite {
    /// Le nombre de mails (* n EXISTS)
    pub existe: u32,
    pub uidvalidity: u32,
    pub uidnext: Option<u32>,
}

/// Un texte de serveur montrable : 200 caractères au plus, sans caractères de contrôle.
pub(crate) fn resume(o: &[u8]) -> String {
    String::from_utf8_lossy(o).chars().filter(|c| !c.is_control()).take(200).collect::<String>().trim().to_string()
}

fn erreur_io(e: std::io::Error) -> ErreurMail {
    match e.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => ErreurMail::Delai,
        _ => ErreurMail::Reseau(e.to_string()),
    }
}

fn coupe() -> ErreurMail {
    ErreurMail::Reseau("la connexion a été coupée".into())
}

fn commence_par(o: &[u8], debut: &[u8]) -> bool {
    o.len() >= debut.len() && o[..debut.len()].eq_ignore_ascii_case(debut)
}

/// La position d'un mot dans un texte, sans tenir compte des majuscules.
fn chercher(o: &[u8], mot: &[u8]) -> Option<usize> {
    if mot.is_empty() || o.len() < mot.len() {
        return None;
    }
    (0..=o.len() - mot.len()).find(|&i| o[i..i + mot.len()].eq_ignore_ascii_case(mot))
}

/// Le nombre écrit juste après un mot (« UIDVALIDITY 42 » : 42).
fn nombre_apres(o: &[u8], mot: &[u8]) -> Option<u32> {
    let mut depart = 0;
    while let Some(p) = chercher(&o[depart..], mot) {
        let i = depart + p;
        // (un mot entier : « UID » ne doit pas se trouver au milieu de « UIDVALIDITY »)
        let avant_ok = i == 0 || !o[i - 1].is_ascii_alphanumeric() && o[i - 1] != b'.';
        let suite = &o[i + mot.len()..];
        let chiffres: Vec<u8> =
            suite.iter().skip_while(|c| **c == b' ').take_while(|c| c.is_ascii_digit()).copied().collect();
        if avant_ok
            && suite.first() == Some(&b' ')
            && !chiffres.is_empty()
            && chiffres.len() <= 10
            && let Ok(n) = std::str::from_utf8(&chiffres).unwrap_or("").parse::<u32>()
        {
            return Some(n);
        }
        depart = i + mot.len();
    }
    None
}

/// La taille du littéral annoncé en fin de ligne (« {123} » ou « {123+} »).
fn litteral(seg: &[u8]) -> Option<usize> {
    if seg.last() != Some(&b'}') {
        return None;
    }
    let ouvre = seg.iter().rposition(|&c| c == b'{')?;
    let mut dedans = &seg[ouvre + 1..seg.len() - 1];
    if dedans.last() == Some(&b'+') {
        dedans = &dedans[..dedans.len() - 1];
    }
    if dedans.is_empty() || dedans.len() > 10 || !dedans.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(dedans).ok()?.parse().ok()
}

/// Le client, sur un flux déjà ouvert.
pub struct Client<S: Read + Write> {
    flux: S,
    tampon: Box<[u8]>,
    debut: usize,
    fin: usize,
    numero: u32,
}

impl<S: Read + Write> Client<S> {
    pub fn new(flux: S) -> Client<S> {
        Client { flux, tampon: vec![0u8; 64 * 1024].into_boxed_slice(), debut: 0, fin: 0, numero: 0 }
    }

    fn remplir(&mut self) -> Result<(), ErreurMail> {
        loop {
            match self.flux.read(&mut self.tampon) {
                Ok(0) => return Err(coupe()),
                Ok(n) => {
                    self.debut = 0;
                    self.fin = n;
                    return Ok(());
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(erreur_io(e)),
            }
        }
    }

    /// Une ligne du flux, sans sa fin (CRLF ou LF).
    fn ligne_physique(&mut self) -> Result<Vec<u8>, ErreurMail> {
        let mut v = Vec::new();
        loop {
            if self.debut >= self.fin {
                self.remplir()?;
            }
            let dispo = &self.tampon[self.debut..self.fin];
            if let Some(p) = dispo.iter().position(|&c| c == b'\n') {
                v.extend_from_slice(&dispo[..p]);
                self.debut += p + 1;
                break;
            }
            v.extend_from_slice(dispo);
            self.debut = self.fin;
            if v.len() > LIGNE_MAX {
                return Err(ErreurMail::Illisible("ligne trop longue".into()));
            }
        }
        if v.last() == Some(&b'\r') {
            v.pop();
        }
        if v.len() > LIGNE_MAX {
            return Err(ErreurMail::Illisible("ligne trop longue".into()));
        }
        Ok(v)
    }

    fn lire_exact(&mut self, buf: &mut [u8]) -> Result<(), ErreurMail> {
        let mut fait = 0;
        while fait < buf.len() {
            if self.debut >= self.fin {
                self.remplir()?;
            }
            let n = (self.fin - self.debut).min(buf.len() - fait);
            buf[fait..fait + n].copy_from_slice(&self.tampon[self.debut..self.debut + n]);
            self.debut += n;
            fait += n;
        }
        Ok(())
    }

    /// Une ligne de réponse entière, littéraux compris, dans le budget restant de la réponse.
    fn ligne(&mut self, budget: &mut usize) -> Result<Ligne, ErreurMail> {
        let trop = || ErreurMail::Illisible("réponse trop longue".into());
        let mut l = Ligne::default();
        loop {
            let seg = self.ligne_physique()?;
            *budget = budget.checked_sub(seg.len() + 2).ok_or_else(trop)?;
            if l.texte.len() + seg.len() > LIGNE_MAX {
                return Err(ErreurMail::Illisible("ligne trop longue".into()));
            }
            let n = litteral(&seg);
            l.texte.extend_from_slice(&seg);
            let Some(n) = n else { return Ok(l) };
            *budget = budget.checked_sub(n).ok_or_else(trop)?;
            if l.litteraux.len() >= LITTERAUX_MAX {
                return Err(ErreurMail::Illisible("trop de littéraux".into()));
            }
            let mut v = vec![0u8; n];
            self.lire_exact(&mut v)?;
            l.litteraux.push(v);
        }
    }

    /// L'accueil du serveur : « * OK » attendu (« * BYE » : l'accès est fermé).
    pub fn accueil(&mut self) -> Result<(), ErreurMail> {
        let mut budget = REPONSE_MAX;
        let l = self.ligne(&mut budget)?;
        if commence_par(&l.texte, b"* OK") || commence_par(&l.texte, b"* PREAUTH") {
            Ok(())
        } else if commence_par(&l.texte, b"* BYE") {
            Err(ErreurMail::ImapFerme(resume(&l.texte[5..])))
        } else {
            Err(ErreurMail::Illisible("accueil inattendu".into()))
        }
    }

    /// Envoie une commande (étiquetée A001, A002...) et lit toute sa réponse. La ligne envoyée est effacée de la
    /// mémoire ensuite (elle peut contenir le mot de passe).
    pub fn executer(&mut self, c: &Commande) -> Result<Reponse, ErreurMail> {
        let mut texte = c.texte()?;
        self.numero += 1;
        let etiquette = format!("A{:03}", self.numero);
        let mut ligne = format!("{etiquette} {texte}\r\n");
        crate::effacer(&mut texte);
        let r = self.flux.write_all(ligne.as_bytes()).and_then(|_| self.flux.flush());
        crate::effacer(&mut ligne);
        r.map_err(erreur_io)?;
        self.reponse(&etiquette, matches!(c, Commande::Logout))
    }

    fn reponse(&mut self, etiquette: &str, logout: bool) -> Result<Reponse, ErreurMail> {
        let mut budget = REPONSE_MAX;
        let mut lignes = Vec::new();
        let e = etiquette.as_bytes();
        loop {
            let l = self.ligne(&mut budget)?;
            if l.texte.len() > e.len() && l.texte.starts_with(e) && l.texte[e.len()] == b' ' {
                let reste = &l.texte[e.len() + 1..];
                let mot_fin = reste.iter().position(|&c| c == b' ').unwrap_or(reste.len());
                let etat = match reste[..mot_fin].to_ascii_uppercase().as_slice() {
                    b"OK" => Etat::Ok,
                    b"NO" => Etat::No,
                    b"BAD" => Etat::Bad,
                    _ => return Err(ErreurMail::Illisible("résultat inconnu".into())),
                };
                return Ok(Reponse { lignes, etat, texte: resume(&reste[mot_fin..]) });
            }
            if commence_par(&l.texte, b"* BYE") && !logout {
                return Err(ErreurMail::ImapFerme(resume(&l.texte[5..])));
            }
            if l.texte.first() == Some(&b'+') {
                // (Jobmind n'envoie jamais de littéral : une demande de suite n'a pas de sens)
                return Err(ErreurMail::Illisible("suite demandée".into()));
            }
            if lignes.len() >= LIGNES_MAX {
                return Err(ErreurMail::Illisible("réponse trop longue".into()));
            }
            if l.texte.first() == Some(&b'*') {
                lignes.push(l);
            }
        }
    }

    /// Un refus du serveur, rendu en erreur.
    fn refus(r: &Reponse) -> ErreurMail {
        ErreurMail::Refus(r.texte.clone())
    }

    /// CAPABILITY : un serveur qui interdit LOGIN (LOGINDISABLED) ferme l'accès. Un refus est sans gravité.
    pub fn capacites(&mut self) -> Result<(), ErreurMail> {
        let r = self.executer(&Commande::Capability)?;
        if r.etat == Etat::Ok && r.lignes.iter().any(|l| chercher(&l.texte, b"LOGINDISABLED").is_some()) {
            return Err(ErreurMail::ImapFerme("LOGIN interdit par le serveur".into()));
        }
        Ok(())
    }

    /// LOGIN. Un refus : [`ErreurMail::Identifiants`] (mauvais mot de passe d'application) ou
    /// [`ErreurMail::ImapFerme`] (IMAP désactivé, connexion par le navigateur demandée, [ALERT]).
    pub fn connecter(&mut self, adresse: &str, mdp: &str) -> Result<(), ErreurMail> {
        let r = self.executer(&Commande::Login { adresse, mdp })?;
        match r.etat {
            Etat::Ok => Ok(()),
            Etat::Bad => Err(Self::refus(&r)),
            Etat::No => {
                let mut tout = r.texte.to_ascii_uppercase();
                for l in &r.lignes {
                    tout.push(' ');
                    tout.push_str(&resume(&l.texte).to_ascii_uppercase());
                }
                let identifiants = ["AUTHENTICATIONFAILED", "INVALID CREDENTIALS", "PASSWORD", "AUTHORIZATIONFAILED"];
                let ferme = ["ALERT", "WEBALERT", "WEB LOGIN", "NOT ENABLED", "UNAVAILABLE", "IMAP"];
                if identifiants.iter().any(|m| tout.contains(m)) {
                    Err(ErreurMail::Identifiants)
                } else if ferme.iter().any(|m| tout.contains(m)) {
                    Err(ErreurMail::ImapFerme(r.texte.clone()))
                } else {
                    Err(ErreurMail::Identifiants)
                }
            }
        }
    }

    /// EXAMINE INBOX : le nombre de mails, UIDVALIDITY (obligatoire) et UIDNEXT.
    pub fn examiner(&mut self) -> Result<Boite, ErreurMail> {
        let r = self.executer(&Commande::Examine)?;
        if r.etat != Etat::Ok {
            return Err(Self::refus(&r));
        }
        let mut b = Boite::default();
        let mut validite = None;
        for l in &r.lignes {
            let t = &l.texte;
            if let Some(v) = nombre_apres(t, b"UIDVALIDITY") {
                validite = Some(v);
            }
            if let Some(v) = nombre_apres(t, b"UIDNEXT") {
                b.uidnext = Some(v);
            }
            let mots: Vec<&[u8]> = t.split(|&c| c == b' ').collect();
            if mots.len() >= 3 && mots[0] == b"*" && mots[2].eq_ignore_ascii_case(b"EXISTS") {
                b.existe = std::str::from_utf8(mots[1]).ok().and_then(|s| s.parse().ok()).unwrap_or(0);
            }
        }
        b.uidvalidity = validite.ok_or_else(|| ErreurMail::Illisible("UIDVALIDITY absent".into()))?;
        Ok(b)
    }

    fn chercher_uids(&mut self, c: &Commande) -> Result<Vec<u32>, ErreurMail> {
        let r = self.executer(c)?;
        if r.etat != Etat::Ok {
            return Err(Self::refus(&r));
        }
        let mut uids = Vec::new();
        for l in r.lignes.iter().filter(|l| commence_par(&l.texte, b"* SEARCH")) {
            for mot in l.texte[8..].split(|&c| c == b' ') {
                if let Some(n) = std::str::from_utf8(mot).ok().and_then(|s| s.parse::<u32>().ok()) {
                    uids.push(n);
                }
            }
        }
        Ok(uids)
    }

    /// UID SEARCH UID n:* (attention : le serveur rend toujours au moins le dernier mail, même si son UID est plus
    /// petit que n ; à filtrer).
    pub fn chercher_depuis_uid(&mut self, uid: u32) -> Result<Vec<u32>, ErreurMail> {
        self.chercher_uids(&Commande::UidSearchDepuisUid(uid))
    }

    /// UID SEARCH SINCE : les mails reçus depuis ce jour.
    pub fn chercher_depuis_jour(&mut self, annee: i32, mois: u32, jour: u32) -> Result<Vec<u32>, ErreurMail> {
        self.chercher_uids(&Commande::UidSearchDepuisJour(annee, mois, jour))
    }

    /// La ligne FETCH de ce mail dans une réponse (un refus NO : rien ; BAD : une erreur).
    fn fetch(&mut self, c: &Commande, uid: u32) -> Result<Option<Ligne>, ErreurMail> {
        let r = self.executer(c)?;
        match r.etat {
            Etat::Ok => {}
            Etat::No => return Ok(None),
            Etat::Bad => return Err(Self::refus(&r)),
        }
        Ok(r.lignes.into_iter().find(|l| {
            commence_par(&l.texte, b"* ")
                && chercher(&l.texte, b" FETCH (").is_some()
                && nombre_apres(&l.texte, b"UID") == Some(uid)
        }))
    }

    /// La taille du mail (RFC822.SIZE). Rien : il n'existe plus.
    pub fn taille(&mut self, uid: u32) -> Result<Option<u32>, ErreurMail> {
        Ok(self.fetch(&Commande::UidFetchTaille(uid), uid)?.and_then(|l| nombre_apres(&l.texte, b"RFC822.SIZE")))
    }

    /// Le contenu rendu par un FETCH : le littéral, ou une chaîne entre guillemets.
    fn contenu(l: Ligne) -> Option<Vec<u8>> {
        if let Some(v) = l.litteraux.into_iter().next() {
            return Some(v);
        }
        let t = &l.texte;
        let p = chercher(t, b"] \"").map(|p| p + 3).or_else(|| chercher(t, b"> \"").map(|p| p + 3))?;
        let mut v = Vec::new();
        let mut i = p;
        while i < t.len() {
            match t[i] {
                b'\\' if i + 1 < t.len() => {
                    v.push(t[i + 1]);
                    i += 2;
                }
                b'"' => return Some(v),
                c => {
                    v.push(c);
                    i += 1;
                }
            }
        }
        None
    }

    /// Le mail entier, sans le marquer lu (BODY.PEEK[]).
    pub fn corps(&mut self, uid: u32) -> Result<Option<Vec<u8>>, ErreurMail> {
        Ok(self.fetch(&Commande::UidFetchCorps(uid), uid)?.and_then(Self::contenu))
    }

    /// L'en-tête du mail, sans le marquer lu (BODY.PEEK[HEADER]).
    pub fn en_tete(&mut self, uid: u32) -> Result<Option<Vec<u8>>, ErreurMail> {
        Ok(self.fetch(&Commande::UidFetchEnTete(uid), uid)?.and_then(Self::contenu))
    }

    /// Les `octets` premiers octets du corps, sans le marquer lu (BODY.PEEK[TEXT]<0.n>).
    pub fn debut(&mut self, uid: u32, octets: u32) -> Result<Option<Vec<u8>>, ErreurMail> {
        Ok(self.fetch(&Commande::UidFetchDebut(uid, octets), uid)?.and_then(Self::contenu))
    }

    /// LOGOUT, sans attendre de réponse utile (une erreur ici ne change rien : tout est déjà lu).
    pub fn deconnecter(&mut self) {
        let _ = self.executer(&Commande::Logout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Un faux flux : ce que le serveur répond, et ce que le client a écrit.
    struct Simule {
        entree: Cursor<Vec<u8>>,
        sortie: Vec<u8>,
    }

    impl Read for Simule {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.entree.read(buf)
        }
    }

    impl Write for Simule {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.sortie.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn client(reponses: &[u8]) -> Client<Simule> {
        Client::new(Simule { entree: Cursor::new(reponses.to_vec()), sortie: Vec::new() })
    }

    fn ecrit(c: &Client<Simule>) -> String {
        String::from_utf8_lossy(&c.flux.sortie).into_owned()
    }

    #[test]
    fn les_textes_des_commandes() {
        let toutes = [
            Commande::Capability,
            Commande::Login { adresse: "a@b.fr", mdp: "x" },
            Commande::Examine,
            Commande::UidSearchDepuisUid(4),
            Commande::UidSearchDepuisJour(2026, 11, 19),
            Commande::UidFetchTaille(4),
            Commande::UidFetchCorps(4),
            Commande::UidFetchEnTete(4),
            Commande::UidFetchDebut(4, 300_000),
            Commande::Logout,
        ];
        let textes: Vec<String> = toutes.iter().map(|c| c.texte().expect("texte")).collect();
        assert_eq!(
            textes,
            [
                "CAPABILITY",
                "LOGIN \"a@b.fr\" \"x\"",
                "EXAMINE INBOX",
                "UID SEARCH UID 4:*",
                "UID SEARCH SINCE 19-Nov-2026",
                "UID FETCH 4 (UID RFC822.SIZE)",
                "UID FETCH 4 (UID BODY.PEEK[])",
                "UID FETCH 4 (UID BODY.PEEK[HEADER])",
                "UID FETCH 4 (UID BODY.PEEK[TEXT]<0.300000>)",
                "LOGOUT",
            ]
        );
        assert_eq!(jour_imap(0, 0, 0), "1-Jan-1970");
        assert_eq!(jour_imap(2026, 13, 40), "31-Dec-2026");
        assert_eq!(jour_imap(2026, 3, 5), "5-Mar-2026");
        assert!(!format!("{:?}", Commande::Login { adresse: "a@b.fr", mdp: "secret-123" }).contains("secret"));
    }

    #[test]
    fn login_echappe_et_refuse_avant_d_envoyer() {
        let t = Commande::Login { adresse: "a@b.fr", mdp: "a\"b\\c d" }.texte().expect("texte");
        assert_eq!(t, "LOGIN \"a@b.fr\" \"a\\\"b\\\\c d\"");
        for (a, m) in
            [("a@b.fr", "x\r\nA2 STORE"), ("a@b.fr", "mot\u{e9}"), ("a@b.fr\n", "x"), ("", "x"), ("a@b.fr", "")]
        {
            let mut c = client(b"");
            assert_eq!(c.connecter(a, m), Err(ErreurMail::Identifiants));
            assert!(c.flux.sortie.is_empty(), "rien envoyé");
        }
    }

    #[test]
    fn une_session_lue() {
        let mut c = client(
            b"* OK Gimap ready\r\n\
              * CAPABILITY IMAP4rev1 UNSELECT IDLE AUTH=PLAIN\r\nA001 OK Thats all she wrote!\r\n\
              * CAPABILITY IMAP4rev1\r\nA002 OK jordan authenticated (Success)\r\n\
              * FLAGS (\\Answered \\Seen)\r\n* OK [PERMANENTFLAGS ()] Flags permitted.\r\n\
              * OK [UIDVALIDITY 654321] UIDs valid.\r\n* 42 EXISTS\r\n* 0 RECENT\r\n* OK [UIDNEXT 990] Predicted next UID.\r\n\
              A003 OK [READ-ONLY] INBOX selected. (Success)\r\n\
              * SEARCH 980 985 989\r\n* SEARCH 2\r\nA004 OK SEARCH completed (Success)\r\n\
              * 40 FETCH (UID 985 RFC822.SIZE 17)\r\nA005 OK Success\r\n\
              * 40 FETCH (UID 985 BODY[] {18}\r\nSubject: ok\r\n\r\nx\r\n)\r\n* 3 FETCH (FLAGS (\\Seen))\r\nA006 OK Success\r\n\
              * 40 FETCH (BODY[HEADER] \"Subject: \\\"q\\\"\" UID 985)\r\nA007 OK Success\r\n\
              * BYE LOGOUT Requested\r\nA008 OK 73 good day (Success)\r\n",
        );
        c.accueil().expect("accueil");
        c.capacites().expect("capacités");
        c.connecter("jordan.essai@exemple.fr", "abcd efgh").expect("login");
        assert_eq!(c.examiner(), Ok(Boite { existe: 42, uidvalidity: 654321, uidnext: Some(990) }));
        assert_eq!(c.chercher_depuis_uid(985), Ok(vec![980, 985, 989, 2]));
        assert_eq!(c.taille(985), Ok(Some(17)));
        assert_eq!(c.corps(985), Ok(Some(b"Subject: ok\r\n\r\nx\r\n".to_vec())));
        assert_eq!(c.en_tete(985), Ok(Some(b"Subject: \"q\"".to_vec())));
        c.deconnecter();
        let e = ecrit(&c);
        assert!(e.starts_with("A001 CAPABILITY\r\nA002 LOGIN \"jordan.essai@exemple.fr\" \"abcd efgh\"\r\n"));
        assert!(e.contains("A003 EXAMINE INBOX\r\n") && e.contains("A008 LOGOUT\r\n"), "{e}");
    }

    #[test]
    fn les_refus_du_serveur() {
        let mut c = client(b"* BYE Too many connections\r\n");
        assert!(matches!(c.accueil(), Err(ErreurMail::ImapFerme(_))));
        let mut c = client(b"HTTP/1.1 200 OK\r\n");
        assert!(matches!(c.accueil(), Err(ErreurMail::Illisible(_))));
        let mut c = client(b"A001 NO [AUTHENTICATIONFAILED] Invalid credentials (Failure)\r\n");
        assert_eq!(c.connecter("a@b.fr", "x"), Err(ErreurMail::Identifiants));
        let mut c = client(b"A001 NO [ALERT] Application-specific password required\r\n");
        assert_eq!(c.connecter("a@b.fr", "x"), Err(ErreurMail::Identifiants));
        let mut c = client(b"A001 NO [ALERT] Your account is not enabled for IMAP use.\r\n");
        assert!(matches!(c.connecter("a@b.fr", "x"), Err(ErreurMail::ImapFerme(_))));
        let mut c = client(
            b"* NO [WEBALERT https://accounts.google.com/x] Web login required.\r\nA001 NO [ALERT] Please log in\r\n",
        );
        assert!(matches!(c.connecter("a@b.fr", "x"), Err(ErreurMail::ImapFerme(_))));
        let mut c = client(b"* BYE System error\r\n");
        assert!(matches!(c.connecter("a@b.fr", "x"), Err(ErreurMail::ImapFerme(_))));
        let mut c = client(b"A001 BAD Command syntax error\r\n");
        assert!(matches!(c.examiner(), Err(ErreurMail::Refus(_))));
        let mut c = client(b"* 3 EXISTS\r\nA001 OK [READ-ONLY]\r\n");
        assert!(matches!(c.examiner(), Err(ErreurMail::Illisible(_))));
        let mut c = client(b"A001 NO Some messages could not be FETCHed (Failure)\r\n");
        assert_eq!(c.corps(5), Ok(None));
        let mut c = client(b"+ go ahead\r\n");
        assert!(matches!(c.taille(5), Err(ErreurMail::Illisible(_))));
        let mut c = client(b"* 1 FETCH (UID 5 RFC822.SIZE 1");
        assert!(matches!(c.taille(5), Err(ErreurMail::Reseau(_))));
        let mut c = client(b"A001 PEUT-ETRE\r\n");
        assert!(matches!(c.taille(5), Err(ErreurMail::Illisible(_))));
        // (le mauvais mail : rien)
        let mut c = client(b"* 1 FETCH (UID 6 RFC822.SIZE 10)\r\nA001 OK\r\n");
        assert_eq!(c.taille(5), Ok(None));
    }

    #[test]
    fn les_bornes() {
        let mut longue = b"* OK ".to_vec();
        longue.extend(std::iter::repeat_n(b'x', LIGNE_MAX + 10));
        longue.extend_from_slice(b"\r\n");
        let mut c = client(&longue);
        assert!(matches!(c.accueil(), Err(ErreurMail::Illisible(_))));
        // (un littéral de 9 Mo annoncé : refusé sans le lire ni le garder)
        let mut c = client(b"* 1 FETCH (UID 5 BODY[] {9000000}\r\nabc");
        assert!(matches!(c.corps(5), Err(ErreurMail::Illisible(_))));
        let mut c = client(b"* 1 FETCH (UID 5 BODY[] {99999999999999}\r\nabc");
        assert!(matches!(c.corps(5), Err(ErreurMail::Reseau(_)) | Err(ErreurMail::Illisible(_))));
        // (plusieurs littéraux qui dépassent ensemble 8 Mo)
        let mut r = Vec::new();
        for _ in 0..3 {
            r.extend_from_slice(b"* 1 FETCH (UID 5 BODY[] {3000000}\r\n");
            r.extend(std::iter::repeat_n(b'a', 3_000_000));
            r.extend_from_slice(b")\r\n");
        }
        r.extend_from_slice(b"A001 OK\r\n");
        let mut c = client(&r);
        assert!(matches!(c.corps(5), Err(ErreurMail::Illisible(_))));
        assert_eq!(litteral(b"BODY[] {12+}"), Some(12));
        assert_eq!(litteral(b"BODY[] {}"), None);
        assert_eq!(litteral(b"BODY[] {1a}"), None);
        assert_eq!(nombre_apres(b"* OK [UIDVALIDITY 7] x", b"UID"), None);
        assert_eq!(nombre_apres(b"* 1 FETCH (UID 9)", b"UID"), Some(9));
    }

    #[test]
    fn des_reponses_au_hasard_sans_panique() {
        let mut graine: u64 = 0x2545_f491_4f6c_dd1d;
        let mut hasard = move || {
            graine ^= graine << 13;
            graine ^= graine >> 7;
            graine ^= graine << 17;
            graine
        };
        let morceaux: [&[u8]; 12] = [
            b"* OK ",
            b"A001 ",
            b"OK",
            b"NO",
            b"{5}\r\n",
            b"{3+}\r\n",
            b"\r\n",
            b"* 1 FETCH (UID 1 ",
            b"BODY[] ",
            b"* SEARCH 1 2",
            b"* BYE",
            b"[UIDVALIDITY 3]",
        ];
        for _ in 0..3000 {
            let mut r = Vec::new();
            for _ in 0..(hasard() % 20) {
                if hasard() % 3 == 0 {
                    r.push((hasard() & 0xff) as u8);
                } else {
                    r.extend_from_slice(morceaux[(hasard() % 12) as usize]);
                }
            }
            let mut c = client(&r);
            let _ = c.accueil();
            let _ = c.examiner();
            let _ = c.corps(1);
            let _ = c.chercher_depuis_jour(2026, 11, 19);
        }
    }
}
