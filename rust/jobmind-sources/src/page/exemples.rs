//! Les pages d'exemple des sites (étape 5), toutes inventées : les tests de la lecture d'une page, et la construction
//! d'essai de l'appli (option « faux », jamais livrée), où le faux serveur local (127.0.0.1) les sert au panneau des
//! sites. Aucun vrai site n'est jamais ouvert par un essai.
//!
//! Le faux serveur imite chaque site sous `/site/<hôte>/<chemin>` : l'essai ouvre par exemple
//! `http://127.0.0.1:<port>/site/www.linkedin.com/jobs/view/4012345678/` à la place de
//! `https://www.linkedin.com/jobs/view/4012345678/`. Sous `/essai/`, les pages des essais du panneau : la session gardée
//! (un cookie posé, puis relu), une page qui essaie d'appeler Jobmind (elle doit échouer), un nouvel onglet, un
//! téléchargement.

use crate::faux::RepFaux;

pub const LINKEDIN_CONNECTE: &str = include_str!("exemples/linkedin-connecte.html");
pub const LINKEDIN_PUBLIC: &str = include_str!("exemples/linkedin-public.html");
pub const INDEED: &str = include_str!("exemples/indeed.html");
pub const WTTJ: &str = include_str!("exemples/wttj.html");
pub const FRANCE_TRAVAIL: &str = include_str!("exemples/francetravail.html");
pub const AUTRE_OFFRE: &str = include_str!("exemples/autre-offre.html");
pub const PAS_UNE_OFFRE: &str = include_str!("exemples/pas-une-offre.html");
pub const RECETTE: &str = include_str!("exemples/recette.html");
pub const PIEGE: &str = include_str!("exemples/piege.html");
pub const LISTE: &str = include_str!("exemples/liste.html");

/// Les adresses des pages d'exemple (comme sur les vrais sites), et leur page.
pub const PAGES: [(&str, &str); 10] = [
    ("https://www.linkedin.com/jobs/collections/recommended/?currentJobId=4012345678", LINKEDIN_CONNECTE),
    (
        "https://fr.linkedin.com/jobs/view/preparateur-de-commandes-h-f-at-logistique-des-trois-frontieres-4019876543?trk=public_jobs",
        LINKEDIN_PUBLIC,
    ),
    ("https://fr.indeed.com/viewjob?jk=a1b2c3d4e5f60718&from=serp&vjs=3", INDEED),
    ("https://www.welcometothejungle.com/fr/companies/atelier-moselle-bois/jobs/menuisier-poseur-h-f_metz", WTTJ),
    ("https://candidat.francetravail.fr/offres/recherche/detail/201XKLM", FRANCE_TRAVAIL),
    ("https://emplois.ville-exemple.fr/offres/agent-polyvalent-espaces-verts", AUTRE_OFFRE),
    ("https://www.linkedin.com/feed/", PAS_UNE_OFFRE),
    ("https://www.recettes-exemple.fr/quiche-lorraine", RECETTE),
    ("https://piege.exemple.fr/offre", PIEGE),
    ("https://offres.exemple.fr/liste/cariste-metz", LISTE),
];

/// L'accueil d'un site imité (les raccourcis du panneau) : des liens vers les pages d'exemple.
const ACCUEIL: &str = "<!DOCTYPE html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>Sites d'exemple</title>\
<style>body{font-family:sans-serif;margin:32px;color:#222}a{display:block;margin:10px 0;font-size:18px}</style></head>\
<body><h1>Sites d'exemple (essai de Jobmind)</h1><p>Toutes ces pages sont inventées.</p>\
<a href=\"/site/www.linkedin.com/jobs/collections/recommended/?currentJobId=4012345678\">Cariste H/F, LinkedIn (connecté)</a>\
<a href=\"/site/fr.linkedin.com/jobs/view/preparateur-de-commandes-h-f-at-logistique-des-trois-frontieres-4019876543?trk=public_jobs\">Préparateur de commandes, LinkedIn (public)</a>\
<a href=\"/site/fr.indeed.com/viewjob?jk=a1b2c3d4e5f60718&amp;from=serp&amp;vjs=3\">Agent d'entretien, Indeed</a>\
<a href=\"/site/www.welcometothejungle.com/fr/companies/atelier-moselle-bois/jobs/menuisier-poseur-h-f_metz\">Menuisier poseur, Welcome to the Jungle</a>\
<a href=\"/site/candidat.francetravail.fr/offres/recherche/detail/201XKLM\">Vendeur en magasin, France Travail</a>\
<a href=\"/site/emplois.ville-exemple.fr/offres/agent-polyvalent-espaces-verts\">Agent des espaces verts, autre site</a>\
<a href=\"/site/www.linkedin.com/feed/\">Le fil d'actualité (pas une offre)</a>\
<a href=\"/essai/nouvel-onglet\">Un lien qui ouvre un nouvel onglet</a>\
</body></html>";

/// L'essai de la session : un cookie posé pour un jour.
const SESSION_POSER: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>session posee</title></head>\
<body><h1>Session posée</h1><p>Un cookie d'essai vient d'être posé.</p></body></html>";

