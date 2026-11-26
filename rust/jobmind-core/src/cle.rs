//! Les clés (étape 3 du plan) : celle de l'IA Claude, la boîte Gmail et les identifiants gardés des sites. Ici,
//! seulement leur forme : une clé collée est remise au propre et vérifiée ([`verifier_cle`]) avant d'être
//! chiffrée et gardée (jobmind-storage, `cles`). La clé d'Adzuna (source retirée le 20/11/2026), gardée par une
//! ancienne version, n'est plus lue : son nom est inconnu ici.
//!
//! Une clé n'est jamais montrée ni écrite au journal : l'écran ne montre que ses 4 derniers caractères ([`apercu`]),
//! et seulement pour une clé assez longue pour que cela n'en dise presque rien ([`Cle::montrable`]).

/// Une partie de clé gardée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cle {
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
    pub const TOUTES: [Cle; 11] = [
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
            Cle::Claude => Service::Claude,
            Cle::GmailAdresse | Cle::GmailMdp => Service::Gmail,
            _ => Service::Sites,
        }
    }

    /// Ce que l'écran peut en dire (ses 4 derniers caractères) : jamais pour l'identifiant ni le mot de passe d'un site,
    /// qui ne sortent jamais de Rust, ni pour le mot de passe d'application de Gmail (16 lettres : 4 caractères en
    /// seraient le quart). Restent la clé Claude (une centaine de caractères) et l'adresse Gmail, qui n'est pas un
    /// secret.
    pub fn montrable(self) -> bool {
        self.service() != Service::Sites && self != Cle::GmailMdp
    }

    /// Les longueurs permises, et les caractères en plus des lettres et des chiffres.
    fn forme(self) -> (usize, usize, &'static [char]) {
        match self {
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
    Claude,
    /// Étape 7 : la boîte Gmail (pas dans Mes clés : sa rubrique « Ma boîte mail » à elle)
    Gmail,
    /// Les identifiants gardés des sites (pas dans Mes clés : la carte de connexion de chaque site)
    Sites,
}

impl Service {
    pub const TOUS: [Service; 1] = [Service::Claude];

    pub fn code(self) -> &'static str {
        match self {
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
    if n < min {
        return Err(format!("Cette {} semble incomplète : elle est trop courte.", nom_dans_phrase(cle)));
    }
    if n > max {
        return Err(format!("Cette {} est trop longue : copie-la de nouveau, seule.", nom_dans_phrase(cle)));
    }
    Ok(t.to_string())
}

/// « clé Claude », « adresse Gmail » (dans une phrase).
fn nom_dans_phrase(cle: Cle) -> &'static str {
    match cle {
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
    const CLAUDE: &str = "sk-ant-api03-InventeeInventeeInventeeInventeeInventee_0000-AAAA";

    #[test]
    fn cles_de_bonne_forme() {
        assert_eq!(verifier_cle(Cle::Claude, &format!("  \"{CLAUDE}\"\n")).unwrap(), CLAUDE);
        assert_eq!(verifier_cle(Cle::Claude, &format!("« {CLAUDE} »")).unwrap(), CLAUDE);
        assert_eq!(apercu(CLAUDE), "se termine par AAAA");
        assert_eq!(apercu("ab"), "se termine par ab");
        for c in Cle::TOUTES {
            assert_eq!(Cle::depuis_code(c.code()), Some(c));
            assert!(c.service().parties().contains(&c));
        }
        assert_eq!(Service::depuis_code("claude").unwrap().parties(), [Cle::Claude]);
        // (la clé d'une source retirée, gardée par une ancienne version : inconnue, jamais une erreur)
        assert_eq!(Cle::depuis_code("jooble"), None);
        assert_eq!(Service::depuis_code("jooble"), None);
        for code in ["adzuna_id", "adzuna_cle", "adzuna"] {
            assert_eq!(Cle::depuis_code(code), None);
            assert_eq!(Service::depuis_code(code), None);
        }
    }

    /// Seules la clé Claude et l'adresse Gmail ont un aperçu à l'écran : jamais un mot de passe (celui de Gmail fait
    /// 16 lettres, ses 4 dernières en seraient le quart), ni l'identifiant d'un site.
    #[test]
    fn apercu_jamais_pour_un_mot_de_passe() {
        let montrables: Vec<Cle> = Cle::TOUTES.into_iter().filter(|c| c.montrable()).collect();
        assert_eq!(montrables, [Cle::Claude, Cle::GmailAdresse]);
        assert!(!Cle::GmailMdp.montrable());
        // (l'aperçu ne montre jamais plus du dixième d'une clé montrable de la plus petite taille permise)
        let (min, _, _) = Cle::Claude.forme();
        assert!(min >= 40);
    }

    /// Contre-épreuve : une clé mal collée est refusée, avec une phrase qui ne la répète pas.
    #[test]
    fn cles_mal_collees() {
        for (cle, colle, attendu) in [
            (Cle::Claude, "   ", "Colle ici ta clé Claude"),
            (Cle::Claude, "sk-ant-api03-Inventee Inventee-InventeeInventeeInventee", "espace"),
            (Cle::Claude, "api03-InventeeInventeeInventeeInventeeInventee", "sk-ant-"),
            (Cle::Claude, "sk-ant-court", "incomplète"),
            (Cle::Claude, "sk-ant-api03-InventeeInventeeInventeeInventeeInventee!", "inattendus"),
            (Cle::Claude, &format!("sk-ant-{}", "a".repeat(260)), "trop longue"),
            (Cle::GmailMdp, "abcdabcdabcd", "incomplète"),
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
