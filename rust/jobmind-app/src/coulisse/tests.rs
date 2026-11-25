//! Les tests des sites en coulisse, sans fenêtre : un petit « navigateur » d'essai demande les pages aux quatre sites
//! imités (jobmind_sources::sites::exemples, inventés, servis sans réseau), suit leurs redirections, garde leurs
//! cookies et envoie leurs formulaires comme la vue des sites le ferait.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use jobmind_core::recherches::NouvelleRecherche;
use jobmind_sources::faux::{RepFaux, Requete};
use jobmind_sources::html::Document;
use jobmind_sources::sites::exemples::{self, MAIL_ESSAI, MDP_ESSAI, ModeSites};
use jobmind_sources::sites::{adresse_recherche, comprendre, encoder};

use super::*;

fn m(h: u32, mi: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, mi, 0).unwrap()
}

fn jour() -> jobmind_core::Jour {
    m(9, 0).jour()
}

/// Une base avec une recherche gardée (« cariste », Metz, 30 km) : sa base et son numéro.
fn base() -> (Base, i64) {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let n = NouvelleRecherche {
        nom: "Cariste à Metz".into(),
        mots: "cariste".into(),
        lieu: Lieu::Ville { nom: "Metz".into(), rayon: 30 },
    };
    let id = base_offres::ajouter_recherche(b.connexion(), &n, m(9, 0)).unwrap();
    (b, id)
}

/// Le navigateur d'essai : la page affichée, les cookies, le mode des sites imités.
struct Navigateur {
    adresse: String,
    html: String,
    cookies: Vec<String>,
    mode: Mutex<ModeSites>,
}

impl Navigateur {
    fn nouveau() -> Navigateur {
        Navigateur {
            adresse: String::new(),
            html: String::new(),
            cookies: Vec::new(),
            mode: Mutex::new(ModeSites::Normal),
        }
    }

    fn demander(&mut self, methode: &str, adresse: &str, corps: &str) {
        let chemin = format!("/site/{}", adresse.split("://").nth(1).unwrap_or_default());
        let r = Requete { methode: methode.into(), chemin, corps: corps.into(), cookies: self.cookies.join("; ") };
        let rep: RepFaux =
            exemples::repondre(&r, &self.mode).unwrap_or_else(|| jobmind_sources::page::exemples::repondre(&r.chemin));
        for (k, v) in &rep.entetes {
            if k == "Set-Cookie" {
                let c = v.split(';').next().unwrap_or_default().to_string();
                self.cookies.retain(|x| x.split('=').next() != c.split('=').next());
                self.cookies.push(c);
            }
        }
        match rep.entetes.iter().find(|(k, _)| k == "Location") {
            Some((_, l)) => {
                let suite = format!("https://{}", l.trim_start_matches("/site/"));
                self.demander("GET", &suite, "");
            }
            None => {
                self.adresse = adresse.to_string();
                self.html = rep.corps;
            }
        }
    }

    fn aller(&mut self, adresse: &str) {
        self.demander("GET", adresse, "");
    }

    fn lecture(&self) -> Lecture {
        Lecture { brute: self.adresse.clone(), comprise: comprendre(&self.adresse, &self.html), formulaire: None }
    }

    /// Ce que fait le script de saisie : le mail et le mot de passe dans leurs cases, puis le formulaire envoyé.
    fn taper(&mut self, mail: Option<&str>, mdp: Option<&str>) {
        let d = Document::lire(&self.html);
        let f = d.trouver("form").expect("un formulaire");
        let action = d.attr(f, "action").unwrap_or_default().to_string();
        let mut corps = Vec::new();
        for i in d.tous("form input", 50) {
            let nom = d.attr(i, "name").unwrap_or_default();
            let v = match d.attr(i, "type").unwrap_or("text") {
                "hidden" => d.attr(i, "value").unwrap_or_default().to_string(),
                "password" => mdp.unwrap_or_default().to_string(),
                _ => mail.unwrap_or_default().to_string(),
            };
            corps.push(format!("{nom}={}", encoder(&v)));
        }
        let cible = format!("https://{}", action.trim_start_matches("/site/"));
        self.demander("POST", &cible, &corps.join("&"));
    }
}

/// Le premier ordre « Lire » d'un fait (son numéro).
fn lire_no(f: &Fait) -> u64 {
    f.coulisse
        .iter()
        .find_map(|o| match o {
            Ordre::Lire { no, .. } => Some(*no),
            _ => None,
        })
        .unwrap_or_else(|| panic!("pas de lecture dans {f:?}"))
}

