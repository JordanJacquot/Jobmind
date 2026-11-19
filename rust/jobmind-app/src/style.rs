//! Mon profil, rubrique Mon style, partie « Mes règles » (étape 9 du plan) : Jordan voit les règles fixes de Jobmind
//! (le plancher professionnel, qui ne se coupe pas ; les formules toutes faites et les mots trop recherchés, qu'il
//! coupe ou remet), ajoute ses propres expressions à éviter (avec un remplacement s'il veut), règle la longueur des
//! phrases et l'intensité par défaut de la réécriture, et peut tout remettre comme au départ. Chaque saisie est
//! vérifiée par jobmind-core (`style`) puis écrite (jobmind-storage, `style`).

use jobmind_core::style::{Intensite, PERSO_MAX, PHRASE_MIN, PHRASE_PLUS, verifier_regle};
use jobmind_humanizer::Genre;
use jobmind_storage::{Base, rusqlite, style as base_style};
use serde_json::{Value, json};

use crate::suivi::{Fait, PAS_ENREGISTRE, SANS_BASE};

/// Les ids des gestes.
pub const REGLE: &str = "sty-regle";
pub const AJOUTER: &str = "sty-ajouter";
pub const SUPPRIMER: &str = "sty-supprimer";
pub const ZERO: &str = "sty-zero";
pub const PHRASE: &str = "sty-phrase";
pub const INTENSITE: &str = "sty-intensite";
const GESTES: [&str; 6] = [REGLE, AJOUTER, SUPPRIMER, ZERO, PHRASE, INTENSITE];

/// Un geste de « Mes règles ».
pub fn est_geste(id: &str) -> bool {
    GESTES.contains(&id)
}

/// Ce qui ne se coupe jamais (montré à Jordan, en mots simples).
pub const PLANCHER: [&str; 5] = [
    "Toujours le vouvoiement, jamais le tutoiement.",
    "Une formule d'appel (Madame, Monsieur,) et une formule de politesse à la fin.",
    "Aucun mot familier ni tournure orale (salut, boulot, du coup...).",
    "Ni point d'exclamation, ni tiret long, ni émoji.",
    "Rien d'inventé : aucun fait, nombre ou nom absent de ton CV, de l'offre ou de ta lettre.",
];

fn message(m: impl Into<String>, journal: String) -> Fait {
    Fait { message: Some(m.into()), journal: Some(journal), change: true, ..Fait::default() }
}

