//! La lecture et le tri dans la base (étape 6) : la migration 6 sur une base de l'étape 5, la lecture par les règles
//! (relue quand le texte change), la note refaite quand « Ce que je cherche » change, « Pas hors sujet », et ce que
//! rend Claude (une seule fois par offre, jamais par-dessus les règles).

use jobmind_core::lecture::{Confiance, Diplome, Experience, LuPar};
use jobmind_core::pertinence::{self, Profil};
use jobmind_core::recherche::{SaisieRecherche, verifier_recherche};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::lecture::{self, ResultatIa};
use jobmind_storage::offres::{self, Marque};
use jobmind_storage::{Base, Moment};

fn m(h: u32) -> Moment {
    Moment::nouveau(2026, 11, 19, h, 0, 0).unwrap()
}

fn trouvee(id: &str, titre: &str, entreprise: &str, description: &str) -> OffreTrouvee {
    OffreTrouvee {
        id_source: id.into(),
        titre: titre.into(),
        entreprise: entreprise.into(),
        lieu: "Metz (57)".into(),
        zones: Vec::new(),
        url: String::new(),
        description: description.into(),
        contrat: String::new(),
        salaire: String::new(),
        publiee_le: String::new(),
        latitude: None,
        longitude: None,
        brut: "{}".into(),
    }
}

const LONG: &str = "Au sein de notre entrepôt de Metz, vous préparez les commandes de nos clients, vous chargez les \
                    camions et vous tenez votre zone propre. Horaires en journée, du lundi au vendredi, travail en \
                    équipe de cinq personnes avec un chef d'équipe présent.";

#[test]
fn migration_6_sur_une_base_de_l_etape_5() {
    let mut b = Base::en_memoire_jusqua(5, m(9)).unwrap();
    offres::ajouter_trouvees(
        b.connexion(),
        "francetravail",
        &[trouvee("1", "Cariste", "Logi", "Sans diplôme.")],
        None,
        m(9),
    )
    .unwrap();
    assert_eq!(b.migrer_tout(m(10)).unwrap(), vec![6, 7, 8, 9, 10, 11, 12]);
    // L'offre d'avant est là, pas encore lue
    let l = offres::lister(b.connexion()).unwrap();
    assert_eq!((l.len(), l[0].diplome, l[0].pertinence), (1, None, None));
    assert_eq!(lecture::relire(b.connexion()).unwrap(), 1);
    assert_eq!(offres::lister(b.connexion()).unwrap()[0].diplome, Some(Diplome::Aucun));
    // Les tables et colonnes nouvelles
    let n: i64 = b.connexion().query_row("SELECT COUNT(*) FROM lectures_ia", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn lecture_par_les_regles_et_relecture() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    offres::ajouter_trouvees(
        c,
        "francetravail",
        &[
            trouvee("1", "Agent de quai H/F", "Logi", "Débutants acceptés, formation assurée. Permis B."),
            trouvee("2", "Comptable", "Cabinet", "BTS comptabilité exigé, 3 ans d'expérience."),
            trouvee("3", "Agent de tri", "Poste", LONG),
        ],
        None,
        m(9),
    )
    .unwrap();
    assert_eq!(lecture::relire(c).unwrap(), 3);
    // Une seconde fois : rien à relire
    assert_eq!(lecture::relire(c).unwrap(), 0);
    let l = offres::lister(c).unwrap();
    let quai = l.iter().find(|o| o.titre.starts_with("Agent de quai")).unwrap();
    assert_eq!((quai.diplome, quai.experience), (Some(Diplome::Aucun), Some(Experience::Debutant)));
    let x = offres::lire(c, quai.id).unwrap().unwrap().lecture;
    let d = x.diplome.unwrap();
    assert_eq!((d.par, d.confiance), (LuPar::Regles, Confiance::Moyenne));
    assert!(d.preuve.contains("formation assurée"));
    assert_eq!(x.autres, "Permis B");
    let compta = l.iter().find(|o| o.titre == "Comptable").unwrap();
    assert_eq!((compta.diplome, compta.experience), (Some(Diplome::Bac2), Some(Experience::Duree(36))));
    // Les offres floues : seulement celle dont le texte est assez long et rien trouvé
    let floues = lecture::a_lire_par_ia(c, 10).unwrap();
    assert_eq!(floues.iter().map(|f| f.1.as_str()).collect::<Vec<_>>(), ["Agent de tri"]);
    assert_eq!(lecture::nombre_floues(c).unwrap(), 1);
    // Le même site la redonne avec un texte plus long (« Ajouter à Jobmind ») : relue
    let longue = trouvee("3", "Agent de tri", "Poste", &format!("{LONG} Aucune expérience exigée."));
    offres::ajouter_page(c, "francetravail", &longue, None, m(11)).unwrap();
    assert_eq!(lecture::relire(c).unwrap(), 1);
    let tri = offres::lister(c).unwrap().into_iter().find(|o| o.titre == "Agent de tri").unwrap();
    assert_eq!(tri.experience, Some(Experience::Debutant));
}

#[test]
fn la_note_suit_ce_que_je_cherche() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    offres::ajouter_trouvees(
        c,
        "francetravail",
        &[trouvee("1", "Cariste", "Logi", "Sans diplôme, débutant accepté."), trouvee("2", "Boulanger", "Fournil", "")],
        None,
        m(9),
    )
    .unwrap();
    lecture::relire(c).unwrap();
    let saisie = |postes: &str| SaisieRecherche {
        postes: postes.into(),
        lieu: "partout".into(),
        teletravail: "peu_importe".into(),
        ..SaisieRecherche::default()
    };
    let r = verifier_recherche(&saisie("cariste")).unwrap();
    let noter = |r: &jobmind_core::recherche::Recherche| {
        let p = Profil::nouveau(r, "");
        let sig = pertinence::signature(r, "");
        lecture::renoter(c, &sig, |o| {
            pertinence::noter(
                &p,
                &pertinence::OffreANoter {
                    titre: &o.titre,
                    description: &o.description,
                    lieu: &o.lieu,
                    diplome: o.diplome,
                    experience: o.experience,
                    ..Default::default()
                },
            )
        })
        .unwrap()
    };
    assert_eq!(noter(&r), 2);
    assert_eq!(noter(&r), 0);
    let l = offres::lister(c).unwrap();
    let cariste = l.iter().find(|o| o.titre == "Cariste").unwrap();
    let boulanger = l.iter().find(|o| o.titre == "Boulanger").unwrap();
    assert!(cariste.pertinence.unwrap() > 70 && !cariste.hors_sujet);
    assert!(cariste.raison.starts_with("Aucun diplôme demandé"), "{}", cariste.raison);
    assert!(boulanger.hors_sujet && boulanger.rangee_hors_sujet());
    // « Pas hors sujet », puis un autre « Ce que je cherche » : tout est renoté
    assert_eq!(offres::basculer(c, boulanger.id, Marque::PasHorsSujet).unwrap(), Some(true));
    assert!(!offres::lister(c).unwrap().iter().find(|o| o.titre == "Boulanger").unwrap().rangee_hors_sujet());
    let r2 = verifier_recherche(&saisie("boulanger")).unwrap();
    assert_eq!(noter(&r2), 2);
    let l = offres::lister(c).unwrap();
    assert!(l.iter().find(|o| o.titre == "Cariste").unwrap().hors_sujet);
    assert!(!l.iter().find(|o| o.titre == "Boulanger").unwrap().hors_sujet);
    // Sans « Ce que je cherche » : plus de note
    lecture::effacer_notes(c).unwrap();
    assert!(offres::lister(c).unwrap().iter().all(|o| o.pertinence.is_none() && !o.hors_sujet));
}

