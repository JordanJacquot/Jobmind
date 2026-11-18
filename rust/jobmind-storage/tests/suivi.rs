//! Le suivi des candidatures dans la base (étape 2) : ajouter, modifier, supprimer, changer le statut (la frise),
//! les notes, le délai de relance ; puis les bulles près de l'horloge : une seule fois par événement, une à la fois,
//! 6 s d'écart, abandonnée après 5 minutes, et le clic sur une bulle (afficher.demande). Tout sur une base en mémoire
//! ou un dossier d'essai sous %TEMP% (jamais les vraies données).

use std::path::PathBuf;

use jobmind_core::candidature::{Entretien, Fiche, Moyen};
use jobmind_core::suivi::bulles_dues;
use jobmind_core::{Heure, Jour, Statut};
use jobmind_storage::bulles::{self, Demande, Envoi};
use jobmind_storage::candidatures::{self, Change};
use jobmind_storage::{Base, Journal, Moment, horloge_figee};

struct Essai(PathBuf);

impl Essai {
    fn nouveau(nom: &str) -> Essai {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Essai(d)
    }
}

impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn m(j: u32, h: u32, mi: u32, s: u32) -> Moment {
    Moment::nouveau(2026, 11, j, h, mi, s).unwrap()
}

fn jour(a: i32, mo: u32, j: u32) -> Jour {
    Jour::nouveau(a, mo, j).unwrap()
}

fn fiche(entreprise: &str, envoyee: Jour) -> Fiche {
    Fiche {
        entreprise: entreprise.into(),
        poste: "Agent de quai".into(),
        ville: "Woippy".into(),
        envoyee_le: envoyee,
        moyen: Moyen::Mail,
        source: "Indeed".into(),
        lien: "https://exemple.fr/offre/1".into(),
        envoye: "Madame, Monsieur,\n\nJe vous adresse ma candidature.".into(),
        cv: "CV 2026.pdf".into(),
        notes: String::new(),
    }
}

fn base() -> Base {
    Base::en_memoire(m(18, 10, 0, 0)).unwrap().0
}

#[test]
fn ajouter_modifier_supprimer() {
    let b = base();
    let c = b.connexion();
    let id = candidatures::ajouter(c, &fiche("Transports Moselle Fret", jour(2026, 11, 2)), m(18, 10, 0, 0)).unwrap();
    let x = candidatures::lire(c, id).unwrap().unwrap();
    assert_eq!(x.statut, Statut::Envoyee);
    assert_eq!(x.statut_le, jour(2026, 11, 2));
    assert_eq!((x.moyen, x.source.as_str(), x.cv.as_str()), (Moyen::Mail, "Indeed", "CV 2026.pdf"));
    assert_eq!(x.envoye, "Madame, Monsieur,\n\nJe vous adresse ma candidature.");
    // Le premier point de la frise : l'envoi, à son jour
    let f = candidatures::evenements(c, id).unwrap();
    assert_eq!(f.len(), 1);
    assert_eq!((f[0].jour, f[0].statut), (jour(2026, 11, 2), Statut::Envoyee));
    // Modifiée : le jour d'envoi déplace la frise et le jour du statut
    let mut g = fiche("Transports Moselle Fret SA", jour(2026, 11, 3));
    g.notes = "Contact : M. Durand".into();
    assert!(candidatures::modifier(c, id, &g, m(18, 11, 0, 0)).unwrap());
    let x = candidatures::lire(c, id).unwrap().unwrap();
    assert_eq!(
        (x.entreprise.as_str(), x.statut_le, x.notes.as_str()),
        ("Transports Moselle Fret SA", jour(2026, 11, 3), "Contact : M. Durand")
    );
    assert_eq!(candidatures::evenements(c, id).unwrap()[0].jour, jour(2026, 11, 3));
    assert!(!candidatures::modifier(c, 999, &g, m(18, 11, 0, 0)).unwrap());
    // Les notes seules
    assert!(candidatures::noter(c, id, "Rappeler jeudi", m(18, 12, 0, 0)).unwrap());
    assert_eq!(candidatures::lire(c, id).unwrap().unwrap().notes, "Rappeler jeudi");
    // Supprimée, avec sa frise
    let autre = candidatures::ajouter(c, &fiche("Brico Lorraine", jour(2026, 11, 10)), m(18, 10, 0, 0)).unwrap();
    assert!(candidatures::supprimer(c, id).unwrap());
    assert!(!candidatures::supprimer(c, id).unwrap());
    assert!(candidatures::lire(c, id).unwrap().is_none());
    let n: i64 = c.query_row("SELECT COUNT(*) FROM evenements WHERE candidature_id = ?1", [id], |r| r.get(0)).unwrap();
    assert_eq!(n, 0);
    let (l, illisibles) = candidatures::lister(c).unwrap();
    assert_eq!((l.len(), illisibles, l[0].id), (1, 0, autre));
}

