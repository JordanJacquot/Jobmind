//! Le scénario de l'écran contre le faux Claude local (127.0.0.1, jobmind-llm, option « faux ») : jamais la vraie API,
//! jamais une vraie clé.

use std::time::Duration;

use jobmind_core::cle::Cle;
use jobmind_llm::claude::Claude;
use jobmind_llm::faux::{ModeFaux, PHRASE_INVENTEE, ServeurClaude};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::cles::Coffre;
use jobmind_storage::profil::{GenreCv, NouveauCv};

use super::*;

const CLE: &str = "sk-ant-api03-InventeeInventeeInventeeInventeeInventee_0000-AAAA";
const CV: &str = "Jordan Exemple\nMetz (57), permis B\n\nExpérience\n2023 à 2025 : préparateur de commandes et cariste en \
                  intérim, entrepôt de Woippy\n2021 à 2022 : vendeur dans un magasin de sport, Metz\n\nFormation\nCAP \
                  Opérateur logistique, 2018\nCACES 1, 3 et 5";

fn m(h: u32, mi: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, mi, 0).unwrap()
}

struct Faux;
impl Coffre for Faux {
    fn fermer(&self, c: &[u8]) -> Result<Vec<u8>, String> {
        Ok(c.iter().rev().map(|b| b ^ 0x5a).collect())
    }
    fn ouvrir(&self, c: &[u8]) -> Result<Vec<u8>, String> {
        Ok(c.iter().rev().map(|b| b ^ 0x5a).collect())
    }
}