/// Le premier chargement d'un fait.
fn charge(f: &Fait) -> String {
    f.coulisse
        .iter()
        .find_map(|o| match o {
            Ordre::Charger(u) => Some(u.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("pas de chargement dans {f:?}"))
}

/// Charge la page demandée par `f` dans le navigateur, puis la fait lire : ce que la lecture a donné.
fn charger_et_lire(c: &mut Coulisse, b: Option<&Base>, n: &mut Navigateur, f: &Fait) -> Fait {
    n.aller(&charge(f));
    let p = c.page_chargee(&n.adresse);
    c.lu(b, lire_no(&p), n.lecture(), m(9, 5))
}

/// Les pas de la lecture automatique jusqu'au prochain chargement (au plus `max` secondes) : le moment du
/// chargement et ce qu'il a donné.
fn jusqu_au_chargement(c: &mut Coulisse, depuis: Instant, max: u64) -> Option<(Instant, Fait)> {
    for s in 0..=max {
        let t = depuis + Duration::from_secs(s);
        let f = c.lecture_auto(t, true);
        if f.coulisse.iter().any(|o| matches!(o, Ordre::Charger(_))) {
            return Some((t, f));
        }
    }
    None
}

/// 0.2.0 : la lecture de toutes les offres menée jusqu'au bout (faux temps, à partir de `depuis`) : chaque offre
/// demandée est lue, chaque page suivante chargée toute seule aussi. Le nombre de pages chargées, le nombre d'offres
/// lues, et le moment du dernier chargement.
fn lire_jusqu_au_bout(c: &mut Coulisse, b: &Base, n: &mut Navigateur, depuis: Instant) -> (usize, usize, Instant) {
    let (mut t, mut pages, mut offres, mut dernier) = (depuis, 0, 0, depuis);
    while let Some((x, f)) = jusqu_au_chargement(c, t, 10) {
        match c.attente.as_ref().map(|a| a.but) {
            Some(But::Liste) => pages += 1,
            Some(But::Lecture(_)) => offres += 1,
            but => panic!("un chargement inattendu ({but:?}) : {f:?}"),
        }
        assert!(pages + offres <= 40, "la lecture tourne en rond : {pages} pages, {offres} offres");
        charger_et_lire(c, Some(b), n, &f);
        dernier = x;
        t = x + Duration::from_secs(1);
    }
    (pages, offres, dernier)
}

/// Tous les textes d'une valeur.
fn textes_de(v: &Value, l: &mut Vec<String>) {
    match v {
        Value::String(s) => l.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| textes_de(x, l)),
        Value::Object(o) => o.values().for_each(|x| textes_de(x, l)),
        _ => {}
    }
}

fn sans_tiret_long(v: &Value) {
    let mut l = Vec::new();
    textes_de(v, &mut l);
    assert!(l.iter().all(|t| !t.contains(['\u{2013}', '\u{2014}'])), "{l:?}");
}

#[test]
fn liste_offre_et_ajout() {
    let (b, rid) = base();
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    // La source choisie (un code inconnu : mes offres)
    let f = c.geste(Some(&b), SOURCE, &json!({ "site": "linkedin" }), m(9, 0));
    assert!(f.change && c.source == Some(Site::LinkedIn));
    c.geste(Some(&b), SOURCE, &json!({ "site": "javascript:alert(1)" }), m(9, 0));
    assert_eq!(c.source, None);
    // « Chercher sur LinkedIn » : l'adresse de la page de résultats, rien d'autre
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 0));
    let u = charge(&f);
    let metz = LieuSite::Ville { nom: "Metz".into(), rayon_km: 30 };
    assert_eq!(u, adresse_recherche(Site::LinkedIn, "cariste", &metz, 1).unwrap());
    assert_eq!(f.coulisse.len(), 1);
    assert!(c.vue(jour())["attente"].as_str().unwrap().contains("Jobmind lit les résultats sur LinkedIn"));
    // Un second clic pendant la lecture : rien de chargé
    let f2 = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 0));
    assert!(f2.coulisse.is_empty() && f2.message.is_some());
    // Une page chargée : lue un peu après ; une lecture d'un autre numéro est laissée
    n.aller(&u);
    let p = c.page_chargee(&n.adresse);
    assert!(matches!(p.coulisse[..], [Ordre::Lire { delai_ms: ATTENTE_LECTURE_MS, .. }]));
    assert_eq!(c.lu(Some(&b), 999, n.lecture(), m(9, 1)), Fait::default());
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    assert!(f.change && f.coulisse.is_empty(), "{f:?}");
    assert!(f.journal.as_deref().unwrap().contains("liste lue de linkedin (6 résultats, page 1, lue par regles)"));
    // (0.2.0 : la liste est lue, 6 offres et une page derrière ; elle est montrée en cartes, et une offre n'a sa
    // carte qu'une fois analysée par Jobmind : aucune pour l'instant, une phrase le dit)
    let l = c.liste_de(Site::LinkedIn).unwrap();
    assert_eq!((l.resultats.len(), l.resultats[0].titre.as_str(), l.suivante), (6, "Cariste H/F", true));
    assert!(l.fusion && l.tout && l.carte_page == 1, "{l:?}");
    let v = c.vue(jour());
    assert_eq!(v["liste"]["cartes"], true);
    assert!(v["liste"]["lignes"].as_array().unwrap().is_empty(), "{v}");
    assert_eq!(v["liste"]["sous"], "0 offre analysée");
    assert!(v["liste"]["vide_texte"].as_str().unwrap().starts_with("Jobmind analyse les offres"), "{v}");
    assert_eq!(v["liste"]["lecture"]["texte"], "Jobmind lit toutes les offres : 0 lues");
    assert_eq!((&v["liste"]["precedente"], &v["liste"]["suivante"]), (&json!(false), &json!(false)));
    assert_eq!(v["sites"][0]["etat"], "Pas connecté à LinkedIn");
    assert!(v["attente"].is_null());
    // (la liste déroulante « Où chercher » : « Mes offres gardées », sans recherche, pour les offres gardées)
    assert_eq!(v["textes"]["ou_t"], "Où chercher");
    assert_eq!(v["textes"]["mes_offres"], "Mes offres gardées");
    assert!(!v.to_string().contains("Adzuna"));
    // (au lancement de Jobmind, « Où chercher » s'ouvre sur France Travail)
    let depart = Coulisse::avec_gardes([false; 6]);
    assert_eq!(depart.source, Some(Site::FranceTravail));
    assert_eq!(depart.vue(jour())["source"], "francetravail");
    sans_tiret_long(&v);
    // Un résultat ouvert : son offre lue en coulisse, montrée dans le design de Jobmind
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 2));
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(f.journal.as_deref().unwrap().starts_with("Sites : offre lue de linkedin"), "{f:?}");
    let v = c.vue(jour());
    assert_eq!(v["fiche"]["titre"], "Agent de quai (H/F)");
    assert_eq!(v["fiche"]["candidats"], "12 candidats sur LinkedIn");
    // (l'offre ouverte est lue : elle a sa carte, la seule, marquée ; son `i` reste son rang dans les résultats)
    let lignes = v["liste"]["lignes"].as_array().unwrap();
    assert_eq!(lignes.len(), 1, "{lignes:?}");
    assert_eq!((&lignes[0]["i"], &lignes[0]["sel"], &lignes[0]["etat"]), (&json!(1), &json!(true), &json!("lue")));
    assert_eq!(lignes[0]["titre"], "Agent de quai (H/F)");
    assert_eq!(v["liste"]["sous"], "1 offre analysée");
    sans_tiret_long(&v);
    // « Ajouter à Jobmind » : gardée dans l'archive
    let f = c.geste(Some(&b), AJOUTER, &Value::Null, m(9, 3));
    assert!(f.bravo, "{f:?}");
    let id = c.fiche.as_ref().unwrap().ajoutee.unwrap();
    let o = base_offres::lire(b.connexion(), id).unwrap().unwrap();
    assert_eq!(o.offre.titre, "Agent de quai (H/F)");
    assert_eq!(o.candidats.unwrap().texte, "12 candidats");
    assert_eq!(c.vue(jour())["fiche"]["ajoutee"], id);
    // Le même résultat rouvert : rien de rechargé
    assert!(c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 4)).coulisse.is_empty());
    // « Page suivante » et « Page précédente » (0.2.0) : ce sont les pages de cartes de Jobmind, plus celles du site ;
    // une seule page de cartes ici : rien ne bouge, et surtout rien n'est chargé (avant, la page 2 du site partait
    // sur ce clic)
    let v = c.vue(jour());
    assert_eq!((&v["liste"]["precedente"], &v["liste"]["suivante"]), (&json!(false), &json!(false)));
    assert_eq!(c.geste(Some(&b), PRECEDENTE, &Value::Null, m(9, 5)), Fait::default(), "pas de page avant la 1");
    assert_eq!(c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 5)), Fait::default(), "pas de page de cartes après");
    assert!(c.attente.is_none());
    assert_eq!(c.liste_de(Site::LinkedIn).unwrap().page, 1);
    // La page 2 du site vient toute seule : Jobmind lit les offres de la page 1 (pas la 2e, déjà lue par le clic),
    // puis charge la page suivante, dont les offres s'ajoutent au bout de la liste
    let (mut t, mut pages, mut lues) = (Instant::now(), 0, Vec::new());
    while let Some((x, f)) = jusqu_au_chargement(&mut c, t, 10) {
        match c.attente.as_ref().unwrap().but {
            But::Liste => {
                pages += 1;
                assert!(charge(&f).ends_with("&start=25"), "{f:?}");
                assert_eq!(f.journal.as_deref(), Some("Sites : page 2 de linkedin (lecture de toutes les offres)"));
            }
            But::Lecture(i) => lues.push(i),
            but => panic!("{but:?}"),
        }
        assert!(pages <= 1 && lues.len() <= 8, "{pages} pages, {lues:?}");
        charger_et_lire(&mut c, Some(&b), &mut n, &f);
        t = x + Duration::from_secs(1);
    }
    assert_eq!((pages, lues), (1, vec![0, 2, 3, 4, 5, 6, 7, 8]));
    let v = c.vue(jour());
    // (les 3 offres de la page 2 sont à la suite des 6 de la page 1, dont les rangs n'ont pas bougé ; l'offre ouverte
    // reste ouverte, sa carte est la seule marquée)
    let lignes = v["liste"]["lignes"].as_array().unwrap();
    assert_eq!(lignes.len(), 9);
    assert!(lignes.iter().enumerate().all(|(k, l)| l["i"] == k && l["etat"] == "lue"), "{lignes:?}");
    assert_eq!(lignes[0]["titre"], "Cariste H/F");
    assert!(lignes[6..].iter().all(|l| lignes[..6].iter().all(|x| x["titre"] != l["titre"])), "{lignes:?}");
    assert!(lignes.iter().all(|l| (l["sel"] == true) == (l["i"] == 1)), "{lignes:?}");
    assert_eq!(v["fiche"]["titre"], "Agent de quai (H/F)");
    assert_eq!((&v["liste"]["precedente"], &v["liste"]["suivante"]), (&json!(false), &json!(false)));
    assert_eq!(v["liste"]["sous"], "9 offres analysées");
    assert_eq!(v["liste"]["page_texte"], "Page 1 sur 1 · offres 1 à 9");
    assert_eq!(v["liste"]["page"], 1);
    assert_eq!(v["liste"]["lecture"]["texte"], "Les 9 offres de la liste sont lues");
    assert_eq!(v["textes"]["suivante"], "Page suivante");
    assert_eq!(v["textes"]["precedente"], "Page précédente");
    sans_tiret_long(&v);
    // (toujours rien à tourner à la main, et rien de chargé)
    assert_eq!(c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 6)), Fait::default());
    assert_eq!(c.geste(Some(&b), PRECEDENTE, &Value::Null, m(9, 6)), Fait::default());
    // (le même résultat rouvert après la page 2 : toujours rien de rechargé)
    assert!(c.geste(Some(&b), OUVRIR, &json!({ "i": 1 }), m(9, 6)).coulisse.is_empty());
    // (une offre de la page 2, lue par Jobmind : ouverte tout de suite, sans rien recharger)
    let f = c.geste(Some(&b), OUVRIR, &json!({ "i": 7 }), m(9, 6));
    assert!(f.change && f.coulisse.is_empty(), "{f:?}");
    assert_eq!(c.vue(jour())["liste"]["lignes"][7]["sel"], true);
    let l = c.liste_de(Site::LinkedIn).unwrap();
    assert_eq!((l.page, l.resultats.len(), l.gardees.len(), l.suivante), (2, 9, 0, false));
    assert!(!l.tout && l.lecture.finie, "{l:?}");
    // « Voir dans mes offres »
    let f = c.geste(Some(&b), VOIR_OFFRE, &json!({ "id": id }), m(9, 7));
    assert_eq!(f.offre, Some(id));
    assert_eq!(c.source, None);
}