#[test]
fn statuts_et_frise() {
    let b = base();
    let c = b.connexion();
    let id = candidatures::ajouter(c, &fiche("Brico Lorraine", jour(2026, 10, 30)), m(1, 9, 0, 0)).unwrap();
    // Relancée le 13 : la relance est notée
    assert_eq!(candidatures::changer_statut(c, id, Statut::Relancee, None, m(13, 9, 0, 0)).unwrap(), Change::Fait);
    let x = candidatures::lire(c, id).unwrap().unwrap();
    assert_eq!((x.statut, x.relance_le, x.statut_le), (Statut::Relancee, Some(jour(2026, 11, 13)), jour(2026, 11, 13)));
    // Le même statut : rien
    assert_eq!(candidatures::changer_statut(c, id, Statut::Relancee, None, m(14, 9, 0, 0)).unwrap(), Change::Pareil);
    // Entretien avec son moment et son lieu, puis déplacé
    let e = Entretien { jour: jour(2026, 11, 20), heure: Heure::nouvelle(9, 30), lieu: "Zone Actipôle".into() };
    assert_eq!(candidatures::changer_statut(c, id, Statut::Entretien, Some(&e), m(15, 9, 0, 0)).unwrap(), Change::Fait);
    assert_eq!(
        candidatures::changer_statut(c, id, Statut::Entretien, Some(&e), m(15, 9, 5, 0)).unwrap(),
        Change::Pareil
    );
    let e2 = Entretien { jour: jour(2026, 11, 21), ..e.clone() };
    assert_eq!(
        candidatures::changer_statut(c, id, Statut::Entretien, Some(&e2), m(16, 9, 0, 0)).unwrap(),
        Change::Fait
    );
    assert_eq!(candidatures::lire(c, id).unwrap().unwrap().entretien, Some(e2));
    assert_eq!(candidatures::changer_statut(c, id, Statut::Positive, None, m(22, 9, 0, 0)).unwrap(), Change::Fait);
    let f = candidatures::evenements(c, id).unwrap();
    let vu: Vec<(String, Statut)> = f.iter().map(|e| (e.jour.iso(), e.statut)).collect();
    assert_eq!(
        vu,
        [
            ("2026-10-30".into(), Statut::Envoyee),
            ("2026-11-13".into(), Statut::Relancee),
            ("2026-11-15".into(), Statut::Entretien),
            ("2026-11-16".into(), Statut::Entretien),
            ("2026-11-22".into(), Statut::Positive),
        ]
    );
    assert_eq!(f[2].texte, "Le 20/11/2026 à 9 h 30, Zone Actipôle");
    assert_eq!(
        candidatures::changer_statut(c, 999, Statut::Positive, None, m(22, 9, 0, 0)).unwrap(),
        Change::Introuvable
    );
}

#[test]
fn le_delai_de_relance() {
    let b = base();
    let c = b.connexion();
    assert_eq!(candidatures::delai_relance(c).unwrap(), 14);
    assert_eq!(candidatures::changer_delai_relance(c, 21).unwrap(), 21);
    assert_eq!(candidatures::delai_relance(c).unwrap(), 21);
    // Toujours entre 3 et 60, même tapé de travers dans la base
    assert_eq!(candidatures::changer_delai_relance(c, 1).unwrap(), 3);
    assert_eq!(candidatures::changer_delai_relance(c, 900).unwrap(), 60);
    c.execute("UPDATE parametres SET valeur = 'beaucoup' WHERE cle = 'relance_jours'", []).unwrap();
    assert_eq!(candidatures::delai_relance(c).unwrap(), 14);
}

fn journal(e: &Essai) -> Journal {
    Journal::ouvrir(&e.0.join("journal.txt"), horloge_figee(m(18, 10, 0, 0)))
}

