//! Les formulaires de candidature imités (étape 11), tout inventés : les tests de [`super::lire`], et la construction
//! d'essai de l'appli (option « faux », jamais livrée), où le faux serveur local (127.0.0.1) les sert à la vue des
//! sites. Aucun vrai site n'est jamais ouvert, aucune vraie candidature n'est envoyée.
//!
//! Sous `/site/<hôte>/<chemin>` comme les autres pages imitées :
//! - Indeed : l'offre et son lien « Postuler maintenant », puis un formulaire en trois étapes (coordonnées ; CV, une
//!   question à choix et une question écrite ; vérification et consentement), puis la confirmation ;
//! - Welcome to the Jungle : l'offre, dont le bouton « Postuler » montre le formulaire sur la même page (un champ
//!   obligatoire que Jobmind ne sait pas remplir : le salaire souhaité ; consentement, newsletter) ;
//! - France Travail : l'offre et son lien « Postuler », qui demande d'abord d'être connecté (la session factice de
//!   l'étape 5 bis), puis un formulaire avec une civilité à choisir et un captcha factice qui se fait seul quelques
//!   secondes après que la page est montrée dans la petite fenêtre (comme si Jordan l'avait fait) ;
//! - un site carrière inconnu : un formulaire aux libellés inhabituels ;
//! - une page que Jobmind ne sait pas lire (le formulaire dans un cadre).
//!
//! [`ModeFormulaires::Refus`] : le prochain envoi est refusé par le site (une seule fois). La page de confirmation dit
//! le nom du fichier de CV reçu : l'essai vérifie ainsi que le CV a bien été déposé dans la case.

use std::sync::Mutex;

use crate::faux::{RepFaux, Requete};

/// Ce que fera le prochain envoi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeFormulaires {
    #[default]
    Normal,
    /// Le site refuse le prochain envoi (une seule fois)
    Refus,
}

/// Une offre de l'essai pour postuler sur le site : (source, id chez la source, adresse, titre, entreprise, ville,
/// texte).
pub type OffreEssai =
    (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str);

/// Les offres de l'essai (étape 11) : une par formulaire imité, plus une offre de LinkedIn.
pub const OFFRES_ESSAI: [OffreEssai; 6] = [
    (
        "indeed",
        "postuler-indeed",
        "https://fr.indeed.com/viewjob?jk=postuler-indeed",
        "Préparateur de commandes (H/F)",
        "Entrepôts de la Seille",
        "Metz",
        "Préparation de commandes en entrepôt, port de charges. Débutant accepté. Permis B souhaité. Poste à pourvoir vite.",
    ),
    (
        "wttj",
        "postuler-wttj",
        "https://www.welcometothejungle.com/fr/companies/essai-boulangerie/jobs/vendeur-postuler",
        "Vendeur en boulangerie",
        "Boulangerie des Trois Ponts",
        "Metz",
        "Accueil des clients, mise en rayon, encaissement. Horaires du matin. Débutant accepté.",
    ),
    (
        "francetravail",
        "postuler-ft",
        "https://candidat.francetravail.fr/offres/recherche/detail/postuler-ft",
        "Agent de quai (H/F)",
        "Transports Mirabelle",
        "Woippy",
        "Chargement et déchargement des camions, contrôle des colis. CACES 1 apprécié.",
    ),
    (
        "autre",
        "postuler-inconnu",
        "https://recrutement.exemple-transport.fr/offres/42",
        "Cariste (H/F)",
        "Transports de l'Exemple",
        "Thionville",
        "Conduite de chariot élévateur, CACES 3 et 5 demandés.",
    ),
    (
        "autre",
        "postuler-cadre",
        "https://emplois.exemple-cadre.fr/postuler/7",
        "Cariste magasinier (H/F)",
        "Société Exemple Cadre",
        "Metz",
        "Réception et rangement des marchandises au chariot élévateur. CACES 3 demandé.",
    ),
    (
        "linkedin",
        "postuler-linkedin",
        "https://www.linkedin.com/jobs/view/4000000099/",
        "Préparateur de commandes et cariste (H/F)",
        "Logistique de l'Est Exemple",
        "Woippy",
        "Préparation de commandes, chargement des camions, inventaires. Postulez sur LinkedIn.",
    ),
];