#[test]
fn une_recherche_est_necessaire() {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let mut c = Coulisse::default();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "indeed" }), m(9, 0));
    assert!(f.coulisse.is_empty());
    assert!(c.annonces[1].as_ref().unwrap().texte.contains("Nouvelle recherche"));
    let f = c.geste(None, CHERCHER, &json!({ "site": "indeed" }), m(9, 0));
    assert_eq!(f.message.as_deref(), Some(SANS_BASE));
}

#[test]
fn connexion_en_un_ecran() {
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    // Rien sans mail ni mot de passe, ni pour un autre site
    assert!(c.connecter("linkedin", " ", Secret::new(MDP_ESSAI.into())).coulisse.is_empty());
    assert!(c.connecter("linkedin", MAIL_ESSAI, Secret::new("  ".into())).coulisse.is_empty());
    assert!(c.connecter("myspace", MAIL_ESSAI, Secret::new(MDP_ESSAI.into())).coulisse.is_empty());
    let f = c.connecter("linkedin", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    assert_eq!(charge(&f), "https://www.linkedin.com/login/fr");
    // Le mot de passe n'est jamais dans ce qui s'affiche ou se note
    assert!(!format!("{c:?}").contains(MDP_ESSAI));
    assert!(!c.vue(jour()).to_string().contains(MDP_ESSAI));
    // La page de connexion lue : le mail et le mot de passe tapés, une seule fois
    let f = charger_et_lire(&mut c, None, &mut n, &f);
    let (mail, mdp) = match &f.coulisse[0] {
        Ordre::Taper { attendu, mail, mdp, .. } => {
            assert_eq!(attendu, "https://www.linkedin.com/login/fr");
            (mail.clone(), mdp.clone())
        }
        o => panic!("{o:?}"),
    };
    assert_eq!(mail.as_deref(), Some(MAIL_ESSAI));
    assert_eq!(mdp.as_ref().map(Secret::texte), Some(MDP_ESSAI));
    assert!(!format!("{f:?}").contains(MDP_ESSAI), "le mot de passe ne s'affiche pas");
    assert!(!f.journal.as_deref().unwrap().contains(MDP_ESSAI));
    // (Jobmind ne le garde plus : il est parti avec la saisie)
    assert!(c.connexion.as_ref().unwrap().mdp.is_none());
    let no = lire_no(&f);
    // Le formulaire envoyé : le site pose la session ; la page suivante est lue
    n.taper(mail.as_deref(), mdp.as_ref().map(Secret::texte));
    drop(mdp);
    assert!(n.cookies.iter().any(|x| x == "essai_linkedin=1"), "{:?}", n.cookies);
    let p = c.page_chargee(&n.adresse);
    assert_ne!(lire_no(&p), no, "la page chargée remplace la lecture prévue");
    let f = c.lu(None, lire_no(&p), n.lecture(), m(9, 2));
    assert_eq!(f.message.as_deref(), Some("Tu es connecté à LinkedIn."));
    assert!(c.connexion.is_none() && c.attente.is_none());
    assert_eq!(c.vue(jour())["sites"][0]["etat"], "Connecté à LinkedIn");
    assert_eq!(c.vue(jour())["sites"][0]["connecte"], true);
}

#[test]
fn connexion_en_deux_ecrans() {
    for (site, i) in [("indeed", 1), ("francetravail", 3)] {
        let mut c = Coulisse::default();
        let mut n = Navigateur::nouveau();
        let f = c.connecter(site, MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
        // 1. le mail seul
        let f = charger_et_lire(&mut c, None, &mut n, &f);
        let Ordre::Taper { mail, mdp, .. } = &f.coulisse[0] else { panic!("{f:?}") };
        assert_eq!((mail.as_deref(), mdp.is_none()), (Some(MAIL_ESSAI), true), "{site}");
        n.taper(mail.as_deref(), None);
        // 2. le mot de passe seul
        let p = c.page_chargee(&n.adresse);
        let f = c.lu(None, lire_no(&p), n.lecture(), m(9, 1));
        let Ordre::Taper { mail, mdp, .. } = &f.coulisse[0] else { panic!("{site} : {f:?}") };
        assert_eq!((mail.is_none(), mdp.as_ref().map(Secret::texte)), (true, Some(MDP_ESSAI)), "{site}");
        n.taper(None, mdp.as_ref().map(Secret::texte));
        // 3. connecté
        let p = c.page_chargee(&n.adresse);
        let f = c.lu(None, lire_no(&p), n.lecture(), m(9, 2));
        assert!(f.bravo, "{site} : {f:?}");
        assert_eq!(c.connecte[i], Some(true));
    }
}

#[test]
fn un_seul_essai_si_le_site_refuse() {
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    let f = c.connecter("linkedin", MAIL_ESSAI, Secret::new("pas-le-bon".into()));
    let f = charger_et_lire(&mut c, None, &mut n, &f);
    let Ordre::Taper { mail, mdp, .. } = &f.coulisse[0] else { panic!("{f:?}") };
    n.taper(mail.as_deref(), mdp.as_ref().map(Secret::texte));
    // La page de connexion revient (le site a refusé) : aucun second essai
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(None, lire_no(&p), n.lecture(), m(9, 1));
    assert!(f.coulisse.is_empty(), "{f:?}");
    assert!(f.journal.unwrap().contains("refusée par le site"));
    let a = c.annonces[0].as_ref().unwrap();
    assert!(a.texte.starts_with("LinkedIn n'a pas accepté ce mail ou ce mot de passe."));
    assert_eq!(a.bouton, Some("page_site"));
    assert!(c.connexion.is_none() && c.attente.is_none());
}

#[test]
fn google_ou_apple_par_la_page_du_site() {
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    let f = c.geste(None, PAGE_SITE, &json!({ "site": "wttj" }), m(9, 0));
    assert!(f.coulisse.contains(&Ordre::Montrer));
    assert_eq!(c.petite.as_ref().unwrap().raison, Raison::ConnexionSite);
    assert!(c.vue(jour())["petite"]["titre"].as_str().unwrap().starts_with("Connecte-toi à Welcome to the Jungle"));
    // La page de connexion : rien n'est tapé, Jobmind attend Jordan (relue sans recharger)
    let f = charger_et_lire(&mut c, None, &mut n, &f);
    assert!(matches!(f.coulisse[..], [Ordre::Lire { delai_ms: VERIFIER_MS, .. }]), "{f:?}");
    // Jordan se connecte lui-même : la petite fenêtre se ferme seule
    n.taper(Some(MAIL_ESSAI), Some(MDP_ESSAI));
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(None, lire_no(&p), n.lecture(), m(9, 1));
    assert!(f.coulisse.contains(&Ordre::Cacher) && f.bravo, "{f:?}");
    assert!(c.petite.is_none());
    assert_eq!(c.connecte[2], Some(true));
}

#[test]
fn captcha_puis_reprise() {
    let (b, rid) = base();
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    *n.mode.lock().unwrap() = ModeSites::Captcha;
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 0));
    let demandee = charge(&f);
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(n.adresse.contains("/checkpoint/challenge/"), "{}", n.adresse);
    assert!(f.coulisse.contains(&Ordre::Montrer), "{f:?}");
    assert!(f.journal.clone().unwrap().contains("demande une personne (captcha)"));
    let v = c.vue(jour());
    assert_eq!(v["petite"]["titre"], "LinkedIn vérifie que tu es une personne");
    assert!(v["attente"].is_null(), "rien ne s'abandonne pendant que Jordan fait la vérification");
    assert!(c.attend_personne());
    assert_eq!(c.trop_long(), Fait::default());
    // Toujours le captcha : relu plus tard, sans recharger
    let f = c.lu(Some(&b), lire_no(&f), n.lecture(), m(9, 1));
    assert!(matches!(f.coulisse[..], [Ordre::Lire { delai_ms: VERIFIER_MS, .. }]), "{f:?}");
    // Jordan a fait la vérification : le site revient à la liste ; la petite fenêtre se ferme, la liste est lue
    n.aller(&demandee);
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 2));
    assert_eq!(f.coulisse.first(), Some(&Ordre::Cacher), "{f:?}");
    assert!(c.petite.is_none());
    assert_eq!(c.liste_de(Site::LinkedIn).unwrap().resultats.len(), 6);
    // Une vérification qui ramène ailleurs : la page demandée est rechargée une seule fois (0.2.0 : la page 2 n'est
    // plus demandée par « Page suivante », qui ne charge plus rien ; Jobmind la charge toute seule, une fois les
    // offres de la page 1 lues)
    assert_eq!(c.geste(Some(&b), SUIVANTE, &Value::Null, m(9, 3)), Fait::default());
    let mut t = Instant::now();
    let f = loop {
        let (x, f) = jusqu_au_chargement(&mut c, t, 10).expect("la page 2 est chargée toute seule");
        if c.attente.as_ref().unwrap().but == But::Liste {
            break f;
        }
        charger_et_lire(&mut c, Some(&b), &mut n, &f);
        t = x + Duration::from_secs(1);
    };
    assert!(charge(&f).ends_with("&start=25"), "{f:?}");
    *n.mode.lock().unwrap() = ModeSites::Captcha;
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(c.attend_personne());
    n.aller("https://www.linkedin.com/feed/");
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 4));
    assert_eq!(f.coulisse[0], Ordre::Cacher);
    assert!(charge(&f).ends_with("&start=25"), "{f:?}");
}

