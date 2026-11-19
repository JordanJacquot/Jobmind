//! La carte vivante de Jobmind dans l'accueil de Mindset (etat.json, format commun du contrat, partie 5). Repris de la
//! carte de Mindcare (`EtatMindset`) : écrite à côté puis remplacée d'un coup, et seulement quand elle change.
//!
//! etat.json : { Version, Titre, Scores : [ { Nom, Valeur 0 à 100 } ] (2 au plus), Lignes : [ { Texte, Etat } ]
//! (3 au plus), Maj }. Etat : ok (vert), info (blanc), attention (orange), urgent (rouge). Mindset coupe chaque ligne à
//! 80 caractères.
//!
//! Étape 2 : deux jauges (les candidatures en attente de réponse, le taux de réponse) et trois lignes (les relances à
//! faire, le prochain entretien, les envois du mois), calculées par jobmind-core (`suivi::resume_carte`). Sans
//! candidature, une seule ligne le dit.

use std::path::{Path, PathBuf};

use jobmind_core::suivi::{couper, resume_carte_avec_reponses};
use jobmind_core::{Candidature, Jour};
use serde_json::{Value, json};

use crate::fichiers::Fichiers;
use crate::valeurs::egal;

/// La longueur d'une ligne de la carte (en caractères .NET : unités UTF-16).
pub const LIGNE_MAX: usize = 80;
/// Le titre de la carte.
pub const TITRE: &str = "Mes candidatures";
/// La ligne quand il n'y a encore aucune candidature.
pub const AUCUN: &str = "Aucune candidature pour l'instant";

/// La longueur d'un texte comme `.Length` de .NET.
pub fn longueur(t: &str) -> usize {
    t.encode_utf16().count()
}

/// Une ligne de la carte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ligne {
    pub texte: String,
    /// ok, info, attention, urgent
    pub etat: String,
}

/// La carte de Mindset.
#[derive(Debug, Clone, PartialEq)]
pub struct Carte {
    pub titre: String,
    pub scores: Vec<Value>,
    pub lignes: Vec<Ligne>,
}

fn ligne(texte: &str, etat: &str) -> Ligne {
    // (coupée bien avant 80 unités UTF-16 : une lettre accentuée ou un emoji n'en prend jamais plus de deux)
    Ligne { texte: couper(texte, 70), etat: etat.to_string() }
}

/// Ce que la carte montre, d'après les candidatures, le jour et le délai de relance.
pub fn etat_carte(liste: &[Candidature], aujourdhui: Jour, delai: u32) -> Carte {
    etat_carte_avec(liste, aujourdhui, delai, 0)
}

/// [`etat_carte`], avec les réponses reçues qui attendent Jordan (étape 7 : un mail à ranger, un statut proposé) : une
/// ligne en tête (« 2 réponses reçues à voir »), la carte gardant ses 3 lignes au plus.
pub fn etat_carte_avec(liste: &[Candidature], aujourdhui: Jour, delai: u32, a_voir: usize) -> Carte {
    etat_carte_complete(liste, aujourdhui, delai, a_voir, 0)
}

/// [`etat_carte_avec`], avec les nouvelles offres trouvées par le rafraîchissement automatique et pas encore vues
/// (étape 12 : « 5 nouvelles offres », avant la ligne des envois du mois ; la carte garde ses 3 lignes au plus).
pub fn etat_carte_complete(
    liste: &[Candidature],
    aujourdhui: Jour,
    delai: u32,
    a_voir: usize,
    nouvelles: usize,
) -> Carte {
    etat_carte_reponses(liste, aujourdhui, delai, a_voir, nouvelles, &[])
}

/// [`etat_carte_complete`], avec les candidatures dont une réponse arrivée attend Jordan (`mails::candidatures_a_voir`) :
/// elles ne sont pas comptées dans « relances à faire » (la réponse est déjà là).
pub fn etat_carte_reponses(
    liste: &[Candidature],
    aujourdhui: Jour,
    delai: u32,
    a_voir: usize,
    nouvelles: usize,
    reponses: &[i64],
) -> Carte {
    let mut c = carte_seule(liste, aujourdhui, delai, reponses);
    if nouvelles > 0 {
        let t = if nouvelles == 1 { "1 nouvelle offre".to_string() } else { format!("{nouvelles} nouvelles offres") };
        if c.lignes.len() == 1 && c.lignes[0].texte == AUCUN {
            c.lignes.clear();
        }
        // (avant la dernière ligne, celle des envois du mois, quand il y a des candidatures)
        let i = if liste.is_empty() { c.lignes.len() } else { c.lignes.len().saturating_sub(1) };
        c.lignes.insert(i, ligne(&t, "info"));
    }
    if a_voir > 0 {
        let t = if a_voir == 1 {
            "1 réponse reçue à voir".to_string()
        } else {
            format!("{a_voir} réponses reçues à voir")
        };
        if c.lignes.len() == 1 && c.lignes[0].texte == AUCUN {
            c.lignes.clear();
        }
        c.lignes.insert(0, ligne(&t, "attention"));
    }
    c.lignes.truncate(3);
    c
}

