//! Les clés (étape 3 du plan) : celle de la source d'offres (Adzuna : deux codes) et celle de l'IA Claude. Ici,
//! seulement leur forme : une clé collée est remise au propre et vérifiée ([`verifier_cle`]) avant d'être
//! chiffrée et gardée (jobmind-storage, `cles`). Aucune clé n'est essayée sur internet à cette étape (étapes 4 et 8).
//!
//! Une clé n'est jamais montrée ni écrite au journal : l'écran ne montre que ses 4 derniers caractères ([`apercu`]).

/// Une partie de clé gardée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cle {
    /// Adzuna : l'« Application ID »
    AdzunaId,
    /// Adzuna : l'« Application Key »
    AdzunaCle,
    Claude,
    /// Étape 7 : l'adresse de la boîte Gmail branchée, et son mot de passe d'application (16 lettres, crate::mails)
    GmailAdresse,
    GmailMdp,
    /// « Garder le mot de passe » : l'identifiant et le mot de passe de chaque site de la vue des sites, gardés après
    /// une connexion réussie pour s'y reconnecter tout seul (jobmind-app, `coulisse::garde`). Jamais vérifiés par
    /// [`verifier_cle`] : ce que le site a accepté.
    LinkedinMail,
    LinkedinMdp,
    IndeedMail,
    IndeedMdp,
    WttjMail,
    WttjMdp,
    FranceTravailMail,
    FranceTravailMdp,
}

impl Cle {
    pub const TOUTES: [Cle; 13] = [
        Cle::AdzunaId,
        Cle::AdzunaCle,
        Cle::Claude,
        Cle::GmailAdresse,
        Cle::GmailMdp,
        Cle::LinkedinMail,
        Cle::LinkedinMdp,
        Cle::IndeedMail,
        Cle::IndeedMdp,
        Cle::WttjMail,
        Cle::WttjMdp,
        Cle::FranceTravailMail,
        Cle::FranceTravailMdp,
    ];

    /// L'identifiant et le mot de passe gardés d'un site, d'après son code (« linkedin », « francetravail »).
    pub fn du_site(code: &str) -> Option<(Cle, Cle)> {
        match code {
            "linkedin" => Some((Cle::LinkedinMail, Cle::LinkedinMdp)),
            "indeed" => Some((Cle::IndeedMail, Cle::IndeedMdp)),
            "wttj" => Some((Cle::WttjMail, Cle::WttjMdp)),
            "francetravail" => Some((Cle::FranceTravailMail, Cle::FranceTravailMdp)),
            _ => None,
        }
    }

    /// Le nom gardé dans le fichier des clés (un autre nom, celui d'une clé d'une ancienne version, n'est pas lu).
    pub fn code(self) -> &'static str {
        match self {
            Cle::AdzunaId => "adzuna_id",
            Cle::AdzunaCle => "adzuna_cle",
            Cle::Claude => "claude",
            Cle::GmailAdresse => "gmail_adresse",
            Cle::GmailMdp => "gmail_mdp",
            Cle::LinkedinMail => "site_linkedin_mail",
            Cle::LinkedinMdp => "site_linkedin_mdp",
            Cle::IndeedMail => "site_indeed_mail",
            Cle::IndeedMdp => "site_indeed_mdp",
            Cle::WttjMail => "site_wttj_mail",
            Cle::WttjMdp => "site_wttj_mdp",
            Cle::FranceTravailMail => "site_francetravail_mail",
            Cle::FranceTravailMdp => "site_francetravail_mdp",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Cle> {
        Cle::TOUTES.into_iter().find(|c| c.code() == code)
    }

    /// Son nom à l'écran.
    pub fn nom(self) -> &'static str {
        match self {
            Cle::AdzunaId => "Application ID",
            Cle::AdzunaCle => "Application Key",
            Cle::Claude => "Clé Claude",
            Cle::GmailAdresse => "Adresse Gmail",
            Cle::GmailMdp => "Mot de passe d'application",
            Cle::LinkedinMail | Cle::IndeedMail | Cle::WttjMail | Cle::FranceTravailMail => "Identifiant du site",
            Cle::LinkedinMdp | Cle::IndeedMdp | Cle::WttjMdp | Cle::FranceTravailMdp => "Mot de passe du site",
        }
    }