#[test]
fn fermer_pendant_une_verification() {
    let (b, rid) = base();
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    *n.mode.lock().unwrap() = ModeSites::Captcha;
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 0));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    let f = c.geste(Some(&b), FERMER, &Value::Null, m(9, 1));
    assert_eq!(f.coulisse, vec![Ordre::Cacher]);
    assert!(c.petite.is_none() && c.attente.is_none());
    assert!(c.annonces[0].as_ref().unwrap().texte.contains("La vérification n'est pas finie"));
}

#[test]
fn lecture_qui_casse() {
    let (b, rid) = base();
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    *n.mode.lock().unwrap() = ModeSites::Casse;
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 0));
    let mut f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    // Relue (le site se dessine peut-être encore), sans recharger, puis cassée
    for _ in 0..RELECTURES {
        assert!(matches!(f.coulisse[..], [Ordre::Lire { delai_ms: ECART_RELECTURE_MS, .. }]), "{f:?}");
        f = c.lu(Some(&b), lire_no(&f), n.lecture(), m(9, 1));
    }
    assert!(f.coulisse.is_empty());
    let j = f.journal.unwrap();
    assert!(
        j.starts_with("Sites : LECTURE CASSÉE (linkedin, liste) adresse https://www.linkedin.com/jobs/search/"),
        "{j}"
    );
    assert!(j.contains("li.scaffold-layout__list-item=0"), "{j}");
    let v = c.vue(jour());
    assert_eq!(v["annonce"]["texte"], "Jobmind n'arrive pas à lire cette page de LinkedIn.");
    assert_eq!(v["annonce"]["bouton"], "voir");
    // « Voir la page du site » : la petite fenêtre sur la page de résultats
    let f = c.geste(Some(&b), VOIR, &Value::Null, m(9, 2));
    assert_eq!(f.coulisse.len(), 2);
    assert!(charge(&f).starts_with("https://www.linkedin.com/jobs/search/") && f.coulisse[1] == Ordre::Montrer);
    assert_eq!(c.petite.as_ref().unwrap().raison, Raison::Voir);
    assert!(!c.attend_personne());
}

