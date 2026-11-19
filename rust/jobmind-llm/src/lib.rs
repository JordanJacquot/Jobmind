//! L'IA de Jobmind, derrière un même trait ([`LlmProvider`]) : Claude d'abord (étapes 6 et 8 du plan), un moteur sur
//! le PC plus tard (llama.cpp comme Mindcare, ou Ollama), sans rien changer au reste de Jobmind.
//!
//! Deux usages : lire les offres que les règles simples n'ont pas su classer (le petit modèle, bon marché, étape 6 :
//! [`lecture`]), et préparer une candidature (un modèle fort, étape 8). L'IA ne reçoit que l'offre (et plus tard le
//! profil), jamais les clés ; la clé n'est jamais écrite dans une erreur, le journal ou la page.
//!
//! - [`claude`] : le client de l'API Messages de Claude (HTTP par ureq, sur un fil à part de l'appli) ;
//! - [`lecture`] : la demande « trouve le diplôme et l'expérience » (réponse en JSON imposé par un schéma), et la
//!   réponse vérifiée : chaque citation doit se trouver dans l'offre, sinon rien n'est gardé (rien d'inventé) ;
//! - [`reponse`] (étape 7) : ce qu'annonce une réponse de recruteur que les règles n'ont pas su lire, avec la phrase
//!   du mail qui le dit, vérifiée de la même façon ;
//! - [`candidature`] (étape 8) : la candidature préparée par le modèle fort (la lettre, les réponses aux questions de
//!   l'offre, les conseils pour le CV), chaque phrase sur Jordan vérifiée dans son CV ;
//! - `faux` (option « faux » et tests) : un faux Claude local, jamais la vraie API.

pub mod candidature;
pub mod claude;
#[cfg(any(test, feature = "faux"))]
pub mod faux;
pub mod lecture;
pub mod reponse;

/// Le modèle à prendre pour une demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modele {
    /// Petit et bon marché : lire une offre, trouver le diplôme et l'expérience demandés
    Petit,
    /// Fort : la lettre, les réponses aux questions, le conseil pour le CV
    Fort,
}

impl Modele {
    /// Le nom du modèle chez Claude : le Haiku le plus récent, et l'Opus le plus récent.
    pub fn id(self) -> &'static str {
        match self {
            Modele::Petit => "claude-haiku-5-5",
            Modele::Fort => "claude-opus-5-5",
        }
    }

    /// Le coût d'une réponse, en millionièmes de dollar (prix par million de jetons : Haiku 5.5 0,10 $ en entrée et
    /// 0,50 $ en sortie ; Opus 5.5 4 $ et 20 $), arrondi au-dessus.
    pub fn cout_millioniemes(self, entree: u32, sortie: u32) -> u64 {
        let (e, s) = match self {
            Modele::Petit => (10u64, 50u64),
            Modele::Fort => (400, 2000),
        };
        (u64::from(entree) * e + u64::from(sortie) * s).div_ceil(100)
    }
}

/// Une demande à l'IA.
#[derive(Debug, Clone, PartialEq)]
pub struct Demande {
    pub modele: Modele,
    /// Les consignes (toujours professionnel, rien d'inventé)
    pub consignes: String,
    /// Le texte à traiter (l'offre, le profil)
    pub texte: String,
    /// La longueur de réponse permise, en jetons
    pub jetons_max: u32,
    /// Le schéma JSON imposé à la réponse (rien : du texte libre)
    pub schema: Option<serde_json::Value>,
}

/// La réponse de l'IA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reponse {
    pub texte: String,
    pub jetons_entree: u32,
    pub jetons_sortie: u32,
}