    /// Le service de la clé.
    pub fn service(self) -> Service {
        match self {
            Cle::AdzunaId | Cle::AdzunaCle => Service::Adzuna,
            Cle::Claude => Service::Claude,
            Cle::GmailAdresse | Cle::GmailMdp => Service::Gmail,
            _ => Service::Sites,
        }
    }

    /// Ce que l'écran peut en dire (ses 4 derniers caractères) : jamais pour l'identifiant ni le mot de passe d'un site,
    /// qui ne sortent jamais de Rust.
    pub fn montrable(self) -> bool {
        self.service() != Service::Sites
    }

    /// Les longueurs permises, et les caractères en plus des lettres et des chiffres.
    fn forme(self) -> (usize, usize, &'static [char]) {
        match self {
            // (8 caractères chez Adzuna aujourd'hui)
            Cle::AdzunaId => (6, 16, &[]),
            // (32 caractères chez Adzuna aujourd'hui)
            Cle::AdzunaCle => (20, 64, &[]),
            // (« sk-ant-api03-... », une centaine de caractères)
            Cle::Claude => (40, 256, &['-', '_']),
            // (vérifiées par crate::mails, jamais par verifier_cle)
            Cle::GmailAdresse => (3, 254, &['@', '.', '-', '_', '+']),
            Cle::GmailMdp => (16, 16, &[]),
            // (jamais vérifiées : ce que le site a accepté)
            _ => (1, 300, &[]),
        }
    }
}

/// Un service qui demande une clé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    Adzuna,
    Claude,
    /// Étape 7 : la boîte Gmail (pas dans Mes clés : sa rubrique « Ma boîte mail » à elle)
    Gmail,
    /// Les identifiants gardés des sites (pas dans Mes clés : la carte de connexion de chaque site)
    Sites,
}

impl Service {
    pub const TOUS: [Service; 2] = [Service::Adzuna, Service::Claude];

    pub fn code(self) -> &'static str {
        match self {
            Service::Adzuna => "adzuna",
            Service::Claude => "claude",
            Service::Gmail => "gmail",
            Service::Sites => "sites",
        }
    }

    pub fn depuis_code(code: &str) -> Option<Service> {
        Service::TOUS.into_iter().find(|s| s.code() == code)
    }

    /// Les parties de sa clé, dans l'ordre de l'écran.
    pub fn parties(self) -> &'static [Cle] {
        match self {
            Service::Adzuna => &[Cle::AdzunaId, Cle::AdzunaCle],
            Service::Claude => &[Cle::Claude],
            Service::Gmail => &[Cle::GmailAdresse, Cle::GmailMdp],
            Service::Sites => &[
                Cle::LinkedinMail,
                Cle::LinkedinMdp,
                Cle::IndeedMail,
                Cle::IndeedMdp,
                Cle::WttjMail,
                Cle::WttjMdp,
                Cle::FranceTravailMail,
                Cle::FranceTravailMdp,
            ],
        }
    }
}

/// Le début d'une clé Claude.
pub const DEBUT_CLAUDE: &str = "sk-ant-";

/// Remet au propre une clé collée (espaces et guillemets autour retirés), puis vérifie sa forme. L'erreur est une
/// phrase pour la bulle de l'écran ; elle ne répète jamais la clé.
pub fn verifier_cle(cle: Cle, colle: &str) -> Result<String, String> {
    let t = colle.trim().trim_matches(|c: char| matches!(c, '"' | '\'' | '«' | '»' | '`') || c.is_whitespace());
    if t.is_empty() {
        return Err(format!("Colle ici ta {}.", nom_dans_phrase(cle)));
    }
    if t.chars().any(char::is_whitespace) {
        return Err("Il y a une espace au milieu de la clé : copie-la de nouveau, en entier et sans espace.".into());
    }
    let (min, max, en_plus) = cle.forme();
    if cle == Cle::Claude && !t.starts_with(DEBUT_CLAUDE) {
        return Err(format!(
            "Une clé Claude commence par « {DEBUT_CLAUDE} ». Copie-la de nouveau depuis la page des clés de Claude."
        ));
    }
    if !t.chars().all(|c| c.is_ascii_alphanumeric() || en_plus.contains(&c)) {
        return Err(format!(
            "Cette {} contient des caractères inattendus : copie-la de nouveau.",
            nom_dans_phrase(cle)
        ));
    }
    let n = t.chars().count();
    if cle == Cle::AdzunaId && (20..=64).contains(&n) {
        return Err("L'Application ID est plus court : tu as peut-être collé l'Application Key à sa place.".into());
    }
    if n < min {
        return Err(format!("Cette {} semble incomplète : elle est trop courte.", nom_dans_phrase(cle)));
    }
    if n > max {
        return Err(format!("Cette {} est trop longue : copie-la de nouveau, seule.", nom_dans_phrase(cle)));
    }
    Ok(t.to_string())
}