const STYLE: &str = "<style>body{font-family:sans-serif;margin:0;background:#f4f4f4;color:#222}main{max-width:760px;\
margin:0 auto;padding:16px}form{display:flex;flex-direction:column;gap:10px;max-width:520px;background:#fff;padding:16px;\
border-radius:8px}label{display:flex;flex-direction:column;gap:4px}input,select,textarea{padding:7px}textarea{min-height:90px}\
.err{color:#b00}.case{flex-direction:row;align-items:center}</style>";

fn page(titre: &str, corps: &str) -> String {
    format!(
        "<!DOCTYPE html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>{titre}</title>{STYLE}</head><body>{corps}</body></html>"
    )
}

fn esc(t: &str) -> String {
    t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Une redirection (303) vers ce chemin du faux serveur.
fn vers(chemin: &str, entetes: Vec<(String, String)>) -> RepFaux {
    let mut e = vec![("Location".to_string(), chemin.to_string())];
    e.extend(entetes);
    RepFaux { entetes: e, ..RepFaux::html(303, "") }
}

/// Les champs envoyés par un formulaire (`multipart/form-data` ou `application/x-www-form-urlencoded`) : (nom, valeur
/// ou nom du fichier).
pub fn champs_envoyes(corps: &str) -> Vec<(String, String)> {
    let mut l = Vec::new();
    if corps.contains("Content-Disposition: form-data") {
        for part in corps.split("Content-Disposition: form-data").skip(1) {
            let entete = part.split("\r\n\r\n").next().unwrap_or_default();
            let valeur = part.split_once("\r\n\r\n").map(|x| x.1).unwrap_or_default();
            let valeur = valeur.rsplit_once("\r\n--").map_or(valeur, |x| x.0);
            let nom = entete.split("name=\"").nth(1).and_then(|x| x.split('"').next()).unwrap_or_default();
            if let Some(f) = entete.split("filename=\"").nth(1).and_then(|x| x.split('"').next()) {
                l.push((nom.to_string(), format!("fichier:{f}")));
            } else {
                l.push((nom.to_string(), valeur.to_string()));
            }
        }
    } else {
        for p in corps.split('&').filter(|p| !p.is_empty()) {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            l.push((crate::sites::exemples::decoder(k), crate::sites::exemples::decoder(v)));
        }
    }
    l
}

fn valeur<'a>(l: &'a [(String, String)], nom: &str) -> &'a str {
    l.iter().find(|(k, _)| k == nom).map_or("", |(_, v)| v.as_str())
}

fn cookie(r: &Requete, nom: &str) -> Option<String> {
    r.cookies.split(';').find_map(|c| c.trim().strip_prefix(&format!("{nom}=")).map(crate::sites::exemples::decoder))
}

// ---------------------------------------------------------------------------------------------------------------
// Les pages
// ---------------------------------------------------------------------------------------------------------------

fn offre(titre: &str, entreprise: &str, texte: &str, postuler: &str) -> String {
    page(
        &format!("{titre} - {entreprise}"),
        &format!(
            "<header><form role=\"search\" action=\"/recherche\"><input type=\"search\" name=\"q\" aria-label=\"Rechercher un emploi\"><button>Rechercher</button></form></header>\
<main><h1>{titre}</h1><p class=\"ent\">{entreprise}</p><p>{texte}</p>{postuler}</main>"
        ),
    )
}