#[test]
fn ajouter_depuis_la_petite_fenetre() {
    let (b, _) = base();
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    let o = c.voir("https://www.linkedin.com/jobs/view/4012345678/");
    assert_eq!(o, vec![Ordre::Charger("https://www.linkedin.com/jobs/view/4012345678/".into()), Ordre::Montrer]);
    n.aller("https://www.linkedin.com/jobs/view/4012345678/");
    // (une page chargée dans la petite fenêtre n'est pas lue d'elle-même)
    assert!(c.page_chargee(&n.adresse).coulisse.is_empty());
    let v = c.vue(jour());
    assert_eq!(v["petite"]["ajouter"], true);
    assert!(v["petite"]["texte"].as_str().unwrap().contains("« Postuler »"));
    let f = c.geste(Some(&b), AJOUTER_PAGE, &Value::Null, m(9, 0));
    assert_eq!(c.vue(jour())["petite"]["lecture"], true);
    let f = c.lu(Some(&b), lire_no(&f), n.lecture(), m(9, 1));
    assert!(f.bravo, "{f:?}");
    let a = c.petite.as_ref().unwrap().annonce.clone().unwrap();
    assert_eq!(a.texte, "« Cariste H/F » est ajoutée à tes offres. 34 candidats sur LinkedIn.");
    assert!(a.offre.is_some());
    // Une page qui n'est pas une offre : une phrase simple
    n.aller("https://www.linkedin.com/feed/");
    let f = c.geste(Some(&b), AJOUTER_PAGE, &Value::Null, m(9, 2));
    c.lu(Some(&b), lire_no(&f), n.lecture(), m(9, 3));
    assert!(c.petite.as_ref().unwrap().annonce.as_ref().unwrap().texte.starts_with("Cette page n'est pas une offre"));
    // « Fermer » : en coulisse
    assert_eq!(c.geste(Some(&b), FERMER, &Value::Null, m(9, 4)).coulisse, vec![Ordre::Cacher]);
}

