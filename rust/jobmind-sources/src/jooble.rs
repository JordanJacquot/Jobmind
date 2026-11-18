//! Jooble, par son API officielle (https://jooble.org/api/about, documentation REST du centre d'aide de Jooble) : une
//! demande POST sur `https://jooble.org/api/<clé>`, un corps JSON `{ "keywords", "location", "radius", "page",
//! "ResultOnPage", "companysearch" }`. Le rayon n'accepte que 0, 4, 8, 16, 26, 40 ou 80 km : le plus proche au-dessus
//! est pris. La réponse : `{ "totalCount": ..., "jobs": [ { "id", "title", "location", "snippet" (le début du texte, avec
//! du HTML), "salary", "source", "type", "link", "company", "updated" } ] }`. Erreurs : 403 (clé refusée), 429 (les
//! 100 demandes du jour dépassées).
//!
//! Vu le 18/11/2026 avec une clé inventée : le site répond parfois par une page de protection anti-robots (403, HTML,
//! en-tête `cf-mitigated`) au lieu de l'API ; Jobmind le dit tel quel (« bloquée »), sans accuser la clé.

use serde_json::{Value, json};

use crate::http::{Demande, Http};
use crate::texte::{self, DESCRIPTION_MAX};
use crate::{ErreurSource, JobSource, OffreTrouvee, Recherche, erreur_statut, jour_iso, masquer, texte_json};

/// L'adresse de l'API de Jooble (la clé s'ajoute au bout).
pub const ADRESSE: &str = "https://jooble.org/api/";
pub const NOM: &str = "Jooble";
/// Les rayons que Jooble accepte, en km.
pub const RAYONS: [u32; 7] = [0, 4, 8, 16, 26, 40, 80];

/// Le rayon accepté par Jooble pour celui voulu : le plus petit au-dessus (80 km au plus).
pub fn rayon(km: u32) -> u32 {
    RAYONS.into_iter().find(|r| *r >= km).unwrap_or(80)
}

/// Jooble, avec sa clé.
pub struct Jooble<'h> {
    http: &'h dyn Http,
    adresse: String,
    cle: String,
}

impl<'h> Jooble<'h> {
    /// `adresse` : [`ADRESSE`] (le faux serveur dans les essais).
    pub fn nouvelle(http: &'h dyn Http, adresse: &str, cle: &str) -> Jooble<'h> {
        let mut a = adresse.to_string();
        if !a.ends_with('/') {
            a.push('/');
        }
        Jooble { http, adresse: a, cle: cle.to_string() }
    }

    /// La demande de cette recherche (première page seulement).
    pub fn demande(&self, r: &Recherche) -> Demande {
        let lieu = if r.lieu.trim().is_empty() { "France".to_string() } else { r.lieu.trim().to_string() };
        let mut corps = json!({
            "keywords": r.mots_cles.trim(),
            "location": lieu,
            "page": "1",
            "ResultOnPage": r.nombre_demande().to_string(),
            "companysearch": "false",
        });
        if !r.lieu.trim().is_empty() && r.rayon_km > 0 {
            corps["radius"] = json!(rayon(r.rayon_km).to_string());
        }
        Demande::Post { url: format!("{}{}", self.adresse, self.cle), json: corps.to_string() }
    }
}

/// Une offre de la réponse (rien sans titre ; sans identifiant, son lien en tient lieu).
fn offre(v: &Value) -> Option<OffreTrouvee> {
    let titre = texte::ligne(&texte_json(v.get("title")));
    let url = texte::adresse(&texte_json(v.get("link")));
    let id = match texte_json(v.get("id")) {
        i if !i.trim().is_empty() => i.trim().to_string(),
        _ if !url.is_empty() => url.clone(),
        _ => return None,
    };
    if titre.is_empty() || id.len() > 2000 {
        return None;
    }
    Some(OffreTrouvee {
        id_source: id,
        titre,
        entreprise: texte::ligne(&texte_json(v.get("company"))),
        lieu: texte::ligne(&texte_json(v.get("location"))),
        zones: Vec::new(),
        url,
        description: texte::propre(&texte_json(v.get("snippet")), DESCRIPTION_MAX),
        contrat: texte::ligne(&texte_json(v.get("type"))),
        salaire: texte::ligne(&texte_json(v.get("salary"))),
        publiee_le: jour_iso(&texte_json(v.get("updated"))),
        latitude: None,
        longitude: None,
        brut: v.to_string(),
    })
}

/// Lit une réponse de Jooble (son statut et son corps).
pub fn lire(statut: u16, corps: &str, bloquee: bool) -> Result<Vec<OffreTrouvee>, ErreurSource> {
    if statut != 200 {
        let bloquee = bloquee || corps.trim_start().starts_with('<');
        return Err(erreur_statut(NOM, statut, bloquee));
    }
    let json: Value = serde_json::from_str(corps).map_err(|_| {
        if corps.trim_start().starts_with('<') {
            ErreurSource::Bloquee(NOM.into(), "page HTML au lieu de la réponse".into())
        } else {
            ErreurSource::Illisible(NOM.into(), "pas du JSON".into())
        }
    })?;
    let Some(l) = json.get("jobs").and_then(Value::as_array) else {
        return Err(ErreurSource::Illisible(NOM.into(), "pas de liste « jobs »".into()));
    };
    Ok(l.iter().filter_map(offre).collect())
}

impl JobSource for Jooble<'_> {
    fn id(&self) -> &str {
        "jooble"
    }

    fn nom(&self) -> &str {
        NOM
    }