/// Indeed, étape 1 : les coordonnées.
pub fn indeed_coordonnees() -> String {
    page(
        "Postuler - Coordonnées",
        "<main><p class=\"etape\">Étape 1 sur 3</p><h1>Ajoutez vos coordonnées</h1>\
<form method=\"post\" action=\"coordonnees\">\
<label for=\"p\">Prénom</label><input id=\"p\" name=\"firstName\" required autocomplete=\"given-name\">\
<label for=\"n\">Nom</label><input id=\"n\" name=\"lastName\" required autocomplete=\"family-name\">\
<label for=\"m\">Adresse e-mail</label><input id=\"m\" name=\"email\" type=\"email\" required>\
<label for=\"t\">Numéro de téléphone (facultatif)</label><input id=\"t\" name=\"phone\" type=\"tel\">\
<input type=\"hidden\" name=\"jk\" value=\"postuler-indeed\">\
<button type=\"submit\">Continuer</button></form></main>",
    )
}

/// Indeed, étape 2 : le CV et deux questions (avec l'erreur du site si le CV manque).
pub fn indeed_cv(erreur: bool) -> String {
    let e = if erreur { "<div role=\"alert\" class=\"err\">Ajoutez votre CV pour continuer.</div>" } else { "" };
    page(
        "Postuler - CV et questions",
        &format!(
            "<main><p class=\"etape\">Étape 2 sur 3</p><h1>Votre CV et quelques questions</h1>{e}\
<form method=\"post\" action=\"cv\" enctype=\"multipart/form-data\">\
<label>CV (PDF ou Word)<input type=\"file\" name=\"resume\" accept=\".pdf,.doc,.docx\" required></label>\
<fieldset><legend>Avez-vous le permis B ? *</legend>\
<label class=\"case\"><input type=\"radio\" name=\"q_permis\" value=\"1\" required> Oui</label>\
<label class=\"case\"><input type=\"radio\" name=\"q_permis\" value=\"0\"> Non</label></fieldset>\
<label for=\"dispo\">Quelle est votre disponibilité ? *</label><input id=\"dispo\" name=\"q_dispo\" required>\
<button type=\"button\" onclick=\"history.back()\">Retour</button><button type=\"submit\">Continuer</button></form></main>"
        ),
    )
}

/// Indeed, étape 3 : la vérification (avec l'erreur du site après un envoi refusé).
pub fn indeed_verifier(erreur: bool) -> String {
    let e = if erreur {
        "<div role=\"alert\" class=\"err\">Le service est momentanément indisponible. Réessayez dans quelques minutes.</div>"
    } else {
        ""
    };
    page(
        "Postuler - Vérification",
        &format!(
            "<main><p class=\"etape\">Étape 3 sur 3</p><h1>Vérifiez votre candidature</h1>{e}\
<form method=\"post\" action=\"verifier\">\
<label class=\"case\"><input type=\"checkbox\" name=\"accord\" value=\"oui\" required> J'accepte que mes données soient transmises à l'employeur</label>\
<label class=\"case\"><input type=\"checkbox\" name=\"alertes\" value=\"oui\"> Recevoir des offres similaires par mail</label>\
<button type=\"submit\">Déposer ma candidature</button></form></main>"
        ),
    )
}

fn confirmation(phrase: &str, cv: Option<&str>) -> String {
    page(
        "Candidature envoyée",
        &format!(
            "<main><h1>{phrase}</h1><p>Le recruteur reviendra vers vous.</p><p id=\"cv-recu\">CV reçu : {}</p></main>",
            esc(cv.unwrap_or("aucun"))
        ),
    )
}

