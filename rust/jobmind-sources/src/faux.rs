//! Un faux serveur HTTP local (127.0.0.1, un port libre), pour les tests et pour la construction d'essai de l'appli
//! (option « faux », jamais livrée) : aucune demande ne part sur internet. Il répond à chaque demande par ce que dit
//! son répondeur, et garde les demandes reçues (les tests vérifient ce qui a été envoyé).
//!
//! [`imitation_avec_sites`] sert les pages d'exemple des sites (`/site/...`), des essais du panneau (`/essai/...`,
//! crate::page::exemples), les quatre sites imités (crate::sites::exemples) et les formulaires de candidature
//! imités. [`offres_inventees`] : des offres inventées (entreprises qui n'existent pas, villes de Moselle), pour les
//! tests de l'archive et la base d'exemple de l'essai.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

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

/// Une offre inventée : (id, titre, entreprise, ville, département, latitude, longitude, contrat, temps,
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

/// Les offres inventées (aucune entreprise réelle).
pub const OFFRES: [Inventee; 12] = [
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
        // (0.1.5 : dans la forme réelle d'une offre de France Travail, comme Jobmind la garde)
        "Offre n° 5100000001\n\nAgent de quai H/F\n\nENNERY (57)Localiser avec Mappy\n\nDébutant accepté\n\n\
Actualisée le 17 novembre 2026\n\n\
Les Entrepôts du Val de Moselle, plateforme logistique de 40 000 m² au nord de Metz, renforcent leur équipe de quai \
pour suivre la hausse de leur activité.\n\n\
Vos missions :\n\
- Charger et décharger les camions au transpalette électrique\n\
- Contrôler les palettes à l'arrivée (quantités, état des colis)\n\
- Ranger les marchandises en zone de stockage\n\
- Préparer les documents de transport avec le chef de quai\n\n\
Aucun diplôme demandé, formation assurée sur place par un tuteur.\n\n\
Profil souhaité\n\nExpérience Débutant accepté\n\nQualification Employé non qualifié\n\nPublic cible Tous\n\n\
Compétences\n\n- Charger et décharger des marchandises\n\n- Contrôler l'état des colis\n\n- Règles de sécurité\n\n\
Savoir-être professionnels\n\n- Travailler en équipe\n\n- Faire preuve de rigueur et de précision\n\n\
- Faire preuve de réactivité\n\n\
Permis\n\n- B - Véhicule léger Souhaité\n\n\
Critères de l\u{2019}offre\n\nIntitulé métier Agent / Agente de quai (N1103)\n\nLieu de travail 57 - ENNERY\n\n\
Contrat CDI - Contrat travail\n\n\
Conditions horaires\n\n- 35H/semaine\n\n- Travail en journée\n\n- Horaires : 7h-12h et 13h-16h du lundi au vendredi\n\n\
Salaire Annuel de 21800.00 Euros à 23500.00 Euros sur 12 mois\n\n\
Avantages\n\n- Titres restaurant / Prime de panier\n\n- Mutuelle\n\n- Prime de fin d'année\n\n\
L'employeur\n\nENTREPOTS DU VAL DE MOSELLE\n\n\
Secteur d'activité Entreposage et stockage non frigorifique\n\nEffectif 20 à 49 salariés\n\n\
Plateforme logistique indépendante installée à Ennery depuis 2009, les Entrepôts du Val de Moselle stockent et \
expédient chaque jour les marchandises des commerces de la région.\n\nVoir toutes les offres de l\u{2019}employeur",
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
        // (0.1.5 : dans la forme réelle d'une offre de France Travail)
        "Offre n° 5100000004\n\nVendeur conseil bricolage\n\nAUGNY (57)Localiser avec Mappy\n\n\
Actualisée le 14 novembre 2026\n\n\
Maison et Outils Lorraine, magasin de bricolage de la zone commerciale d'Augny, recherche un vendeur conseil pour \
son rayon outillage.\n\n\
Vos missions :\n\
- Accueillir et conseiller les clients au rayon outillage\n\
- Assurer la mise en rayon et le balisage des prix\n\
- Passer les commandes des clients professionnels\n\
- Tenir l'encaissement en renfort le samedi\n\n\
Ce que nous vous apportons :\n\
- Une formation aux produits par nos fournisseurs\n\
- Des horaires fixes, connus à l'avance\n\n\
Profil souhaité\n\nExpérience 1 An - Poste équivalent\n\nQualification Employé qualifié\n\n\
Formation\n\n- Bac+2 ou équivalents Commerce Exigé\n\n\
Compétences\n\n- Conseiller un client sur un produit\n\n- Mettre en rayon des produits\n\n\
- Outillage électroportatif\n\n\
Savoir-être professionnels\n\n- Avoir le sens du service\n\n- Faire preuve de curiosité\n\n\
- Travailler en équipe\n\n\
Critères de l\u{2019}offre\n\nIntitulé métier Vendeur / Vendeuse en bricolage (D1213)\n\nLieu de travail 57 - AUGNY\n\n\
Contrat CDI - Contrat travail\n\n\
Conditions horaires\n\n- 28H/semaine\n\n- Travail en journée\n\n- Travail le samedi\n\n\
- Horaires : 9h-12h30 et 14h-18h\n\n\
Salaire Mensuel de 1750.00 Euros à 1900.00 Euros sur 12 mois\n\n\
Avantages\n\n- Remise sur les achats\n\n- Mutuelle\n\n\
L'employeur\n\nMAISON ET OUTILS LORRAINE\n\n\
Secteur d'activité Commerce de détail de quincaillerie, peintures et verres en grandes surfaces\n\n\
Effectif 50 à 99 salariés\n\n\
Magasin indépendant de 6 000 m², Maison et Outils Lorraine équipe les particuliers et les artisans du sud messin \
depuis 1991.\n\nVoir toutes les offres de l\u{2019}employeur",
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
        // (0.1.5 : dans la forme réelle d'une offre de France Travail)
        "Offre n° 5100000007\n\nAgent d'entretien\n\nMETZ (57)Localiser avec Mappy\n\nDébutant accepté\n\n\
Actualisée le 11 novembre 2026\n\n\
Netéa Propreté, entreprise de nettoyage installée à Metz, recherche un agent d'entretien pour un ensemble de bureaux \
du quartier de l'Amphithéâtre.\n\n\
Vos principales missions seront de :\n\
- Vider les corbeilles et trier les déchets\n\
- Dépoussiérer les postes de travail\n\
- Nettoyer les sanitaires et la cuisine\n\
- Laver les sols à l'autolaveuse\n\n\
Profil souhaité\n\nExpérience Débutant accepté\n\nQualification Employé non qualifié\n\nPublic cible Tous\n\n\
Compétences\n\n- Nettoyer des locaux\n\n- Utiliser une autolaveuse\n\n- Règles d'hygiène et de propreté\n\n\
Savoir-être professionnels\n\n- Faire preuve d'autonomie\n\n- Être ponctuel\n\n\
Critères de l\u{2019}offre\n\nIntitulé métier Agent / Agente d'entretien de locaux (K2204)\n\n\
Lieu de travail 57 - METZ\n\n\
Contrat CDD - 6 Mois - Contrat travail\n\n\
Conditions horaires\n\n- 15H/semaine\n\n- Travail en journée\n\n- Horaires : 6h-9h du lundi au vendredi\n\n\
Salaire Horaire de 11.88 Euros\n\n\
Avantages\n\n- Véhicule de service\n\n\
L'employeur\n\nNETEA PROPRETE\n\n\
Secteur d'activité Nettoyage courant des bâtiments\n\nEffectif 10 à 19 salariés\n\n\
Voir toutes les offres de l\u{2019}employeur",
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

/// « 24 000 » (une espace pour les milliers).
fn milliers(n: u32) -> String {
    let s = n.to_string();
    let mut t = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            t.push(' ');
        }
        t.push(c);
    }
    t
}

