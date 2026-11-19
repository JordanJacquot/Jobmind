//! Un faux serveur HTTP local (127.0.0.1, un port libre), pour les tests du client et pour la construction d'essai de
//! l'appli (option « faux », jamais livrée) : aucune demande ne part sur internet. Il répond à chaque demande par ce
//! que dit son répondeur, et garde les demandes reçues (les tests vérifient ce qui a été envoyé).
//!
//! [`imitation`] imite Adzuna (`GET /v1/api/jobs/fr/search/1`) avec des offres inventées (entreprises qui n'existent
//! pas, villes de Moselle). Son mode change sa réponse : normale, trop de demandes (429), clé refusée.
//! Étape 5 : il sert aussi les pages d'exemple des sites (`/site/...`) et des essais du panneau (`/essai/...`) :
//! crate::page::exemples.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

/// Une demande reçue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requete {
    pub methode: String,
    /// Le chemin et ses paramètres (« /v1/api/jobs/fr/search/1?app_id=... »)
    pub chemin: String,
    pub corps: String,
    /// L'en-tête `Cookie` reçu (étape 5 bis : les sessions factices des sites imités)
    pub cookies: String,
}

/// Une réponse à rendre.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepFaux {
    pub statut: u16,
    pub corps: String,
    pub type_contenu: String,
    /// Des en-têtes en plus (« cf-mitigated »)
    pub entetes: Vec<(String, String)>,
    /// Le corps envoyé en morceaux (« Transfer-Encoding: chunked »)
    pub morceaux: bool,
    /// Attendre avant de répondre (un serveur trop lent)
    pub attente: Duration,
}

impl RepFaux {
    pub fn json(statut: u16, corps: impl Into<String>) -> RepFaux {
        RepFaux {
            statut,
            corps: corps.into(),
            type_contenu: "application/json; charset=utf-8".into(),
            entetes: Vec::new(),
            morceaux: false,
            attente: Duration::ZERO,
        }
    }

    /// Une page HTML (étape 5 : les pages d'exemple des sites).
    pub fn html(statut: u16, corps: impl Into<String>) -> RepFaux {
        RepFaux { type_contenu: "text/html; charset=utf-8".into(), ..RepFaux::json(statut, corps) }
    }
}

type Repondeur = Arc<dyn Fn(&Requete) -> RepFaux + Send + Sync>;

/// Le serveur, arrêté quand il est lâché.
pub struct FauxServeur {
    port: u16,
    fin: Arc<AtomicBool>,
    recues: Arc<Mutex<Vec<Requete>>>,
}

impl FauxServeur {
    /// Démarre sur un port libre de 127.0.0.1.
    pub fn demarrer(repondeur: impl Fn(&Requete) -> RepFaux + Send + Sync + 'static) -> std::io::Result<FauxServeur> {
        let ecoute = TcpListener::bind("127.0.0.1:0")?;
        let port = ecoute.local_addr()?.port();
        let fin = Arc::new(AtomicBool::new(false));
        let recues = Arc::new(Mutex::new(Vec::new()));
        let repondeur: Repondeur = Arc::new(repondeur);
        let (f, r) = (Arc::clone(&fin), Arc::clone(&recues));
        std::thread::Builder::new().name("faux-serveur".into()).spawn(move || {
            for c in ecoute.incoming() {
                if f.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(c) = c else { continue };
                let (rep, rec) = (Arc::clone(&repondeur), Arc::clone(&r));
                let _ = std::thread::Builder::new().name("faux-client".into()).spawn(move || servir(c, &rep, &rec));
            }
        })?;
        Ok(FauxServeur { port, fin, recues })
    }

    /// « http://127.0.0.1:<port> »
    pub fn adresse(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Les demandes reçues jusqu'ici.
    pub fn recues(&self) -> Vec<Requete> {
        self.recues.lock().map(|l| l.clone()).unwrap_or_default()
    }
}

impl Drop for FauxServeur {
    fn drop(&mut self) {
        self.fin.store(true, Ordering::SeqCst);
        // (une connexion pour débloquer l'attente)
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.port).into(), Duration::from_millis(200));
    }
}

