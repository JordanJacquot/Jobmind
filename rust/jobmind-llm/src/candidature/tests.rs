//! La candidature contre le faux Claude local (127.0.0.1, crate::faux) : jamais la vraie API, jamais une vraie clé
//! (« sk-ant-essai... » est inventée).

use std::time::Duration;

use super::*;
use crate::LlmProvider;
use crate::claude::Claude;
use crate::faux::{ModeFaux, PHRASE_INVENTEE, ServeurClaude};
use jobmind_core::preparation::Raison;

const CLE: &str = "sk-ant-essai-0000000000000000000000000000";

const CV: &str = "Jordan Exemple\nMetz (57), permis B\n\nExpérience\n2023 à 2025 : préparateur de commandes et cariste en \
                  intérim, entrepôt de Woippy\n2021 à 2022 : vendeur dans un magasin de sport, Metz\n\nFormation\nCAP \
                  Opérateur logistique, 2018\nCACES 1, 3 et 5";

fn entree(texte_offre: &str, cv: &str) -> Entree {
    Entree {
        titre: "Agent de quai".into(),
        entreprise: "Transports Moselle Fret".into(),
        lieu: "Woippy".into(),
        texte_offre: texte_offre.into(),
        cv: cv.into(),
        profil: "Postes : agent de quai, cariste\nLieu : Metz, 30 km autour\nContrats : CDI, CDD".into(),
        consigne: None,
        precedente: None,
    }
}

const OFFRE_Q: &str = "Chargement et déchargement des camions. CACES 3 exigé. Poste en journée.\nPourquoi souhaitez-vous \
                       rejoindre notre équipe ?";

fn preparer(s: &ServeurClaude, e: &Entree) -> Result<Preparee, ErreurLlm> {
    let c = Claude::nouveau(&s.adresse(), CLE, false, Duration::from_secs(10));
    let r = c.repondre(&demande(e))?;
    comprendre(e, &r.texte)
}

#[test]
fn une_candidature_normale_et_la_demande_envoyee() {
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let e = entree(OFFRE_Q, CV);
    let p = preparer(&s, &e).unwrap();
    // La lettre : les phrases appuyées sur le CV sont sûres, la phrase inventée est à vérifier
    let t = p.lettre.texte();
    assert!(t.starts_with("Madame, Monsieur,\n\n"), "{t}");
    assert!(t.ends_with("Jordan Exemple"), "{t}");
    let av = p.lettre.a_verifier();
    assert_eq!(av.len(), 1, "{av:?}");
    assert_eq!(av[0].texte, PHRASE_INVENTEE);
    assert_eq!(av[0].a_verifier, Some(Raison::ExtraitAbsent));
    assert!(p.lettre.paragraphes.iter().flatten().filter(|x| x.a_verifier.is_none()).count() >= 4);
    // La question de l'offre, sa réponse appuyée sur le CV
    assert_eq!(p.questions.len(), 1);
    assert_eq!(p.questions[0].question, "Pourquoi souhaitez-vous rejoindre notre équipe ?");
    assert!(p.questions[0].reponse.iter().all(|x| x.a_verifier.is_none()));
    // Les conseils : un passage du CV, un passage de l'offre (« absent » : seulement si c'est vrai)
    assert!(p.conseils.len() >= 2, "{:?}", p.conseils);
    assert!(p.objet.starts_with("Candidature au poste de"));
    // Ce qui a été envoyé : le modèle fort, l'effort, le secours en cas de refus, le schéma, sans réflexion forcée
    let recu = s.recus.lock().unwrap()[0].clone();
    let v: Value = serde_json::from_str(&recu).unwrap();
    assert_eq!(v["model"], "claude-opus-5-5");
    assert_eq!(v["output_config"]["effort"], "medium");
    assert_eq!(v["output_config"]["format"]["type"], "json_schema");
    assert_eq!(v["fallbacks"], "default");
    assert!(v.get("thinking").is_none() && v.get("temperature").is_none());
    let u = v["messages"][0]["content"].as_str().unwrap();
    assert!(u.contains("<cv>\nJordan Exemple") && u.contains("<offre>\nTitre : Agent de quai"));
    assert!(!u.contains("<consigne>") && !u.contains(CLE));
}

