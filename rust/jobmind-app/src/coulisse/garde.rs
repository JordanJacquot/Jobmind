//! « Garder le mot de passe » (décision de Jordan) : quand une connexion tapée par Jobmind réussit, l'identifiant et
//! le mot de passe de CE site sont gardés avec les clés (`data\cles.json`, chiffrés par DPAPI pour ce compte Windows et
//! ce PC : jobmind_storage::cles, les clés `site_<site>_mail` et `site_<site>_mdp`). Ils ne sortent jamais de Rust :
//! ni dans l'état de la page (seulement « gardés » ou non), ni au journal, ni dans la base.
//!
//! Quand une page lue montre plus tard que le site demande de se reconnecter (sa page de connexion, ou « pas
//! connecté » quand plus rien n'est en cours), Jobmind les retape tout seul en coulisse, sans rien demander : au plus
//! une fois par site depuis le lancement ([`Coulisse::essais_auto`]), pour ne pas faire bloquer le compte. Refusés par
//! le site : gardés quand même, et la carte de connexion s'ouvre comme avant. Un captcha ou un code : la petite
//! fenêtre, comme avant. « Oublier ma connexion » les efface ; une connexion réussie avec un nouveau mot de passe
//! remplace l'ancien.
//!
//! Le module [`super`] ne fait que décider (sans clés) ; après chaque geste, le programme appelle [`suite`], qui
//! garde, efface et relit les identifiants.

use jobmind_core::cle::Cle;
use jobmind_sources::page::Site;
use jobmind_storage::cles::{Cles, EtatCle};

use super::{Connexion, Coulisse, Secret, indice, ordres};
use crate::suivi::Fait;

/// L'identifiant et le mot de passe gardés d'un site.
fn cles_de(s: Site) -> Option<(Cle, Cle)> {
    Cle::du_site(s.code())
}

/// Les sites dont l'identifiant et le mot de passe sont gardés et lisibles sur ce PC (dans l'ordre de `Site::CONNUS`).
pub fn gardes(cles: &Cles) -> [bool; 6] {
    let mut g = [false; 6];
    for (i, s) in Site::CONNUS.iter().enumerate() {
        g[i] = cles_de(*s).is_some_and(|(m, p)| {
            matches!(cles.etat(m), Some(EtatCle::Gardee(_))) && matches!(cles.etat(p), Some(EtatCle::Gardee(_)))
        });
    }
    g
}

impl Coulisse {
    /// Au lancement : les sites dont les identifiants sont gardés ([`gardes`]).
    pub fn avec_gardes(gardes: [bool; 6]) -> Coulisse {
        // (« Où chercher » s'ouvre sur France Travail, la source de référence depuis le retrait d'Adzuna)
        Coulisse { gardes, source: Some(Site::FranceTravail), ..Coulisse::default() }
    }

    /// La reconnexion automatique : les identifiants gardés tapés en coulisse, comme « Me connecter », mais sans
    /// rien demander. Ce que la page de connexion avait interrompu est rechargé une fois connecté.
    pub(crate) fn connecter_auto(&mut self, s: Site, mail: String, mdp: Secret) -> Fait {
        let apres = self.apres_reconnexion.take();
        if !self.libre() || mail.trim().is_empty() || mdp.est_vide() {
            return Fait::default();
        }
        let Some(u) = jobmind_sources::sites::adresse_connexion(s) else { return Fait::default() };
        let id = self.nouvel_id();
        self.connexion = Some(Connexion {
            id,
            site: s,
            mail,
            mdp: Some(mdp),
            mail_tape: false,
            mdp_tape: false,
            garder: None,
            auto: true,
            apres,
        });
        self.annonces[indice(s)] = None;
        let o = self.charger(s, super::But::Connexion, u.to_string());
        Fait {
            journal: Some(format!("Sites : reconnexion automatique à {} (identifiants gardés)", s.code())),
            ..ordres(o, true)
        }
    }
}

/// Après chaque geste : les identifiants à effacer (« Oublier ma connexion », « Me déconnecter de tous les sites »),
/// ceux d'une connexion qui vient de réussir à garder, la liste des sites gardés relue, puis la reconnexion
/// automatique demandée, avec les identifiants gardés. Rien à faire : `Fait::default()`.
pub fn suite(c: &mut Coulisse, cles: Option<&mut Cles>) -> Fait {
    let Some(cles) = cles else {
        // (sans coffre, rien ne se garde ni ne se relit)
        c.a_garder = None;
        c.a_oublier.clear();
        c.reconnecter = None;
        c.apres_reconnexion = None;
        return Fait::default();
    };
    let mut journal = Vec::new();
    let mut message = None;
    let avant = c.gardes;
    for s in std::mem::take(&mut c.a_oublier) {
        let Some((m, p)) = cles_de(s) else { continue };
        match cles.effacer(&[m, p]) {
            Ok(true) => journal.push(format!("Sites : connexion gardée de {} oubliée", s.code())),
            Ok(false) => {}
            Err(e) => {
                journal.push(format!("Sites : connexion gardée de {} non effacée ({e})", s.code()));
                message =
                    Some("Jobmind n'a pas pu oublier ta connexion. Ferme Jobmind, rouvre-la, puis réessaie.".into());
            }
        }
    }
    if let Some(x) = c.a_garder.take()
        && let Some((m, p)) = cles_de(x.site)
    {
        // (l'erreur vient du chiffrement ou du disque : jamais le mail ni le mot de passe dedans)
        match cles.garder_plusieurs(&[(m, x.mail.as_str()), (p, x.mdp.texte())]) {
            Ok(()) => journal.push(format!("Sites : connexion à {} gardée sur ce PC (chiffrée)", x.site.code())),
            Err(e) => journal.push(format!("Sites : connexion à {} non gardée ({e})", x.site.code())),
        }
    }
    c.gardes = gardes(cles);
    let mut f = Fait::default();
    if let Some(s) = c.reconnecter.take() {
        let lus = cles_de(s).and_then(|(m, p)| Some((cles.lire(m)?, Secret::new(cles.lire(p)?))));
        match lus {
            Some((mail, mdp)) if c.gardes[indice(s)] => f = c.connecter_auto(s, mail, mdp),
            _ => {
                c.apres_reconnexion = None;
                journal.push(format!("Sites : identifiants gardés de {} illisibles, pas de reconnexion", s.code()));
            }
        }
    }
    if let Some(j) = f.journal.take() {
        journal.push(j);
    }
    if !journal.is_empty() {
        f.journal = Some(journal.join(" ; "));
    }
    f.change |= c.gardes != avant || f.journal.is_some();
    if message.is_some() {
        f.message = message;
    }
    f
}