/// Welcome to the Jungle : l'offre, et son formulaire montré par le bouton « Postuler ».
pub fn wttj_offre() -> String {
    let o = &OFFRES_ESSAI[1];
    offre(
        o.3,
        o.4,
        o.6,
        "<button type=\"button\" id=\"postuler\" onclick=\"document.getElementById('candidature').hidden=false;this.hidden=true;\">Postuler</button>\
<form id=\"candidature\" hidden method=\"post\" action=\"vendeur-postuler/envoi\" enctype=\"multipart/form-data\">\
<h2>Postuler chez Boulangerie des Trois Ponts</h2>\
<label>Prénom *<input name=\"firstname\" required></label>\
<label>Nom *<input name=\"lastname\" required></label>\
<label>Email *<input name=\"email\" type=\"email\" required></label>\
<label>Téléphone<input name=\"phone\" type=\"tel\"></label>\
<label>CV *<input type=\"file\" name=\"cv\" required></label>\
<label>Lettre de motivation<textarea name=\"cover_letter\"></textarea></label>\
<label>Lien vers votre profil LinkedIn<input name=\"linkedin\" type=\"url\"></label>\
<label>Quel est votre salaire souhaité ? *<input name=\"salaire\" required></label>\
<input name=\"website\" style=\"display:none\" tabindex=\"-1\">\
<label class=\"case\"><input type=\"checkbox\" name=\"rgpd\" required> J'accepte la politique de confidentialité *</label>\
<label class=\"case\"><input type=\"checkbox\" name=\"newsletter\"> Je veux recevoir la newsletter</label>\
<button type=\"submit\">Envoyer ma candidature</button></form>",
    )
}

/// France Travail : le formulaire, avec le captcha factice (et l'erreur du site s'il n'est pas fait).
pub fn ft_formulaire(erreur: Option<&str>) -> String {
    let e = erreur.map(|x| format!("<p role=\"alert\" class=\"err\">{x}</p>")).unwrap_or_default();
    page(
        "Postuler à l'offre - France Travail",
        &format!(
            "<header id=\"nav-connect\"><span class=\"compte\">Mon espace</span> <a href=\"/deconnexion\">Se déconnecter</a></header>\
<main><h1>Postuler à l'offre Agent de quai (H/F)</h1>{e}\
<form method=\"post\" action=\"postuler\" enctype=\"multipart/form-data\">\
<label for=\"civ\">Civilité *</label><select id=\"civ\" name=\"civilite\" required><option value=\"\">Choisir</option><option value=\"mme\">Madame</option><option value=\"m\">Monsieur</option></select>\
<label for=\"cour\">Courriel *</label><input id=\"cour\" name=\"courriel\" type=\"email\" required>\
<label for=\"tel\">Téléphone *</label><input id=\"tel\" name=\"telephone\" required>\
<label for=\"cvf\">Votre CV *</label><input id=\"cvf\" type=\"file\" name=\"fichierCv\" required>\
<label for=\"lm\">Lettre de motivation *</label><textarea id=\"lm\" name=\"lettre\" required></textarea>\
<div class=\"g-recaptcha\" data-sitekey=\"essai-invente\"><label class=\"case\"><input type=\"checkbox\" id=\"robot\" disabled> Je ne suis pas un robot</label></div>\
<textarea name=\"g-recaptcha-response\" style=\"display:none\"></textarea>\
<button type=\"submit\">Envoyer ma candidature</button></form>\
<script>(function(){{var fait=false;setInterval(function(){{if(fait||window.innerWidth>=1000)return;fait=true;setTimeout(function(){{\
document.getElementById('robot').checked=true;document.querySelector('[name=g-recaptcha-response]').value='essai-ok';}},4000);}},500);}})();</script></main>"
        ),
    )
}