#[test]
fn une_nouvelle_version_avec_une_consigne() {
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let mut e = entree(OFFRE_Q, CV);
    let p1 = preparer(&s, &e).unwrap();
    e.consigne = Some("plus court".into());
    e.precedente = Some(p1.lettre.texte());
    let p2 = preparer(&s, &e).unwrap();
    assert!(p2.lettre.phrases() < p1.lettre.phrases());
    let recu = s.recus.lock().unwrap()[1].clone();
    let v: Value = serde_json::from_str(&recu).unwrap();
    let u = v["messages"][0]["content"].as_str().unwrap();
    assert!(u.contains("<consigne>\nplus court\n</consigne>") && u.contains("<lettre_precedente>\nMadame"));
}

#[test]
fn une_offre_sans_question() {
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let p = preparer(&s, &entree("Chargement des camions. Poste en journée. Débutants acceptés.", CV)).unwrap();
    assert!(p.questions.is_empty());
}

#[test]
fn une_offre_tres_longue() {
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let longue =
        format!("{} Quelles sont vos disponibilités ?", "Nous chargeons et déchargeons des camions. ".repeat(900));
    let p = preparer(&s, &entree(&longue, CV)).unwrap();
    assert!(p.lettre.phrases() > 0);
    // (le texte envoyé est coupé : la question de la fin n'arrive pas à Claude)
    assert!(texte_envoye(&entree(&longue, CV)).chars().count() < OFFRE_MAX + CV_MAX + PROFIL_MAX + 500);
    assert!(p.questions.is_empty());
}

#[test]
fn un_cv_vide() {
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let p = preparer(&s, &entree(OFFRE_Q, "")).unwrap();
    // Rien ne peut s'appuyer sur un CV vide : chaque phrase sur Jordan est à vérifier, aucune signature
    let sur_moi = p.lettre.paragraphes.iter().flatten().filter(|x| x.texte.starts_with("J'ai")).count();
    assert!(sur_moi >= 1);
    assert!(
        p.lettre.paragraphes.iter().flatten().filter(|x| x.texte.starts_with("J'ai")).all(|x| x.a_verifier.is_some())
    );
    assert_eq!(p.lettre.signature, "");
    assert!(p.conseils.iter().all(|c| c.genre == jobmind_core::preparation::GenreConseil::Absent));
}

#[test]
fn les_ennuis_de_l_api() {
    let s = ServeurClaude::demarrer(ModeFaux::Refus).unwrap();
    let e = entree(OFFRE_Q, CV);
    assert!(matches!(preparer(&s, &e), Err(ErreurLlm::Refus(_))));
    *s.mode.lock().unwrap() = ModeFaux::Quota;
    assert_eq!(preparer(&s, &e).unwrap_err(), ErreurLlm::Limite);
    *s.mode.lock().unwrap() = ModeFaux::CleRefusee;
    assert!(matches!(preparer(&s, &e), Err(ErreurLlm::CleRefusee(_))));
    *s.mode.lock().unwrap() = ModeFaux::Surcharge;
    assert!(matches!(preparer(&s, &e), Err(ErreurLlm::Surcharge(_))));
    // Une réponse abîmée : illisible, rien de gardé
    *s.mode.lock().unwrap() = ModeFaux::Abime;
    assert!(matches!(preparer(&s, &e), Err(ErreurLlm::Illisible(_))));
    // Trop lent pour le délai
    *s.mode.lock().unwrap() = ModeFaux::Lent;
    let c = Claude::nouveau(&s.adresse(), CLE, false, Duration::from_secs(2));
    assert_eq!(c.repondre(&demande(&e)).unwrap_err(), ErreurLlm::Delai);
}

#[test]
fn des_reponses_abimees_ou_vides() {
    let e = entree(OFFRE_Q, CV);
    for r in [
        "",
        "non",
        "[]",
        "{\"paragraphes\": ",
        r#"{"paragraphes": [[{"texte": "  ", "sur_moi": false, "extraits": []}]]}"#,
    ] {
        assert!(matches!(comprendre(&e, r), Err(ErreurLlm::Illisible(_))), "{r}");
    }
}