/// Pourquoi l'IA n'a pas répondu.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErreurLlm {
    #[error("la clé de l'IA n'est pas encore donnée")]
    CleManquante,
    #[error("la clé de l'IA est refusée ({0})")]
    CleRefusee(String),
    #[error("trop de demandes à l'IA pour l'instant (statut 429)")]
    Limite,
    #[error("l'IA est surchargée pour l'instant ({0})")]
    Surcharge(String),
    #[error("l'IA a mis trop de temps à répondre")]
    Delai,
    #[error("l'IA ne répond pas ({0})")]
    Reseau(String),
    #[error("l'IA a refusé la demande ({0})")]
    Refus(String),
    #[error("réponse de l'IA illisible ({0})")]
    Illisible(String),
}

impl ErreurLlm {
    /// Un code court pour le journal.
    pub fn code(&self) -> &'static str {
        match self {
            ErreurLlm::CleManquante => "cle_manquante",
            ErreurLlm::CleRefusee(_) => "cle_refusee",
            ErreurLlm::Limite => "limite",
            ErreurLlm::Surcharge(_) => "surcharge",
            ErreurLlm::Delai => "delai",
            ErreurLlm::Reseau(_) => "reseau",
            ErreurLlm::Refus(_) => "refus",
            ErreurLlm::Illisible(_) => "illisible",
        }
    }

    /// La phrase pour Jordan, en mots simples.
    pub fn message(&self) -> String {
        match self {
            ErreurLlm::CleManquante => {
                "Ajoute ta clé Claude dans Mon profil, rubrique Mes clés, pour que Jobmind lise aussi les offres floues."
                    .into()
            }
            ErreurLlm::CleRefusee(_) => "Claude refuse ta clé. Vérifie-la dans Mon profil, rubrique Mes clés.".into(),
            ErreurLlm::Limite => "Claude reçoit trop de demandes pour l'instant. Réessaie dans quelques minutes.".into(),
            ErreurLlm::Surcharge(_) => "Claude est très demandé en ce moment. Réessaie un peu plus tard.".into(),
            ErreurLlm::Delai => "Claude a mis trop de temps à répondre. Réessaie plus tard.".into(),
            ErreurLlm::Reseau(_) => "Claude ne répond pas. Vérifie ta connexion à internet, puis réessaie.".into(),
            ErreurLlm::Refus(_) | ErreurLlm::Illisible(_) => {
                "Claude n'a pas su lire cette offre. Jobmind garde ce que ses règles ont trouvé.".into()
            }
        }
    }

    /// L'erreur arrête le lot (les offres suivantes ne sont pas demandées).
    pub fn arrete_le_lot(&self) -> bool {
        !matches!(self, ErreurLlm::Refus(_) | ErreurLlm::Illisible(_))
    }
}

/// Une IA qui répond à une demande.
pub trait LlmProvider {
    /// Le nom montré à l'écran (« Claude »)
    fn nom(&self) -> &str;
    /// La réponse à cette demande.
    fn repondre(&self, demande: &Demande) -> Result<Reponse, ErreurLlm>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn couts_et_messages() {
        // 1 000 jetons en entrée, 100 en sortie, chez Haiku 5.5 : 0,0001 $ + 0,00005 $ = 150 millionièmes
        assert_eq!(Modele::Petit.cout_millioniemes(1000, 100), 150);
        assert_eq!(Modele::Petit.cout_millioniemes(1, 0), 1);
        assert_eq!(Modele::Petit.id(), "claude-haiku-5-5");
        for e in [
            ErreurLlm::CleManquante,
            ErreurLlm::CleRefusee("401".into()),
            ErreurLlm::Limite,
            ErreurLlm::Surcharge("529".into()),
            ErreurLlm::Delai,
            ErreurLlm::Reseau("x".into()),
            ErreurLlm::Refus("x".into()),
            ErreurLlm::Illisible("x".into()),
        ] {
            assert!(!e.message().contains(['\u{2013}', '\u{2014}']), "{e}");
            assert!(!e.code().is_empty());
        }
        assert!(ErreurLlm::Limite.arrete_le_lot() && !ErreurLlm::Illisible("x".into()).arrete_le_lot());
    }
}