/// « clé Claude », « Application ID » (dans une phrase).
fn nom_dans_phrase(cle: Cle) -> &'static str {
    match cle {
        Cle::AdzunaId => "Application ID",
        Cle::AdzunaCle => "Application Key",
        Cle::Claude => "clé Claude",
        Cle::GmailAdresse => "adresse Gmail",
        Cle::GmailMdp => "mot de passe d'application",
        Cle::LinkedinMail | Cle::IndeedMail | Cle::WttjMail | Cle::FranceTravailMail => "identifiant",
        Cle::LinkedinMdp | Cle::IndeedMdp | Cle::WttjMdp | Cle::FranceTravailMdp => "mot de passe",
    }
}

/// Ce que l'écran montre d'une clé gardée : ses 4 derniers caractères (« se termine par 4f2a »).
pub fn apercu(cle: &str) -> String {
    let n = cle.chars().count();
    let fin: String = cle.chars().skip(n.saturating_sub(4)).collect();
    format!("se termine par {fin}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // (des clés inventées, de la bonne forme)
    const ID: &str = "1a2b3c4d";
    const CLE_ADZUNA: &str = "0123456789abcdef0123456789abcdef";
    const CLAUDE: &str = "sk-ant-api03-InventeeInventeeInventeeInventeeInventee_0000-AAAA";

    #[test]
    fn cles_de_bonne_forme() {
        assert_eq!(verifier_cle(Cle::AdzunaId, ID).unwrap(), ID);
        assert_eq!(verifier_cle(Cle::AdzunaCle, &format!("  \"{CLE_ADZUNA}\"\n")).unwrap(), CLE_ADZUNA);
        assert_eq!(verifier_cle(Cle::Claude, &format!("« {CLAUDE} »")).unwrap(), CLAUDE);
        assert_eq!(apercu(CLE_ADZUNA), "se termine par cdef");
        assert_eq!(apercu("ab"), "se termine par ab");
        for c in Cle::TOUTES {
            assert_eq!(Cle::depuis_code(c.code()), Some(c));
            assert!(c.service().parties().contains(&c));
        }
        assert_eq!(Service::depuis_code("adzuna").unwrap().parties(), [Cle::AdzunaId, Cle::AdzunaCle]);
        // (la clé d'une source retirée, gardée par une ancienne version : inconnue, jamais une erreur)
        assert_eq!(Cle::depuis_code("jooble"), None);
        assert_eq!(Service::depuis_code("jooble"), None);
    }

    /// Contre-épreuve : une clé mal collée est refusée, avec une phrase qui ne la répète pas.
    #[test]
    fn cles_mal_collees() {
        for (cle, colle, attendu) in [
            (Cle::Claude, "   ", "Colle ici ta clé Claude"),
            (Cle::AdzunaCle, "0123456789abcdef 0123456789abcdef", "espace"),
            (Cle::Claude, "api03-InventeeInventeeInventeeInventeeInventee", "sk-ant-"),
            (Cle::Claude, "sk-ant-court", "incomplète"),
            (Cle::AdzunaId, CLE_ADZUNA, "Application Key à sa place"),
            (Cle::AdzunaId, "1a2", "incomplète"),
            (Cle::AdzunaCle, "0123456789abcdef0123456789abcdef!", "inattendus"),
            (Cle::AdzunaCle, &"a".repeat(65), "trop longue"),
            (Cle::Claude, "sk-ant-api03-clé-avec-accent-é-InventeeInventeeInventee", "inattendus"),
        ] {
            let e = verifier_cle(cle, colle).unwrap_err();
            assert!(e.contains(attendu), "{e} / {attendu}");
            if colle.trim().len() > 4 {
                assert!(!e.contains(colle.trim()), "la clé est répétée : {e}");
            }
        }
    }
}