fn servir(c: TcpStream, repondeur: &Repondeur, recues: &Mutex<Vec<Requete>>) {
    let _ = c.set_read_timeout(Some(Duration::from_secs(5)));
    let Ok(copie) = c.try_clone() else { return };
    let mut lecteur = BufReader::new(copie);
    let mut premiere = String::new();
    if lecteur.read_line(&mut premiere).is_err() {
        return;
    }
    let mut morceaux = premiere.split_whitespace();
    let methode = morceaux.next().unwrap_or_default().to_string();
    let chemin = morceaux.next().unwrap_or_default().to_string();
    let mut longueur = 0usize;
    let mut cookies = String::new();
    loop {
        let mut l = String::new();
        if lecteur.read_line(&mut l).unwrap_or(0) == 0 || l.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                longueur = v.trim().parse().unwrap_or(0).min(1_000_000);
            } else if k.trim().eq_ignore_ascii_case("cookie") {
                cookies = v.trim().chars().take(4000).collect();
            }
        }
    }
    let mut corps = vec![0u8; longueur];
    if lecteur.read_exact(&mut corps).is_err() {
        return;
    }
    let requete = Requete { methode, chemin, corps: String::from_utf8_lossy(&corps).into_owned(), cookies };
    if let Ok(mut l) = recues.lock() {
        l.push(requete.clone());
    }
    let r = (**repondeur)(&requete);
    if !r.attente.is_zero() {
        std::thread::sleep(r.attente);
    }
    let mut t = format!("HTTP/1.1 {} Faux\r\nContent-Type: {}\r\nConnection: close\r\n", r.statut, r.type_contenu);
    for (k, v) in &r.entetes {
        t.push_str(&format!("{k}: {v}\r\n"));
    }
    let mut c = c;
    if r.morceaux {
        t.push_str("Transfer-Encoding: chunked\r\n\r\n");
        let o = r.corps.as_bytes();
        for m in o.chunks(37) {
            t.push_str(&format!("{:x}\r\n", m.len()));
            let _ = c.write_all(t.as_bytes());
            t.clear();
            let _ = c.write_all(m);
            t.push_str("\r\n");
        }
        t.push_str("0\r\n\r\n");
        let _ = c.write_all(t.as_bytes());
    } else {
        t.push_str(&format!("Content-Length: {}\r\n\r\n", r.corps.len()));
        let _ = c.write_all(t.as_bytes());
        let _ = c.write_all(r.corps.as_bytes());
    }
    let _ = c.flush();
}

/// Le mode de l'imitation d'Adzuna.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    /// 429 : trop de demandes aujourd'hui
    Quota,
    /// Clé refusée (Adzuna : 401 AUTH_FAIL)
    CleRefusee,
}

/// Une offre inventée : (id, titre, entreprise, ville, département, latitude, longitude, contrat Adzuna, temps,
/// salaire min, salaire max, jour de novembre 2026, texte).
type Inventee = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    f64,
    f64,
    &'static str,
    &'static str,
    u32,
    u32,
    u32,
    &'static str,
);

