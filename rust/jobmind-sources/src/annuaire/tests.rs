//! Les tests de l'annuaire, sur des réponses inventées (aucun appel réseau).

use super::*;

fn demande(nom: &str, lieu: &str) -> Demande {
    Demande::depuis_offre(nom, lieu).unwrap_or_else(|| panic!("pas de demande pour {nom} / {lieu}"))
}

/// Un résultat de l'annuaire, inventé, de la forme de la documentation publique.
fn resultat(siren: &str, nom: &str, commune: &str, cp: &str, tranche: &str, section: &str) -> Value {
    serde_json::json!({
        "siren": siren,
        "nom_complet": nom,
        "nom_raison_sociale": nom,
        "sigle": null,
        "siege": {
            "adresse": format!("12 RUE DES LILAS {cp} {}", commune.to_uppercase()),
            "code_postal": cp,
            "libelle_commune": commune.to_uppercase(),
            "commune": "57463",
            "departement": &cp[..2],
        },
        "activite_principale": "49.41A",
        "section_activite_principale": section,
        "tranche_effectif_salarie": tranche,
        "annee_tranche_effectif_salarie": "2023",
        "etat_administratif": "A",
        "matching_etablissements": [],
        "complements": { "est_ess": false },
    })
}

fn reponse(r: Vec<Value>) -> String {
    serde_json::json!({ "results": r, "total_results": 1, "page": 1, "per_page": 5, "total_pages": 1 }).to_string()
}

#[test]
fn la_demande_d_une_offre() {
    // (Metz a trois codes postaux : le département ; Sarrebourg n'en a qu'un : le code postal)
    let d = demande("Transports Exemple", "Metz (57)");
    assert_eq!(d.code_postal, None);
    assert_eq!(d.departement, "57");
    assert_eq!(d.commune, "metz");
    assert_eq!(
        d.adresse(ADRESSE),
        "https://recherche-entreprises.api.gouv.fr/search?q=Transports%20Exemple&departement=57&etat_administratif=A&per_page=5"
    );
    let d = demande("Hôtel du Parc & Spa", "57 - SARREBOURG");
    assert_eq!(d.code_postal.as_deref(), Some("57400"));
    assert_eq!(d.commune, "sarrebourg");
    assert!(d.adresse(ADRESSE).contains("q=H%C3%B4tel%20du%20Parc%20%26%20Spa&code_postal=57400"));
    assert_eq!(demande("Dupont SAS", "Metz, Grand Est, France").departement, "57");
    // (un département seul)
    let d = demande("Dupont", "Moselle (57)");
    assert_eq!((d.departement.as_str(), d.commune.as_str(), d.code_postal.clone()), ("57", "", None));
    // (la même entreprise, le même lieu : la même clé)
    assert_eq!(demande("TRANSPORTS EXEMPLE SAS", "Metz (57)").cle(), demande("Transports Exemple", "Metz").cle());
    // Rien sans nom, sans lieu, ou avec un nom qui n'en est pas un
    assert_eq!(Demande::depuis_offre("", "Metz (57)"), None);
    assert_eq!(Demande::depuis_offre("Transports Exemple", ""), None);
    assert_eq!(Demande::depuis_offre("Transports Exemple", "Télétravail"), None);
    assert_eq!(Demande::depuis_offre("Entreprise non précisée", "Metz (57)"), None);
    assert_eq!(Demande::depuis_offre("Confidentiel", "Metz (57)"), None);
}

#[test]
fn un_resultat_qui_correspond() {
    let d = demande("Transports Exemple", "Metz (57)");
    let r = reponse(vec![resultat("123456789", "TRANSPORTS EXEMPLE", "Metz", "57070", "21", "H")]);
    let f = lire_reponse(&d, &r).unwrap().expect("trouvée");
    assert_eq!(f.nom, "TRANSPORTS EXEMPLE");
    assert_eq!(f.secteur, "Transports et entreposage");
    assert_eq!(f.taille, "50 à 99 salariés (chiffre de 2023)");
    assert_eq!(f.adresse, "12 RUE DES LILAS 57070 METZ");
    assert_eq!(f.commune, "METZ");
    // (la forme juridique et un nom plus long qui contient le nom cherché)
    let r = reponse(vec![resultat("123456789", "TRANSPORTS EXEMPLE LORRAINE SARL", "Ennery", "57365", "12", "H")]);
    let f = lire_reponse(&demande("Transports Exemple SAS", "Metz (57)"), &r).unwrap().expect("même département");
    assert_eq!(f.taille, "20 à 49 salariés (chiffre de 2023)");
}