/// Un geste : vérifié, puis écrit.
pub fn geste(base: Option<&Base>, id: &str, v: &Value) -> Fait {
    let Some(base) = base else {
        return Fait::refus(SANS_BASE, format!("Style : {id} impossible, la base n'est pas ouverte"));
    };
    let c = base.connexion();
    let r: rusqlite::Result<Fait> = (|| match id {
        REGLE => {
            let active = v.get("active").and_then(Value::as_bool).unwrap_or(true);
            let fait = if let Some(code) = v.get("code").and_then(Value::as_str) {
                base_style::basculer_depart(c, code, active)?
            } else if let Some(n) = v.get("id").and_then(Value::as_i64) {
                base_style::basculer_perso(c, n, active)?
            } else {
                false
            };
            Ok(Fait {
                journal: fait
                    .then(|| format!("Style : règle {} {}", texte_id(v), if active { "remise" } else { "coupée" })),
                change: true,
                ..Fait::default()
            })
        }
        AJOUTER => {
            let motif = v.get("motif").and_then(Value::as_str).unwrap_or_default();
            let rempl = v.get("remplacement").and_then(Value::as_str).unwrap_or_default();
            let (m, r) = match verifier_regle(motif, rempl) {
                Ok(x) => x,
                Err(e) => {
                    return Ok(Fait {
                        message: Some(e),
                        journal: Some("Style : expression refusée".into()),
                        ..Fait::default()
                    });
                }
            };
            match base_style::ajouter_perso(c, &m, r.as_deref())? {
                Some(n) => Ok(Fait {
                    message: Some("C'est ajouté : Jobmind l'évitera dans tes lettres.".into()),
                    bravo: true,
                    journal: Some(format!("Style : expression {n} ajoutée")),
                    change: true,
                    ordre: Some(json!({ "kit": "accueil", "style_regle": n })),
                    ..Fait::default()
                }),
                None => Ok(message(
                    format!(
                        "Elle y est déjà, ou tu en as déjà {PERSO_MAX} : supprime-en une pour en ajouter une autre."
                    ),
                    "Style : expression non ajoutée (déjà là ou plafond)".into(),
                )),
            }
        }
        SUPPRIMER => {
            let n = v.get("id").and_then(Value::as_i64).unwrap_or(0);
            let ok = base_style::supprimer_perso(c, n)?;
            Ok(Fait {
                journal: ok.then(|| format!("Style : expression {n} supprimée")),
                change: true,
                ..Fait::default()
            })
        }
        ZERO => {
            base_style::remettre(c)?;
            Ok(Fait {
                message: Some("Tout est remis comme au départ.".into()),
                journal: Some("Style : règles remises comme au départ".into()),
                change: true,
                ordre: Some(json!({ "kit": "accueil", "style_zero": true })),
                ..Fait::default()
            })
        }
        PHRASE => {
            let n = v.get("max").and_then(Value::as_i64).unwrap_or(0);
            let n = base_style::changer_phrase_max(c, n)?;
            Ok(Fait { journal: Some(format!("Style : phrases de {n} mots au plus")), change: true, ..Fait::default() })
        }
        INTENSITE => match v.get("code").and_then(Value::as_str).and_then(Intensite::depuis_code) {
            Some(i) => {
                base_style::changer_intensite(c, i)?;
                Ok(Fait { journal: Some(format!("Style : intensité {}", i.code())), change: true, ..Fait::default() })
            }
            None => Ok(Fait::default()),
        },
        _ => Ok(Fait::default()),
    })();
    r.unwrap_or_else(|e| Fait::refus(PAS_ENREGISTRE, format!("Style : {id} non enregistré ({e})")))
}

fn texte_id(v: &Value) -> String {
    match (v.get("code").and_then(Value::as_str), v.get("id").and_then(Value::as_i64)) {
        (Some(c), _) => c.chars().filter(|x| x.is_ascii_alphanumeric() || *x == '-').take(40).collect(),
        (_, Some(n)) => format!("perso-{n}"),
        _ => "?".into(),
    }
}