/// Les offres inventées d'Adzuna (aucune entreprise réelle).
pub const ADZUNA: [Inventee; 12] = [
    (
        "5100000001",
        "Agent de quai H/F",
        "Entrepôts du Val de Moselle",
        "Ennery",
        "Moselle",
        49.226,
        6.218,
        "permanent",
        "full_time",
        21800,
        23500,
        17,
        "Chargement et déchargement des camions, contrôle des palettes à l'arrivée, rangement en zone de stockage. Débutant accepté, formation assurée sur place. Aucun diplôme demandé.",
    ),
    (
        "5100000002",
        "Préparateur de commandes",
        "Logipôle Lorraine",
        "Maizières-lès-Metz",
        "Moselle",
        49.211,
        6.159,
        "contract",
        "full_time",
        0,
        0,
        16,
        "Mission d'intérim de 3 mois renouvelable. Préparation des commandes au scanner, filmage des palettes. Horaires en équipe 5 h à 12 h. Première expérience en entrepôt appréciée.",
    ),
    (
        "5100000003",
        "Cariste CACES 3",
        "Comptoir Mosellan des Matériaux",
        "Woippy",
        "Moselle",
        49.151,
        6.151,
        "permanent",
        "full_time",
        23000,
        25000,
        15,
        "Conduite de chariot élévateur, réception des livraisons, inventaires. CACES 3 à jour exigé, 2 ans d'expérience minimum.",
    ),
    (
        "5100000004",
        "Vendeur conseil bricolage",
        "Maison et Outils Lorraine",
        "Augny",
        "Moselle",
        49.059,
        6.118,
        "permanent",
        "part_time",
        1750,
        1900,
        14,
        "Accueil et conseil des clients au rayon outillage, mise en rayon, encaissement. Temps partiel 28 h. BTS commerce exigé.",
    ),
    (
        "5100000005",
        "Technicien support informatique",
        "Avenir Numérique Est",
        "Metz",
        "Moselle",
        49.1193,
        6.1757,
        "permanent",
        "full_time",
        26000,
        30000,
        13,
        "Assistance aux utilisateurs par téléphone, installation de postes. Télétravail possible deux jours par semaine.",
    ),
    (
        "5100000006",
        "Magasinier",
        "Pièces Auto Moselle",
        "Hagondange",
        "Moselle",
        49.254,
        6.163,
        "contract",
        "full_time",
        0,
        0,
        12,
        "CDD de 6 mois. Réception et rangement des pièces, préparation des expéditions du matin. Bac pro logistique souhaité.",
    ),
    (
        "5100000007",
        "Agent d'entretien",
        "Netéa Propreté",
        "Metz",
        "Moselle",
        49.1193,
        6.1757,
        "contract",
        "part_time",
        11,
        12,
        11,
        "Entretien de bureaux le matin, de 6 h à 9 h. Débutant accepté.",
    ),
    (
        "5100000008",
        "Opérateur logistique de nuit",
        "Entrepôts du Val de Moselle",
        "Ennery",
        "Moselle",
        49.226,
        6.218,
        "permanent",
        "full_time",
        24000,
        26000,
        10,
        "Préparation et contrôle des expéditions de nuit, 21 h à 5 h. Prime de nuit. Sans diplôme, vous serez formé au poste.",
    ),
    (
        "5100000009",
        "Employé libre-service",
        "Super Marché des Arènes",
        "Metz",
        "Moselle",
        49.1193,
        6.1757,
        "contract",
        "full_time",
        0,
        0,
        9,
        "Mise en rayon, rotation des produits frais, aide aux clients. Contrat saisonnier pour les fêtes.",
    ),
    (
        "5100000010",
        "Développeur web en télétravail",
        "Code et Compagnie",
        "Nancy",
        "Meurthe-et-Moselle",
        48.692,
        6.184,
        "permanent",
        "full_time",
        32000,
        38000,
        8,
        "Poste 100 % télétravail. Développement d'applications web, travail en petite équipe.",
    ),
    (
        "5100000011",
        "Apprenti agent logistique",
        "Logipôle Lorraine",
        "Thionville",
        "Moselle",
        49.357,
        6.168,
        "contract",
        "full_time",
        0,
        0,
        7,
        "Contrat d'apprentissage de 2 ans en alternance avec un CAP Opérateur logistique.",
    ),
    (
        "5100000012",
        "Chauffeur livreur",
        "",
        "Pont-à-Mousson",
        "Meurthe-et-Moselle",
        48.905,
        6.054,
        "permanent",
        "full_time",
        0,
        0,
        6,
        "Livraison de colis dans le secteur de Pont-à-Mousson. Permis B exigé.",
    ),
];

