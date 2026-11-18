//! Adzuna France, par son API officielle (https://developer.adzuna.com/docs/search) : une demande GET sur
//! `https://api.adzuna.com/v1/api/jobs/fr/search/1` avec `app_id` et `app_key`, les mots (`what`), le lieu (`where`)
//! et le rayon en km (`distance`), les offres des 30 derniers jours (`max_days_old`), les plus récentes d'abord
//! (`sort_by=date`), 50 au plus (`results_per_page`).
//!
//! La réponse : `{ "count": ..., "results": [ { "id", "title", "description" (le début du texte seulement),
//! "company": { "display_name" }, "location": { "display_name", "area": [...] }, "latitude", "longitude",
//! "redirect_url", "created", "contract_type", "contract_time", "salary_min", "salary_max", "salary_is_predicted",
//! "category": { "label" } } ] }`. Une clé refusée : statut 401 et `{ "exception": "AUTH_FAIL", ... }` (vu le 18/11/2026
//! avec une clé inventée). Un salaire estimé par Adzuna (`salary_is_predicted` à 1) n'est pas gardé : ses conditions
//! demandent alors une mention à part.
//!
//! Conditions d'Adzuna (18/11/2026) : usage personnel permis ; chaque annonce affichée porte le mot « Adzuna », lié à
//! adzuna.fr ; le lien de l'offre est son `redirect_url`.

use serde_json::Value;

use crate::http::{Demande, Http};
use crate::texte::{self, DESCRIPTION_MAX};
use crate::{
    ErreurSource, JobSource, OffreTrouvee, Recherche, erreur_statut, jour_iso, masquer, nombre_json, texte_json,
};

/// L'adresse de la recherche d'Adzuna France (sans le numéro de page).
pub const ADRESSE: &str = "https://api.adzuna.com/v1/api/jobs/fr/search";
/// Le site d'Adzuna France, où mène la mention « Adzuna » de chaque offre.
pub const SITE: &str = "https://www.adzuna.fr";
/// Les offres de moins de ce nombre de jours.
pub const JOURS_MAX: u32 = 30;
pub const NOM: &str = "Adzuna";

/// Adzuna, avec ses deux codes.
pub struct Adzuna<'h> {
    http: &'h dyn Http,
    adresse: String,
    app_id: String,
    app_key: String,
}

impl<'h> Adzuna<'h> {
    /// `adresse` : [`ADRESSE`] (le faux serveur dans les essais).
    pub fn nouvelle(http: &'h dyn Http, adresse: &str, app_id: &str, app_key: &str) -> Adzuna<'h> {
        Adzuna {
            http,
            adresse: adresse.trim_end_matches('/').to_string(),
            app_id: app_id.to_string(),
            app_key: app_key.to_string(),
        }
    }

    fn secrets(&self) -> [&str; 2] {
        [&self.app_id, &self.app_key]
    }

    /// La demande de cette recherche (première page seulement).
    pub fn demande(&self, r: &Recherche) -> Demande {
        let mut params: Vec<(String, String)> = vec![
            ("app_id".into(), self.app_id.clone()),
            ("app_key".into(), self.app_key.clone()),
            ("results_per_page".into(), r.nombre_demande().to_string()),
            ("max_days_old".into(), JOURS_MAX.to_string()),
            ("sort_by".into(), "date".into()),
        ];
        if !r.mots_cles.trim().is_empty() {
            params.push(("what".into(), r.mots_cles.trim().to_string()));
        }
        if !r.lieu.trim().is_empty() {
            params.push(("where".into(), r.lieu.trim().to_string()));
            if r.rayon_km > 0 {
                params.push(("distance".into(), r.rayon_km.to_string()));
            }
        }
        Demande::Get { url: format!("{}/1", self.adresse), params }
    }
}

/// « 24 000 » (une espace pour les milliers).
fn milliers(n: u64) -> String {
    let s = n.to_string();
    let mut t = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            t.push(' ');
        }
        t.push(c);
    }
    t
}

