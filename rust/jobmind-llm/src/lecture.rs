//! La lecture d'une offre par le petit modèle de Claude (étape 6 du plan), seulement pour les offres que les règles
//! n'ont pas su classer : le diplôme et l'expérience demandés, chacun avec une citation recopiée mot pour mot de
//! l'offre. La réponse est imposée en JSON par un schéma ; elle est ensuite vérifiée ici : une citation qui ne se
//! trouve pas dans l'offre ([`jobmind_core::lecture::citation_presente`]) n'est jamais gardée, la chose reste « non
//! dit » (rien d'inventé). Claude ne reçoit que l'offre : ni le profil, ni le CV, ni aucune clé.

use jobmind_core::lecture::{Diplome, Experience, citation_presente};
use serde_json::{Value, json};

use crate::{Demande, ErreurLlm, Modele};

/// Le texte d'une offre envoyé à Claude, au plus (en caractères).
pub const TEXTE_MAX: usize = 8000;

/// Les consignes données à Claude.
pub const CONSIGNES: &str = "Tu lis une offre d'emploi française pour une personne qui cherche un travail. \
Trouve seulement ce que l'offre dit, jamais ce qui est probable ou habituel pour ce métier.\n\
1. Le diplôme demandé : aucun (l'offre dit qu'aucun diplôme n'est demandé), cap_bep, bac, bac2, bac3 (licence, \
bachelor, bac+3 ou bac+4), bac5 (master, ingénieur, bac+5 et plus), ou non_dit si l'offre n'en parle pas. Plusieurs \
niveaux : le plus bas.\n\
2. L'expérience demandée : debutant (débutants acceptés, sans expérience), duree (une durée minimale, donne-la en mois \
dans experience_mois), demandee (exigée sans durée), souhaitee (appréciée mais pas exigée), ou non_dit.\n\
Pour chacun, recopie dans la citation, mot pour mot et sans rien changer, le bout de phrase de l'offre qui le dit (3 à \
25 mots). Si c'est non_dit, la citation est vide. Ne devine jamais : sans phrase qui le dit, réponds non_dit.";

/// Le schéma imposé à la réponse.
pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "diplome": { "type": "string", "enum": ["aucun", "cap_bep", "bac", "bac2", "bac3", "bac5", "non_dit"] },
            "diplome_citation": { "type": "string" },
            "experience": { "type": "string", "enum": ["debutant", "duree", "demandee", "souhaitee", "non_dit"] },
            "experience_mois": { "type": "integer" },
            "experience_citation": { "type": "string" },
        },
        "required": ["diplome", "diplome_citation", "experience", "experience_mois", "experience_citation"],
        "additionalProperties": false,
    })
}

/// Le texte de l'offre tel que Claude le reçoit : son titre, puis son texte (coupé à [`TEXTE_MAX`] caractères).
pub fn texte_envoye(titre: &str, texte: &str) -> String {
    let t: String = texte.chars().take(TEXTE_MAX).collect();
    format!("Titre : {titre}\n\n{t}")
}

/// La demande pour une offre.
pub fn demande(titre: &str, texte: &str) -> Demande {
    Demande {
        modele: Modele::Petit,
        consignes: CONSIGNES.into(),
        texte: texte_envoye(titre, texte),
        jetons_max: 400,
        schema: Some(schema()),
    }
}

/// Ce que Claude a trouvé, vérifié.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LuParClaude {
    /// Le diplôme et sa citation (rien : non dit, ou citation rejetée)
    pub diplome: Option<(Diplome, String)>,
    pub experience: Option<(Experience, String)>,
    /// Au moins une citation ne se trouvait pas dans l'offre (rejetée)
    pub citation_inventee: bool,
}