#[test]
fn verification_et_ecran_inconnu_pendant_la_connexion() {
    let mut c = Coulisse::default();
    c.connecter("indeed", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let p = c.page_chargee("https://secure.indeed.com/auth/otp");
    let html = "<html><body><h1>Code envoyé</h1><input autocomplete=\"one-time-code\" type=\"text\"></body></html>";
    let u = "https://secure.indeed.com/auth/otp";
    let l = Lecture { brute: u.into(), comprise: comprendre(u, html), formulaire: None };
    let f = c.lu(None, lire_no(&p), l, m(9, 0));
    assert!(f.coulisse.contains(&Ordre::Montrer));
    assert_eq!(c.petite.as_ref().unwrap().raison, Raison::Verification);
    assert!(c.connexion.is_none(), "le mot de passe est oublié : Jordan finit lui-même");
    // Un écran inconnu dès le début : la page du site
    let mut c = Coulisse::default();
    c.connecter("wttj", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let u = "https://www.welcometothejungle.com/fr/signin";
    let p = c.page_chargee(u);
    let html = "<html><body><h1>Bienvenue</h1><p>Une page toute nouvelle.</p></body></html>";
    let l = Lecture { brute: u.into(), comprise: comprendre(u, html), formulaire: None };
    let f = c.lu(None, lire_no(&p), l, m(9, 0));
    assert!(f.coulisse.contains(&Ordre::Montrer));
    assert_eq!(c.petite.as_ref().unwrap().raison, Raison::Inconnue);
    // La saisie ratée (les cases ont disparu) : la page du site aussi
    let mut c = Coulisse::default();
    c.connecter("linkedin", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let f = c.saisie_ratee("sans_mdp");
    assert!(f.coulisse.contains(&Ordre::Montrer) && c.connexion.is_none());
}

#[test]
fn temps_depasses_et_lectures_perdues() {
    let (b, rid) = base();
    let mut c = Coulisse::default();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "indeed", "recherche": rid }), m(9, 0));
    assert!(!f.coulisse.is_empty());
    let f = c.trop_long();
    assert!(f.change && c.attente.is_none());
    assert_eq!(c.annonces[1].as_ref().unwrap().texte, "Indeed ne répond pas. Réessaie dans un moment.");
    // Une lecture perdue (la page ne répond pas) : la lecture casse, le bouton montre la page
    c.geste(Some(&b), CHERCHER, &json!({ "site": "indeed", "recherche": rid }), m(9, 1));
    let p = c.page_chargee("https://fr.indeed.com/jobs?q=cariste");
    assert_eq!(c.lecture_perdue(12345, "x"), Fait::default());
    let f = c.lecture_perdue(lire_no(&p), "la page n'a pas répondu");
    assert!(f.journal.unwrap().contains("lecture perdue"));
    // Le mot de passe d'une connexion qui n'avance pas : oublié
    c.connecter("indeed", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let f = c.oublier_connexion();
    assert!(f.journal.unwrap().contains("mot de passe oublié") && c.connexion.is_none());
    // « Annuler » une connexion
    c.connecter("indeed", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    assert!(c.geste(None, ANNULER, &Value::Null, m(9, 2)).change);
    assert!(c.connexion.is_none() && c.attente.is_none());
    // « Me déconnecter de tous les sites »
    c.connecte[0] = Some(true);
    let f = c.geste(None, DECONNECTER, &Value::Null, m(9, 3));
    assert_eq!(f.coulisse, vec![Ordre::Cacher, Ordre::Effacer]);
    assert_eq!(c.connecte, [Some(false); 6]);
}

#[test]
fn le_secret_ne_se_montre_pas_et_s_efface() {
    let s = Secret::new(MDP_ESSAI.into());
    assert_eq!(format!("{s:?}"), "Secret(caché)");
    let o = Ordre::Taper { no: 1, attendu: "x".into(), mail: Some(MAIL_ESSAI.into()), mdp: Some(s) };
    assert!(!format!("{o:?}").contains(MDP_ESSAI));
    let mut t = String::from(MDP_ESSAI);
    effacer(&mut t);
    assert!(t.is_empty());
    assert!(est_geste(CONNECTER) && est_geste(ZONE) && !est_geste("pn-ouvrir"));
    assert_eq!(
        lire_zone(&json!({ "x": 10, "y": 20, "l": 800, "h": 400 })),
        Some(Zone { x: 10.0, y: 20.0, l: 800.0, h: 400.0 })
    );
    assert_eq!(lire_zone(&json!({ "x": -1, "y": 20, "l": 800, "h": 400 })), None);
    assert_eq!(lire_zone(&json!({ "x": 1, "y": 20, "l": 8, "h": 400 })), None);
    // Les textes de la vue : simples, sans tiret long
    let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    c.petite = Some(Petite { raison: Raison::Captcha, site: Site::Indeed, offre: false, lecture: None, annonce: None });
    sans_tiret_long(&c.vue(jour()));
}

/// Étape 6 : ce que les règles lisent dans une offre d'un site, et les filtres « Aucun diplôme requis » et « Débutant
/// accepté » sur les résultats (0.1.4 : un résultat pas encore lu est caché quand un filtre est coché).
#[test]
fn lecture_et_filtres_des_resultats() {
    let sans = lecture::lire("Agent de tri", "Poste sans diplôme. Débutants acceptés.");
    let bac = lecture::lire("Comptable", "BTS exigé, 3 ans d'expérience.");
    assert_eq!(lu_court(&sans), "Aucun diplôme demandé · Débutant accepté");
    assert_eq!(lu_court(&bac), "Bac+2 · 3 ans d'expérience");
    assert!(passe(Some(&sans), (true, true)));
    assert!(!passe(Some(&bac), (true, false)));
    assert!(!passe(Some(&bac), (false, true)));
    assert!(passe(Some(&bac), (false, false)));
    // (0.1.4 : une offre pas encore lue ne passe pas un filtre coché ; sans filtre, elle passe)
    assert!(!passe(None, (true, true)));
    assert!(!passe(None, (true, false)) && !passe(None, (false, true)));
    assert!(passe(None, (false, false)));
    let v = lecture_vue(&sans);
    assert_eq!(v[0]["valeur"], "Aucun diplôme demandé");
    assert_eq!(v[0]["preuve"], "Poste sans diplôme");
    assert_eq!(v[1]["par"], "lu par Jobmind");
}

/// Vu en vrai le 19/11/2026 (France Travail) : la petite fenêtre ouverte sur un écran inconnu, Jordan se connecte ; la
/// page devient l'espace personnel (candidat.francetravail.fr/espacepersonnel/, sans repère connu). Elle est reconnue :
/// la petite fenêtre se ferme seule, « Connecté à France Travail », plus d'annonce ; et quand Jordan ferme lui-même la
/// petite fenêtre alors qu'il est déjà connecté, la page est relue : « La connexion n'est pas finie » s'en va.
#[test]
fn france_travail_espace_personnel_reconnu() {
    // 1. L'écran inconnu, puis Jordan connecté : la petite fenêtre se ferme seule
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    c.connecter("francetravail", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let f = c.saisie_ratee("sans_mdp");
    assert_eq!(c.petite.as_ref().unwrap().raison, Raison::Inconnue);
    n.aller("https://candidat.francetravail.fr/espacepersonnel/");
    assert!(n.adresse.contains("authentification-candidat"), "{}", n.adresse);
    n.taper(Some(MAIL_ESSAI), None);
    n.taper(None, Some(MDP_ESSAI));
    assert_eq!(n.adresse, "https://candidat.francetravail.fr/espacepersonnel/");
    assert_eq!(n.lecture().comprise.titre, "Accueil - Espace numérique | France Travail");
    let f = c.lu(None, lire_no(&f), n.lecture(), m(9, 1));
    assert!(f.coulisse.contains(&Ordre::Cacher) && f.bravo, "{f:?}");
    assert_eq!(f.message.as_deref(), Some("Tu es connecté à France Travail."));
    assert!(c.petite.is_none() && c.attente.is_none());
    let v = c.vue(jour());
    assert_eq!(v["sites"][3]["etat"], "Connecté à France Travail");
    // 2. Jordan ferme lui-même la petite fenêtre, déjà connecté : relue, l'annonce s'en va
    let mut c = Coulisse::default();
    c.geste(None, PAGE_SITE, &json!({ "site": "francetravail" }), m(9, 2));
    let f = c.geste(None, FERMER, &Value::Null, m(9, 3));
    assert_eq!(f.coulisse[0], Ordre::Cacher);
    assert!(c.annonces[3].as_ref().unwrap().texte.contains("La connexion à France Travail n'est pas finie"));
    c.source = Some(Site::FranceTravail);
    assert_eq!(c.vue(jour())["annonce"]["bouton"], "connexion");
    let f = c.lu(None, lire_no(&f), n.lecture(), m(9, 4));
    assert!(f.journal.unwrap().contains("annonce de connexion à francetravail retirée"));
    assert!(c.annonces[3].is_none());
    let v = c.vue(jour());
    assert!(v["annonce"].is_null());
    assert_eq!(v["sites"][3]["etat"], "Connecté à France Travail");
    // 3. Fermée sans être connecté : l'annonce reste
    let mut c = Coulisse::default();
    let mut dehors = Navigateur::nouveau();
    dehors.aller("https://candidat.francetravail.fr/espacepersonnel/");
    c.geste(None, PAGE_SITE, &json!({ "site": "francetravail" }), m(9, 5));
    let f = c.geste(None, FERMER, &Value::Null, m(9, 6));
    c.lu(None, lire_no(&f), dehors.lecture(), m(9, 7));
    assert!(c.annonces[3].is_some());
    assert_ne!(c.connecte[3], Some(true));
}

/// « Fermer » (ou Échap, qui fait le même geste) : la petite fenêtre se referme, et la prochaine sera annoncée.
fn fermer_et_oublier(c: &mut Coulisse, b: Option<&Base>) {
    let f = c.geste(b, FERMER, &Value::Null, m(9, 30));
    assert!(f.coulisse.contains(&Ordre::Cacher) && f.change, "{f:?}");
    assert!(c.petite.is_none() && c.vue(jour())["petite"].is_null());
    assert!(!c.petite_a_annoncer());
}

#[test]
fn chaque_chemin_envoie_la_petite_fenetre_a_la_page() {
    // 1. « Me connecter », puis « Me connecter sur la page du site » (France Travail, puis les autres sites)
    let mut c = Coulisse { source: Some(Site::FranceTravail), ..Coulisse::default() };
    let f = c.geste(None, PAGE_SITE, &json!({ "site": "francetravail" }), m(9, 0));
    crate::coulisse::bandeau_envoye(&mut c, &f, "Connecte-toi à France Travail");
    fermer_et_oublier(&mut c, None);
    // (rouverte : annoncée de nouveau, son bandeau noté de nouveau)
    let f = c.geste(None, PAGE_SITE, &json!({ "site": "francetravail" }), m(9, 1));
    crate::coulisse::bandeau_envoye(&mut c, &f, "Connecte-toi à France Travail");
    fermer_et_oublier(&mut c, None);
    for (s, nom) in [("linkedin", "LinkedIn"), ("indeed", "Indeed"), ("wttj", "Welcome to the Jungle")] {
        let f = c.geste(None, PAGE_SITE, &json!({ "site": s }), m(9, 2));
        crate::coulisse::bandeau_envoye(&mut c, &f, &format!("Connecte-toi à {nom}"));
        fermer_et_oublier(&mut c, None);
    }
    // 2. Un captcha pendant une recherche
    let (b, rid) = base();
    let mut n = Navigateur::nouveau();
    *n.mode.lock().unwrap() = ModeSites::Captcha;
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 3));
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    crate::coulisse::bandeau_envoye(&mut c, &f, "LinkedIn vérifie que tu es une personne");
    fermer_et_oublier(&mut c, Some(&b));
    // 3. Une vérification (un code) pendant la connexion
    let mut c = Coulisse::default();
    c.connecter("indeed", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let u = "https://secure.indeed.com/auth/otp";
    let p = c.page_chargee(u);
    let html = "<html><body><h1>Code envoyé</h1><input autocomplete=\"one-time-code\" type=\"text\"></body></html>";
    let f =
        c.lu(None, lire_no(&p), Lecture { brute: u.into(), comprise: comprendre(u, html), formulaire: None }, m(9, 4));
    crate::coulisse::bandeau_envoye(&mut c, &f, "Indeed demande une vérification");
    fermer_et_oublier(&mut c, None);
    // 4. Un écran inconnu pendant la connexion (lu, puis la saisie ratée)
    c.connecter("wttj", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let u = "https://www.welcometothejungle.com/fr/signin";
    let p = c.page_chargee(u);
    let html = "<html><body><h1>Bienvenue</h1><p>Une page toute nouvelle.</p></body></html>";
    let f =
        c.lu(None, lire_no(&p), Lecture { brute: u.into(), comprise: comprendre(u, html), formulaire: None }, m(9, 5));
    crate::coulisse::bandeau_envoye(&mut c, &f, "Termine la connexion sur Welcome to the Jungle");
    fermer_et_oublier(&mut c, None);
    c.connecter("francetravail", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let f = c.saisie_ratee("sans_mdp");
    crate::coulisse::bandeau_envoye(&mut c, &f, "Termine la connexion sur France Travail");
    fermer_et_oublier(&mut c, None);
    // 5. « Voir la page du site » (une lecture qui casse)
    let mut c = Coulisse::default();
    let mut n = Navigateur::nouveau();
    *n.mode.lock().unwrap() = ModeSites::Casse;
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 6));
    let mut f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    for _ in 0..RELECTURES {
        f = c.lu(Some(&b), lire_no(&f), n.lecture(), m(9, 6));
    }
    c.source = Some(Site::LinkedIn);
    let f = c.geste(Some(&b), VOIR, &Value::Null, m(9, 7));
    crate::coulisse::bandeau_envoye(&mut c, &f, "La page du site : LinkedIn");
    fermer_et_oublier(&mut c, Some(&b));
}

mod analyses;
mod garde;
mod lecture_auto;