fn dossier(nom: &str) -> std::path::PathBuf {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let d = std::env::temp_dir().join(format!("JobmindEssai-{}-{nom}-{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Une base avec une offre (et sa question) ; le CV seulement si demandé.
fn base(avec_cv: bool) -> (Base, i64) {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    let c = b.connexion();
    let o = OffreTrouvee {
        id_source: "1".into(),
        titre: "Agent de quai H/F".into(),
        entreprise: "Transports Moselle Fret".into(),
        lieu: "Woippy (57)".into(),
        zones: Vec::new(),
        url: String::new(),
        description: "Chargement et déchargement des camions. CACES 3 exigé. Poste en journée.\nPourquoi \
                      souhaitez-vous rejoindre notre équipe ?"
            .into(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    };
    base_offres::ajouter_trouvees(c, "adzuna", &[o], None, m(9, 0)).unwrap();
    if avec_cv {
        let n = NouveauCv { nom: "CV 2026", fichier: None, genre: GenreCv::Texte, texte: CV, taille: None };
        profil::ajouter_cv(c, &n, m(9, 0)).unwrap();
    }
    let id = base_offres::lister(c).unwrap()[0].id;
    (b, id)
}

/// Fait tourner la tâche contre le faux Claude, comme le fil du programme.
fn tourner(s: &ServeurClaude, t: &Tache) -> Retour {
    travailler(&Claude::nouveau(&s.adresse(), CLE, false, Duration::from_secs(10)), t)
}

#[test]
fn sans_cle_puis_sans_cv_rien_ne_casse() {
    let d = dossier("prep-sans");
    let (b, id) = base(false);
    let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
    let mut p = Preparation::default();
    // Sans clé : la phrase simple qui renvoie à Mes clés, rien ne part
    let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
    let f = p.geste(&ctx, PREPARER, &json!({ "id": id }), m(10, 0));
    assert!(f.preparation.is_none());
    assert!(f.message.unwrap().contains("Mes clés"));
    let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(10, 0).jour());
    assert_eq!(v["claude"], false);
    assert_eq!(v["annonce"]["cles"], true);
    assert!(v["version"].is_null() && v["versions"].as_array().unwrap().is_empty());
    // Avec la clé mais sans CV : « Ajoute d'abord ton CV »
    cles.garder(Cle::Claude, CLE).unwrap();
    let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
    let f = p.geste(&ctx, PREPARER, &json!({ "id": id }), m(10, 1));
    assert!(f.preparation.is_none());
    assert!(f.message.unwrap().contains("Mon CV"));
    let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(10, 0).jour());
    assert_eq!(v["annonce"]["cv"], true);
    // (rien de compté : aucune demande n'est partie)
    assert_eq!(base_prep::appels_du_jour(b.connexion(), m(10, 0).jour()).unwrap().0, 0);
}

#[test]
fn scenario_preparer_verifier_modifier_refaire() {
    let d = dossier("prep");
    let (b, id) = base(true);
    let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
    cles.garder(Cle::Claude, CLE).unwrap();
    let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let mut p = Preparation::default();
    // « Préparer ma candidature » : la tâche part, sans clé dedans
    let f = p.geste(&ctx, PREPARER, &json!({ "id": id }), m(10, 0));
    let t = f.preparation.unwrap();
    assert!(!format!("{t:?}").contains("sk-ant"));
    assert!(t.entree.cv.contains("cariste") && t.entree.precedente.is_none());
    // Une seule demande à la fois
    assert!(p.geste(&ctx, PREPARER, &json!({ "id": id }), m(10, 0)).message.unwrap().contains("déjà"));
    let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(10, 0).jour());
    assert!(v["en_cours"].as_str().unwrap().starts_with("Claude prépare"));
    let f = p.fini(Some(&b), tourner(&s, &t), m(10, 1));
    assert!(
        f.message.as_deref().unwrap().contains("1 phrase") || f.message.as_deref().unwrap().contains("Une phrase"),
        "{f:?}"
    );
    assert!(f.journal.unwrap().contains("version 1 prête"));
    // L'écran : la lettre, la phrase inventée surlignée, la question, les conseils, la version 1
    let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(10, 1).jour());
    let x = &v["version"];
    assert_eq!(x["numero"], 1);
    assert!(x["lettre"].as_str().unwrap().contains(PHRASE_INVENTEE));
    let surlignes: Vec<&Value> = x["morceaux"].as_array().unwrap().iter().filter(|m| !m["n"].is_null()).collect();
    assert_eq!(surlignes.len(), 1);
    assert_eq!(surlignes[0]["t"], PHRASE_INVENTEE);
    assert_eq!(x["a_verifier"].as_array().unwrap().len(), 1);
    assert_eq!(x["questions"].as_array().unwrap().len(), 1);
    assert!(!x["conseils"].as_array().unwrap().is_empty());
    assert_eq!(x["lettre"], x["brute"]);
    assert_eq!(v["reste"], PLAFOND - 1);
    let pid = x["id"].as_i64().unwrap();
    // Retirer la phrase inventée : elle part de la lettre de Jordan, la lettre brute ne change pas
    let f = p.geste(&ctx, RETIRER, &json!({ "id": pid, "n": 0 }), m(10, 2));
    assert!(f.change);
    let g = base_prep::lire(b.connexion(), pid).unwrap().unwrap();
    assert!(!g.lettre.contains(PHRASE_INVENTEE) && g.lettre_brute.contains(PHRASE_INVENTEE));
    assert!(g.reponse_brute.contains("encadré une équipe"));
    // Modifier à la main ; une lettre vide est refusée
    assert!(p.geste(&ctx, LETTRE, &json!({ "id": pid, "lettre": "  " }), m(10, 3)).message.unwrap().contains("vide"));
    let f = p.geste(&ctx, LETTRE, &json!({ "id": pid, "lettre": "Madame, Monsieur,\n\nMa lettre." }), m(10, 3));
    assert!(f.bravo);
    let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(10, 3).jour());
    assert_eq!(v["version"]["modifiee"], true);
    // Refaire avec une consigne : elle part de la lettre de Jordan ; une consigne trop longue est refusée
    let long = "x".repeat(200);
    assert!(p.geste(&ctx, PREPARER, &json!({ "id": id, "consigne": long }), m(10, 4)).preparation.is_none());
    let t = p.geste(&ctx, PREPARER, &json!({ "id": id, "consigne": "plus court" }), m(10, 4)).preparation.unwrap();
    assert_eq!(t.entree.consigne.as_deref(), Some("plus court"));
    assert_eq!(t.entree.precedente.as_deref(), Some("Madame, Monsieur,\n\nMa lettre."));
    p.fini(Some(&b), tourner(&s, &t), m(10, 5));
    let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(10, 5).jour());
    assert_eq!(v["version"]["numero"], 2);
    assert_eq!(v["versions"].as_array().unwrap().len(), 2);
    assert!(v["version"]["meta"].as_str().unwrap().contains("consigne « plus court »"));
    // L'historique : la version 1 se montre de nouveau, avec la lettre de Jordan
    p.geste(&ctx, VERSION, &json!({ "id": pid }), m(10, 6));
    let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(10, 6).jour());
    assert_eq!(v["version"]["lettre"], "Madame, Monsieur,\n\nMa lettre.");
    // « C'est juste » sur la version 2
    let v2 = base_prep::lister(b.connexion(), id).unwrap()[0].id;
    p.geste(&ctx, JUSTE, &json!({ "id": v2, "n": 0 }), m(10, 7));
    assert!(base_prep::lire(b.connexion(), v2).unwrap().unwrap().a_verifier[0].vue);
    // Le dossier d'une candidature liée à l'offre
    assert!(pour_dossier(Some(&b), 1).is_null());
}

