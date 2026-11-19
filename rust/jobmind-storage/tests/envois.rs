//! L'envoi par mail dans la base (étape 10, migration 10) : la migration sur une base de l'étape 9, l'envoi noté avant
//! de partir puis fini (parti : le contenu exact et la candidature du suivi, en une transaction ; pas parti ;
//! incertain puis confirmé ou non par Jordan), le Message-ID donné à la relève, le rythme, la reprise au démarrage.

use jobmind_core::candidature::{Fiche, Moyen};
use jobmind_core::{Jour, Statut};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::envois::{self, EtatTente, Tentative};
use jobmind_storage::{Base, Moment, candidatures, mails, offres};

fn m(h: u32, mi: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, mi, 0).unwrap()
}

fn trouvee(id: &str, titre: &str) -> OffreTrouvee {
    OffreTrouvee {
        id_source: id.into(),
        titre: titre.into(),
        entreprise: "Logi Est".into(),
        lieu: "Metz (57)".into(),
        zones: Vec::new(),
        url: String::new(),
        description: "Préparation de commandes. Envoyez CV et lettre à rh@logi-est-exemple.fr.".into(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    }
}

fn tentative(offre: i64, id: &str) -> Tentative {
    Tentative {
        offre_id: offre,
        preparation_id: None,
        cv_id: None,
        expediteur: "jordan.essai@gmail.com".into(),
        destinataire: "rh@logi-est-exemple.fr".into(),
        objet: "Candidature au poste de cariste".into(),
        contenu: "Madame, Monsieur,\n\nJe vous propose ma candidature.\n\nJordan".into(),
        piece_jointe: Some("CV Jordan.pdf".into()),
        piece_taille: Some(182_000),
        message_id: format!("{id}.jobmind@gmail.com"),
    }
}

fn fiche() -> Fiche {
    Fiche {
        entreprise: "Logi Est".into(),
        poste: "Cariste".into(),
        ville: "Metz".into(),
        envoyee_le: Jour::nouveau(2026, 11, 19).unwrap(),
        moyen: Moyen::Mail,
        source: "Adzuna".into(),
        lien: String::new(),
        envoye: "Madame, Monsieur,\n\nJe vous propose ma candidature.\n\nJordan".into(),
        cv: "CV Jordan.pdf".into(),
        notes: String::new(),
    }
}

fn base_avec_offre() -> (Base, i64) {
    let (b, _) = Base::en_memoire(m(9, 0)).unwrap();
    offres::ajouter_trouvees(b.connexion(), "adzuna", &[trouvee("1", "Cariste")], None, m(9, 0)).unwrap();
    let id = offres::lister(b.connexion()).unwrap()[0].id;
    (b, id)
}

#[test]
fn migration_10_sur_une_base_de_l_etape_9() {
    let mut b = Base::en_memoire_jusqua(9, m(9, 0)).unwrap();
    offres::ajouter_trouvees(b.connexion(), "adzuna", &[trouvee("1", "Cariste")], None, m(9, 0)).unwrap();
    assert_eq!(b.migrer_tout(m(10, 0)).unwrap(), vec![10]);
    let id = offres::lister(b.connexion()).unwrap()[0].id;
    assert_eq!(envois::rythme(b.connexion(), m(10, 0).jour()).unwrap(), (None, 0));
    assert!(envois::incertain(b.connexion(), id).unwrap().is_none());
}

/// Le chemin normal : noté avant, parti, la candidature entre dans le suivi avec le contenu exact, et le Message-ID
/// est donné à la relève.
#[test]
fn parti_entre_dans_le_suivi() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    let t = envois::commencer(c, &tentative(offre, "a1"), m(10, 0)).unwrap();
    assert_eq!(envois::lire_tente(c, t).unwrap().unwrap().etat, EtatTente::EnCours);
    let k = envois::partir(c, t, &fiche(), false, m(10, 0)).unwrap().expect("candidature");
    // (une seconde fin ne fait rien : pas deux entrées)
    assert_eq!(envois::partir(c, t, &fiche(), false, m(10, 1)).unwrap(), None);
    let x = candidatures::lire(c, k).unwrap().unwrap();
    assert_eq!((x.statut, x.moyen, x.cv.as_str()), (Statut::Envoyee, Moyen::Mail, "CV Jordan.pdf"));
    assert_eq!(x.envoye, fiche().envoye);
    assert_eq!(x.envoyee_le, Jour::nouveau(2026, 11, 19).unwrap());
    assert_eq!(offres::lire(c, offre).unwrap().unwrap().offre.candidature, Some(k));
    let l = envois::du_dossier(c, k).unwrap();
    assert_eq!(l.len(), 1);
    let e = &l[0];
    assert_eq!(e.destinataire.as_deref(), Some("rh@logi-est-exemple.fr"));
    assert_eq!(e.objet.as_deref(), Some("Candidature au poste de cariste"));
    assert_eq!(e.contenu, fiche().envoye);
    assert_eq!((e.piece_jointe.as_deref(), e.piece_taille), (Some("CV Jordan.pdf"), Some(182_000)));
    assert_eq!(e.message_id.as_deref(), Some("a1.jobmind@gmail.com"));
    assert_eq!(e.confirme_par.as_deref(), Some("gmail"));
    assert_eq!(e.envoye_le, m(10, 0));
    let r = mails::envoyes(c).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!((r[0].candidature, r[0].message_id.as_str()), (k, "a1.jobmind@gmail.com"));
    assert_eq!(envois::envoyee(c, offre).unwrap(), Some((k, m(10, 0))));
    assert_eq!(candidatures::evenements(c, k).unwrap().len(), 1);
}