#[test]
fn un_homonyme_ailleurs_est_ecarte() {
    let d = demande("Transports Exemple", "Metz (57)");
    // (la même entreprise de nom, mais à Lyon : rien)
    let r = reponse(vec![resultat("987654321", "TRANSPORTS EXEMPLE", "Lyon", "69003", "21", "H")]);
    assert_eq!(lire_reponse(&d, &r).unwrap(), None);
    // (l'homonyme de Lyon en premier, celle de Moselle ensuite : celle de Moselle)
    let r = reponse(vec![
        resultat("987654321", "TRANSPORTS EXEMPLE", "Lyon", "69003", "21", "H"),
        resultat("123456789", "TRANSPORTS EXEMPLE", "Metz", "57000", "03", "H"),
    ]);
    assert_eq!(lire_reponse(&d, &r).unwrap().unwrap().taille, "6 à 9 salariés (chiffre de 2023)");
    // (deux entreprises différentes du même nom dans le même département : pas de certitude, rien)
    let r = reponse(vec![
        resultat("111111111", "TRANSPORTS EXEMPLE", "Metz", "57000", "21", "H"),
        resultat("222222222", "TRANSPORTS EXEMPLE", "Thionville", "57100", "03", "H"),
    ]);
    assert_eq!(lire_reponse(&d, &r).unwrap(), None);
    // (le même nom exact l'emporte sur un nom qui le contient)
    let r = reponse(vec![
        resultat("111111111", "TRANSPORTS EXEMPLE ET FILS", "Metz", "57000", "21", "H"),
        resultat("222222222", "TRANSPORTS EXEMPLE", "Metz", "57000", "03", "H"),
    ]);
    assert_eq!(lire_reponse(&d, &r).unwrap().unwrap().nom, "TRANSPORTS EXEMPLE");
    // (un autre nom : rien ; un nom trop court ne compte pas comme contenu)
    let r = reponse(vec![resultat("333333333", "BOULANGERIE MARTIN", "Metz", "57000", "01", "C")]);
    assert_eq!(lire_reponse(&d, &r).unwrap(), None);
    let r = reponse(vec![resultat("333333333", "ABC LOGISTIQUE", "Metz", "57000", "01", "H")]);
    assert_eq!(lire_reponse(&demande("ABC", "Metz (57)"), &r).unwrap(), None);
    // (une entreprise fermée : rien)
    let mut ferme = resultat("123456789", "TRANSPORTS EXEMPLE", "Metz", "57000", "21", "H");
    ferme["etat_administratif"] = serde_json::json!("C");
    assert_eq!(lire_reponse(&d, &reponse(vec![ferme])).unwrap(), None);
}

#[test]
fn les_champs_manquants() {
    let d = demande("Transports Exemple", "Metz (57)");
    // (seulement le nom et la commune du siège : le reste reste vide, rien n'est inventé)
    let r = serde_json::json!({ "results": [{
        "siren": "123456789",
        "nom_complet": "TRANSPORTS EXEMPLE",
        "siege": { "libelle_commune": "METZ" },
        "tranche_effectif_salarie": null,
    }] })
    .to_string();
    let f = lire_reponse(&d, &r).unwrap().unwrap();
    assert_eq!(f, Fiche { nom: "TRANSPORTS EXEMPLE".into(), commune: "METZ".into(), ..Fiche::default() });
    // (« NN » : pas de salarié déclaré, pas de taille ; une section inconnue : pas de secteur)
    let mut x = resultat("123456789", "TRANSPORTS EXEMPLE", "Metz", "57000", "NN", "Z");
    x["annee_tranche_effectif_salarie"] = Value::Null;
    let f = lire_reponse(&d, &reponse(vec![x])).unwrap().unwrap();
    assert_eq!((f.taille.as_str(), f.secteur.as_str()), ("", ""));
    // Aucun résultat : rien ; pas la réponse de l'annuaire : illisible
    assert_eq!(lire_reponse(&d, &reponse(Vec::new())).unwrap(), None);
    assert_eq!(lire_reponse(&d, "<html>erreur</html>"), Err(ErreurAnnuaire::Illisible));
    assert_eq!(lire_reponse(&d, "{\"erreur\": 1}"), Err(ErreurAnnuaire::Illisible));
}

#[test]
fn les_tranches_d_effectif() {
    assert_eq!(tranche_effectif("00"), Some("Aucun salarié"));
    assert_eq!(tranche_effectif("01"), Some("1 ou 2 salariés"));
    assert_eq!(tranche_effectif("21"), Some("50 à 99 salariés"));
    assert_eq!(tranche_effectif("42"), Some("1 000 à 1 999 salariés"));
    assert_eq!(tranche_effectif("53"), Some("10 000 salariés et plus"));
    assert_eq!(tranche_effectif("NN"), None);
    assert_eq!(tranche_effectif(""), None);
    for c in ["00", "01", "02", "03", "11", "12", "21", "22", "31", "32", "41", "42", "51", "52", "53"] {
        assert!(!tranche_effectif(c).unwrap().contains(['\u{2013}', '\u{2014}']));
    }
}

#[test]
fn la_recherche_web_bien_encodee() {
    assert_eq!(
        adresse_recherche_web("Transports Exemple", "Metz"),
        "https://duckduckgo.com/?q=Transports%20Exemple%20Metz"
    );
    assert_eq!(
        adresse_recherche_web("Hôtel L'Étoile & Cie", "Saint-Avold"),
        "https://duckduckgo.com/?q=H%C3%B4tel%20L%27%C3%89toile%20%26%20Cie%20Saint-Avold"
    );
    // (rien qui casse l'adresse : ni « # », ni « ? », ni retour à la ligne)
    let u = adresse_recherche_web("A#B?C=D\nE", "");
    assert_eq!(u, "https://duckduckgo.com/?q=A%23B%3FC%3DD%20E");
    assert_eq!(encoder("abc-_.~09"), "abc-_.~09");
}
