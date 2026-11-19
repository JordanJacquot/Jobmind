//! Le client de Claude : l'API Messages (`POST /v1/messages`), en HTTP par ureq (bloquant : il tourne sur un fil à part
//! de l'appli, jamais sur celui de la fenêtre), avec le chiffrement de Windows (SChannel).
//!
//! La demande : le modèle, la longueur permise, les consignes (`system`), le texte (`messages`), et pour une réponse
//! en JSON le schéma imposé (`output_config.format`, type `json_schema`). Le petit modèle (Haiku 5.5) lit vite et peu
//! cher : effort `low`, réflexion coupée (`thinking.type = disabled`, accepté à cet effort).
//!
//! Les erreurs : 401 et 403 (clé refusée), 429 (trop de demandes), 529 et `overloaded_error` (surchargé), les autres
//! 5xx (le serveur), 400 et les autres (refusée), un délai dépassé, une réponse illisible, `stop_reason` « refusal » ou
//! « max_tokens ». Aucune n'écrit la clé ni l'adresse entière.

use std::time::Duration;

use serde_json::{Value, json};

use crate::{Demande, ErreurLlm, LlmProvider, Modele, Reponse};

/// L'adresse de l'API Messages.
pub const ADRESSE: &str = "https://api.anthropic.com/v1/messages";
/// La version de l'API envoyée dans l'en-tête `anthropic-version`.
pub const VERSION_API: &str = "2026-10-21";
/// Le délai pour une lecture d'offre.
pub const DELAI: Duration = Duration::from_secs(30);
/// La taille la plus grande d'une réponse.
const TAILLE_MAX: u64 = 1024 * 1024;

/// Claude, avec sa clé (lue au dernier moment dans Mes clés, jamais gardée ailleurs que dans ce client).
pub struct Claude {
    agent: ureq::Agent,
    adresse: String,
    cle: String,
}

impl std::fmt::Debug for Claude {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // (jamais la clé)
        f.debug_struct("Claude").field("adresse", &self.adresse).finish_non_exhaustive()
    }
}

impl Claude {
    /// `https_seulement` : refuse une adresse en http (le vrai Jobmind ; le faux serveur des essais parle en http).
    pub fn nouveau(adresse: &str, cle: &str, https_seulement: bool, delai: Duration) -> Claude {
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(10).min(delai)))
            .timeout_global(Some(delai))
            .http_status_as_error(false)
            .https_only(https_seulement)
            .max_redirects(0)
            .user_agent("Jobmind")
            .tls_config(tls)
            .build();
        Claude { agent: config.into(), adresse: adresse.to_string(), cle: cle.to_string() }
    }
}

/// Le corps JSON de la demande.
pub fn corps(d: &Demande) -> Value {
    let mut c = json!({
        "model": d.modele.id(),
        "max_tokens": d.jetons_max,
        "system": d.consignes,
        "messages": [{ "role": "user", "content": d.texte }],
    });
    let mut sortie = serde_json::Map::new();
    if d.modele == Modele::Petit {
        c["thinking"] = json!({ "type": "disabled" });
        sortie.insert("effort".into(), json!("low"));
    }
    if let Some(s) = &d.schema {
        sortie.insert("format".into(), json!({ "type": "json_schema", "schema": s }));
    }
    if !sortie.is_empty() {
        c["output_config"] = Value::Object(sortie);
    }
    c
}

fn raison(e: &ureq::Error) -> ErreurLlm {
    match e {
        ureq::Error::Timeout(_) => ErreurLlm::Delai,
        ureq::Error::HostNotFound => ErreurLlm::Reseau("nom du site introuvable".into()),
        ureq::Error::ConnectionFailed => ErreurLlm::Reseau("connexion impossible".into()),
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => ErreurLlm::Delai,
        ureq::Error::Io(io) => ErreurLlm::Reseau(format!("réseau : {}", io.kind())),
        ureq::Error::BodyExceedsLimit(_) => ErreurLlm::Illisible("réponse trop grosse".into()),
        ureq::Error::RequireHttpsOnly(_) => ErreurLlm::Reseau("adresse refusée (https seulement)".into()),
        ureq::Error::Tls(_) | ureq::Error::NativeTls(_) | ureq::Error::TlsRequired => {
            ErreurLlm::Reseau("connexion chiffrée impossible".into())
        }
        ureq::Error::Protocol(_) => ErreurLlm::Illisible("réponse mal formée".into()),
        _ => ErreurLlm::Reseau("erreur du client HTTP".into()),
    }
}

/// Comprend la réponse de l'API (son statut et son corps).
pub fn lire_reponse(statut: u16, corps: &str) -> Result<Reponse, ErreurLlm> {
    let v: Option<Value> = serde_json::from_str(corps).ok();
    let genre = v.as_ref().and_then(|v| v.pointer("/error/type")).and_then(Value::as_str).unwrap_or_default();
    match statut {
        200 => {}
        401 | 403 => return Err(ErreurLlm::CleRefusee(format!("statut {statut}"))),
        429 => return Err(ErreurLlm::Limite),
        529 => return Err(ErreurLlm::Surcharge("statut 529".into())),
        _ if genre == "overloaded_error" => return Err(ErreurLlm::Surcharge(format!("statut {statut}"))),
        500..=599 => return Err(ErreurLlm::Reseau(format!("le serveur de Claude a une erreur (statut {statut})"))),
        _ => {
            let g = if genre.is_empty() { "?" } else { genre };
            return Err(ErreurLlm::Refus(format!("statut {statut}, {g}")));
        }
    }
    let v = v.ok_or_else(|| ErreurLlm::Illisible("JSON abîmé".into()))?;
    match v.get("stop_reason").and_then(Value::as_str) {
        Some("refusal") => return Err(ErreurLlm::Refus("refusal".into())),
        Some("max_tokens") => return Err(ErreurLlm::Illisible("réponse coupée".into())),
        _ => {}
    }
    // (le texte : les blocs « text », dans l'ordre ; jamais le premier bloc par sa place)
    let texte: String = v
        .get("content")
        .and_then(Value::as_array)
        .map(|l| {
            l.iter()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    if texte.trim().is_empty() {
        return Err(ErreurLlm::Illisible("aucun texte".into()));
    }
    let jetons = |n: &str| {
        v.pointer(&format!("/usage/{n}")).and_then(Value::as_u64).map_or(0, |x| u32::try_from(x).unwrap_or(u32::MAX))
    };
    Ok(Reponse { texte, jetons_entree: jetons("input_tokens"), jetons_sortie: jetons("output_tokens") })
}

impl LlmProvider for Claude {
    fn nom(&self) -> &str {
        "Claude"
    }

    fn repondre(&self, d: &Demande) -> Result<Reponse, ErreurLlm> {
        if self.cle.trim().is_empty() {
            return Err(ErreurLlm::CleManquante);
        }
        let corps = corps(d).to_string();
        let mut r = self
            .agent
            .post(&self.adresse)
            .header("x-api-key", &self.cle)
            .header("anthropic-version", VERSION_API)
            .header("content-type", "application/json")
            .send(corps.as_bytes())
            .map_err(|e| raison(&e))?;
        let statut = r.status().as_u16();
        let texte = r.body_mut().with_config().limit(TAILLE_MAX).read_to_string().map_err(|e| raison(&e))?;
        lire_reponse(statut, &texte)
    }
}

#[cfg(test)]
mod tests;