/// Une offre déjà dans le suivi (« J'ai postulé », puis relancée) : la même candidature, remise à « Envoyée ».
#[test]
fn candidature_existante_mise_a_jour() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    let mut f = fiche();
    f.moyen = Moyen::Site;
    f.envoye = String::new();
    let k = candidatures::ajouter(c, &f, m(9, 0)).unwrap();
    offres::lier_candidature(c, offre, k).unwrap();
    candidatures::changer_statut(c, k, Statut::Relancee, None, m(9, 30)).unwrap();
    let t = envois::commencer(c, &tentative(offre, "b2"), m(10, 0)).unwrap();
    assert_eq!(envois::partir(c, t, &fiche(), false, m(10, 0)).unwrap(), Some(k));
    let x = candidatures::lire(c, k).unwrap().unwrap();
    assert_eq!((x.statut, x.moyen), (Statut::Envoyee, Moyen::Mail));
    assert_eq!(x.envoye, fiche().envoye);
    assert_eq!(candidatures::lister(c).unwrap().0.len(), 1);
    let fr = candidatures::evenements(c, k).unwrap();
    assert_eq!(fr.last().map(|e| e.statut), Some(Statut::Envoyee));
}

/// Pas parti : rien dans le suivi ; incertain : rien non plus tant que Jordan ne dit pas ce qu'il a vu.
#[test]
fn echec_et_incertain() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    let t = envois::commencer(c, &tentative(offre, "c3"), m(10, 0)).unwrap();
    assert!(envois::echec(c, t, "destinataire", m(10, 0)).unwrap());
    assert_eq!(envois::partir(c, t, &fiche(), false, m(10, 0)).unwrap(), None);
    assert!(candidatures::lister(c).unwrap().0.is_empty());
    assert!(mails::envoyes(c).unwrap().is_empty());
    // Incertain, puis « Il n'est pas parti »
    let t2 = envois::commencer(c, &tentative(offre, "c4"), m(10, 5)).unwrap();
    assert!(envois::rendre_incertain(c, t2, "coupé", m(10, 5)).unwrap());
    assert_eq!(envois::incertain(c, offre).unwrap().map(|x| x.id), Some(t2));
    assert!(candidatures::lister(c).unwrap().0.is_empty());
    assert!(envois::pas_parti(c, t2, m(10, 8)).unwrap());
    assert!(envois::incertain(c, offre).unwrap().is_none());
    // Incertain, puis « Il est bien parti » : dans le suivi, confirmé par Jordan
    let t3 = envois::commencer(c, &tentative(offre, "c5"), m(10, 10)).unwrap();
    envois::rendre_incertain(c, t3, "coupé", m(10, 10)).unwrap();
    assert_eq!(envois::partir(c, t3, &fiche(), false, m(10, 11)).unwrap(), None, "pas sans Jordan");
    let k = envois::partir(c, t3, &fiche(), true, m(10, 12)).unwrap().expect("confirmé");
    assert_eq!(envois::du_dossier(c, k).unwrap()[0].confirme_par.as_deref(), Some("jordan"));
    assert_eq!(envois::lire_tente(c, t3).unwrap().unwrap().etat, EtatTente::Confirme);
    assert_eq!(mails::envoyes(c).unwrap()[0].message_id, "c5.jobmind@gmail.com");
}

#[test]
fn rythme_et_reprise() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    let jour = m(10, 0).jour();
    let t = envois::commencer(c, &tentative(offre, "d1"), m(10, 0)).unwrap();
    assert_eq!(envois::rythme(c, jour).unwrap(), (Some(m(10, 0)), 1));
    envois::echec(c, t, "réseau", m(10, 0)).unwrap();
    // (un échec ne compte ni dans le plafond du jour, ni dans l'écart : Jordan peut réessayer tout de suite)
    assert_eq!(envois::rythme(c, jour).unwrap(), (None, 0));
    let t2 = envois::commencer(c, &tentative(offre, "d2"), m(11, 0)).unwrap();
    // Jobmind s'arrête pendant l'envoi : au démarrage, incertain
    assert_eq!(envois::reprendre(c).unwrap(), 1);
    let x = envois::lire_tente(c, t2).unwrap().unwrap();
    assert_eq!(x.etat, EtatTente::Incertain);
    assert!(x.erreur.unwrap().contains("arrêtée"));
    assert_eq!(envois::rythme(c, jour).unwrap(), (Some(m(11, 0)), 1));
}

#[test]
fn adresse_tapee_gardee() {
    let (b, offre) = base_avec_offre();
    let c = b.connexion();
    assert_eq!(envois::adresse_gardee(c, offre).unwrap(), None);
    assert!(envois::garder_adresse(c, offre, "rh@logi-est-exemple.fr").unwrap());
    assert_eq!(envois::adresse_gardee(c, offre).unwrap().as_deref(), Some("rh@logi-est-exemple.fr"));
}