/// Le salaire en mots : « 21 800 à 23 500 € par an », « 1 750 à 1 900 € par mois », « 11 à 12 € de l'heure ».
fn salaire(min: u32, max: u32) -> String {
    if min == 0 {
        return String::new();
    }
    let unite = match max {
        0..200 => "de l'heure",
        200..10_000 => "par mois",
        _ => "par an",
    };
    format!("{} à {} € {unite}", milliers(min), milliers(max))
}

/// Les offres inventées, telles qu'une source les donne (pour les tests de l'archive, et la base d'exemple de
/// l'essai) : le lieu et ses zones, l'adresse de l'offre, le contrat, le salaire donné, le jour de publication.
pub fn offres_inventees() -> Vec<crate::OffreTrouvee> {
    OFFRES
        .iter()
        .map(|(id, titre, ent, ville, dep, lat, lon, ct, tps, smin, smax, jour, texte)| crate::OffreTrouvee {
            id_source: (*id).into(),
            titre: (*titre).into(),
            entreprise: (*ent).into(),
            lieu: format!("{ville}, {dep}"),
            zones: ["France", "Grand Est", dep, ville].iter().map(|z| (*z).to_string()).collect(),
            url: format!("https://candidat.francetravail.fr/offres/recherche/detail/{id}"),
            description: (*texte).into(),
            contrat: format!("{ct} {tps}"),
            salaire: salaire(*smin, *smax),
            publiee_le: format!("2026-11-{jour:02}"),
            latitude: Some(*lat),
            longitude: Some(*lon),
            brut: json!({ "id": id, "title": titre, "description": texte }).to_string(),
        })
        .collect()
}