/// L'essai de la session : le cookie relu (le titre de la page le dit).
const SESSION_VOIR: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>session ?</title></head>\
<body><h1 id=\"r\">Session ?</h1><script>var ok = document.cookie.indexOf('jobmind_essai=ok') >= 0;\
document.title = 'session ' + (ok ? 'gardee' : 'vide'); document.getElementById('r').textContent = document.title;\
</script></body></html>";

/// L'essai de sécurité : une page du panneau qui essaie d'appeler Jobmind par tous les chemins connus. Tout doit
/// échouer ; le titre de la page dit ce qui s'est passé (« ipc internals=... invoke=refuse ... »).
const PIEGE_IPC: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>ipc en cours</title></head>\
<body><h1>Essai de sécurité</h1><p id=\"r\">en cours</p><script>\
(async function () {\
  var r = [];\
  var I = window.__TAURI_INTERNALS__;\
  r.push('internals=' + (I ? 'present' : 'absent'));\
  r.push('global=' + (window.__TAURI__ ? 'present' : 'absent'));\
  if (I && typeof I.invoke === 'function') {\
    try { await I.invoke('action', { message: { type: 'action', id: 'o-favori', valeur: { id: 1 }, ecran: 'accueil' } }); r.push('invoke=accepte'); }\
    catch (e) { r.push('invoke=refuse'); }\
    try { await I.invoke('noter_visibilite', { visible: false }); r.push('visibilite=accepte'); }\
    catch (e) { r.push('visibilite=refuse'); }\
  } else { r.push('invoke=absent'); }\
  try {\
    var x = await fetch('http://ipc.localhost/action', { method: 'POST', headers: { 'Content-Type': 'application/json',\
      'Tauri-Callback': '1', 'Tauri-Error': '2', 'Tauri-Invoke-Key': String(window.__TAURI_INVOKE_KEY__ || '') },\
      body: JSON.stringify({ message: { type: 'menu' } }) });\
    r.push('fetch=' + x.status);\
  } catch (e) { r.push('fetch=echec'); }\
  try { var p = await fetch('http://tauri.localhost/index.html'); r.push('page=' + p.status); } catch (e) { r.push('page=echec'); }\
  try {\
    if (window.chrome && window.chrome.webview) { window.chrome.webview.postMessage('{\"cmd\":\"action\"}'); r.push('post=envoye'); }\
    else { r.push('post=absent'); }\
  } catch (e) { r.push('post=echec'); }\
  document.getElementById('r').textContent = r.join(' ');\
  document.title = 'ipc ' + r.join(' ');\
})();\
</script></body></html>";

/// L'essai des nouveaux onglets : un lien `target="_blank"` et un `window.open`, qui doivent rester dans le panneau.
const NOUVEL_ONGLET: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>nouvel onglet</title></head>\
<body><h1>Nouvel onglet</h1>\
<a id=\"lien\" target=\"_blank\" href=\"/site/www.welcometothejungle.com/fr/companies/atelier-moselle-bois/jobs/menuisier-poseur-h-f_metz\">Ouvrir l'offre dans un nouvel onglet</a>\
<script>setTimeout(function () { window.open('/site/www.welcometothejungle.com/fr/companies/atelier-moselle-bois/jobs/menuisier-poseur-h-f_metz', '_blank'); }, 400);</script>\
</body></html>";

/// L'essai des téléchargements : un fichier demandé tout de suite (il doit être refusé).
const TELECHARGEMENT: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>telechargement</title></head>\
<body><h1>Téléchargement</h1><a id=\"f\" href=\"/essai/fichier.txt\" download>Le fichier</a>\
<script>setTimeout(function () { document.getElementById('f').click(); }, 300);</script></body></html>";

/// La page d'exemple de cette adresse (« www.linkedin.com/jobs/... », sans « https:// »).
pub fn page(hote_chemin: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|(u, _)| u.trim_start_matches("https://").split('?').next() == hote_chemin.split('?').next())
        .map(|(_, p)| *p)
}

/// La réponse du faux serveur à un chemin `/site/...` ou `/essai/...`.
pub fn repondre(chemin: &str) -> RepFaux {
    if let Some(reste) = chemin.strip_prefix("/site/") {
        return RepFaux::html(200, page(reste).unwrap_or(ACCUEIL));
    }
    match chemin.split('?').next().unwrap_or_default() {
        "/essai/session-poser" => RepFaux {
            entetes: vec![("Set-Cookie".into(), "jobmind_essai=ok; Max-Age=86400; Path=/".into())],
            ..RepFaux::html(200, SESSION_POSER)
        },
        "/essai/session-voir" => RepFaux::html(200, SESSION_VOIR),
        "/essai/piege-ipc" => RepFaux::html(200, PIEGE_IPC),
        "/essai/nouvel-onglet" => RepFaux::html(200, NOUVEL_ONGLET),
        "/essai/telechargement" => RepFaux::html(200, TELECHARGEMENT),
        "/essai/fichier.txt" => RepFaux {
            type_contenu: "application/octet-stream".into(),
            entetes: vec![("Content-Disposition".into(), "attachment; filename=\"essai.txt\"".into())],
            ..RepFaux::html(200, "un fichier d'essai")
        },
        _ => RepFaux::html(404, ACCUEIL),
    }
}
