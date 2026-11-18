//! Le client HTTP des sources : une demande, une réponse, avec des délais d'attente. Le vrai ([`HttpUreq`]) passe par
//! ureq (bloquant : il tourne sur le fil des recherches, jamais sur celui de la fenêtre) et le chiffrement de Windows
//! (SChannel). Les tests et la construction d'essai le font parler à un faux serveur local (`faux`), en http.
//!
//! Une erreur rendue ici est une raison courte, pour le journal : la source la passe par [`crate::masquer`] avant de
//! la rendre (une adresse peut contenir une clé).

use std::time::Duration;

/// Une demande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Demande {
    /// GET, avec ses paramètres (encodés par le client)
    Get { url: String, params: Vec<(String, String)> },
    /// POST d'un corps JSON
    Post { url: String, json: String },
}

/// Une réponse, quel que soit son statut.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reponse {
    pub statut: u16,
    pub corps: String,
    /// Une page de protection anti-robots plutôt que la réponse de l'API (en-tête `cf-mitigated`, ou une page HTML)
    pub bloquee: bool,
}

/// Ce qui envoie une demande.
pub trait Http: Send + Sync {
    /// La réponse, ou la raison pour laquelle il n'y en a pas eu (réseau coupé, délai dépassé...).
    fn envoyer(&self, demande: &Demande) -> Result<Reponse, String>;
}

/// Le délai pour joindre le site.
pub const DELAI_CONNEXION: Duration = Duration::from_secs(10);
/// Le délai pour toute la demande, réponse comprise.
pub const DELAI_TOTAL: Duration = Duration::from_secs(25);
/// La taille la plus grande d'une réponse (50 offres d'Adzuna : environ 100 Ko).
pub const TAILLE_MAX: u64 = 4 * 1024 * 1024;

/// Le vrai client : ureq, SChannel, les certificats de Windows.
pub struct HttpUreq {
    agent: ureq::Agent,
}

impl HttpUreq {
    /// `https_seulement` : refuse toute adresse en http (le vrai Jobmind ; le faux serveur des essais parle en http).
    pub fn nouveau(https_seulement: bool) -> HttpUreq {
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(DELAI_CONNEXION))
            .timeout_global(Some(DELAI_TOTAL))
            .http_status_as_error(false)
            .https_only(https_seulement)
            .max_redirects(2)
            .user_agent("Jobmind")
            .accept("application/json")
            .tls_config(tls)
            .build();
        HttpUreq { agent: config.into() }
    }
}

/// Une raison courte, sans l'adresse (elle peut contenir une clé).
fn raison(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Timeout(_) => "délai dépassé".into(),
        ureq::Error::HostNotFound => "nom du site introuvable".into(),
        ureq::Error::ConnectionFailed => "connexion impossible".into(),
        ureq::Error::Io(io) => format!("réseau : {}", io.kind()),
        ureq::Error::BodyExceedsLimit(_) => "réponse trop grosse".into(),
        ureq::Error::RequireHttpsOnly(_) => "adresse refusée (https seulement)".into(),
        ureq::Error::BadUri(_) => "adresse mal formée".into(),
        ureq::Error::Tls(_) | ureq::Error::NativeTls(_) | ureq::Error::TlsRequired => {
            "connexion chiffrée impossible".into()
        }
        ureq::Error::TooManyRedirects | ureq::Error::RedirectFailed => "trop de renvois vers d'autres adresses".into(),
        ureq::Error::Protocol(_) => "réponse mal formée".into(),
        _ => "erreur du client HTTP".into(),
    }
}

impl Http for HttpUreq {
    fn envoyer(&self, demande: &Demande) -> Result<Reponse, String> {
        let r = match demande {
            Demande::Get { url, params } => {
                let mut d = self.agent.get(url);
                for (k, v) in params {
                    d = d.query(k, v);
                }
                d.call()
            }
            Demande::Post { url, json } => {
                self.agent.post(url).header("Content-Type", "application/json").send(json.as_bytes())
            }
        };
        let mut r = r.map_err(|e| raison(&e))?;
        let statut = r.status().as_u16();
        let entete = |n: &str| r.headers().get(n).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
        let html = entete("content-type").to_ascii_lowercase().contains("text/html");
        let bloquee = !entete("cf-mitigated").is_empty() || html;
        let corps = r.body_mut().with_config().limit(TAILLE_MAX).read_to_string().map_err(|e| raison(&e))?;
        Ok(Reponse { statut, corps, bloquee })
    }
}
