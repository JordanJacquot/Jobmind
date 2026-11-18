//! L'IA de Jobmind, derrière un même trait ([`LlmProvider`]) : Claude d'abord (étapes 6 et 8 du plan), un moteur sur
//! le PC plus tard (llama.cpp comme Mindcare, ou Ollama), sans rien changer au reste de Jobmind.
//!
//! Deux usages : lire et trier les offres que les règles simples n'ont pas su classer (un petit modèle bon marché), et
//! préparer une candidature (un modèle fort). L'IA ne reçoit que l'offre et le profil, jamais les clés.
//!
//! Étape 1 : le trait seulement, sans aucun branchement.

/// Le modèle à prendre pour une demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modele {
    /// Petit et bon marché : lire une offre, trouver le diplôme et l'expérience demandés
    Petit,
    /// Fort : la lettre, les réponses aux questions, le conseil pour le CV
    Fort,
}

/// Une demande à l'IA.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Demande {
    pub modele: Modele,
    /// Les consignes (toujours professionnel, rien d'inventé)
    pub consignes: String,
    /// Le texte à traiter (l'offre, le profil)
    pub texte: String,
    /// La longueur de réponse permise, en jetons
    pub jetons_max: u32,
}

/// Pourquoi l'IA n'a pas répondu.
#[derive(Debug, thiserror::Error)]
pub enum ErreurLlm {
    #[error("la clé de l'IA n'est pas encore donnée")]
    CleManquante,
    #[error("l'IA ne répond pas ({0})")]
    Reseau(String),
    #[error("l'IA a refusé la demande ({0})")]
    Refus(String),
}

/// Une IA qui répond à une demande.
pub trait LlmProvider {
    /// Le nom montré à l'écran (« Claude »)
    fn nom(&self) -> &str;
    /// La réponse à cette demande, en texte.
    fn repondre(&self, demande: &Demande) -> Result<String, ErreurLlm>;
}
