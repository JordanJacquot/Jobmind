//! Les mails dans la base (étape 7) : une alerte jamais relue, une réponse rangée et son statut proposé, un mail à
//! ranger puis rangé par Jordan ou mis de côté (« Rien à voir » efface son texte), la lecture par Claude d'un mail
//! flou, les repères de la relève, les offres venues d'une alerte, la bulle d'une réponse (une seule fois), la carte de
//! Mindset, et une candidature supprimée dont les mails reviennent à ranger. Tout sur une base en mémoire.

use jobmind_core::candidature::{Fiche, Moyen};
use jobmind_core::mails::Proposition;
use jobmind_core::{Jour, Statut};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::mails::{self, Etat, LuPar, NouveauMail, Rangement};
use jobmind_storage::{Base, Moment, bulles, candidatures, etat_carte_avec, offres};

fn m(j: u32, h: u32) -> Moment {
    Moment::nouveau(2026, 11, j, h, 0, 0).unwrap()
}

fn candidature(b: &Base, e: &str, p: &str) -> i64 {
    let f = Fiche {
        entreprise: e.into(),
        poste: p.into(),
        ville: "Metz".into(),
        envoyee_le: Jour::nouveau(2026, 11, 3).unwrap(),
        moyen: Moyen::Mail,
        source: String::new(),
        lien: String::new(),
        envoye: String::new(),
        cv: String::new(),
        notes: String::new(),
    };
    candidatures::ajouter(b.connexion(), &f, m(3, 9)).unwrap()
}

fn mail(id: &str, objet: &str, texte: &str) -> NouveauMail {
    NouveauMail {
        message_id: id.into(),
        uid: Some(7),
        de_adresse: "rh@exemple.fr".into(),
        de_nom: "Recrutement".into(),
        objet: objet.into(),
        recu_le: "2026-11-19T08:30:00".into(),
        texte: texte.into(),
        pieces_jointes: 1,
    }
}