#[test]
fn une_bulle_par_evenement_jamais_en_boucle() {
    let e = Essai::nouveau("bulles");
    let j = journal(&e);
    let b = base();
    let c = b.connexion();
    let a = candidatures::ajouter(c, &fiche("Transports Moselle Fret", jour(2026, 11, 4)), m(4, 9, 0, 0)).unwrap();
    let r = candidatures::ajouter(c, &fiche("Brico Lorraine", jour(2026, 11, 4)), m(4, 9, 0, 0)).unwrap();
    let ent = candidatures::ajouter(c, &fiche("Hôtel des Remparts", jour(2026, 11, 10)), m(10, 9, 0, 0)).unwrap();
    let rdv = Entretien { jour: jour(2026, 11, 19), heure: Heure::nouvelle(9, 30), lieu: "Nancy".into() };
    candidatures::changer_statut(c, ent, Statut::Entretien, Some(&rdv), m(12, 9, 0, 0)).unwrap();
    let dues = |jour_: Jour| bulles_dues(&candidatures::lister(c).unwrap().0, jour_, 14);
    // Le 17 : rien n'est dû
    assert!(bulles::retenir(c, &dues(jour(2026, 11, 17)), m(17, 9, 0, 0)).unwrap().is_empty());
    // Le 18 : deux relances (une seule bulle) et l'entretien de demain (une autre)
    let faites = bulles::retenir(c, &dues(jour(2026, 11, 18)), m(18, 9, 0, 0)).unwrap();
    assert_eq!(faites.len(), 2);
    assert_eq!(faites[0].titre, "2 relances à faire");
    assert_eq!(faites[0].candidature, None);
    assert_eq!(faites[1].titre, "Entretien demain");
    assert_eq!(faites[1].candidature, Some(ent));
    assert!(faites.iter().all(|b| bulles::id_bulle_valable(&b.id)));
    // Revérifié toutes les 30 s, puis le lendemain, puis après un redémarrage : plus jamais de bulle pour eux
    for (jj, h) in [(18, 9), (18, 10), (19, 8)] {
        assert!(bulles::retenir(c, &dues(jour(2026, 11, jj)), m(jj, h, 0, 0)).unwrap().is_empty(), "{jj} {h}");
    }
    // Un nouveau jour d'envoi refait une relance (une nouvelle clé) ; ce que Jordan voit déjà ne fait pas de bulle
    let mut f = fiche("Transports Moselle Fret", jour(2026, 11, 3));
    candidatures::modifier(c, a, &f, m(18, 11, 0, 0)).unwrap();
    let pour_a: Vec<_> = dues(jour(2026, 11, 18)).into_iter().filter(|d| d.candidature == a).collect();
    bulles::marquer_vues(c, &pour_a, m(18, 11, 0, 0)).unwrap();
    assert!(bulles::retenir(c, &dues(jour(2026, 11, 18)), m(18, 11, 0, 30)).unwrap().is_empty());
    f.envoyee_le = jour(2026, 11, 2);
    candidatures::modifier(c, a, &f, m(18, 11, 1, 0)).unwrap();
    let neuve = bulles::retenir(c, &dues(jour(2026, 11, 18)), m(18, 11, 1, 30)).unwrap();
    assert_eq!(neuve.len(), 1);
    assert_eq!(neuve[0].candidature, Some(a));
    assert_eq!(neuve[0].titre, "Une relance à faire");
    let _ = (r, j);
}

