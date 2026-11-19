//! Les mails d'exemple (étape 7), tous inventés (entreprises, villes, personnes, identifiants) d'après la forme des
//! alertes des sites en 2025 et 2026 : les tests de la lecture des alertes, et la fausse boîte Gmail de la construction
//! d'essai de l'appli (option « faux », jamais livrée).

use super::MailAlerte;

pub const LINKEDIN_1: &str = include_str!("exemples/linkedin-1.html");
pub const INDEED_1: &str = include_str!("exemples/indeed-1.html");
pub const WTTJ_1: &str = include_str!("exemples/wttj-1.html");
pub const LINKEDIN_2: &str = include_str!("exemples/linkedin-2.html");
pub const INDEED_2: &str = include_str!("exemples/indeed-2.html");
pub const FRANCE_TRAVAIL_1: &str = include_str!("exemples/francetravail-1.html");
pub const LINKEDIN_TEXTE: &str = include_str!("exemples/linkedin-texte.txt");
pub const WTTJ_2: &str = include_str!("exemples/wttj-2.html");
pub const FRANCE_TRAVAIL_2: &str = include_str!("exemples/francetravail-2.html");
pub const LINKEDIN_MESSAGE: &str = include_str!("exemples/linkedin-message.html");
pub const LINKEDIN_CANDIDATURE: &str = include_str!("exemples/linkedin-candidature.html");
pub const INDEED_CANDIDATURE: &str = include_str!("exemples/indeed-candidature.html");
pub const WTTJ_NEWSLETTER: &str = include_str!("exemples/wttj-newsletter.html");
pub const FRANCE_TRAVAIL_MOT_DE_PASSE: &str = include_str!("exemples/francetravail-mot-de-passe.html");

/// Un mail d'exemple en abrégé : nom, expéditeur, nom de l'expéditeur, objet, HTML, texte.
const fn mail(
    nom: &'static str,
    de: &'static str,
    de_nom: &'static str,
    objet: &'static str,
    html: &'static str,
    texte: &'static str,
) -> ExempleMail {
    ExempleMail { nom, de, de_nom, objet, html, texte }
}

/// Des mails des mêmes sites qui ne sont pas des alertes (aucun ne donne rien, même ceux qui ont des liens d'offres).
pub const PAS_ALERTES: [ExempleMail; 11] = [
    mail(
        "linkedin-message",
        "messages-noreply@linkedin.com",
        "Camille Exemple via LinkedIn",
        "Camille Exemple vous a envoyé un nouveau message",
        LINKEDIN_MESSAGE,
        "",
    ),
    mail(
        "linkedin-message-jobs",
        "jobs-noreply@linkedin.com",
        "LinkedIn",
        "Camille vous a envoyé un message à propos d'une offre",
        LINKEDIN_MESSAGE,
        "",
    ),
    mail(
        "linkedin-inmail",
        "inmail-hit-reply@linkedin.com",
        "Camille Exemple",
        "Une opportunité de cariste chez Transports Moselle Fret",
        LINKEDIN_MESSAGE,
        "",
    ),
    mail(
        "linkedin-candidature",
        "jobs-noreply@linkedin.com",
        "LinkedIn",
        "Camille, votre candidature a été envoyée à Transports Moselle Fret",
        LINKEDIN_CANDIDATURE,
        "",
    ),
    mail(
        "linkedin-profil",
        "notifications-noreply@linkedin.com",
        "LinkedIn",
        "Quelqu'un a consulté votre profil",
        LINKEDIN_1,
        "",
    ),
    mail(
        "indeed-candidature",
        "indeedapply@indeed.com",
        "Indeed",
        "Votre candidature a été envoyée",
        INDEED_CANDIDATURE,
        "",
    ),
    mail(
        "indeed-mise-a-jour",
        "alert@indeed.com",
        "Indeed",
        "Mise à jour de votre candidature chez Netéa Services",
        INDEED_CANDIDATURE,
        "",
    ),
    mail("indeed-cv", "alert@indeed.com", "Indeed", "Un employeur a consulté votre CV", INDEED_1, ""),
    mail(
        "wttj-newsletter",
        "hello@welcometothejungle.com",
        "Welcome to the Jungle",
        "Welcome Weekly : les métiers qui recrutent cet hiver",
        WTTJ_NEWSLETTER,
        "",
    ),
    mail(
        "wttj-confirmation",
        "no-reply@welcometothejungle.com",
        "Welcome to the Jungle",
        "Confirmez votre adresse e-mail",
        WTTJ_1,
        "",
    ),
    mail(
        "francetravail-mot-de-passe",
        "noreply@francetravail.fr",
        "France Travail",
        "Réinitialisation de votre mot de passe",
        FRANCE_TRAVAIL_MOT_DE_PASSE,
        "",
    ),
];