/// Le site carrière inconnu.
pub fn inconnu_formulaire() -> String {
    page(
        "Rejoignez-nous - Transports de l'Exemple",
        "<header><nav><a href=\"/\">Accueil</a> <a href=\"/offres\">Nos offres</a></nav></header>\
<main><h1>Cariste (H/F)</h1><p>Conduite de chariot élévateur, CACES 3 et 5 demandés.</p>\
<form method=\"post\" action=\"42/depot\" enctype=\"multipart/form-data\" class=\"dossier\">\
<div class=\"ligne\"><div class=\"lib\">Votre patronyme <span>*</span></div><div><input name=\"f_1\" required></div></div>\
<div class=\"ligne\"><div class=\"lib\">Prénom usuel <span>*</span></div><div><input name=\"f_2\" required></div></div>\
<div class=\"ligne\"><div class=\"lib\">Adresse électronique <span>*</span></div><div><input name=\"f_3\" required></div></div>\
<div class=\"ligne\"><div class=\"lib\">Numéro de mobile</div><div><input name=\"f_4\"></div></div>\
<div class=\"ligne\"><div class=\"lib\">Joindre votre curriculum vitae <span>*</span></div><div><input type=\"file\" name=\"f_5\" required></div></div>\
<div class=\"ligne\"><div class=\"lib\">Comment avez-vous connu cette offre ? <span>*</span></div><div><select name=\"f_6\" required><option value=\"\">--</option><option>Site d'emploi</option><option>Bouche à oreille</option><option>Agence</option></select></div></div>\
<div class=\"ligne\"><div class=\"lib\">Message au recruteur</div><div><textarea name=\"f_7\"></textarea></div></div>\
<div><button type=\"reset\">Effacer</button> <input type=\"submit\" value=\"Valider\"></div></form></main>",
    )
}

/// Une page dont le formulaire est dans un cadre : Jobmind ne sait pas la lire.
pub fn cadre() -> String {
    page(
        "Postuler - Société Exemple Cadre",
        "<main><h1>Magasinier (H/F)</h1><p>Remplissez le formulaire ci-dessous.</p><iframe title=\"Formulaire\" src=\"about:blank\" width=\"600\" height=\"500\"></iframe></main>",
    )
}

// ---------------------------------------------------------------------------------------------------------------
// Le répondeur
// ---------------------------------------------------------------------------------------------------------------

/// Le mode des formulaires imités de l'essai (le faux serveur le lit à chaque envoi).
pub static MODE: Mutex<ModeFormulaires> = Mutex::new(ModeFormulaires::Normal);

