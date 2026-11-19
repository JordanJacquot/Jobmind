//! « Garder le mot de passe » (coulisse\garde.rs) : chiffré avec les clés, la reconnexion automatique une seule fois.
//! Les clés dans un dossier d'essai, avec un faux coffre ; les sites imités, sans réseau.

use jobmind_core::cle::Cle;
use jobmind_storage::cles::{Cles, Coffre, EtatCle};

use super::*;

/// Un faux coffre (le vrai, DPAPI, est essayé dans windows\dpapi.rs) : chaque octet mélangé, une marque devant.
struct FauxCoffre;
impl Coffre for FauxCoffre {
    fn fermer(&self, clair: &[u8]) -> Result<Vec<u8>, String> {
        Ok([b"FC".as_slice(), &clair.iter().map(|b| b ^ 0x5A).collect::<Vec<_>>()].concat())
    }
    fn ouvrir(&self, chiffre: &[u8]) -> Result<Vec<u8>, String> {
        Ok(chiffre.strip_prefix(b"FC").ok_or("pas à nous")?.iter().map(|b| b ^ 0x5A).collect())
    }
}

/// Un dossier de données d'essai (jamais celui de Jordan).
fn dossier_essai(nom: &str) -> std::path::PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("JobmindEssai-{}-garde-{nom}-{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Jobmind (re)lancée : les clés relues, la vue des sites sans cookies de session (le site a oublié Jordan).
fn lancer(d: &std::path::Path) -> (Cles, Coulisse, Navigateur) {
    let (cles, _) = Cles::ouvrir(d, Box::new(FauxCoffre));
    let c = Coulisse { gardes: crate::coulisse::garde::gardes(&cles), ..Coulisse::default() };
    (cles, c, Navigateur::nouveau())
}

/// Ce que le programme fait après chaque geste : garde::suite, sa ligne du journal notée.
fn suite(c: &mut Coulisse, cles: &mut Cles, journal: &mut Vec<String>) -> Fait {
    let f = crate::coulisse::garde::suite(c, Some(cles));
    journal.extend(f.journal.clone());
    f
}

/// Le journal, la page et le fichier des clés ne contiennent jamais le mot de passe ni l'identifiant en clair.
fn rien_en_clair(journal: &[String], c: &Coulisse, d: &std::path::Path, mdp: &str) {
    for l in journal {
        assert!(!l.contains(mdp) && !l.contains(MAIL_ESSAI), "au journal : {l}");
    }
    let v = c.vue(jour()).to_string();
    assert!(!v.contains(mdp) && !v.contains(MAIL_ESSAI), "dans la page : {v}");
    if let Ok(f) = std::fs::read_to_string(Cles::chemin_dans(d)) {
        assert!(!f.contains(mdp) && !f.contains(MAIL_ESSAI), "{f}");
    }
}

/// La saisie demandée par un fait (le mail et le mot de passe), faite dans le navigateur d'essai.
fn taper(n: &mut Navigateur, f: &Fait) {
    let Ordre::Taper { mail, mdp, .. } = &f.coulisse[0] else { panic!("pas de saisie : {f:?}") };
    n.taper(mail.as_deref(), mdp.as_ref().map(Secret::texte));
}

/// Une connexion réussie est gardée (chiffrée) ; Jobmind relancée, le site dit « pas connecté » : Jobmind retape les
/// identifiants gardés toute seule, sans bulle, une seule fois depuis le lancement.
#[test]
fn garder_apres_succes_puis_reconnexion_automatique_une_fois() {
    let d = dossier_essai("succes");
    let (b, rid) = base();
    let mut journal = Vec::new();
    let (mut cles, mut c, mut n) = lancer(&d);
    assert_eq!(c.gardes, [false; 4]);
    assert_eq!(c.vue(jour())["sites"][0]["garde"], false);
    // « Me connecter » par la carte de Jobmind
    let f = c.connecter("linkedin", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    assert!(!format!("{c:?}").contains(MDP_ESSAI), "la copie gardée ne s'affiche pas");
    journal.extend(f.journal.clone());
    let f = charger_et_lire(&mut c, None, &mut n, &f);
    // (rien n'est gardé avant que le site accepte)
    assert_eq!(suite(&mut c, &mut cles, &mut journal), Fait::default());
    assert!(!Cles::chemin_dans(&d).exists());
    taper(&mut n, &f);
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(None, lire_no(&p), n.lecture(), m(9, 1));
    journal.extend(f.journal.clone());
    assert_eq!(f.message.as_deref(), Some("Tu es connecté à LinkedIn."));
    // Gardés, chiffrés
    let f = suite(&mut c, &mut cles, &mut journal);
    assert!(f.change && f.journal.as_deref().unwrap().contains("connexion à linkedin gardée sur ce PC (chiffrée)"));
    assert_eq!(c.gardes, [true, false, false, false]);
    assert_eq!(cles.etat(Cle::LinkedinMdp), Some(&EtatCle::Gardee(String::new())), "aucun aperçu du mot de passe");
    assert_eq!(cles.lire(Cle::LinkedinMdp).as_deref(), Some(MDP_ESSAI));
    let v = c.vue(jour());
    assert_eq!(v["sites"][0]["garde"], true);
    assert_eq!(v["textes"]["oublier"], "Oublier ma connexion");
    sans_tiret_long(&v);
    rien_en_clair(&journal, &c, &d, MDP_ESSAI);

    // Jobmind relancée : le site a oublié la session ; la liste montre « pas connecté »
    let (mut cles, mut c, mut n) = lancer(&d);
    assert_eq!(c.gardes, [true, false, false, false]);
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(10, 0));
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(f.journal.unwrap().contains("liste lue de linkedin"));
    assert_eq!(c.connecte[0], Some(false));
    assert_eq!(c.reconnecter, Some(Site::LinkedIn));
    // Le programme lit les identifiants gardés : la page de connexion chargée en coulisse
    let f = suite(&mut c, &mut cles, &mut journal);
    assert_eq!(charge(&f), "https://www.linkedin.com/login/fr");
    assert!(f.journal.as_deref().unwrap().contains("reconnexion automatique à linkedin"), "{f:?}");
    assert!(f.message.is_none() && c.petite.is_none(), "rien n'est demandé à Jordan");
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    journal.extend(f.journal.clone());
    let Ordre::Taper { mail, mdp, .. } = &f.coulisse[0] else { panic!("{f:?}") };
    assert_eq!((mail.as_deref(), mdp.as_ref().map(Secret::texte)), (Some(MAIL_ESSAI), Some(MDP_ESSAI)));
    taper(&mut n, &f);
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(10, 1));
    journal.extend(f.journal.clone());
    assert_eq!(f.message, None, "reconnecté tout seul : pas de bulle");
    assert!(f.journal.as_deref().unwrap().contains("reconnecté tout seul à linkedin"), "{f:?}");
    assert_eq!(c.connecte[0], Some(true));
    assert!(c.a_garder.is_none(), "rien de réécrit");
    // La session perdue de nouveau, depuis le même lancement : pas de seconde tentative
    n.cookies.clear();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(10, 5));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert_eq!(c.connecte[0], Some(false));
    assert_eq!(c.reconnecter, None);
    assert_eq!(suite(&mut c, &mut cles, &mut journal), Fait::default());
    rien_en_clair(&journal, &c, &d, MDP_ESSAI);
    let _ = std::fs::remove_dir_all(&d);
}

