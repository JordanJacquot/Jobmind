//! Les bulles près de l'horloge (étape 2 du plan), comme les rappels de Paymind (contrat-appli, partie 5), gardées dans
//! la base : une relance devenue à faire, un entretien demain.
//!
//! - **Une seule fois par événement, jamais en boucle** : chaque événement a sa clé (jobmind-core, `bulles_dues`),
//!   notée dans `bulles_cles` en même temps que la bulle est mise en attente (une transaction : tout ou rien). Une clé
//!   déjà notée ne refait jamais de bulle, même après un redémarrage. Un événement que Jordan voit déjà à l'écran (il
//!   vient de taper la candidature) est noté sans bulle ([`marquer_vues`]).
//! - Plusieurs relances dues en même temps : une seule bulle qui les regroupe (elle ouvre la liste) ; une seule : elle
//!   ouvre son dossier.
//! - Les règles d'écriture de Nevermind et Paymind : une bulle à la fois (la précédente pas encore lue par Mindset : on
//!   attend), 6 s d'écart au moins, écrite à côté puis renommée, abandonnée si Mindset ne l'a pas lue en 5 minutes.
//!   Une bulle est notée envoyée AVANT que notification.demande soit écrit : au pire perdue, jamais montrée deux fois.
//! - afficher.demande : « notif:<id> » n'est accepté que pour une bulle que Jobmind a envoyée elle-même ; un numéro de
//!   processus demande d'afficher la fenêtre ; tout le reste est ignoré. Le fichier est toujours supprimé, rien n'est
//!   exécuté.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};

use jobmind_core::suivi::{BulleDue, GenreBulle, texte_bulle};
use rusqlite::{Connection, OptionalExtension, params};

use crate::journal::Journal;
use crate::moment::Moment;

/// La bulle demandée à Mindset.
pub const NOTIFICATION: &str = "notification.demande";
/// La demande d'affichage (le champ `afficher_fichier` de l'entrée de Jobmind).
pub const AFFICHER: &str = "afficher.demande";
/// L'écart entre deux bulles, en secondes.
pub const ECART: i64 = 6;
/// Une bulle que Mindset n'a pas lue en 5 minutes est abandonnée.
pub const ABANDON: i64 = 5 * 60;
/// Une demande d'affichage plus grosse est supprimée sans être lue.
const AFFICHER_MAX: u64 = 64;

/// Un numéro de bulle : 12 chiffres hexadécimaux en minuscules (ce que Mindset accepte).
pub fn id_bulle_valable(id: &str) -> bool {
    id.len() == 12 && id.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Un numéro de bulle neuf, au hasard (48 bits).
fn nouvel_id(graine: i64, n: usize) -> String {
    let mut h = RandomState::new().build_hasher();
    h.write_i64(graine);
    h.write_usize(n);
    h.write_u32(std::process::id());
    format!("{:012x}", h.finish() & 0xffff_ffff_ffff)
}

/// Une bulle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bulle {
    pub id: String,
    pub titre: String,
    pub texte: String,
    /// La candidature qu'elle ouvre (rien : la liste)
    pub candidature: Option<i64>,
}

impl Bulle {
    /// Ce que Mindset lit dans notification.demande : `{ titre, texte, id }`.
    pub fn notification(&self) -> String {
        serde_json::json!({ "titre": self.titre, "texte": self.texte, "id": self.id }).to_string()
    }
}

/// Ce que demandait afficher.demande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Demande {
    /// Rien à faire (pas de fichier, une bulle inconnue, un contenu illisible)
    Rien,
    /// Un numéro de processus : montrer la fenêtre (dans Mindset si elle est gérée)
    Montrer,
    /// Un clic sur une de nos bulles : ouvrir ce qu'elle annonçait (une candidature, ou la liste)
    Ouvrir(Option<i64>),
}

/// Note ces événements comme déjà vus, sans bulle (Jordan vient de taper la candidature : il les voit à l'écran).
pub fn marquer_vues(c: &Connection, dues: &[BulleDue], maintenant: Moment) -> rusqlite::Result<()> {
    let t = c.unchecked_transaction()?;
    for d in dues {
        t.execute(
            "INSERT OR IGNORE INTO bulles_cles (cle, bulle_id, vue_le) VALUES (?1, NULL, ?2)",
            params![d.cle, maintenant.iso()],
        )?;
    }
    t.commit()
}