/// Le salaire donné par l'offre en mots : « 24 000 à 28 000 € par an », « 1 800 € par mois », « 12 € de l'heure ».
/// Rien s'il est estimé par Adzuna, absent ou absurde.
pub fn salaire(min: Option<f64>, max: Option<f64>, estime: bool) -> String {
    if estime {
        return String::new();
    }
    let ok = |x: Option<f64>| x.filter(|v| *v >= 1.0 && *v < 10_000_000.0).map(|v| v.round() as u64);
    let (a, b) = match (ok(min), ok(max)) {
        (Some(a), Some(b)) if b < a => (b, a),
        (Some(a), Some(b)) => (a, b),
        (Some(a), None) | (None, Some(a)) => (a, a),
        (None, None) => return String::new(),
    };
    let unite = match b {
        0..200 => "de l'heure",
        200..10_000 => "par mois",
        _ => "par an",
    };
    if a == b {
        format!("{} € {unite}", milliers(a))
    } else {
        format!("{} à {} € {unite}", milliers(a), milliers(b))
    }
}

/// Une offre de la réponse (rien sans identifiant ni titre).
fn offre(v: &Value) -> Option<OffreTrouvee> {
    let id = texte_json(v.get("id"));
    let titre = texte::ligne(&texte_json(v.get("title")));
    if id.trim().is_empty() || titre.is_empty() || id.len() > 80 {
        return None;
    }
    let lieu = v.get("location");
    let zones: Vec<String> = lieu
        .and_then(|l| l.get("area"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(texte::ligne).filter(|z| !z.is_empty()).take(8).collect())
        .unwrap_or_default();
    let estime = matches!(v.get("salary_is_predicted"), Some(Value::String(s)) if s == "1")
        || matches!(v.get("salary_is_predicted"), Some(Value::Number(n)) if n.as_u64() == Some(1))
        || v.get("salary_is_predicted") == Some(&Value::Bool(true));
    let contrat = [texte_json(v.get("contract_type")), texte_json(v.get("contract_time"))]
        .into_iter()
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    Some(OffreTrouvee {
        id_source: id.trim().to_string(),
        titre,
        entreprise: texte::ligne(&texte_json(v.get("company").and_then(|c| c.get("display_name")))),
        lieu: texte::ligne(&texte_json(lieu.and_then(|l| l.get("display_name")))),
        zones,
        url: texte::adresse(&texte_json(v.get("redirect_url"))),
        description: texte::propre(&texte_json(v.get("description")), DESCRIPTION_MAX),
        contrat: texte::ligne(&contrat),
        salaire: salaire(nombre_json(v.get("salary_min")), nombre_json(v.get("salary_max")), estime),
        publiee_le: jour_iso(&texte_json(v.get("created"))),
        latitude: nombre_json(v.get("latitude")).filter(|x| (-90.0..=90.0).contains(x)),
        longitude: nombre_json(v.get("longitude")).filter(|x| (-180.0..=180.0).contains(x)),
        brut: v.to_string(),
    })
}

/// Lit une réponse d'Adzuna (son statut et son corps).
pub fn lire(statut: u16, corps: &str, bloquee: bool) -> Result<Vec<OffreTrouvee>, ErreurSource> {
    let json: Option<Value> = serde_json::from_str(corps).ok();
    let exception = json.as_ref().and_then(|j| j.get("exception")).and_then(Value::as_str).unwrap_or_default();
    if exception == "AUTH_FAIL" {
        return Err(ErreurSource::CleRefusee(NOM.into(), format!("statut {statut}, AUTH_FAIL")));
    }
    if statut != 200 {
        // (une page HTML à la place du JSON : la protection du site, pas la clé)
        let bloquee = bloquee || (json.is_none() && corps.trim_start().starts_with('<'));
        let mut e = erreur_statut(NOM, statut, bloquee);
        if !exception.is_empty()
            && let ErreurSource::Serveur(_, r) = &mut e
        {
            r.push_str(&format!(", {}", exception.chars().take(40).collect::<String>()));
        }
        return Err(e);
    }
    let Some(json) = json else {
        return Err(ErreurSource::Illisible(NOM.into(), "pas du JSON".into()));
    };
    let Some(l) = json.get("results").and_then(Value::as_array) else {
        return Err(ErreurSource::Illisible(NOM.into(), "pas de liste « results »".into()));
    };
    Ok(l.iter().filter_map(offre).collect())
}

impl JobSource for Adzuna<'_> {
    fn id(&self) -> &str {
        "adzuna"
    }

    fn nom(&self) -> &str {
        NOM
    }

    fn chercher(&self, r: &Recherche) -> Result<Vec<OffreTrouvee>, ErreurSource> {
        if self.app_id.trim().is_empty() || self.app_key.trim().is_empty() {
            return Err(ErreurSource::CleManquante(NOM.into()));
        }
        let rep = self
            .http
            .envoyer(&self.demande(r))
            .map_err(|e| ErreurSource::Reseau(NOM.into(), masquer(&e, &self.secrets())))?;
        lire(rep.statut, &rep.corps, rep.bloquee).map_err(|e| match e {
            ErreurSource::Illisible(n, r) => ErreurSource::Illisible(n, masquer(&r, &self.secrets())),
            e => e,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une réponse d'Adzuna d'après sa documentation (les champs de https://developer.adzuna.com/docs/search), avec
    /// des offres inventées : complète, sans entreprise, salaire estimé, identifiant en nombre, HTML dans le texte.
    pub(crate) const REPONSE: &str = r#"{
      "__CLASS__": "Adzuna::API::Response::JobSearchResults",
      "count": 4, "mean": 26500.5,
      "results": [
        {
          "__CLASS__": "Adzuna::API::Response::Job",
          "id": "4987654321", "adref": "eyJhbGciOiJIUzI1NiJ9",
          "title": "Agent de quai H/F",
          "description": "Chargement et déchargement des camions, contrôle des palettes à l'arrivée&hellip;",
          "company": { "__CLASS__": "Adzuna::API::Response::Company", "display_name": "Transports Exemple Moselle" },
          "location": { "__CLASS__": "Adzuna::API::Response::Location", "display_name": "Metz, Moselle",
                        "area": ["France", "Grand Est", "Moselle", "Metz"] },
          "latitude": 49.1193, "longitude": 6.1757,
          "redirect_url": "https://www.adzuna.fr/land/ad/4987654321?se=abc&utm_medium=api",
          "created": "2026-11-12T08:30:00Z",
          "contract_type": "permanent", "contract_time": "full_time",
          "salary_min": 22000, "salary_max": 24000, "salary_is_predicted": "0",
          "category": { "__CLASS__": "Adzuna::API::Response::Category", "label": "Logistique", "tag": "logistics-warehouse-jobs" }
        },
        {
          "id": 4987650000, "title": "Préparateur de commandes <strong>nuit</strong>",
          "description": "Préparation des commandes en équipe de nuit.",
          "location": { "display_name": "Woippy, Moselle", "area": ["France", "Grand Est", "Moselle", "Woippy"] },
          "redirect_url": "https://www.adzuna.fr/land/ad/4987650000",
          "created": "2026-11-15T10:00:00Z",
          "salary_min": 21500, "salary_max": 21500, "salary_is_predicted": "1"
        },
        { "id": "", "title": "Sans identifiant" },
        { "id": "123", "title": "   " }
      ]
    }"#;

    #[test]
    fn une_reponse_complete() {
        let l = lire(200, REPONSE, false).unwrap();
        assert_eq!(l.len(), 2);
        let o = &l[0];
        assert_eq!(o.id_source, "4987654321");
        assert_eq!(o.titre, "Agent de quai H/F");
        assert_eq!(o.entreprise, "Transports Exemple Moselle");
        assert_eq!(o.lieu, "Metz, Moselle");
        assert_eq!(o.zones, ["France", "Grand Est", "Moselle", "Metz"]);
        assert_eq!(o.url, "https://www.adzuna.fr/land/ad/4987654321?se=abc&utm_medium=api");
        assert!(o.description.ends_with("à l'arrivée…"), "{}", o.description);
        assert_eq!(o.contrat, "permanent full_time");
        assert_eq!(o.salaire, "22 000 à 24 000 € par an");
        assert_eq!(o.publiee_le, "2026-11-12");
        assert_eq!((o.latitude, o.longitude), (Some(49.1193), Some(6.1757)));
        assert!(o.brut.contains("4987654321"));
        // Sans entreprise, identifiant en nombre, salaire estimé : pas de salaire, pas de coordonnées
        let o = &l[1];
        assert_eq!((o.id_source.as_str(), o.entreprise.as_str()), ("4987650000", ""));
        assert_eq!(o.titre, "Préparateur de commandes nuit");
        assert_eq!(o.salaire, "");
        assert_eq!(o.latitude, None);
    }

    #[test]
    fn les_salaires() {
        assert_eq!(salaire(Some(1800.0), Some(2100.0), false), "1 800 à 2 100 € par mois");
        assert_eq!(salaire(Some(12.5), None, false), "13 € de l'heure");
        assert_eq!(salaire(Some(30000.0), Some(25000.0), false), "25 000 à 30 000 € par an");
        assert_eq!(salaire(Some(30000.0), None, true), "");
        assert_eq!(salaire(Some(0.0), Some(-3.0), false), "");
    }

    /// Les erreurs : la vraie forme d'un refus de clé (vue le 18/11/2026), un quota, une page HTML, un JSON abîmé.
    #[test]
    fn les_erreurs() {
        let refus = r#"{"exception":"AUTH_FAIL","doc":"https://api.adzuna.com/v1/doc","__CLASS__":"Adzuna::API::Response::Exception","display":"Authorisation failed"}"#;
        assert_eq!(lire(401, refus, false).unwrap_err().code(), "cle_refusee");
        assert_eq!(lire(403, "Forbidden", false).unwrap_err().code(), "cle_refusee");
        assert_eq!(lire(403, "<!DOCTYPE html><title>Just a moment...</title>", false).unwrap_err().code(), "bloquee");
        assert_eq!(lire(429, "", false).unwrap_err().code(), "quota");
        assert_eq!(lire(500, "<html>oups</html>", false).unwrap_err().code(), "serveur");
        let e = lire(400, r#"{"exception":"BAD_ARGUMENT","display":"x"}"#, false).unwrap_err();
        assert_eq!(e, ErreurSource::Serveur(NOM.into(), "statut 400, BAD_ARGUMENT".into()));
        assert_eq!(lire(200, r#"{"results": [ {"id": "1", "#, false).unwrap_err().code(), "illisible");
        assert_eq!(lire(200, r#"{"count": 0}"#, false).unwrap_err().code(), "illisible");
        assert!(lire(200, r#"{"count": 0, "results": []}"#, false).unwrap().is_empty());
    }

    /// La demande : la clé dans les paramètres (jamais ailleurs), le rayon seulement avec un lieu.
    #[test]
    fn la_demande() {
        struct Rien;
        impl Http for Rien {
            fn envoyer(&self, _: &Demande) -> Result<crate::http::Reponse, String> {
                Err("réseau : ConnectionRefused (https://api.adzuna.com/?app_key=0123456789abcdef)".into())
            }
        }
        let a = Adzuna::nouvelle(&Rien, ADRESSE, "1a2b3c4d", "0123456789abcdef");
        let r = Recherche { mots_cles: " agent de quai ".into(), lieu: "Metz".into(), rayon_km: 30, nombre: 0 };
        let Demande::Get { url, params } = a.demande(&r) else { panic!() };
        assert_eq!(url, "https://api.adzuna.com/v1/api/jobs/fr/search/1");
        let p = |k: &str| params.iter().find(|(x, _)| x == k).map(|(_, v)| v.as_str());
        assert_eq!((p("what"), p("where"), p("distance")), (Some("agent de quai"), Some("Metz"), Some("30")));
        assert_eq!((p("results_per_page"), p("sort_by"), p("max_days_old")), (Some("50"), Some("date"), Some("30")));
        let r = Recherche { lieu: String::new(), ..r };
        let Demande::Get { params, .. } = a.demande(&r) else { panic!() };
        assert!(!params.iter().any(|(k, _)| k == "where" || k == "distance"));
        // L'erreur du réseau ne répète pas la clé
        let e = a.chercher(&r).unwrap_err();
        assert_eq!(e.code(), "reseau");
        assert!(!e.to_string().contains("0123456789abcdef"), "{e}");
        // Sans clé : aucune demande
        let sans = Adzuna::nouvelle(&Rien, ADRESSE, "", "");
        assert_eq!(sans.chercher(&r).unwrap_err(), ErreurSource::CleManquante("Adzuna".into()));
    }
}