/// Ce que « Mes règles » montre (rien : la base est illisible).
pub fn vue(base: Option<&Base>) -> Value {
    let Some(b) = base else { return Value::Null };
    let c = b.connexion();
    let lu = (|| -> rusqlite::Result<_> { Ok((base_style::regles(c)?, base_style::intensite(c)?)) })();
    let Ok((regles, intensite)) = lu else { return Value::Null };
    let groupe = |g: Genre| -> Value {
        let l: Vec<Value> = regles
            .liste
            .iter()
            .filter(|r| r.genre == g)
            .map(|r| {
                let id = r.code.strip_prefix("perso-").and_then(|x| x.parse::<i64>().ok());
                json!({
                    "code": if id.is_some() { Value::Null } else { json!(r.code) },
                    "id": id,
                    "motif": r.motif,
                    "sous": match r.remplacement.as_deref() {
                        Some("") => "Retirée d'elle-même de tes lettres.".to_string(),
                        Some(x) => format!("Devient « {x} » d'elle-même."),
                        None => r.raison.clone(),
                    },
                    "raison": r.raison,
                    "active": r.active,
                })
            })
            .collect();
        json!({ "id": g.code(), "titre": g.titre(), "liste": l })
    };
    let coupees = regles.liste.iter().filter(|r| !r.active && r.genre != Genre::Perso).count();
    json!({
        "plancher": PLANCHER,
        "phrase_max": regles.phrase_max,
        "phrase_min": PHRASE_MIN,
        "phrase_plus": PHRASE_PLUS,
        "intensite": intensite.code(),
        "intensites": Intensite::TOUTES.iter().filter(|i| i.avec_claude()).map(|i| json!({ "code": i.code(), "nom": i.nom(), "texte": i.explication() })).collect::<Vec<_>>(),
        "groupes": [groupe(Genre::Perso), groupe(Genre::Ia), groupe(Genre::Pompeux)],
        "coupees": coupees,
        "perso_max": PERSO_MAX,
        "textes": {
            "p_textes": "Mes textes",
            "p_regles": "Mes règles",
            "titre": "Mes règles",
            "intro": "Jobmind applique ces règles à chaque lettre, avant et après Claude. Ce qu'elles trouvent est surligné dans ta lettre et baisse le score naturel. Coupe celles qui ne te vont pas, ajoute les tiennes.",
            "plancher_t": "Toujours appliqué",
            "plancher_s": "Une candidature reste professionnelle quoi qu'il arrive : ces règles ne se coupent pas.",
            "longueur_t": "La longueur des phrases",
            "longueur": "mots au plus par phrase",
            "moins": "Moins",
            "plus": "Plus",
            "intensite_t": "La réécriture par Claude, par défaut",
            "ajouter_t": "Ajouter une expression à éviter",
            "motif": "L'expression",
            "i_motif": "Par exemple : je me permets de",
            "remplacement": "La remplacer par",
            "facultatif": "facultatif : sinon elle est seulement surlignée",
            "i_remplacement": "Par exemple : je vous adresse",
            "ajouter": "Ajouter",
            "active": "Active",
            "coupee": "Coupée",
            "supprimer": "Supprimer",
            "devient": "devient",
            "aucune_perso": "Aucune pour l'instant.",
            "zero": "Tout remettre comme au départ",
            "zero_q": "Remettre toutes les règles comme au départ ? Tes expressions seront supprimées.",
            "oui": "Oui, tout remettre",
            "non": "Non, garder",
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobmind_storage::Moment;

    #[test]
    fn mes_regles() {
        let (b, _) = Base::en_memoire(Moment::nouveau(2026, 11, 19, 9, 0, 0).unwrap()).unwrap();
        let v = vue(Some(&b));
        assert_eq!(v["phrase_max"], 25);
        assert_eq!(v["groupes"][1]["id"], "ia");
        assert!(v["groupes"][0]["liste"].as_array().unwrap().is_empty());
        // Couper une règle de départ
        let f = geste(Some(&b), REGLE, &json!({ "code": "ia-afin-de", "active": false }));
        assert!(f.change && f.journal.unwrap().contains("coupée"));
        assert_eq!(vue(Some(&b))["coupees"], 1);
        // Une expression : vide refusée, puis ajoutée, coupée, supprimée
        assert!(
            geste(Some(&b), AJOUTER, &json!({ "motif": " ", "remplacement": "" })).message.unwrap().contains("Écris")
        );
        let f = geste(Some(&b), AJOUTER, &json!({ "motif": "je me permets de", "remplacement": "je" }));
        assert!(f.bravo);
        let id = vue(Some(&b))["groupes"][0]["liste"][0]["id"].as_i64().unwrap();
        assert!(geste(Some(&b), AJOUTER, &json!({ "motif": "Je me permets de" })).message.unwrap().contains("déjà"));
        geste(Some(&b), REGLE, &json!({ "id": id, "active": false }));
        assert_eq!(vue(Some(&b))["groupes"][0]["liste"][0]["active"], false);
        geste(Some(&b), SUPPRIMER, &json!({ "id": id }));
        assert!(vue(Some(&b))["groupes"][0]["liste"].as_array().unwrap().is_empty());
        // Longueur (bornée) et intensité ; une intensité inconnue ne change rien
        geste(Some(&b), PHRASE, &json!({ "max": 2 }));
        assert_eq!(vue(Some(&b))["phrase_max"], PHRASE_MIN);
        geste(Some(&b), INTENSITE, &json!({ "code": "forte" }));
        geste(Some(&b), INTENSITE, &json!({ "code": "folle" }));
        assert_eq!(vue(Some(&b))["intensite"], "forte");
        // Tout remettre
        geste(Some(&b), ZERO, &json!({}));
        let v = vue(Some(&b));
        assert_eq!(
            (v["coupees"].as_u64(), v["phrase_max"].as_u64(), v["intensite"].as_str()),
            (Some(0), Some(25), Some("moyenne"))
        );
        // Sans base
        assert!(geste(None, ZERO, &json!({})).message.is_some());
        assert!(vue(None).is_null());
        // Aucun tiret long
        let t = v.to_string();
        assert!(!t.contains(['\u{2013}', '\u{2014}']));
    }
}