/// La page de connexion à la place de la liste demandée (France Travail, en deux écrans) : Jobmind se reconnecte
/// toute seule, puis recharge la liste interrompue.
#[test]
fn page_de_connexion_reconnexion_puis_page_reprise() {
    let d = dossier_essai("reprise");
    let (b, rid) = base();
    let (mut cles, _, _) = lancer(&d);
    cles.garder_plusieurs(&[(Cle::FranceTravailMail, MAIL_ESSAI), (Cle::FranceTravailMdp, MDP_ESSAI)]).unwrap();
    let (mut cles, mut c, mut n) = lancer(&d);
    assert_eq!(c.gardes, [false, false, false, true]);
    let mut journal = Vec::new();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "francetravail", "recherche": rid }), m(9, 0));
    let liste = charge(&f);
    // (le site montre sa page de connexion au lieu de la liste)
    n.aller(sites::adresse_connexion(Site::FranceTravail).unwrap());
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    assert!(f.journal.unwrap().contains("demande une connexion"));
    assert_eq!(c.reconnecter, Some(Site::FranceTravail));
    let f = suite(&mut c, &mut cles, &mut journal);
    assert!(c.annonces[3].is_none(), "l'annonce « demande de te connecter » s'efface pendant la reconnexion");
    // 1. le mail, 2. le mot de passe
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    taper(&mut n, &f);
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 2));
    taper(&mut n, &f);
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 3));
    journal.extend(f.journal.clone());
    assert_eq!(c.connecte[3], Some(true));
    // La liste interrompue, rechargée une fois connecté : connecté sur la liste, aucune autre tentative
    assert_eq!(charge(&f), liste);
    assert!(f.journal.as_deref().unwrap().contains("la page interrompue est rechargée"));
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert!(f.journal.unwrap().contains("liste lue de francetravail"));
    assert_eq!(c.connecte[3], Some(true));
    assert!(c.reconnecter.is_none() && c.attente.is_none());
    rien_en_clair(&journal, &c, &d, MDP_ESSAI);
    let _ = std::fs::remove_dir_all(&d);
}