/// Les événements dus dont la clé n'est pas encore notée.
fn nouvelles<'a>(c: &Connection, dues: &'a [BulleDue]) -> rusqlite::Result<Vec<&'a BulleDue>> {
    let mut r = c.prepare("SELECT 1 FROM bulles_cles WHERE cle = ?1")?;
    let mut l = Vec::new();
    for d in dues {
        if r.query_row([&d.cle], |_| Ok(())).optional()?.is_none() {
            l.push(d);
        }
    }
    Ok(l)
}

/// Met en attente les bulles des événements dus pas encore annoncés : les relances regroupées en une seule, chaque
/// entretien la sienne ; chaque bulle et ses clés notées ensemble. Rend les bulles mises en attente.
pub fn retenir(c: &Connection, dues: &[BulleDue], maintenant: Moment) -> rusqlite::Result<Vec<Bulle>> {
    let neuves = nouvelles(c, dues)?;
    if neuves.is_empty() {
        return Ok(Vec::new());
    }
    let (relances, entretiens): (Vec<&BulleDue>, Vec<&BulleDue>) =
        neuves.into_iter().partition(|d| matches!(d.genre, GenreBulle::Relance { .. }));
    let mut paquets: Vec<Vec<BulleDue>> = entretiens.into_iter().map(|d| vec![d.clone()]).collect();
    if !relances.is_empty() {
        paquets.insert(0, relances.into_iter().cloned().collect());
    }
    let t = c.unchecked_transaction()?;
    let mut faites = Vec::new();
    for (n, p) in paquets.iter().enumerate() {
        let Some((titre, texte)) = texte_bulle(p) else { continue };
        let b = Bulle {
            id: nouvel_id(maintenant.secondes(), n),
            titre,
            texte,
            candidature: match p.as_slice() {
                [d] => Some(d.candidature),
                _ => None,
            },
        };
        t.execute(
            "INSERT INTO bulles (id, titre, texte, candidature_id, etat, cree_le) VALUES (?1, ?2, ?3, ?4, 'attente', ?5)",
            params![b.id, b.titre, b.texte, b.candidature, maintenant.iso()],
        )?;
        for d in p {
            t.execute(
                "INSERT OR IGNORE INTO bulles_cles (cle, bulle_id, vue_le) VALUES (?1, ?2, ?3)",
                params![d.cle, b.id, maintenant.iso()],
            )?;
        }
        faites.push(b);
    }
    t.commit()?;
    Ok(faites)
}

/// L'envoi des bulles à Mindset : le dossier de données, et le moment de la dernière bulle écrite.
#[derive(Debug, Clone)]
pub struct Envoi {
    dossier: PathBuf,
    /// Le moment (en secondes) où la dernière bulle a été écrite, ou une bulle laissée a été vue
    dernier: Option<i64>,
}

impl Envoi {
    pub fn nouveau(dossier: &Path) -> Envoi {
        Envoi { dossier: dossier.to_path_buf(), dernier: None }
    }

    /// Envoie la plus ancienne bulle en attente, si c'est le moment : la précédente a été lue par Mindset, 6 s ont
    /// passé. Une bulle que Mindset n'a pas lue en 5 minutes est abandonnée (il n'est pas ouvert). Rend la bulle
    /// envoyée.
    pub fn envoyer(&mut self, c: &Connection, journal: &Journal, maintenant: Moment) -> Option<Bulle> {
        let s = maintenant.secondes();
        let notif = self.dossier.join(NOTIFICATION);
        if notif.exists() {
            match self.dernier {
                // (laissée par une copie d'avant : comptée de maintenant)
                None => self.dernier = Some(s),
                Some(e) if s - e > ABANDON || s < e => {
                    if std::fs::remove_file(&notif).is_ok() {
                        journal.ecrire("Bulle abandonnée (Mindset ne l'a pas lue en 5 minutes)");
                    }
                    self.dernier = Some(s);
                }
                Some(_) => {}
            }
            return None;
        }
        if self.dernier.is_some_and(|e| (0..ECART).contains(&(s - e))) {
            return None;
        }
        let b = match premiere_en_attente(c) {
            Ok(Some(b)) => b,
            Ok(None) => return None,
            Err(e) => {
                journal.ecrire(&format!("Bulles : base illisible ({e})"));
                self.dernier = Some(s);
                return None;
            }
        };
        // Notée envoyée AVANT d'être écrite : jamais deux fois, même si l'appli plante juste après
        if let Err(e) = c.execute(
            "UPDATE bulles SET etat = 'envoyee', envoyee_le = ?2 WHERE id = ?1",
            params![b.id, maintenant.iso()],
        ) {
            journal.ecrire(&format!("Bulle pas envoyée : base non écrite ({e})"));
            self.dernier = Some(s);
            return None;
        }
        self.dernier = Some(s);
        match ecrire_a_cote(&notif, &b.notification()) {
            Ok(()) => {
                journal.ecrire(&format!("Bulle envoyée ({}) : {} | {}", b.id, b.titre, b.texte));
                Some(b)
            }
            Err(e) => {
                // (pas écrite : remise en attente, réessayée dans 6 s)
                journal.ecrire(&format!("Bulle pas envoyée : {e}"));
                let _ = c.execute("UPDATE bulles SET etat = 'attente', envoyee_le = NULL WHERE id = ?1", [&b.id]);
                None
            }
        }
    }

