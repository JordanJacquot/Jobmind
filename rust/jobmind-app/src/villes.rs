//! Les villes proposées pendant la frappe : dans chaque case où Jordan tape une ville (la barre des offres « Autour
//! d'une ville », le formulaire d'une recherche, « Ce que je cherche », la ville d'une candidature), dès 2 lettres,
//! la page demande ([`PROPOSER`], `{ k, texte }`) et Rust répond par un ordre à la page
//! (`{ kit: 'accueil', villes: { k, texte, liste: [{ texte: "Sarrebourg (57400)", nom, cp }] } }`), cherché dans la
//! table des communes (jobmind_sources::communes::proposer : la table n'est jamais envoyée entière à la page).
//!
//! Rien au journal à chaque lettre : la demande ne passe pas par le journal des gestes (src\programme.rs).

use jobmind_sources::communes::{self, PROPOSITIONS_MAX};
use serde_json::{Value, json};

/// Le geste de la page : les villes pour ce qui est tapé.
pub const PROPOSER: &str = "villes";
/// Ce qui est tapé, coupé à cette longueur (une ville tient en 60 caractères, code postal compris).
const TEXTE_MAX: usize = 80;

/// La réponse à une demande de la page : l'ordre à lui envoyer ; rien pour une demande mal formée.
pub fn reponse(v: &Value) -> Option<Value> {
    let k = v.get("k").and_then(Value::as_str).filter(|k| crate::pont::id_valable(k))?;
    let texte = v.get("texte").and_then(Value::as_str)?;
    if texte.chars().count() > TEXTE_MAX {
        return None;
    }
    let liste: Vec<Value> = communes::proposer(texte, PROPOSITIONS_MAX)
        .iter()
        .map(|p| json!({ "texte": p.texte(), "nom": p.nom, "cp": p.code_postal }))
        .collect();
    Some(json!({ "kit": "accueil", "villes": { "k": k, "texte": texte, "liste": liste } }))
}

/// 0.1.8 : le geste de la page pour la case « poste » d'une recherche : les métiers pour ce qui est tapé.
pub const PROPOSER_METIERS: &str = "metiers";

/// La réponse pour les métiers : le même ordre que pour les villes (la page a une seule liste de propositions), avec
/// `genre: "metiers"` ; chaque ligne a le nom à poser dans la case et, en petit, sa famille de métiers quand elle dit
/// autre chose. Les noms viennent de la table embarquée (jobmind_sources::metiers), jamais d'Internet.
pub fn reponse_metiers(v: &Value) -> Option<Value> {
    use jobmind_sources::metiers;
    let k = v.get("k").and_then(Value::as_str).filter(|k| crate::pont::id_valable(k))?;
    let texte = v.get("texte").and_then(Value::as_str)?;
    if texte.chars().count() > TEXTE_MAX {
        return None;
    }
    let liste: Vec<Value> = metiers::proposer(texte, metiers::PROPOSITIONS_MAX)
        .iter()
        .map(|p| {
            let sous = if p.famille == p.nom { "" } else { p.famille.as_str() };
            json!({ "texte": p.nom, "nom": p.nom, "sous": sous })
        })
        .collect();
    Some(json!({ "kit": "accueil", "villes": { "k": k, "texte": texte, "genre": "metiers", "liste": liste } }))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reponse_a_la_page() {
        let o = reponse(&json!({ "k": "of:ville", "texte": "Sarreb" })).unwrap();
        assert_eq!(o["kit"], "accueil");
        assert_eq!(o["villes"]["k"], "of:ville");
        assert_eq!(o["villes"]["texte"], "Sarreb", "ce qui était tapé revient (la page ignore une réponse en retard)");
        assert_eq!(o["villes"]["liste"][0]["texte"], "Sarrebourg (57400)");
        assert_eq!(o["villes"]["liste"][0]["cp"], "57400");
        // (une seule lettre : aucune proposition, la page ferme la liste)
        let o = reponse(&json!({ "k": "rc:ville", "texte": "S" })).unwrap();
        assert_eq!(o["villes"]["liste"], json!([]));
        // 8 au plus, jamais toute la table
        let o = reponse(&json!({ "k": "rf:ville", "texte": "sa" })).unwrap();
        assert_eq!(o["villes"]["liste"].as_array().unwrap().len(), PROPOSITIONS_MAX);
    }

    /// Contre-épreuve : une demande mal formée ne donne rien.
    #[test]
    fn demandes_mal_formees() {
        assert_eq!(reponse(&json!({ "texte": "Metz" })), None);
        assert_eq!(reponse(&json!({ "k": "<script>", "texte": "Metz" })), None);
        assert_eq!(reponse(&json!({ "k": "of:ville", "texte": 57 })), None);
        assert_eq!(reponse(&json!({ "k": "of:ville", "texte": "x".repeat(81) })), None);
        assert_eq!(reponse(&json!("Metz")), None);
    }

    /// 0.1.8 : « helpdesk » dans la case du poste : les noms du métier, puis ses voisins.
    #[test]
    fn les_metiers_proposes() {
        let o = reponse_metiers(&json!({ "k": "rf:mots", "texte": "helpdesk" })).unwrap();
        assert_eq!(o["villes"]["genre"], "metiers");
        assert_eq!(o["villes"]["k"], "rf:mots");
        assert_eq!(o["villes"]["texte"], "helpdesk");
        assert_eq!(o["villes"]["liste"][0]["texte"], "Technicien helpdesk");
        assert_eq!(o["villes"]["liste"][0]["sous"], "Conseiller support technique informatique");
        assert_eq!(o["villes"]["liste"].as_array().unwrap().len(), 8);
        // Contre-épreuve : une lettre, ou une demande mal formée
        assert_eq!(reponse_metiers(&json!({ "k": "rf:mots", "texte": "h" })).unwrap()["villes"]["liste"], json!([]));
        assert_eq!(reponse_metiers(&json!({ "k": "<script>", "texte": "helpdesk" })), None);
        assert_eq!(reponse_metiers(&json!({ "k": "rf:mots", "texte": "x".repeat(81) })), None);
    }
}