    fn chercher(&self, r: &Recherche) -> Result<Vec<OffreTrouvee>, ErreurSource> {
        // (la clé va dans le chemin : seulement des lettres, des chiffres et des tirets, vérifiés par jobmind-core)
        let cle = self.cle.trim();
        if cle.is_empty() {
            return Err(ErreurSource::CleManquante(NOM.into()));
        }
        if !cle.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(ErreurSource::CleRefusee(NOM.into(), "forme de la clé".into()));
        }
        let rep =
            self.http.envoyer(&self.demande(r)).map_err(|e| ErreurSource::Reseau(NOM.into(), masquer(&e, &[cle])))?;
        lire(rep.statut, &rep.corps, rep.bloquee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une réponse de Jooble d'après sa documentation REST, avec des offres inventées : complète, sans entreprise ni
    /// salaire, identifiant manquant (le lien en tient lieu), HTML dans l'extrait, lien dangereux.
    pub(crate) const REPONSE: &str = r#"{
      "totalCount": 4,
      "jobs": [
        {
          "title": "Agent de quai (H/F)",
          "location": "Metz (57)",
          "snippet": "&nbsp;...<b>Chargement</b> et déchargement des camions, contrôle des palettes...&nbsp;",
          "salary": "1 900 € par mois",
          "source": "exemple-emploi.fr",
          "type": "CDI",
          "link": "https://jooble.org/desc/-8123456789012345678?ckey=agent+de+quai",
          "company": "Transports Exemple Moselle",
          "updated": "2026-11-13T07:12:45.1230000",
          "id": -8123456789012345678
        },
        {
          "title": "Vendeur rayon bricolage",
          "location": "Nancy",
          "snippet": "Conseil des clients, mise en rayon.",
          "salary": "",
          "source": "exemple.fr",
          "type": "",
          "link": "https://jooble.org/desc/42",
          "company": "",
          "updated": ""
        },
        { "title": "Lien piégé", "link": "javascript:alert(1)", "id": 7 },
        { "title": "", "link": "https://jooble.org/desc/9", "id": 9 }
      ]
    }"#;

    #[test]
    fn une_reponse_complete() {
        let l = lire(200, REPONSE, false).unwrap();
        assert_eq!(l.len(), 3);
        let o = &l[0];
        assert_eq!(o.id_source, "-8123456789012345678");
        assert_eq!(o.titre, "Agent de quai (H/F)");
        assert_eq!((o.lieu.as_str(), o.entreprise.as_str()), ("Metz (57)", "Transports Exemple Moselle"));
        assert_eq!(o.description, "...Chargement et déchargement des camions, contrôle des palettes...");
        assert_eq!((o.contrat.as_str(), o.salaire.as_str()), ("CDI", "1 900 € par mois"));
        assert_eq!(o.publiee_le, "2026-11-13");
        // Sans identifiant : le lien
        assert_eq!(l[1].id_source, "https://jooble.org/desc/42");
        assert_eq!((l[1].entreprise.as_str(), l[1].publiee_le.as_str()), ("", ""));
        // Le lien dangereux n'est pas gardé
        assert_eq!((l[2].titre.as_str(), l[2].url.as_str()), ("Lien piégé", ""));
    }

    #[test]
    fn les_erreurs() {
        assert_eq!(lire(403, "", false).unwrap_err().code(), "cle_refusee");
        // La page de protection vue le 18/11/2026 (403, HTML) : bloquée, pas la clé
        assert_eq!(
            lire(403, "<!DOCTYPE html><html><title>Just a moment...</title>", true).unwrap_err().code(),
            "bloquee"
        );
        assert_eq!(lire(429, "Daily request limit exceeded", false).unwrap_err().code(), "quota");
        assert_eq!(lire(404, "", false).unwrap_err().code(), "serveur");
        assert_eq!(lire(200, "{\"jobs\": [", false).unwrap_err().code(), "illisible");
        assert_eq!(lire(200, "<html>", false).unwrap_err().code(), "bloquee");
        assert_eq!(lire(200, "{\"totalCount\": 0}", false).unwrap_err().code(), "illisible");
    }

    #[test]
    fn la_demande_et_le_rayon() {
        assert_eq!([rayon(0), rayon(5), rayon(16), rayon(30), rayon(200)], [0, 8, 16, 40, 80]);
        struct Rien;
        impl Http for Rien {
            fn envoyer(&self, _: &Demande) -> Result<crate::http::Reponse, String> {
                Err("réseau https://jooble.org/api/aaaabbbb-cccc-dddd".into())
            }
        }
        let j = Jooble::nouvelle(&Rien, ADRESSE, "aaaabbbb-cccc-dddd");
        let r = Recherche { mots_cles: "cariste".into(), lieu: "Metz".into(), rayon_km: 30, nombre: 0 };
        let Demande::Post { url, json } = j.demande(&r) else { panic!() };
        assert_eq!(url, "https://jooble.org/api/aaaabbbb-cccc-dddd");
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            (v["keywords"].as_str(), v["location"].as_str(), v["radius"].as_str()),
            (Some("cariste"), Some("Metz"), Some("40"))
        );
        let Demande::Post { json, .. } = j.demande(&Recherche { lieu: String::new(), ..r.clone() }) else { panic!() };
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["location"], "France");
        assert!(v.get("radius").is_none());
        let e = j.chercher(&r).unwrap_err();
        assert!(!e.to_string().contains("aaaabbbb-cccc-dddd"), "{e}");
        let sans = Jooble::nouvelle(&Rien, ADRESSE, " ");
        assert_eq!(sans.chercher(&r).unwrap_err().code(), "cle_manquante");
        let bizarre = Jooble::nouvelle(&Rien, ADRESSE, "../../x?y");
        assert_eq!(bizarre.chercher(&r).unwrap_err().code(), "cle_refusee");
    }
}
