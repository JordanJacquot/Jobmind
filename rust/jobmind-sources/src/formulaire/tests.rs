use super::exemples::*;
use super::*;

fn genres(f: &Formulaire) -> Vec<(&'static str, bool)> {
    f.champs.iter().map(|c| (c.genre.code(), c.obligatoire)).collect()
}

#[test]
fn indeed_trois_etapes() {
    let f = lire("https://smartapply.indeed.com/postuler/coordonnees", &indeed_coordonnees());
    assert_eq!(genres(&f), vec![("prenom", true), ("nom", true), ("mail", true), ("telephone", false)], "{}", f.trace);
    assert_eq!(f.champs[0].libelle, "Prénom");
    assert_eq!(f.etape, Some((1, 3)));
    let b = f.bouton.as_ref().unwrap();
    assert_eq!((b.texte.as_str(), b.action), ("Continuer", Action::Suivant));
    // (le champ caché « jk » n'est pas un champ, mais il compte dans les rangs de la page)
    assert_eq!(f.champs.iter().map(|c| c.i).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    assert_eq!(b.i, 5);

    let f = lire("https://smartapply.indeed.com/postuler/cv", &indeed_cv(false));
    assert_eq!(genres(&f), vec![("cv", true), ("question", true), ("question", true)], "{}", f.trace);
    assert_eq!(f.champs[1].libelle, "Avez-vous le permis B ?");
    assert_eq!(f.champs[1].sorte, Sorte::Ronds);
    assert_eq!(f.champs[1].choix.iter().map(|c| c.texte.as_str()).collect::<Vec<_>>(), vec!["Oui", "Non"]);
    assert_eq!(f.champs[2].libelle, "Quelle est votre disponibilité ?");
    assert_eq!(f.bouton.as_ref().unwrap().texte, "Continuer");
    assert!(lire("x", &indeed_cv(true)).erreurs.iter().any(|e| e.contains("Ajoutez votre CV")));

    let f = lire("https://smartapply.indeed.com/postuler/verifier", &indeed_verifier(false));
    assert_eq!(genres(&f), vec![("consentement", true), ("autre", false)], "{}", f.trace);
    assert_eq!(f.bouton.as_ref().unwrap().action, Action::Envoyer);
    assert_eq!(f.etape, Some((3, 3)));
}

#[test]
fn wttj_bouton_postuler_puis_formulaire() {
    let html = wttj_offre();
    let f = lire("https://www.welcometothejungle.com/fr/companies/x/jobs/y", &html);
    // (le formulaire est caché tant que « Postuler » n'est pas cliqué)
    assert!(!f.present(), "{}", f.trace);
    match f.postuler {
        Some(Postuler::Bouton { ref texte, .. }) => assert_eq!(texte, "Postuler"),
        ref x => panic!("{x:?}"),
    }
    let ouvert = html.replace("<form id=\"candidature\" hidden", "<form id=\"candidature\"");
    let f = lire("https://www.welcometothejungle.com/fr/companies/x/jobs/y", &ouvert);
    assert_eq!(
        genres(&f),
        vec![
            ("prenom", true),
            ("nom", true),
            ("mail", true),
            ("telephone", false),
            ("cv", true),
            ("lettre", false),
            ("question", false),
            ("question", true),
            ("consentement", true),
            ("autre", false)
        ],
        "{}",
        f.trace
    );
    assert_eq!(f.bouton.as_ref().unwrap().texte, "Envoyer ma candidature");
    assert!(!f.captcha);
}

#[test]
fn france_travail_captcha_et_civilite() {
    let f = lire("https://candidat.francetravail.fr/x/postuler", &ft_formulaire(None));
    assert_eq!(
        genres(&f),
        vec![("question", true), ("mail", true), ("telephone", true), ("cv", true), ("lettre", true)],
        "{}",
        f.trace
    );
    assert_eq!(f.champs[0].choix.len(), 3);
    assert!(f.captcha && !f.captcha_fait);
    // (la réponse du captcha, relue par le script de lecture : fait)
    let fait = ft_formulaire(None).replace(
        "<textarea name=\"g-recaptcha-response\"",
        "<textarea data-jm-valeur=\"essai-ok\" name=\"g-recaptcha-response\"",
    );
    assert!(lire("x", &fait).captcha_fait);
}

#[test]
fn site_inconnu() {
    let f = lire("https://recrutement.exemple-transport.fr/offres/42", &inconnu_formulaire());
    assert_eq!(
        genres(&f),
        vec![
            ("nom", true),
            ("prenom", true),
            ("mail", true),
            ("telephone", false),
            ("cv", true),
            ("question", true),
            ("lettre", false)
        ],
        "{}",
        f.trace
    );
    assert_eq!(f.champs[0].libelle, "Votre patronyme");
    let b = f.bouton.as_ref().unwrap();
    assert_eq!((b.texte.as_str(), b.action, b.balise.as_str()), ("Valider", Action::Envoyer, "input"));
}

#[test]
fn pages_sans_formulaire() {
    let f = lire("x", &cadre());
    assert!(!f.present() && f.confirmation.is_none() && f.postuler.is_none());
    let f = lire("x", "<html><body><main><h1>Votre candidature a bien été envoyée à Exemple</h1></main></body></html>");
    assert_eq!(f.confirmation.as_deref(), Some("Votre candidature a bien été envoyée à Exemple"));
    let f = lire("x", "<html><body><h1>Une erreur est survenue</h1></body></html>");
    assert!(f.erreur_site.is_some());
    let f = lire(
        "https://fr.indeed.com/viewjob?jk=1",
        "<html><body><a href=\"/postuler?jk=1\">Postuler maintenant</a><a href=\"/aide\">Comment postuler</a></body></html>",
    );
    assert_eq!(f.postuler, Some(Postuler::Lien("https://fr.indeed.com/postuler?jk=1".into())));
    let f = lire(
        "x",
        "<form><label>Adresse e-mail<input type=email name=e></label><label>Mot de passe<input type=password name=p></label><button>Se connecter</button></form>",
    );
    assert!(!f.present() && f.connexion);
}

#[test]
fn marques_du_script_de_lecture() {
    // (le rang, la valeur relue et la case cachée posés par le script de lecture de l'appli)
    let html = "<form><label>Prénom<input name=a data-jm-i=\"7\" data-jm-cache=\"0\" data-jm-valeur=\"Jordan\"></label>\
<label>Nom<input name=b data-jm-i=\"8\" data-jm-valeur=\"Exemple\"></label>\
<input name=piege data-jm-i=\"9\" data-jm-cache=\"1\">\
<label>CV<input type=file name=c data-jm-i=\"10\" data-jm-cache=\"1\" data-jm-valeur=\"CV.pdf\"></label>\
<button data-jm-i=\"11\">Envoyer</button></form>";
    let f = lire("x", html);
    assert_eq!(
        f.champs.iter().map(|c| (c.i, c.valeur.as_str())).collect::<Vec<_>>(),
        vec![(7, "Jordan"), (8, "Exemple"), (10, "CV.pdf")]
    );
    assert_eq!(f.bouton.unwrap().i, 11);
}

#[test]
fn page_abimee_ne_casse_rien() {
    for h in ["", "<form", "<form><input", "<<<>>>", &"<div>".repeat(5000), "<select><option>a<option>b</select>"] {
        let _ = lire("x", h);
    }
}

#[test]
fn envois_lus_par_le_faux_serveur() {
    let corps = "------x\r\nContent-Disposition: form-data; name=\"prenom\"\r\n\r\nJordan\r\n------x\r\nContent-Disposition: form-data; name=\"cv\"; filename=\"CV 2026.pdf\"\r\nContent-Type: application/pdf\r\n\r\n%PDF\r\n------x--\r\n";
    let l = champs_envoyes(corps);
    assert_eq!(l, vec![("prenom".into(), "Jordan".into()), ("cv".into(), "fichier:CV 2026.pdf".into())]);
    assert_eq!(champs_envoyes("a=1&b=d%C3%A9j%C3%A0"), vec![("a".into(), "1".into()), ("b".into(), "déjà".into())]);
}

// ---------------------------------------------------------------------------------------------------------------
// La mesure sur des formulaires jamais vus (écrits à part, sans voir les règles : tests\formulaires_jamais_vus)
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn mesure_formulaires_jamais_vus() {
    mesurer("formulaires_jamais_vus");
    mesurer("formulaires_jamais_vus_2");
}

fn mesurer(nom_dossier: &str) {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(nom_dossier);
    let Ok(attendu) = std::fs::read_to_string(dossier.join("attendu.json")) else { return };
    let attendu: serde_json::Value = serde_json::from_str(&attendu).expect("attendu.json lisible");
    let (mut champs, mut genre_ok, mut oblig_ok, mut trouves, mut en_trop) = (0, 0, 0, 0, 0);
    let (mut pages, mut pages_ok, mut boutons, mut boutons_ok) = (0, 0, 0, 0);
    let mut fautes: Vec<String> = Vec::new();
    let mut noms: Vec<&String> = attendu.as_object().unwrap().keys().collect();
    noms.sort();
    for nom in noms {
        let a = &attendu[nom.as_str()];
        let html = std::fs::read_to_string(dossier.join(nom)).unwrap();
        let f = lire("https://emploi.exemple.fr/postuler/1", &html);
        match a["genre"].as_str().unwrap_or_default() {
            "formulaire" => {
                pages += 1;
                let mut page_ok = f.present();
                let lc: Vec<String> = f.champs.iter().map(|c| plat(&c.libelle)).collect();
                let mut vus = vec![false; f.champs.len()];
                for x in a["champs"].as_array().unwrap() {
                    champs += 1;
                    let lib = plat(x["libelle"].as_str().unwrap());
                    let k = lc.iter().enumerate().position(|(k, l)| {
                        !vus[k] && (l.contains(lib.trim()) || (!l.trim().is_empty() && lib.contains(l.trim())))
                    });
                    match k {
                        Some(k) => {
                            vus[k] = true;
                            trouves += 1;
                            let c = &f.champs[k];
                            if c.genre.code() == x["genre"].as_str().unwrap() {
                                genre_ok += 1;
                            } else {
                                page_ok = false;
                                fautes.push(format!(
                                    "{nom} « {} » : {} au lieu de {}",
                                    c.libelle,
                                    c.genre.code(),
                                    x["genre"]
                                ));
                            }
                            if c.obligatoire == x["obligatoire"].as_bool().unwrap_or(false) {
                                oblig_ok += 1;
                            } else {
                                fautes.push(format!(
                                    "{nom} « {} » : obligatoire {} au lieu de {}",
                                    c.libelle, c.obligatoire, x["obligatoire"]
                                ));
                            }
                        }
                        None => {
                            page_ok = false;
                            fautes.push(format!("{nom} : champ « {} » pas trouvé", x["libelle"]));
                        }
                    }
                }
                for (k, v) in vus.iter().enumerate() {
                    if !v {
                        en_trop += 1;
                        fautes.push(format!(
                            "{nom} : champ en trop « {} » ({})",
                            f.champs[k].libelle,
                            f.champs[k].genre.code()
                        ));
                    }
                }
                boutons += 1;
                let bt = a["bouton_texte"].as_str().unwrap_or_default();
                let action = if a["bouton"] == "suivant" { Action::Suivant } else { Action::Envoyer };
                match &f.bouton {
                    Some(b) if plat(&b.texte) == plat(bt) && b.action == action => boutons_ok += 1,
                    b => fautes.push(format!(
                        "{nom} : bouton {:?} au lieu de « {bt} » ({action:?})",
                        b.as_ref().map(|x| (&x.texte, x.action))
                    )),
                }
                if f.captcha != a["captcha"].as_bool().unwrap_or(false) {
                    fautes.push(format!("{nom} : captcha {} au lieu de {}", f.captcha, a["captcha"]));
                }
                if f.erreurs.is_empty() == a["erreurs"].as_bool().unwrap_or(false) {
                    fautes.push(format!("{nom} : erreurs {:?} (attendu {})", f.erreurs, a["erreurs"]));
                }
                if page_ok {
                    pages_ok += 1;
                }
            }
            g => {
                pages += 1;
                let ok = match g {
                    "confirmation" => f.confirmation.is_some() && !f.present(),
                    "offre_postuler" => f.postuler.is_some() && !f.present(),
                    "connexion" => f.connexion && !f.present(),
                    "erreur_site" => f.erreur_site.is_some() && !f.present(),
                    _ => false,
                };
                if ok {
                    pages_ok += 1;
                } else {
                    fautes.push(format!("{nom} : {g} pas reconnue ({})", f.trace));
                }
            }
        }
    }
    println!(
        "MESURE {nom_dossier} : {trouves}/{champs} champs trouvés, {genre_ok}/{champs} genres justes, \
         {oblig_ok}/{champs} obligatoires justes, {en_trop} champs en trop, {boutons_ok}/{boutons} boutons justes, \
         {pages_ok}/{pages} pages entièrement justes"
    );
    for x in &fautes {
        println!("  {x}");
    }
}

/// Audit du 26/11/2026 : le site confirme la candidature et montre sur la même page un autre formulaire (créer un
/// compte, recevoir des offres). La confirmation est lue à côté du formulaire, jamais dedans.
#[test]
fn confirmation_a_cote_d_un_formulaire() {
    let compte = "<form id=\"compte\"><h2>Créez votre espace candidat</h2>\
        <label>Prénom <input name=\"prenom\"></label><label>Nom <input name=\"nom\"></label>\
        <label>Adresse e-mail <input type=\"email\" name=\"mail\"></label>\
        <button type=\"submit\">Envoyer</button></form>";
    // La confirmation et un formulaire qui ressemble à une candidature (prénom, nom, mail)
    let page = format!(
        "<html><body><main><div class=\"bandeau\"><p>Votre candidature a bien été envoyée à Exemple Transports.</p>\
         </div>{compte}</main></body></html>"
    );
    let f = lire("https://recrutement.exemple.invalid/merci", &page);
    assert!(f.present(), "{}", f.trace);
    assert_eq!(f.confirmation, None, "avec un formulaire, l'ancien champ reste vide");
    assert_eq!(
        f.confirmation_a_cote.as_deref(),
        Some("Votre candidature a bien été envoyée à Exemple Transports."),
        "{}",
        f.trace
    );
    assert_eq!(f.confirmee(), Some("Votre candidature a bien été envoyée à Exemple Transports."));
    assert!(f.trace.contains("confirmation à côté du formulaire"), "{}", f.trace);
    // Les cases hors de toute balise <form> : la confirmation est avant la première case
    let sans_form = "<html><body><h1>Merci d'avoir postulé !</h1><div><label>Prénom <input name=\"prenom\"></label>\
        <label>Nom <input name=\"nom\"></label><label>Adresse e-mail <input type=\"email\" name=\"mail\"></label>\
        <button>Envoyer</button></div></body></html>";
    let f = lire("x", sans_form);
    assert!(f.present(), "{}", f.trace);
    assert_eq!(f.confirmee(), Some("Merci d'avoir postulé !"), "{}", f.trace);
    // Une erreur du site sur la même page : un refus, pas une confirmation
    let refus = page.replace("<main>", "<main><p role=\"alert\">Le CV est obligatoire.</p>");
    let f = lire("x", &refus);
    assert!(f.confirmation_a_cote.is_some() && !f.erreurs.is_empty(), "{}", f.trace);
    assert_eq!(f.confirmee(), None);

    // Contre-épreuves. Une phrase DANS le formulaire explique la suite : ce n'est pas une confirmation
    let dedans = "<html><body><form><p>Merci de votre candidature : remplissez ces cases.</p>\
        <p>Votre candidature a bien été préparée, il reste à l'envoyer.</p>\
        <label>Prénom <input name=\"prenom\"></label><label>Nom <input name=\"nom\"></label>\
        <label>Adresse e-mail <input type=\"email\" name=\"mail\"></label>\
        <button type=\"submit\">Envoyer ma candidature</button></form></body></html>";
    let f = lire("x", dedans);
    assert!(f.present(), "{}", f.trace);
    assert_eq!((f.confirmation.as_deref(), f.confirmation_a_cote.as_deref(), f.confirmee()), (None, None, None));
    // Une phrase à côté du formulaire qui annonce seulement la suite : pas une confirmation non plus
    let annonce = "<html><body><p>Une fois votre candidature envoyée, vous recevrez un accusé de réception.</p>\
        <form><label>Prénom <input name=\"prenom\"></label><label>Nom <input name=\"nom\"></label>\
        <label>Adresse e-mail <input type=\"email\" name=\"mail\"></label>\
        <button type=\"submit\">Envoyer ma candidature</button></form></body></html>";
    let f = lire("x", annonce);
    assert!(f.present(), "{}", f.trace);
    assert_eq!((f.confirmation_a_cote.as_deref(), f.confirmee()), (None, None), "{}", f.trace);
    // Les formulaires de candidature des exemples n'ont aucune confirmation à côté
    for html in
        [indeed_coordonnees(), indeed_cv(false), indeed_verifier(false), ft_formulaire(None), inconnu_formulaire()]
    {
        let f = lire("x", &html);
        assert!(f.present() && f.confirmee().is_none(), "{}", f.trace);
    }
    // Sans formulaire de candidature (une simple alerte mail à côté), rien ne change : l'ancien champ
    let alerte = "<html><body><h1>Votre candidature a bien été envoyée à Exemple</h1>\
        <form><label>Recevoir les offres par mail <input type=\"email\" name=\"alerte\"></label>\
        <button>Créer mon alerte</button></form></body></html>";
    let f = lire("x", alerte);
    assert!(!f.present(), "{}", f.trace);
    assert_eq!(f.confirmation.as_deref(), Some("Votre candidature a bien été envoyée à Exemple"));
    assert_eq!(f.confirmation_a_cote, None);
    assert_eq!(f.confirmee(), Some("Votre candidature a bien été envoyée à Exemple"));
}