#[test]
fn alerte_reponse_a_ranger_et_rien() {
    let (b, _) = Base::en_memoire(m(19, 8)).unwrap();
    let c = b.connexion();
    let k1 = candidature(&b, "Transports Moselle Fret", "Agent de quai");
    let k2 = candidature(&b, "Netéa Services", "Agent d'entretien");
    // Une alerte : sa trace seulement, jamais relue
    assert!(mails::garder_alerte(c, &mail("a1@linkedin.com", "5 offres", ""), 5, m(19, 8)).unwrap());
    assert!(!mails::garder_alerte(c, &mail("a1@linkedin.com", "5 offres", ""), 5, m(19, 8)).unwrap());
    assert!(mails::deja_vu(c, "a1@linkedin.com").unwrap());
    // Une réponse sûre, avec un statut proposé par les règles
    let r = Rangement {
        etat: Etat::Rangee,
        candidature: Some(k1),
        raison: "le nom de l'entreprise est dans l'objet".into(),
        proposition: Some((Proposition::Refus, "nous ne pouvons pas donner suite".into(), LuPar::Regles)),
        flou: false,
        attendre: true,
    };
    let id = mails::garder_reponse(
        c,
        &mail("r1@tmf.fr", "Votre candidature", "Bonjour, nous ne pouvons pas donner suite."),
        &r,
        m(19, 8),
    )
    .unwrap()
    .unwrap();
    assert_eq!(mails::garder_reponse(c, &mail("r1@tmf.fr", "x", "y"), &r, m(19, 8)).unwrap(), None);
    let l = mails::de_candidature(c, k1).unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!((l[0].id, l[0].attend, l[0].pieces_jointes), (id, true, 1));
    assert_eq!(l[0].recu_le, Moment::nouveau(2026, 11, 19, 8, 30, 0));
    assert_eq!(mails::nombre_a_voir(c).unwrap(), 1);
    assert_eq!(mails::candidatures_a_voir(c).unwrap(), [k1]);
    // Jordan confirme : n'attend plus ; une seconde fois ne change rien
    assert!(mails::decider(c, id, true).unwrap());
    assert!(!mails::decider(c, id, false).unwrap());
    assert!(!mails::lire(c, id).unwrap().unwrap().attend);
    // Un mail à ranger, flou, avec une suggestion
    let r = Rangement {
        etat: Etat::ARanger,
        candidature: Some(k2),
        raison: "Peut-être : le mail cite le poste.".into(),
        proposition: None,
        flou: true,
        attendre: true,
    };
    let x = mails::garder_reponse(c, &mail("r2@gmail.com", "Votre profil", "Je vous rappelle demain."), &r, m(19, 9))
        .unwrap()
        .unwrap();
    let ar = mails::a_ranger(c).unwrap();
    assert_eq!((ar.len(), ar[0].suggestion, ar[0].candidature), (1, Some(k2), None));
    assert_eq!(mails::nombre_a_voir(c).unwrap(), 1);
    // Claude le lit (une seule fois)
    assert_eq!(mails::floues(c, 8).unwrap().len(), 1);
    mails::garder_ia(c, x, Some((Proposition::Entretien, "Je vous rappelle demain"))).unwrap();
    assert!(mails::floues(c, 8).unwrap().is_empty());
    let lu = mails::lire(c, x).unwrap().unwrap();
    assert_eq!(lu.proposition, Some((Proposition::Entretien, "Je vous rappelle demain".into(), LuPar::Ia)));
    // Rangé par Jordan dans le dossier de Netéa
    assert!(!mails::ranger(c, x, 999).unwrap());
    assert!(mails::ranger(c, x, k2).unwrap());
    assert_eq!(mails::de_candidature(c, k2).unwrap().len(), 1);
    assert!(mails::a_ranger(c).unwrap().is_empty());
    // « Rien à voir » : texte, objet et expéditeur effacés, la trace reste (jamais relu)
    let y = mails::garder_reponse(
        c,
        &mail("r3@x.fr", "Secret", "Un texte à ne pas garder."),
        &Rangement { proposition: None, flou: false, ..r },
        m(19, 10),
    )
    .unwrap()
    .unwrap();
    assert!(mails::rien_a_voir(c, y).unwrap());
    let (objet, texte, de): (Option<String>, Option<String>, Option<String>) = c
        .query_row("SELECT objet, texte, expediteur FROM mails WHERE id = ?1", [y], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!((objet, texte, de), (None, None, None));
    assert!(mails::deja_vu(c, "r3@x.fr").unwrap());
    assert!(mails::lire(c, y).unwrap().is_some_and(|x| x.etat == Etat::Rien));
    // Une candidature supprimée : ses mails reviennent à ranger
    assert!(candidatures::supprimer(c, k2).unwrap());
    let ar = mails::a_ranger(c).unwrap();
    assert_eq!((ar.len(), ar[0].id), (1, x));
    // La carte de Mindset : une ligne en tête, 3 lignes au plus
    let (l, _) = candidatures::lister(c).unwrap();
    let carte = etat_carte_avec(&l, Jour::nouveau(2026, 11, 19).unwrap(), 14, mails::nombre_a_voir(c).unwrap());
    assert_eq!(carte.lignes[0].texte, "1 réponse reçue à voir");
    assert!(carte.lignes.len() <= 3);
    let vide = etat_carte_avec(&[], Jour::nouveau(2026, 11, 19).unwrap(), 14, 2);
    assert_eq!(vide.lignes.iter().map(|x| x.texte.as_str()).collect::<Vec<_>>(), ["2 réponses reçues à voir"]);
    let _ = Statut::Negative;
}

#[test]
fn reperes_offres_venues_et_bulle() {
    let (b, _) = Base::en_memoire(m(19, 8)).unwrap();
    let c = b.connexion();
    assert_eq!(mails::reperes(c).unwrap(), mails::Reperes::default());
    mails::noter_releve(c, 42, 1007, "3 offres ajoutées", m(19, 8)).unwrap();
    let r = mails::reperes(c).unwrap();
    assert_eq!((r.uidvalidity, r.dernier_uid, r.bilan.as_str()), (Some(42), Some(1007), "3 offres ajoutées"));
    mails::noter_echec(c, "Gmail ne répond pas", m(19, 9)).unwrap();
    assert_eq!(mails::reperes(c).unwrap().dernier_uid, Some(1007));
    mails::oublier_reperes(c).unwrap();
    assert_eq!(mails::reperes(c).unwrap(), mails::Reperes::default());
    // Des offres venues d'une alerte LinkedIn
    let o = OffreTrouvee {
        id_source: "4012345678".into(),
        titre: "Cariste H/F".into(),
        entreprise: "Transports Moselle Fret".into(),
        lieu: "Metz".into(),
        url: "https://www.linkedin.com/jobs/view/4012345678/".into(),
        ..OffreTrouvee::default()
    };
    offres::ajouter_trouvees(c, "linkedin", &[o], None, m(19, 8)).unwrap();
    assert_eq!(mails::marquer_venues_par_mail(c, "linkedin", &["4012345678".into()], m(19, 8)).unwrap(), 1);
    // (une seconde alerte ne change pas le jour)
    mails::marquer_venues_par_mail(c, "linkedin", &["4012345678".into()], m(20, 8)).unwrap();
    let id = offres::lister(c).unwrap()[0].id;
    assert_eq!(offres::lire(c, id).unwrap().unwrap().venue_par_mail, Jour::nouveau(2026, 11, 19));
    // La bulle d'une réponse : une seule fois par mail
    let k = candidature(&b, "Netéa Services", "Agent d'entretien");
    let b1 = bulles::retenir_reponse(c, 5, k, "Réponse de Netéa Services", "Agent d'entretien", m(19, 8)).unwrap();
    assert!(b1.is_some_and(|x| x.candidature == Some(k) && bulles::id_bulle_valable(&x.id)));
    assert!(bulles::retenir_reponse(c, 5, k, "Réponse de Netéa Services", "x", m(19, 9)).unwrap().is_none());
}