/// La réponse d'Adzuna inventée.
pub fn reponse_adzuna() -> String {
    let l: Vec<Value> = ADZUNA
        .iter()
        .map(|(id, titre, ent, ville, dep, lat, lon, ct, tps, smin, smax, jour, texte)| {
            let mut o = json!({
                "__CLASS__": "Adzuna::API::Response::Job",
                "id": id,
                "title": titre,
                "description": texte,
                "location": { "display_name": format!("{ville}, {dep}"), "area": ["France", "Grand Est", dep, ville] },
                "latitude": lat,
                "longitude": lon,
                "redirect_url": format!("https://www.adzuna.fr/land/ad/{id}?se=essai"),
                "created": format!("2026-11-{jour:02}T08:00:00Z"),
                "contract_type": ct,
                "contract_time": tps,
                "category": { "label": "Logistique et entrepôt", "tag": "logistics-warehouse-jobs" },
            });
            if !ent.is_empty() {
                o["company"] = json!({ "display_name": ent });
            }
            if *smin > 0 {
                o["salary_min"] = json!(smin);
                o["salary_max"] = json!(smax);
                o["salary_is_predicted"] = json!("0");
            }
            o
        })
        .collect();
    json!({ "__CLASS__": "Adzuna::API::Response::JobSearchResults", "count": l.len(), "mean": 24000, "results": l })
        .to_string()
}

/// Le répondeur qui imite Adzuna ; `mode` se change pendant qu'il tourne.
pub fn imitation(mode: Arc<Mutex<Mode>>) -> impl Fn(&Requete) -> RepFaux + Send + Sync + 'static {
    imitation_avec_sites(mode, Arc::new(Mutex::new(crate::sites::exemples::ModeSites::Normal)))
}

