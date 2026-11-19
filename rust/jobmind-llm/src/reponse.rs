//! La lecture d'une réponse de recruteur par le petit modèle de Claude (étape 7 du plan), seulement quand les règles de
//! Jobmind n'ont pas su dire ce qu'elle annonce (jobmind_core::mails::proposer : rien de clair, ou des phrases qui se
//! contredisent) : une réponse négative, un entretien proposé, une réponse positive, une demande d'informations, un
//! accusé de réception, ou autre chose. Claude recopie la phrase qui le dit ; elle est cherchée dans le mail
//! ([`jobmind_core::lecture::citation_presente`]) : absente, rien n'est proposé (rien d'inventé). Claude ne reçoit que
//! l'objet et le texte du mail : ni le profil, ni l'adresse de Jordan, ni aucune clé. Jordan confirme toujours lui-même.

use jobmind_core::lecture::citation_presente;
use jobmind_core::mails::Proposition;
use serde_json::{Value, json};

use crate::{Demande, ErreurLlm, Modele};

/// Le texte d'un mail envoyé à Claude, au plus (en caractères).
pub const TEXTE_MAX: usize = 6000;

/// Les consignes données à Claude.
pub const CONSIGNES: &str = "Tu lis un mail reçu par une personne qui cherche un emploi, en réponse à une de ses \
candidatures. Dis seulement ce que le mail annonce, sans deviner.\n\
statut : refus (la candidature n'est pas retenue), entretien (un entretien, un rendez-vous ou un appel est proposé), \
positive (le poste est proposé, une embauche), infos (le recruteur demande des informations ou des documents), accuse \
(un simple accusé de réception, la candidature sera étudiée), autre (rien de tout cela, ou ce n'est pas clair). Une \
phrase au conditionnel (« si votre profil est retenu, nous vous proposerons un entretien ») n'annonce rien : accuse \
ou autre.\n\
citation : recopie mot pour mot, sans rien changer, la phrase du mail qui le dit (3 à 40 mots). Pour autre, la \
citation est vide.";

/// Le schéma imposé à la réponse.
pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "statut": { "type": "string", "enum": ["refus", "entretien", "positive", "infos", "accuse", "autre"] },
            "citation": { "type": "string" },
        },
        "required": ["statut", "citation"],
        "additionalProperties": false,
    })
}

/// Le texte envoyé à Claude : l'objet, puis le texte du mail (coupé à [`TEXTE_MAX`] caractères).
pub fn texte_envoye(objet: &str, texte: &str) -> String {
    let t: String = texte.chars().take(TEXTE_MAX).collect();
    format!("Objet : {objet}\n\n{t}")
}

/// La demande pour un mail.
pub fn demande(objet: &str, texte: &str) -> Demande {
    Demande {
        modele: Modele::Petit,
        consignes: CONSIGNES.into(),
        texte: texte_envoye(objet, texte),
        jetons_max: 300,
        schema: Some(schema()),
    }
}

/// Ce que Claude a lu, vérifié.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LuParClaude {
    /// Ce que dit le mail et la phrase qui le dit (rien : autre, ou citation rejetée)
    pub proposition: Option<(Proposition, String)>,
    /// La citation ne se trouvait pas dans le mail (rejetée)
    pub citation_inventee: bool,
}

/// Comprend la réponse de Claude pour ce mail (le texte envoyé).
pub fn comprendre(envoye: &str, reponse: &str) -> Result<LuParClaude, ErreurLlm> {
    let v: Value = serde_json::from_str(reponse.trim()).map_err(|_| ErreurLlm::Illisible("JSON abîmé".into()))?;
    let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let statut = s("statut");
    if statut == "autre" {
        return Ok(LuParClaude::default());
    }
    let p = Proposition::depuis_code(&statut).ok_or_else(|| ErreurLlm::Illisible("statut inconnu".into()))?;
    let c = s("citation");
    if !citation_presente(envoye, &c) {
        return Ok(LuParClaude { proposition: None, citation_inventee: true });
    }
    Ok(LuParClaude { proposition: Some((p, c)), citation_inventee: false })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIL: &str = "Bonjour Jordan,\nNous ne pouvons pas donner suite pour ce poste. En revanche, notre agence de \
                        Thionville recrute : nous souhaiterions vous rencontrer la semaine prochaine.\nCordialement";

    #[test]
    fn une_reponse_juste() {
        let envoye = texte_envoye("Votre candidature", MAIL);
        let r = r#"{"statut":"entretien","citation":"nous souhaiterions vous rencontrer la semaine prochaine"}"#;
        let lu = comprendre(&envoye, r).unwrap();
        assert_eq!(lu.proposition.unwrap().0, Proposition::Entretien);
        assert_eq!(comprendre(&envoye, r#"{"statut":"autre","citation":""}"#).unwrap(), LuParClaude::default());
    }

    /// Rien d'inventé : une phrase absente du mail est rejetée, rien n'est proposé.
    #[test]
    fn une_citation_inventee_est_rejetee() {
        let envoye = texte_envoye("Votre candidature", MAIL);
        let r = r#"{"statut":"positive","citation":"nous avons le plaisir de vous proposer le poste"}"#;
        let lu = comprendre(&envoye, r).unwrap();
        assert!(lu.citation_inventee && lu.proposition.is_none());
        let r = r#"{"statut":"refus","citation":""}"#;
        assert!(comprendre(&envoye, r).unwrap().proposition.is_none());
    }

    #[test]
    fn des_reponses_abimees() {
        for r in ["", "non", "{\"statut\": ", r#"{"statut":"embauche","citation":"Nous ne pouvons pas donner suite"}"#]
        {
            assert!(matches!(comprendre(MAIL, r), Err(ErreurLlm::Illisible(_))), "{r}");
        }
    }

    #[test]
    fn la_demande() {
        let d = demande("Objet", &"x".repeat(20_000));
        assert_eq!(d.modele, Modele::Petit);
        assert!(d.texte.starts_with("Objet : Objet") && d.texte.chars().count() < TEXTE_MAX + 30);
        assert!(!CONSIGNES.contains(['\u{2013}', '\u{2014}']));
    }
}