#[test]
fn envoi_une_a_la_fois_et_clic() {
    let e = Essai::nouveau("envoi");
    let j = journal(&e);
    let b = base();
    let c = b.connexion();
    let a = candidatures::ajouter(c, &fiche("Transports Moselle Fret", jour(2026, 11, 2)), m(2, 9, 0, 0)).unwrap();
    let ent = candidatures::ajouter(c, &fiche("Hôtel des Remparts", jour(2026, 11, 10)), m(10, 9, 0, 0)).unwrap();
    let rdv = Entretien { jour: jour(2026, 11, 19), heure: None, lieu: String::new() };
    candidatures::changer_statut(c, ent, Statut::Entretien, Some(&rdv), m(12, 9, 0, 0)).unwrap();
    let dues = bulles_dues(&candidatures::lister(c).unwrap().0, jour(2026, 11, 18), 14);
    let faites = bulles::retenir(c, &dues, m(18, 9, 0, 0)).unwrap();
    assert_eq!(faites.len(), 2);
    let mut envoi = Envoi::nouveau(&e.0);
    let notif = e.0.join(bulles::NOTIFICATION);
    // La première part
    let b1 = envoi.envoyer(c, &j, m(18, 9, 0, 0)).unwrap();
    assert_eq!(b1, faites[0]);
    let lu: serde_json::Value = serde_json::from_slice(&std::fs::read(&notif).unwrap()).unwrap();
    assert_eq!(lu, serde_json::json!({ "titre": b1.titre, "texte": b1.texte, "id": b1.id }));
    // Pas lue par Mindset : on attend
    assert!(envoi.envoyer(c, &j, m(18, 9, 0, 2)).is_none());
    // Lue (supprimée par Mindset) : 6 s d'écart au moins depuis la précédente
    std::fs::remove_file(&notif).unwrap();
    assert!(envoi.envoyer(c, &j, m(18, 9, 0, 5)).is_none());
    let b2 = envoi.envoyer(c, &j, m(18, 9, 0, 6)).unwrap();
    assert_eq!(b2.candidature, Some(ent));
    // Plus rien en attente
    std::fs::remove_file(&notif).unwrap();
    assert!(envoi.envoyer(c, &j, m(18, 9, 1, 0)).is_none());
    // Le clic sur une bulle : la candidature qu'elle annonçait ; une bulle de plusieurs relances : la liste
    let afficher = e.0.join(bulles::AFFICHER);
    std::fs::write(&afficher, format!("notif:{}", b2.id)).unwrap();
    assert_eq!(envoi.demande(c, &j), Demande::Ouvrir(Some(ent)));
    assert!(!afficher.exists());
    std::fs::write(&afficher, format!("\u{feff}notif:{}\r\n", b1.id)).unwrap();
    assert_eq!(envoi.demande(c, &j), Demande::Ouvrir(Some(a)));
    // Une candidature supprimée depuis : la liste
    candidatures::supprimer(c, ent).unwrap();
    std::fs::write(&afficher, format!("notif:{}", b2.id)).unwrap();
    assert_eq!(envoi.demande(c, &j), Demande::Ouvrir(None));
    // Un numéro de processus : montrer ; le reste : ignoré, toujours supprimé
    std::fs::write(&afficher, "4242").unwrap();
    assert_eq!(envoi.demande(c, &j), Demande::Montrer);
    for t in ["notif:000000000000", "notif:ABCDEF012345", "C:\\Windows\\notepad.exe", "", "notif:../../x"] {
        std::fs::write(&afficher, t).unwrap();
        assert_eq!(envoi.demande(c, &j), Demande::Rien, "{t}");
        assert!(!afficher.exists());
    }
    std::fs::write(&afficher, "1".repeat(100)).unwrap();
    assert_eq!(envoi.demande(c, &j), Demande::Rien);
    assert!(!afficher.exists());
    assert_eq!(envoi.demande(c, &j), Demande::Rien);
}

#[test]
fn bulle_abandonnee_si_mindset_ne_la_lit_pas() {
    let e = Essai::nouveau("abandon");
    let j = journal(&e);
    let b = base();
    let c = b.connexion();
    for (n, nom) in ["A", "B"].iter().enumerate() {
        let id = candidatures::ajouter(c, &fiche(nom, jour(2026, 11, 2)), m(2, 9, 0, 0)).unwrap();
        let rdv = Entretien { jour: jour(2026, 11, 19), heure: Heure::nouvelle(9 + n as u32, 0), lieu: String::new() };
        candidatures::changer_statut(c, id, Statut::Entretien, Some(&rdv), m(12, 9, 0, 0)).unwrap();
    }
    let dues = bulles_dues(&candidatures::lister(c).unwrap().0, jour(2026, 11, 18), 14);
    assert_eq!(bulles::retenir(c, &dues, m(18, 9, 0, 0)).unwrap().len(), 2);
    let mut envoi = Envoi::nouveau(&e.0);
    let notif = e.0.join(bulles::NOTIFICATION);
    assert!(envoi.envoyer(c, &j, m(18, 9, 0, 0)).is_some());
    // Mindset fermé : 5 minutes, puis abandonnée ; la suivante part 6 s plus tard
    assert!(envoi.envoyer(c, &j, m(18, 9, 4, 59)).is_none());
    assert!(notif.exists());
    assert!(envoi.envoyer(c, &j, m(18, 9, 5, 1)).is_none());
    assert!(!notif.exists());
    assert!(envoi.envoyer(c, &j, m(18, 9, 5, 4)).is_none());
    assert!(envoi.envoyer(c, &j, m(18, 9, 5, 7)).is_some());
    let journal = std::fs::read_to_string(e.0.join("journal.txt")).unwrap();
    assert!(journal.contains("Bulle abandonnée (Mindset ne l'a pas lue en 5 minutes)"), "{journal}");
    assert_eq!(journal.matches("Bulle envoyée").count(), 2);
}