/// Le répondeur de l'essai et des tests : les pages imitées des sites ; le mode des quatre sites imités (étape 5 bis :
/// un captcha, une page que Jobmind ne sait plus lire) se change pendant qu'il tourne.
pub fn imitation_avec_sites(
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
        RepFaux::json(404, "{}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le faux serveur local répond, en morceaux ou d'un bloc, et garde ce qu'il a reçu ; une autre adresse : 404.
    #[test]
    fn le_faux_serveur_repond() {
        let s = FauxServeur::demarrer(|r: &Requete| {
            if r.chemin == "/morceaux" {
                RepFaux {
                    morceaux: true,
                    ..RepFaux::json(200, "{\"x\": \"un texte assez long pour plusieurs morceaux\"}")
                }
            } else {
                imitation_avec_sites(Arc::new(Mutex::new(crate::sites::exemples::ModeSites::Normal)))(r)
            }
        })
        .unwrap();
        let demander = |chemin: &str| {
            let mut c = TcpStream::connect(s.adresse().trim_start_matches("http://")).unwrap();
            c.write_all(format!("GET {chemin} HTTP/1.1\r\nHost: x\r\nCookie: a=1\r\n\r\n").as_bytes()).unwrap();
            let mut r = String::new();
            c.read_to_string(&mut r).unwrap();
            r
        };
        let r = demander("/morceaux");
        assert!(r.starts_with("HTTP/1.1 200") && r.contains("chunked"), "{r}");
        assert!(demander("/rien").starts_with("HTTP/1.1 404"));
        let recues = s.recues();
        assert_eq!(recues.len(), 2);
        assert_eq!((recues[0].methode.as_str(), recues[0].cookies.as_str()), ("GET", "a=1"));
    }

    /// Les offres inventées : douze, chacune avec son lieu, son adresse et son texte ; le salaire seulement s'il est
    /// donné.
    #[test]
    fn offres_inventees_completes() {
        let l = offres_inventees();
        assert_eq!(l.len(), OFFRES.len());
        assert_eq!(l[0].titre, "Agent de quai H/F");
        assert_eq!(l[0].lieu, "Ennery, Moselle");
        assert_eq!(l[0].salaire, "21 800 à 23 500 € par an");
        assert_eq!(l[3].salaire, "1 750 à 1 900 € par mois");
        assert_eq!(l[6].salaire, "11 à 12 € de l'heure");
        assert_eq!(l[1].salaire, "");
        assert_eq!(l[11].entreprise, "");
        assert!(l.iter().all(|o| o.url.starts_with("https://") && !o.description.is_empty()));
    }
}