fn carte_seule(liste: &[Candidature], aujourdhui: Jour, delai: u32, reponses: &[i64]) -> Carte {
    if liste.is_empty() {
        return Carte { titre: TITRE.to_string(), scores: Vec::new(), lignes: vec![ligne(AUCUN, "info")] };
    }
    let r = resume_carte_avec_reponses(liste, aujourdhui, delai, reponses);
    let mut scores = Vec::new();
    if let Some(part) = (r.en_attente * 100 + r.comptees / 2).checked_div(r.comptees) {
        let part = part.min(100);
        let nom = match r.en_attente {
            0 => "Aucune en attente".to_string(),
            1 => "1 en attente de réponse".to_string(),
            n => format!("{n} en attente de réponse"),
        };
        scores.push(json!({ "Nom": nom, "Valeur": part }));
    }
    if let Some(t) = r.taux {
        scores.push(json!({ "Nom": "Taux de réponse", "Valeur": t.min(100) }));
    }
    let mut lignes = vec![match r.relances {
        0 => ligne("Aucune relance à faire", "ok"),
        1 => ligne("1 relance à faire", "attention"),
        n => ligne(&format!("{n} relances à faire"), "attention"),
    }];
    if let Some((texte, bientot)) = &r.prochain_entretien {
        lignes.push(ligne(texte, if *bientot { "attention" } else { "info" }));
    }
    lignes.push(ligne(&r.envois, "info"));
    Carte { titre: TITRE.to_string(), scores, lignes }
}

/// L'écriture de etat.json (`Write-EtatMindset`) : seulement quand la carte change.
#[derive(Debug, Clone)]
pub struct EtatMindset {
    fichier: PathBuf,
    /// (la dernière carte écrite : rien de neuf, pas d'écriture)
    ecrit: Option<String>,
}

impl EtatMindset {
    pub fn nouveau(fichier: &Path) -> EtatMindset {
        EtatMindset { fichier: fichier.to_path_buf(), ecrit: None }
    }

    pub fn fichier(&self) -> &Path {
        &self.fichier
    }