/// Comprend la réponse de Claude pour cette offre (le texte envoyé) : le JSON, les codes, et chaque citation cherchée
/// dans l'offre.
pub fn comprendre(envoye: &str, reponse: &str) -> Result<LuParClaude, ErreurLlm> {
    let v: Value = serde_json::from_str(reponse.trim()).map_err(|_| ErreurLlm::Illisible("JSON abîmé".into()))?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let mut lu = LuParClaude::default();
    let d = s("diplome");
    if d != "non_dit" {
        let d = Diplome::depuis_code(&d).ok_or_else(|| ErreurLlm::Illisible("diplôme inconnu".into()))?;
        let c = s("diplome_citation");
        if citation_presente(envoye, &c) {
            lu.diplome = Some((d, c));
        } else {
            lu.citation_inventee = true;
        }
    }
    let e = s("experience");
    if e != "non_dit" {
        let e = match e.as_str() {
            "duree" => {
                let m = v.get("experience_mois").and_then(Value::as_u64).filter(|m| (1..=600).contains(m));
                Experience::Duree(m.ok_or_else(|| ErreurLlm::Illisible("durée absente".into()))? as u16)
            }
            x => Experience::depuis_code(x).ok_or_else(|| ErreurLlm::Illisible("expérience inconnue".into()))?,
        };
        let c = s("experience_citation");
        if citation_presente(envoye, &c) {
            lu.experience = Some((e, c));
        } else {
            lu.citation_inventee = true;
        }
    }
    Ok(lu)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OFFRE: &str = "Nous recrutons un agent d'entretien. Poste ouvert aux débutants, nous formons nos équipes. \
                         Le CAP propreté est un plus.";

    #[test]
    fn une_reponse_juste() {
        let envoye = texte_envoye("Agent d'entretien", OFFRE);
        let r = r#"{"diplome":"cap_bep","diplome_citation":"Le CAP propreté est un plus","experience":"debutant",
                    "experience_mois":0,"experience_citation":"Poste ouvert aux débutants"}"#;
        let lu = comprendre(&envoye, r).unwrap();
        assert_eq!(lu.diplome, Some((Diplome::CapBep, "Le CAP propreté est un plus".into())));
        assert_eq!(lu.experience.unwrap().0, Experience::Debutant);
        assert!(!lu.citation_inventee);
        let r = r#"{"diplome":"non_dit","diplome_citation":"","experience":"duree","experience_mois":24,
                    "experience_citation":"nous formons nos équipes"}"#;
        assert_eq!(comprendre(&envoye, r).unwrap().experience.unwrap().0, Experience::Duree(24));
    }

    /// Rien d'inventé : une citation absente de l'offre est rejetée, la chose reste « non dit ».
    #[test]
    fn une_citation_inventee_est_rejetee() {
        let envoye = texte_envoye("Agent d'entretien", OFFRE);
        let r = r#"{"diplome":"aucun","diplome_citation":"aucun diplôme n'est demandé","experience":"debutant",
                    "experience_mois":0,"experience_citation":"Poste ouvert aux débutants"}"#;
        let lu = comprendre(&envoye, r).unwrap();
        assert!(lu.citation_inventee && lu.diplome.is_none());
        assert!(lu.experience.is_some());
        // Une citation vide pour une chose trouvée : rejetée aussi
        let r = r#"{"diplome":"bac","diplome_citation":"","experience":"non_dit","experience_mois":0,"experience_citation":""}"#;
        assert!(comprendre(&envoye, r).unwrap().diplome.is_none());
    }

    #[test]
    fn des_reponses_abimees() {
        for r in [
            "",
            "pas du JSON",
            "{\"diplome\": ",
            r#"{"diplome":"doctorat","diplome_citation":"x y","experience":"non_dit"}"#,
            r#"{"diplome":"non_dit","experience":"duree","experience_mois":0,"experience_citation":"ouvert aux débutants"}"#,
        ] {
            assert!(matches!(comprendre(OFFRE, r), Err(ErreurLlm::Illisible(_))), "{r}");
        }
    }

    #[test]
    fn la_demande() {
        let d = demande("Cariste", &"x".repeat(20_000));
        assert_eq!(d.modele, Modele::Petit);
        assert!(d.texte.starts_with("Titre : Cariste") && d.texte.chars().count() < TEXTE_MAX + 40);
        assert!(!CONSIGNES.contains(['\u{2013}', '\u{2014}']));
        assert_eq!(schema()["additionalProperties"], false);
    }
}