/// Des mails qui imitent les sites (le nom et le contenu d'une vraie alerte, un autre expéditeur) : aucun ne donne rien.
pub const IMITATIONS: [ExempleMail; 6] = [
    mail(
        "imitation-linkedin",
        "jobalerts-noreply@linkedin.com.arnaque.ru",
        "Alertes emploi LinkedIn",
        "« cariste » : 3 nouvelles offres à Metz",
        LINKEDIN_1,
        "",
    ),
    mail(
        "imitation-linkedin-tiret",
        "jobalerts-noreply@linkedin-alertes.com",
        "LinkedIn Job Alerts",
        "« cariste » : 3 nouvelles offres à Metz",
        LINKEDIN_1,
        LINKEDIN_TEXTE,
    ),
    mail(
        "imitation-indeed",
        "indeed-offres@gmail.com",
        "Indeed",
        "30 nouveaux emplois de agent d'entretien à Metz (57)",
        INDEED_1,
        "",
    ),
    mail(
        "imitation-indeed-employeur",
        "employeur-netea@indeedemail.com",
        "Netéa Services via Indeed",
        "Nouvelles offres : agent d'entretien",
        INDEED_1,
        "",
    ),
    mail(
        "imitation-wttj",
        "alerts@welcometothejungle.com.offres-emploi.top",
        "Welcome to the Jungle",
        "Nouvelles offres pour votre alerte « Menuisier »",
        WTTJ_1,
        "",
    ),
    mail(
        "imitation-francetravail",
        "francetravail.fr@offres-ft.info",
        "France Travail",
        "Vos nouvelles offres d'emploi",
        FRANCE_TRAVAIL_1,
        "",
    ),
];

/// Un mail d'exemple : son expéditeur, son objet, ses parties HTML et texte.
#[derive(Debug, Clone, Copy)]
pub struct ExempleMail {
    /// Le nom de l'exemple (pour les tests et le journal de l'essai)
    pub nom: &'static str,
    pub de: &'static str,
    pub de_nom: &'static str,
    pub objet: &'static str,
    pub html: &'static str,
    pub texte: &'static str,
}

impl ExempleMail {
    /// Le mail tel que la lecture des alertes le reçoit.
    pub fn mail(&self) -> MailAlerte<'static> {
        MailAlerte { de: self.de, de_nom: self.de_nom, objet: self.objet, html: self.html, texte: self.texte }
    }
}

/// Les alertes d'exemple (chacune donne des offres).
pub const ALERTES: [ExempleMail; 9] = [
    ExempleMail {
        nom: "linkedin-1",
        de: "jobalerts-noreply@linkedin.com",
        de_nom: "Alertes emploi LinkedIn",
        objet: "« cariste » : 3 nouvelles offres à Metz",
        html: LINKEDIN_1,
        texte: "",
    },
    ExempleMail {
        nom: "indeed-1",
        de: "alert@indeed.com",
        de_nom: "Indeed",
        objet: "30 nouveaux emplois de agent d'entretien à Metz (57)",
        html: INDEED_1,
        texte: "",
    },
    ExempleMail {
        nom: "wttj-1",
        de: "alerts@welcometothejungle.com",
        de_nom: "Welcome to the Jungle",
        objet: "Nouvelles offres pour votre alerte « Menuisier »",
        html: WTTJ_1,
        texte: "",
    },
    ExempleMail {
        nom: "linkedin-2",
        de: "jobalerts-noreply@linkedin.com",
        de_nom: "LinkedIn Job Alerts",
        objet: "Agent de quai chez Transports Moselle Fret et 2 autres postes",
        html: LINKEDIN_2,
        texte: "",
    },
    ExempleMail {
        nom: "indeed-2",
        de: "donotreply@jobalert.indeed.com",
        de_nom: "Indeed",
        objet: "Nouvelles offres : préparateur de commandes",
        html: INDEED_2,
        texte: "",
    },
    ExempleMail {
        nom: "francetravail-1",
        de: "ne-pas-repondre@francetravail.fr",
        de_nom: "France Travail",
        objet: "2 nouvelles offres correspondent à votre recherche",
        html: FRANCE_TRAVAIL_1,
        texte: "",
    },
    ExempleMail {
        nom: "linkedin-texte",
        de: "jobalerts-noreply@linkedin.com",
        de_nom: "Alertes emploi LinkedIn",
        objet: "« cariste » : 2 nouvelles offres à Metz",
        html: "",
        texte: LINKEDIN_TEXTE,
    },
    ExempleMail {
        nom: "wttj-2",
        de: "hello@welcometothejungle.com",
        de_nom: "Welcome to the Jungle",
        objet: "3 nouveaux jobs qui pourraient vous plaire",
        html: WTTJ_2,
        texte: "",
    },
    ExempleMail {
        nom: "francetravail-2",
        de: "offres@pole-emploi.fr",
        de_nom: "Pôle emploi",
        objet: "Vos nouvelles offres d'emploi",
        html: FRANCE_TRAVAIL_2,
        texte: "",
    },
];