    /// Lit afficher.demande (toujours supprimé ensuite, rien n'est exécuté) : un clic sur une de nos bulles envoyées
    /// (une candidature supprimée depuis : la liste), un numéro de processus, ou rien.
    pub fn demande(&self, c: &Connection, journal: &Journal) -> Demande {
        let f = self.dossier.join(AFFICHER);
        let Ok(m) = std::fs::metadata(&f) else { return Demande::Rien };
        if !m.is_file() {
            return Demande::Rien;
        }
        let octets = if m.len() < AFFICHER_MAX { std::fs::read(&f).ok() } else { None };
        if let Err(e) = std::fs::remove_file(&f) {
            journal.ecrire(&format!("Demande d'affichage pas supprimée : {e}"));
        }
        let Some(octets) = octets else {
            journal.ecrire("Demande d'affichage trop grosse ou illisible, ignorée");
            return Demande::Rien;
        };
        let texte = String::from_utf8_lossy(&octets);
        let t = texte.trim_start_matches('\u{feff}').trim();
        if let Some(id) = t.strip_prefix("notif:")
            && id_bulle_valable(id)
        {
            let trouvee: rusqlite::Result<Option<Option<i64>>> = c
                .query_row("SELECT b.candidature_id FROM bulles b WHERE b.id = ?1 AND b.etat = 'envoyee'", [id], |r| {
                    r.get::<_, Option<i64>>(0)
                })
                .optional();
            return match trouvee {
                Ok(Some(cible)) => {
                    let existe = cible.filter(|n| {
                        c.query_row("SELECT 1 FROM candidatures WHERE id = ?1", [n], |_| Ok(()))
                            .optional()
                            .ok()
                            .flatten()
                            .is_some()
                    });
                    journal.ecrire(&format!("Bulle ouverte : {id}"));
                    Demande::Ouvrir(existe)
                }
                Ok(None) => {
                    journal.ecrire(&format!("Bulle inconnue ignorée ({id})"));
                    Demande::Rien
                }
                Err(e) => {
                    journal.ecrire(&format!("Bulle non lue : base illisible ({e})"));
                    Demande::Rien
                }
            };
        }
        if (1..=10).contains(&t.len()) && t.bytes().all(|c| c.is_ascii_digit()) {
            journal.ecrire(&format!("Demande d'affichage ({t})"));
            return Demande::Montrer;
        }
        journal.ecrire("Demande d'affichage illisible, ignorée");
        Demande::Rien
    }
}

/// La plus ancienne bulle en attente.
fn premiere_en_attente(c: &Connection) -> rusqlite::Result<Option<Bulle>> {
    c.query_row(
        "SELECT id, titre, texte, candidature_id FROM bulles WHERE etat = 'attente' ORDER BY cree_le, rowid LIMIT 1",
        [],
        |r| Ok(Bulle { id: r.get(0)?, titre: r.get(1)?, texte: r.get(2)?, candidature: r.get(3)? }),
    )
    .optional()
}

/// Écrit un fichier à côté (« .ecriture ») puis le renomme : Mindset ne lit jamais un fichier à moitié écrit. UTF-8
/// sans BOM.
fn ecrire_a_cote(fichier: &Path, texte: &str) -> std::io::Result<()> {
    let mut a_cote = fichier.as_os_str().to_owned();
    a_cote.push(".ecriture");
    let a_cote = PathBuf::from(a_cote);
    std::fs::write(&a_cote, texte.as_bytes())?;
    std::fs::rename(&a_cote, fichier).inspect_err(|_| {
        let _ = std::fs::remove_file(&a_cote);
    })
}