/// La réponse du faux serveur à une page des formulaires imités, si c'en est une.
pub fn repondre(r: &Requete, mode: &Mutex<ModeFormulaires>) -> Option<RepFaux> {
    let reste = r.chemin.strip_prefix("/site/")?;
    let (chemin, q) = reste.split_once('?').unwrap_or((reste, ""));
    let post = r.methode == "POST";
    let envoyes = if post { champs_envoyes(&r.corps) } else { Vec::new() };
    let refus = || mode.lock().map(|mut m| std::mem::take(&mut *m) == ModeFormulaires::Refus).unwrap_or(false);
    let o = |n: usize| OFFRES_ESSAI[n];
    Some(match chemin {
        // Indeed
        "fr.indeed.com/viewjob" if q.contains("jk=postuler-indeed") => RepFaux::html(
            200,
            offre(
                o(0).3,
                o(0).4,
                o(0).6,
                "<a class=\"bouton\" href=\"https://smartapply.indeed.com/postuler/coordonnees?jk=postuler-indeed\">Postuler maintenant</a>",
            ),
        ),
        "smartapply.indeed.com/postuler/coordonnees" if post => {
            vers("/site/smartapply.indeed.com/postuler/cv", Vec::new())
        }
        "smartapply.indeed.com/postuler/coordonnees" => RepFaux::html(200, indeed_coordonnees()),
        "smartapply.indeed.com/postuler/cv" if post => {
            let cv = valeur(&envoyes, "resume");
            match cv.strip_prefix("fichier:").filter(|x| !x.is_empty()) {
                Some(nom) => vers(
                    "/site/smartapply.indeed.com/postuler/verifier",
                    vec![("Set-Cookie".into(), format!("essai_cv={}; Path=/", crate::sites::encoder(nom)))],
                ),
                None => RepFaux::html(200, indeed_cv(true)),
            }
        }
        "smartapply.indeed.com/postuler/cv" => RepFaux::html(200, indeed_cv(false)),
        "smartapply.indeed.com/postuler/verifier" if post => {
            if refus() || valeur(&envoyes, "accord") != "oui" {
                RepFaux::html(200, indeed_verifier(true))
            } else {
                vers("/site/smartapply.indeed.com/postuler/envoyee", Vec::new())
            }
        }
        "smartapply.indeed.com/postuler/verifier" => RepFaux::html(200, indeed_verifier(false)),
        "smartapply.indeed.com/postuler/envoyee" => RepFaux::html(
            200,
            confirmation(
                "Votre candidature a bien été envoyée à Entrepôts de la Seille",
                cookie(r, "essai_cv").as_deref(),
            ),
        ),
        // Welcome to the Jungle
        "www.welcometothejungle.com/fr/companies/essai-boulangerie/jobs/vendeur-postuler" => {
            RepFaux::html(200, wttj_offre())
        }
        "www.welcometothejungle.com/fr/companies/essai-boulangerie/jobs/vendeur-postuler/envoi" if post => {
            if refus() {
                RepFaux::html(
                    200,
                    page("Erreur", "<main><h1>Une erreur est survenue</h1><p>Réessayez plus tard.</p></main>"),
                )
            } else {
                let cv = valeur(&envoyes, "cv").strip_prefix("fichier:").map(str::to_string);
                RepFaux::html(200, confirmation("Merci ! Votre candidature a bien été envoyée.", cv.as_deref()))
            }
        }
        // France Travail
        "candidat.francetravail.fr/offres/recherche/detail/postuler-ft" => RepFaux::html(
            200,
            offre(
                o(2).3,
                o(2).4,
                o(2).6,
                "<a href=\"https://candidat.francetravail.fr/offres/recherche/detail/postuler-ft/postuler\">Postuler</a>",
            ),
        ),
        "candidat.francetravail.fr/offres/recherche/detail/postuler-ft/postuler" => {
            let dedans = r.cookies.split(';').any(|c| c.trim() == "essai_francetravail=1");
            if !dedans {
                vers("/site/authentification-candidat.francetravail.fr/connexion/identifiant", Vec::new())
            } else if post {
                if valeur(&envoyes, "g-recaptcha-response").is_empty() {
                    RepFaux::html(200, ft_formulaire(Some("Cochez la case « Je ne suis pas un robot ».")))
                } else if refus() {
                    RepFaux::html(200, ft_formulaire(Some("Votre candidature n'a pas pu être transmise. Réessayez.")))
                } else {
                    let cv = valeur(&envoyes, "fichierCv").strip_prefix("fichier:").map(str::to_string);
                    RepFaux::html(200, confirmation("Votre candidature a été transmise au recruteur.", cv.as_deref()))
                }
            } else {
                RepFaux::html(200, ft_formulaire(None))
            }
        }
        // Le site carrière inconnu
        "recrutement.exemple-transport.fr/offres/42" => RepFaux::html(200, inconnu_formulaire()),
        "recrutement.exemple-transport.fr/offres/42/depot" if post => {
            let cv = valeur(&envoyes, "f_5").strip_prefix("fichier:").map(str::to_string);
            RepFaux::html(200, confirmation("Nous avons bien reçu votre dossier.", cv.as_deref()))
        }
        "emplois.exemple-cadre.fr/postuler/7" => RepFaux::html(200, cadre()),
        // LinkedIn : l'offre, que Jordan regarde dans la petite fenêtre
        "www.linkedin.com/jobs/view/4000000099/" | "www.linkedin.com/jobs/view/4000000099" => RepFaux::html(
            200,
            page(
                "Préparateur de commandes et cariste (H/F) | LinkedIn",
                "<main><h1 class=\"top-card-layout__title\">Préparateur de commandes et cariste (H/F)</h1><p>Logistique de l'Est Exemple · Woippy</p>\
<p>Préparation de commandes, chargement des camions, inventaires.</p><button class=\"jobs-apply-button\">Candidature simplifiée</button></main>",
            ),
        ),
        _ => return None,
    })
}