/// Le même répondeur, avec le mode des quatre sites imités (étape 5 bis : un captcha, une page que Jobmind ne sait
/// plus lire), qui se change pendant qu'il tourne.
pub fn imitation_avec_sites(
    mode: Arc<Mutex<Mode>>,
    sites: Arc<Mutex<crate::sites::exemples::ModeSites>>,
) -> impl Fn(&Requete) -> RepFaux + Send + Sync + 'static {
    move |r: &Requete| {
        // (étape 11 : les formulaires de candidature imités)
        if let Some(rep) = crate::formulaire::exemples::repondre(r, &crate::formulaire::exemples::MODE) {
            return rep;
        }
        // (étape 5 bis : les listes, les offres et les connexions des quatre sites imités)
        if (r.methode == "GET" || r.methode == "POST")
            && let Some(rep) = crate::sites::exemples::repondre(r, &sites)
        {
            return rep;
        }
        // (étape 5 : les pages d'exemple des sites, pour le panneau de l'essai)
        if r.methode == "GET" && (r.chemin.starts_with("/site/") || r.chemin.starts_with("/essai/")) {
            return crate::page::exemples::repondre(&r.chemin);
        }
        let m = mode.lock().map(|m| *m).unwrap_or_default();
        if r.methode != "GET" || !r.chemin.starts_with("/v1/api/jobs/fr/search/") {
            return RepFaux::json(404, "{}");
        }
        match m {
            Mode::Quota => RepFaux::json(429, "{\"exception\":\"TOO_MANY_REQUESTS\"}"),
            Mode::CleRefusee => RepFaux::json(
                401,
                r#"{"exception":"AUTH_FAIL","doc":"https://api.adzuna.com/v1/doc","__CLASS__":"Adzuna::API::Response::Exception","display":"Authorisation failed"}"#,
            ),
            Mode::Normal => RepFaux { morceaux: true, ..RepFaux::json(200, reponse_adzuna()) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adzuna::Adzuna;
    use crate::http::HttpUreq;
    use crate::{JobSource, Recherche};

    fn recherche() -> Recherche {
        Recherche { mots_cles: "agent de quai".into(), lieu: "Metz".into(), rayon_km: 30, nombre: 0 }
    }

    /// De bout en bout, avec le vrai client HTTP (ureq) contre le faux serveur local : la demande envoyée (la clé là
    /// où il faut), la réponse en morceaux lue, les offres rendues.
    #[test]
    fn adzuna_par_le_vrai_client() {
        let mode = Arc::new(Mutex::new(Mode::Normal));
        let s = FauxServeur::demarrer(imitation(Arc::clone(&mode))).unwrap();
        let http = HttpUreq::nouveau(false);
        let a = Adzuna::nouvelle(
            &http,
            &format!("{}/v1/api/jobs/fr/search", s.adresse()),
            "1a2b3c4d",
            "0123456789abcdef0123456789abcdef",
        );
        let l = a.chercher(&recherche()).unwrap();
        assert_eq!(l.len(), ADZUNA.len());
        assert_eq!(l[0].titre, "Agent de quai H/F");
        assert_eq!(l[11].entreprise, "");
        let r = s.recues();
        assert_eq!(r.len(), 1);
        assert!(r[0].chemin.starts_with("/v1/api/jobs/fr/search/1?"), "{}", r[0].chemin);
        assert!(r[0].chemin.contains("app_key=0123456789abcdef0123456789abcdef"));
        assert!(r[0].chemin.contains("what=agent") && r[0].chemin.contains("where=Metz"), "{}", r[0].chemin);
        // Une autre demande (un POST, une autre adresse) : rien
        assert_eq!(imitation(Arc::clone(&mode))(&Requete { methode: "POST".into(), ..r[0].clone() }).statut, 404);
        // Les modes : quota, clé refusée
        *mode.lock().unwrap() = Mode::Quota;
        assert_eq!(a.chercher(&recherche()).unwrap_err().code(), "quota");
        *mode.lock().unwrap() = Mode::CleRefusee;
        assert_eq!(a.chercher(&recherche()).unwrap_err().code(), "cle_refusee");
    }

    /// Le réseau coupé (un port fermé), une page de protection, une réponse abîmée : chaque fois une erreur simple, sans
    /// la clé, et jamais de plantage.
    #[test]
    fn reseau_coupe_et_reponses_abimees() {
        let http = HttpUreq::nouveau(false);
        // (un port qui vient d'être libéré : personne n'écoute)
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let a = Adzuna::nouvelle(
            &http,
            &format!("http://127.0.0.1:{port}/v1/api/jobs/fr/search"),
            "1a2b3c4d",
            "0123456789abcdef0123456789abcdef",
        );
        let e = a.chercher(&recherche()).unwrap_err();
        assert_eq!(e.code(), "reseau");
        assert!(!e.to_string().contains("0123456789abcdef"), "{e}");
        assert!(e.message().contains("connexion à internet"));
        let s = FauxServeur::demarrer(|r: &Requete| {
            if r.chemin.contains("protege") {
                RepFaux {
                    type_contenu: "text/html; charset=UTF-8".into(),
                    entetes: vec![("cf-mitigated".into(), "challenge".into())],
                    ..RepFaux::json(403, "<!DOCTYPE html><title>Just a moment...</title>")
                }
            } else {
                RepFaux::json(200, "{\"results\": [ {\"title\": \"coupé")
            }
        })
        .unwrap();
        let adresse = format!("{}/v1/api/jobs/fr/search", s.adresse());
        let cle = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            Adzuna::nouvelle(&http, &adresse, "protege1", cle).chercher(&recherche()).unwrap_err().code(),
            "bloquee"
        );
        let e = Adzuna::nouvelle(&http, &adresse, "abime123", cle).chercher(&recherche()).unwrap_err();
        assert_eq!(e.code(), "illisible");
        assert!(!e.to_string().contains(cle), "{e}");
        // Le vrai Jobmind refuse le http
        let sur = HttpUreq::nouveau(true);
        assert_eq!(
            Adzuna::nouvelle(&sur, &adresse, "abime123", cle).chercher(&recherche()).unwrap_err().code(),
            "reseau"
        );
    }

    /// Un serveur trop lent : le délai d'attente coupe (ici réglé court pour le test : le vrai est de 25 s).
    #[test]
    fn serveur_trop_lent() {
        let s = FauxServeur::demarrer(|_: &Requete| RepFaux {
            attente: Duration::from_secs(4),
            ..RepFaux::json(200, "{}")
        })
        .unwrap();
        let config = ureq::Agent::config_builder().timeout_global(Some(Duration::from_millis(800))).build();
        let agent: ureq::Agent = config.into();
        let debut = std::time::Instant::now();
        let r = agent.get(&format!("{}/x", s.adresse())).call();
        assert!(matches!(r, Err(ureq::Error::Timeout(_))), "pas de délai dépassé");
        assert!(debut.elapsed() < Duration::from_secs(3));
    }
}