#[test]
fn ce_que_rend_claude() {
    let (b, _) = Base::en_memoire(m(9)).unwrap();
    let c = b.connexion();
    offres::ajouter_trouvees(c, "francetravail", &[trouvee("1", "Agent de tri", "Poste", LONG)], None, m(9)).unwrap();
    lecture::relire(c).unwrap();
    let (id, ..) = lecture::a_lire_par_ia(c, 10).unwrap()[0].clone();
    let r = ResultatIa {
        offre_id: id,
        modele: "claude-haiku-5-5".into(),
        etat: "ok",
        jetons_entree: 900,
        jetons_sortie: 60,
        cout_millioniemes: 120,
        reponse: "{}".into(),
        diplome: None,
        experience: Some((Experience::Souhaitee, "travail en équipe de cinq personnes".into())),
    };
    lecture::garder_ia(c, &r, m(10)).unwrap();
    let x = offres::lire(c, id).unwrap().unwrap().lecture;
    let e = x.experience.unwrap();
    assert_eq!((e.valeur, e.par), (Experience::Souhaitee, LuPar::Ia));
    assert!(x.par_ia && x.diplome.is_none());
    // Plus jamais redonnée à Claude ; comptée dans le jour
    assert!(lecture::a_lire_par_ia(c, 10).unwrap().is_empty());
    assert_eq!(lecture::ia_du_jour(c, m(10).jour()).unwrap(), (1, 120));
    assert_eq!(lecture::ia_du_jour(c, Moment::nouveau(2026, 11, 20, 9, 0, 0).unwrap().jour()).unwrap(), (0, 0));
    // Les règles relues (une nouvelle version) ne trouvent rien : ce que Claude a trouvé reste
    c.execute("UPDATE offres SET lecture_version = NULL", []).unwrap();
    lecture::relire(c).unwrap();
    assert_eq!(offres::lire(c, id).unwrap().unwrap().lecture.experience.unwrap().par, LuPar::Ia);
}