/// Le mot de passe gardé refusé par le site : gardé quand même, aucune nouvelle tentative, la carte de connexion
/// s'ouvre comme avant. Puis Jordan tape le bon : il remplace l'ancien.
#[test]
fn refuse_par_le_site_garde_sans_insister() {
    const ANCIEN: &str = "Ancien-MotDePasse-0000";
    let d = dossier_essai("refus");
    let (b, rid) = base();
    let (mut cles, _, _) = lancer(&d);
    cles.garder_plusieurs(&[(Cle::LinkedinMail, MAIL_ESSAI), (Cle::LinkedinMdp, ANCIEN)]).unwrap();
    let (mut cles, mut c, mut n) = lancer(&d);
    let mut journal = Vec::new();
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 0));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    let f = suite(&mut c, &mut cles, &mut journal);
    let f = charger_et_lire(&mut c, Some(&b), &mut n, &f);
    taper(&mut n, &f);
    let p = c.page_chargee(&n.adresse);
    let f = c.lu(Some(&b), lire_no(&p), n.lecture(), m(9, 1));
    journal.extend(f.journal.clone());
    assert!(f.coulisse.is_empty(), "aucun second essai : {f:?}");
    assert!(f.journal.as_deref().unwrap().contains("reconnexion automatique à linkedin refusée par le site"));
    assert_eq!(f.ordre, Some(json!({ "kit": "accueil", "connexion": { "site": "linkedin" } })), "la carte s'ouvre");
    let a = c.annonces[0].as_ref().unwrap();
    assert!(a.texte.contains("n'a pas accepté le mot de passe gardé"));
    assert_eq!(a.bouton, Some("connexion"));
    assert_eq!(suite(&mut c, &mut cles, &mut journal), Fait::default());
    assert_eq!(cles.lire(Cle::LinkedinMdp).as_deref(), Some(ANCIEN), "gardés quand même");
    assert!(c.gardes[0]);
    // Une autre page « pas connecté » : rien de plus
    let f = c.geste(Some(&b), CHERCHER, &json!({ "site": "linkedin", "recherche": rid }), m(9, 2));
    charger_et_lire(&mut c, Some(&b), &mut n, &f);
    assert_eq!(c.reconnecter, None);
    // Jordan tape le bon mot de passe dans la carte : il remplace l'ancien
    let f = c.connecter("linkedin", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
    let f = charger_et_lire(&mut c, None, &mut n, &f);
    taper(&mut n, &f);
    let p = c.page_chargee(&n.adresse);
    assert!(c.lu(None, lire_no(&p), n.lecture(), m(9, 3)).bravo);
    suite(&mut c, &mut cles, &mut journal);
    assert_eq!(cles.lire(Cle::LinkedinMdp).as_deref(), Some(MDP_ESSAI));
    rien_en_clair(&journal, &c, &d, MDP_ESSAI);
    rien_en_clair(&journal, &c, &d, ANCIEN);
    let _ = std::fs::remove_dir_all(&d);
}