#[test]
fn les_erreurs_ne_perdent_rien() {
    let d = dossier("prep-erreurs");
    let (b, id) = base(true);
    let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
    cles.garder(Cle::Claude, CLE).unwrap();
    let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let mut p = Preparation::default();
    let t = p.geste(&ctx, PREPARER, &json!({ "id": id }), m(10, 0)).preparation.unwrap();
    p.fini(Some(&b), tourner(&s, &t), m(10, 1));
    for (mode, mot) in [
        (ModeFaux::Refus, "pas voulu"),
        (ModeFaux::Quota, "trop de demandes"),
        (ModeFaux::CleRefusee, "refuse ta clé"),
        (ModeFaux::Surcharge, "très demandé"),
        (ModeFaux::Abime, "abîmée"),
    ] {
        *s.mode.lock().unwrap() = mode;
        let t = p.geste(&ctx, PREPARER, &json!({ "id": id }), m(11, 0)).preparation.unwrap();
        let f = p.fini(Some(&b), tourner(&s, &t), m(11, 1));
        assert!(f.message.as_deref().unwrap().contains(mot), "{mode:?} : {f:?}");
        assert!(!format!("{f:?}").contains("sk-ant"));
        // (la version d'avant est toujours là, montrée)
        let (v, _) = p.vue(Some(&b), Some(&cles), Some(id), m(11, 1).jour());
        assert_eq!(v["version"]["numero"], 1, "{mode:?}");
        assert_eq!(v["annonce"]["genre"], "erreur");
    }
    // Chaque demande est comptée, réussie ou non ; la réponse abîmée est gardée telle quelle
    assert_eq!(base_prep::appels_du_jour(b.connexion(), m(11, 0).jour()).unwrap().0, 6);
    let r: Option<String> = b
        .connexion()
        .query_row("SELECT reponse FROM appels_preparation WHERE etat = 'illisible'", [], |r| r.get(0))
        .unwrap();
    assert!(r.unwrap().starts_with("{\"objet\""));
    // Une demande trop longue : abandonnée, un résultat arrivé ensuite est laissé
    *s.mode.lock().unwrap() = ModeFaux::Normal;
    let t = p.geste(&ctx, PREPARER, &json!({ "id": id }), m(12, 0)).preparation.unwrap();
    assert!(p.trop_longue().message.unwrap().contains("trop de temps"));
    let f = p.fini(Some(&b), tourner(&s, &t), m(12, 1));
    assert!(f.journal.unwrap().contains("abandonnée"));
    assert_eq!(base_prep::nombre(b.connexion(), id).unwrap(), 1);
}

#[test]
fn le_plafond_du_jour() {
    let d = dossier("prep-plafond");
    let (b, id) = base(true);
    let (mut cles, _) = Cles::ouvrir(&d, Box::new(Faux));
    cles.garder(Cle::Claude, CLE).unwrap();
    let ctx = Contexte { base: Some(&b), cles: Some(&cles) };
    for _ in 0..PLAFOND {
        base_prep::noter_appel(b.connexion(), id, m(8, 0)).unwrap();
    }
    let mut p = Preparation::default();
    let f = p.geste(&ctx, PREPARER, &json!({ "id": id }), m(10, 0));
    assert!(f.preparation.is_none());
    assert!(f.message.unwrap().contains("La suite demain"));
    // Le lendemain, c'est reparti
    let demain = Moment::nouveau(2026, 11, 20, 9, 0, 0).unwrap();
    assert!(p.geste(&ctx, PREPARER, &json!({ "id": id }), demain).preparation.is_some());
}

#[test]
fn les_textes_sont_simples() {
    fn tous(v: &Value, l: &mut Vec<String>) {
        match v {
            Value::String(s) => l.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| tous(x, l)),
            Value::Object(o) => o.values().for_each(|x| tous(x, l)),
            _ => {}
        }
    }
    let mut l = Vec::new();
    tous(&textes(), &mut l);
    for e in
        [ErreurLlm::CleManquante, ErreurLlm::Refus("x".into()), ErreurLlm::Illisible("x".into()), ErreurLlm::Limite]
    {
        l.push(message_erreur(&e));
    }
    for t in l {
        assert!(!t.contains(['\u{2013}', '\u{2014}']), "{t}");
    }
}