#[test]
fn ce_qui_est_invente_est_laisse_ou_signale() {
    let e = entree(OFFRE_Q, CV);
    let r = json!({
        "objet": "Candidature",
        "appel": "Madame, Monsieur,",
        "paragraphes": [[
            { "texte": "J'ai le permis C.", "sur_moi": true, "extraits": ["permis C"] },
            { "texte": "J'ai été cariste en intérim.", "sur_moi": true, "extraits": ["cariste en intérim"] },
            { "texte": "Je suis titulaire d'un BTS logistique.", "sur_moi": false, "extraits": [] },
        ]],
        "politesse": "Cordialement.",
        "signature": "Jean Dupont",
        "questions": [
            { "question": "Quel salaire ?", "extrait_offre": "Indiquez vos prétentions", "reponse": [{ "texte": "1 900 euros.", "sur_moi": true, "extraits": [] }] },
        ],
        "conseils_cv": [
            { "genre": "mettre_en_avant", "texte": "Mets ton BTS en avant.", "extrait_cv": "BTS logistique", "extrait_offre": "" },
        ],
    });
    let p = comprendre(&e, &r.to_string()).unwrap();
    let l: Vec<_> = p.lettre.paragraphes.iter().flatten().collect();
    assert_eq!(l[0].a_verifier, Some(Raison::ExtraitAbsent));
    assert_eq!(l[1].a_verifier, None);
    assert_eq!(l[2].a_verifier, Some(Raison::FaitNonDeclare));
    assert_eq!(p.lettre.signature, "");
    assert!(p.questions.is_empty() && p.questions_laissees == 1);
    assert!(p.conseils.is_empty() && p.conseils_laisses == 1);
}

#[test]
fn les_consignes_et_le_schema() {
    assert!(!CONSIGNES.contains(['\u{2013}', '\u{2014}']));
    // (le tiret long est interdit à Claude, pas seulement absent des consignes)
    assert!(CONSIGNES.contains("aucun tiret long"));
    let s = schema();
    assert_eq!(s["additionalProperties"], false);
    assert_eq!(s["required"].as_array().unwrap().len(), 7);
    let d = demande(&entree("x", "y"));
    assert_eq!(d.modele, Modele::Fort);
    assert_eq!(d.jetons_max, JETONS_MAX);
}

/// Le faux Claude sur les offres et les CV jamais vus (tests\candidatures_jamais_vues.json, écrits à part) : la phrase
/// inventée exprès est toujours signalée, et chaque phrase recopiée d'une ligne du CV est sûre.
#[test]
fn le_faux_claude_sur_des_offres_et_des_cv_jamais_vus() {
    let jeu: Value = serde_json::from_str(include_str!("../../tests/candidatures_jamais_vues.json")).unwrap();
    let s = ServeurClaude::demarrer(ModeFaux::Normal).unwrap();
    let mut n = 0;
    for o in jeu["offres"].as_array().unwrap() {
        for c in jeu["cv"].as_array().unwrap() {
            let e = Entree {
                titre: o["titre"].as_str().unwrap().into(),
                entreprise: o["entreprise"].as_str().unwrap().into(),
                lieu: o["lieu"].as_str().unwrap().into(),
                texte_offre: o["texte"].as_str().unwrap().into(),
                cv: c["texte"].as_str().unwrap().into(),
                profil: c["profil"].as_str().unwrap_or_default().into(),
                consigne: None,
                precedente: None,
            };
            let p = preparer(&s, &e).unwrap();
            let l: Vec<_> = p.lettre.paragraphes.iter().flatten().collect();
            let inventee = l.iter().find(|x| x.texte == PHRASE_INVENTEE).unwrap();
            assert!(inventee.a_verifier.is_some());
            for x in l.iter().filter(|x| x.texte.starts_with("J'ai travaillé comme")) {
                assert_eq!(x.a_verifier, None, "{} / {}", x.texte, c["id"]);
            }
            n += 1;
        }
    }
    assert_eq!(n, 18);
}
