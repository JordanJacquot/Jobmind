//! La vérification « rien d'inventé » mesurée sur un jeu écrit à part, sans voir les règles
//! (tests\candidatures_jamais_vues.json : 3 CV, 6 offres, des phrases et des questions comme Claude pourrait les
//! rendre, chacune étiquetée d'après son sens : « sure » ou « a_verifier », « gardee » ou « laissee »).
//!
//! Ce qui compte d'abord : une phrase inventée ne doit presque jamais passer pour sûre. Les mesures sont écrites à la
//! sortie du test (`cargo test -p jobmind-llm --test jamais_vus -- --nocapture`).

use jobmind_core::preparation::{PhraseRendue, QuestionRendue, Sources, verifier_phrase, verifier_questions};
use jobmind_llm::candidature::Entree;
use serde_json::Value;

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or_default().to_string()
}

/// Ce que mesure un jeu : (phrases justes, phrases, inventions passées, inventions, questions justes, questions).
fn mesurer(nom: &str, texte: &str) -> (usize, usize, usize, usize, usize, usize) {
    let jeu: Value = serde_json::from_str(texte).unwrap();
    let trouve = |l: &str, id: &str| jeu[l].as_array().unwrap().iter().find(|x| x["id"] == id).unwrap().clone();
    let (mut justes, mut total) = (0, 0);
    let (mut inventees_passees, mut inventees) = (0, 0);
    let (mut sures_signalees, mut sures) = (0, 0);
    let (mut q_justes, mut q_total) = (0, 0);
    let mut erreurs = Vec::new();
    for cas in jeu["cas"].as_array().unwrap() {
        let o = trouve("offres", cas["offre"].as_str().unwrap());
        let c = trouve("cv", cas["cv"].as_str().unwrap());
        let e = Entree {
            titre: s(&o, "titre"),
            entreprise: s(&o, "entreprise"),
            lieu: s(&o, "lieu"),
            texte_offre: s(&o, "texte"),
            cv: s(&c, "texte"),
            profil: s(&c, "profil"),
            consigne: None,
            precedente: None,
        };
        let (profil, offre) = (e.profil_envoye(), e.offre_envoyee());
        let src = Sources { profil: &profil, offre: &offre };
        for p in cas["phrases"].as_array().unwrap() {
            let r = PhraseRendue {
                texte: s(p, "texte"),
                sur_moi: p["sur_moi"].as_bool().unwrap_or(false),
                extraits: p["extraits"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
            };
            let v = verifier_phrase(&r, src).unwrap();
            let attendu_av = s(p, "attendu") == "a_verifier";
            let dit_av = v.a_verifier.is_some();
            total += 1;
            if attendu_av == dit_av {
                justes += 1;
            } else {
                erreurs.push(format!(
                    "[{}] {} : attendu {}, rendu {:?}",
                    s(cas, "id"),
                    r.texte,
                    s(p, "attendu"),
                    v.a_verifier
                ));
            }
            if attendu_av {
                inventees += 1;
                inventees_passees += usize::from(!dit_av);
            } else {
                sures += 1;
                sures_signalees += usize::from(dit_av);
            }
        }
        for q in cas["questions"].as_array().map(Vec::as_slice).unwrap_or_default() {
            let r = QuestionRendue {
                question: s(q, "question"),
                extrait_offre: s(q, "extrait_offre"),
                reponse: vec![PhraseRendue {
                    texte: "Je vous réponds volontiers.".into(),
                    sur_moi: false,
                    extraits: vec![],
                }],
            };
            let (g, _) = verifier_questions(&[r], src);
            q_total += 1;
            let attendu = s(q, "attendu") == "gardee";
            if attendu == (g.len() == 1) {
                q_justes += 1;
            } else {
                erreurs.push(format!(
                    "[{}] question « {} » : attendu {}",
                    s(cas, "id"),
                    s(q, "question"),
                    s(q, "attendu")
                ));
            }
        }
    }
    println!("{nom} : phrases justes : {justes} sur {total}");
    println!("Phrases inventées passées pour sûres : {inventees_passees} sur {inventees}");
    println!("Phrases sûres signalées à tort : {sures_signalees} sur {sures}");
    println!("Questions justes : {q_justes} sur {q_total}");
    for e in &erreurs {
        println!("  {e}");
    }
    (justes, total, inventees_passees, inventees, q_justes, q_total)
}

/// Le premier jeu (celui qui a servi à retoucher les règles).
#[test]
fn mesure_sur_le_jeu_jamais_vu() {
    let (justes, total, inventees_passees, inventees, _, q_total) =
        mesurer("Jeu 1", include_str!("candidatures_jamais_vues.json"));
    assert!(total >= 70 && q_total >= 10);
    // (première mesure, avant retouche : 68 sur 92 justes, 23 inventions sur 44 passées ; après la vérification des
    // mots absents du CV : 83 sur 92, 2 sur 44. Les seuils gardent ce niveau)
    assert!(inventees_passees * 100 <= inventees * SEUIL_INVENTEES_PASSEES, "trop d'inventions passées");
    assert!(justes * 100 >= total * SEUIL_JUSTES, "trop d'erreurs");
}

/// Au plus ce pourcentage de phrases inventées passées pour sûres.
const SEUIL_INVENTEES_PASSEES: usize = 10;
/// Au moins ce pourcentage de phrases justes.
const SEUIL_JUSTES: usize = 85;

/// Le second jeu, écrit après la retouche, jamais vu par les règles : la mesure honnête. Ses seuils sont ceux de sa
/// première mesure (aucune retouche faite d'après lui) : 60 sur 77 justes, 7 inventions sur 35 passées pour sûres,
/// 10 phrases sûres sur 42 signalées à tort, 17 questions sur 17.
#[test]
fn mesure_sur_le_second_jeu_jamais_vu() {
    let (justes, total, inventees_passees, inventees, _, q_total) =
        mesurer("Jeu 2", include_str!("candidatures_jamais_vues_2.json"));
    assert!(total >= 60 && q_total >= 8);
    assert!(inventees_passees * 100 <= inventees * SEUIL2_INVENTEES_PASSEES, "trop d'inventions passées");
    assert!(justes * 100 >= total * SEUIL2_JUSTES, "trop d'erreurs");
}

const SEUIL2_INVENTEES_PASSEES: usize = 20;
const SEUIL2_JUSTES: usize = 77;