/// Un mot de passe refusé à la connexion faite par la carte n'est pas gardé ; une vérification (captcha) après la
/// saisie : gardés seulement quand elle réussit, oubliés si la petite fenêtre est fermée.
#[test]
fn rien_de_garde_sans_succes_et_apres_une_verification() {
    let d = dossier_essai("verif");
    let (mut cles, mut c, mut n) = lancer(&d);
    let mut journal = Vec::new();
    // Refusé : rien de gardé
    let f = c.connecter("linkedin", MAIL_ESSAI, Secret::new("pas-le-bon".into()));
    let f = charger_et_lire(&mut c, None, &mut n, &f);
    taper(&mut n, &f);
    let p = c.page_chargee(&n.adresse);
    assert!(c.lu(None, lire_no(&p), n.lecture(), m(9, 1)).journal.unwrap().contains("refusée par le site"));
    assert_eq!(suite(&mut c, &mut cles, &mut journal), Fait::default());
    assert!(!Cles::chemin_dans(&d).exists() && c.gardes == [false; 4]);
    // Tapés, puis un captcha : la petite fenêtre ; fermée : oubliés ; faite et connecté : gardés
    for fermer in [true, false] {
        let f = c.connecter("linkedin", MAIL_ESSAI, Secret::new(MDP_ESSAI.into()));
        let f = charger_et_lire(&mut c, None, &mut n, &f);
        taper(&mut n, &f);
        *n.mode.lock().unwrap() = ModeSites::Captcha;
        let metz = LieuSite::Ville { nom: "Metz".into(), rayon_km: 30 };
        n.aller(&adresse_recherche(Site::LinkedIn, "cariste", &metz, 1).unwrap());
        assert!(n.adresse.contains("/checkpoint/challenge/"), "{}", n.adresse);
        let p = c.page_chargee(&n.adresse);
        let f = c.lu(None, lire_no(&p), n.lecture(), m(9, 2));
        assert!(f.coulisse.contains(&Ordre::Montrer), "{f:?}");
        assert!(c.verifies.is_some() && c.a_garder.is_none());
        assert_eq!(suite(&mut c, &mut cles, &mut journal), Fait::default());
        if fermer {
            c.geste(None, FERMER, &Value::Null, m(9, 3));
            assert!(c.verifies.is_none(), "fermée : oubliés");
            assert_eq!(suite(&mut c, &mut cles, &mut journal), Fait::default());
            continue;
        }
        n.aller("https://www.linkedin.com/feed/");
        let f = c.lu(None, lire_no(&f), n.lecture(), m(9, 4));
        assert!(f.bravo, "{f:?}");
        let f = suite(&mut c, &mut cles, &mut journal);
        assert!(f.journal.unwrap().contains("gardée sur ce PC"));
    }
    assert!(c.gardes[0]);
    rien_en_clair(&journal, &c, &d, MDP_ESSAI);
    let _ = std::fs::remove_dir_all(&d);
}

/// « Oublier ma connexion » efface les identifiants de ce site seulement ; « Me déconnecter de tous les sites » les
/// efface tous (la clé Claude reste).
#[test]
fn oublier_ma_connexion_et_tout_deconnecter() {
    let d = dossier_essai("oublier");
    let (mut cles, _, _) = lancer(&d);
    cles.garder_plusieurs(&[(Cle::IndeedMail, MAIL_ESSAI), (Cle::IndeedMdp, MDP_ESSAI)]).unwrap();
    cles.garder_plusieurs(&[(Cle::WttjMail, MAIL_ESSAI), (Cle::WttjMdp, MDP_ESSAI)]).unwrap();
    cles.garder(Cle::Claude, "sk-ant-api03-Inventee-1111").unwrap();
    let (mut cles, mut c, _) = lancer(&d);
    let mut journal = Vec::new();
    assert_eq!(c.gardes, [false, true, true, false]);
    // (un site sans identifiants gardés, un code inconnu : rien)
    assert_eq!(c.geste(None, OUBLIER, &json!({ "site": "linkedin" }), m(9, 0)), Fait::default());
    assert_eq!(c.geste(None, OUBLIER, &json!({ "site": "myspace" }), m(9, 0)), Fait::default());
    let f = c.geste(None, OUBLIER, &json!({ "site": "indeed" }), m(9, 1));
    assert!(f.change && f.message.unwrap().contains("Jobmind a oublié ta connexion à Indeed"));
    let f = suite(&mut c, &mut cles, &mut journal);
    assert!(f.journal.unwrap().contains("connexion gardée de indeed oubliée"));
    assert_eq!(c.gardes, [false, false, true, false]);
    assert_eq!(cles.etat(Cle::IndeedMdp), None);
    assert!(cles.lire(Cle::WttjMdp).is_some() && cles.lire(Cle::Claude).is_some());
    // Rouverte : toujours oubliés
    let (mut cles, mut c, _) = lancer(&d);
    assert_eq!(c.gardes, [false, false, true, false]);
    // « Me déconnecter de tous les sites » : tous oubliés
    let f = c.geste(None, DECONNECTER, &Value::Null, m(9, 2));
    assert!(f.coulisse.contains(&Ordre::Effacer));
    let f = suite(&mut c, &mut cles, &mut journal);
    assert!(f.journal.unwrap().contains("connexion gardée de wttj oubliée"));
    assert_eq!(c.gardes, [false; 4]);
    assert_eq!(cles.etat(Cle::WttjMail), None);
    assert!(cles.lire(Cle::Claude).is_some());
    rien_en_clair(&journal, &c, &d, MDP_ESSAI);
    let _ = std::fs::remove_dir_all(&d);
}