    /// Écrit la carte dans etat.json si elle a changé depuis la dernière écriture (UTF-8 sans BOM, à côté puis
    /// remplacé d'un coup).
    pub fn ecrire(&mut self, fichiers: &mut Fichiers, carte: &Carte) {
        let mut etat = serde_json::Map::new();
        etat.insert("Version".into(), json!(1));
        etat.insert("Titre".into(), json!(carte.titre));
        etat.insert("Scores".into(), Value::Array(carte.scores.iter().take(2).cloned().collect()));
        etat.insert(
            "Lignes".into(),
            Value::Array(carte.lignes.iter().take(3).map(|l| json!({ "Texte": l.texte, "Etat": l.etat })).collect()),
        );
        let cle = Value::Object(etat.clone()).to_string();
        // (comparé comme `-eq` de PowerShell : une carte qui ne change que de majuscules n'est pas réécrite)
        if self.ecrit.as_deref().is_some_and(|e| egal(e, &cle)) {
            return;
        }
        etat.insert("Maj".into(), json!(fichiers.journal.maintenant().iso()));
        match fichiers.sauver(&self.fichier, &Value::Object(etat)) {
            Ok(()) => self.ecrit = Some(cle),
            Err(e) => fichiers.journal.ecrire(&format!("etat.json pour Mindset : {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jobmind_core::candidature::{Entretien, Moyen};
    use jobmind_core::{Heure, Statut};

    fn j(a: i32, m: u32, d: u32) -> Jour {
        Jour::nouveau(a, m, d).unwrap()
    }

    fn cand(id: i64, entreprise: &str, statut: Statut, envoyee: Jour) -> Candidature {
        Candidature {
            id,
            entreprise: entreprise.into(),
            poste: "Agent de quai".into(),
            ville: String::new(),
            statut,
            statut_le: envoyee,
            envoyee_le: envoyee,
            moyen: Moyen::Site,
            source: String::new(),
            lien: String::new(),
            envoye: String::new(),
            cv: String::new(),
            notes: String::new(),
            relance_le: None,
            entretien: None,
        }
    }

    fn verifier_limites(c: &Carte) {
        assert!(c.scores.len() <= 2 && c.lignes.len() <= 3);
        for l in &c.lignes {
            assert!(longueur(&l.texte) <= LIGNE_MAX, "{}", l.texte);
            // (aucun tiret long ni demi-cadratin : règle de la suite)
            assert!(!l.texte.contains(['\u{2013}', '\u{2014}']));
            assert!(["ok", "info", "attention", "urgent"].contains(&l.etat.as_str()));
        }
        for s in &c.scores {
            let v = s["Valeur"].as_u64().unwrap();
            assert!(v <= 100);
            assert!(longueur(s["Nom"].as_str().unwrap()) <= LIGNE_MAX);
        }
    }

    #[test]
    fn sans_candidature() {
        let c = etat_carte(&[], j(2026, 11, 18), 14);
        assert_eq!(c.titre, "Mes candidatures");
        assert_eq!(c.lignes, vec![Ligne { texte: "Aucune candidature pour l'instant".into(), etat: "info".into() }]);
        assert!(c.scores.is_empty());
        verifier_limites(&c);
    }

    #[test]
    fn deux_jauges_et_trois_lignes() {
        let auj = j(2026, 11, 18);
        let mut l = vec![
            cand(1, "Transports Moselle Fret", Statut::Envoyee, j(2026, 11, 2)),
            cand(2, "Netéa Services", Statut::Envoyee, j(2026, 11, 16)),
            cand(3, "Avenir Numérique Est", Statut::Positive, j(2026, 11, 3)),
            cand(
                4,
                "Logistique des Trois Frontières, un nom vraiment très très long pour voir",
                Statut::Entretien,
                j(2026, 11, 5),
            ),
        ];
        l[3].entretien = Some(Entretien { jour: j(2026, 11, 19), heure: Heure::nouvelle(9, 30), lieu: String::new() });
        let c = etat_carte(&l, auj, 14);
        verifier_limites(&c);
        assert_eq!(
            Value::Array(c.scores.clone()),
            json!([{ "Nom": "3 en attente de réponse", "Valeur": 75 }, { "Nom": "Taux de réponse", "Valeur": 50 }])
        );
        assert_eq!(c.lignes[0], Ligne { texte: "1 relance à faire".into(), etat: "attention".into() });
        assert!(
            c.lignes[1].texte.starts_with("Entretien demain à 9 h 30 : Logistique des Trois"),
            "{}",
            c.lignes[1].texte
        );
        assert_eq!(c.lignes[1].etat, "attention");
        assert_eq!(c.lignes[2], Ligne { texte: "4 envois en novembre".into(), etat: "info".into() });
        // Sans entretien ni relance
        let c = etat_carte(&l[1..3], auj, 14);
        verifier_limites(&c);
        assert_eq!(
            c.lignes.iter().map(|x| x.texte.as_str()).collect::<Vec<_>>(),
            ["Aucune relance à faire", "2 envois en novembre"]
        );
    }

    /// Étape 12 : les nouvelles offres trouvées toutes seules, sur la carte.
    #[test]
    fn les_nouvelles_offres() {
        let auj = j(2026, 11, 18);
        let l = [cand(2, "Netéa Services", Statut::Envoyee, j(2026, 11, 16))];
        let c = etat_carte_complete(&l, auj, 14, 0, 5);
        verifier_limites(&c);
        assert_eq!(
            c.lignes.iter().map(|x| x.texte.as_str()).collect::<Vec<_>>(),
            ["Aucune relance à faire", "5 nouvelles offres", "1 envoi en novembre"]
        );
        let c = etat_carte_complete(&[], auj, 14, 0, 1);
        assert_eq!(c.lignes.iter().map(|x| x.texte.as_str()).collect::<Vec<_>>(), ["1 nouvelle offre"]);
        // Avec une réponse à voir : 3 lignes au plus, les envois du mois partent
        let c = etat_carte_complete(&l, auj, 14, 2, 5);
        verifier_limites(&c);
        assert_eq!(
            c.lignes.iter().map(|x| x.texte.as_str()).collect::<Vec<_>>(),
            ["2 réponses reçues à voir", "Aucune relance à faire", "5 nouvelles offres"]
        );
        assert_eq!(etat_carte_complete(&l, auj, 14, 0, 0), etat_carte(&l, auj, 14));
    }
}
