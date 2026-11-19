//! Les offres dans la base (étape 4) : la migration 4 sur une base de l'étape 3, l'archive des réponses d'Adzuna et
//! de Jooble (les réponses inventées du faux serveur de jobmind-sources), les doublons fusionnés (et ceux qui ne
//! doivent pas l'être), une offre gardée même quand le site ne la donne plus, la recherche dans tout le texte,
//! favoris, « plus tard », « Pas pour moi », les recherches gardées et le compte des demandes du jour.

use jobmind_core::Jour;
use jobmind_core::recherche::{Contrat, Lieu};
use jobmind_core::recherches::NouvelleRecherche;
use jobmind_sources::{adzuna, faux, jooble};
use jobmind_storage::offres::{self, Marque};
use jobmind_storage::{Base, Moment};

fn m(h: u32) -> Moment {
    Moment::nouveau(2026, 11, 18, h, 0, 0).unwrap()
}

fn base() -> Base {
    Base::en_memoire(m(9)).unwrap().0
}

/// Une base de l'étape 3 avec une candidature et une recherche de la migration 1 : la migration 4 n'en perd rien.
#[test]
fn migration_4_sur_une_base_de_l_etape_3() {
    let mut b = Base::en_memoire_jusqua(3, m(9)).unwrap();
    b.connexion()
        .execute(
            "INSERT INTO candidatures (entreprise, poste, cree_le, change_le) VALUES ('A', 'B', '2026-11-01T10:00:00', 'x')",
            [],
        )
        .unwrap();
    b.connexion()
        .execute(
            "INSERT INTO recherches (nom, mots_cles, lieu, rayon_km, cree_le) VALUES ('Quai', 'quai', 'Metz', 30, 'x')",
            [],
        )
        .unwrap();
    assert_eq!(b.migrer_tout(m(9)).unwrap(), vec![4, 5, 6, 7, 8, 9, 10, 11, 12]);
    let c = b.connexion();
    let sources: Vec<String> = c
        .prepare("SELECT code FROM sources ORDER BY id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(sources, ["adzuna", "jooble", "linkedin", "indeed", "wttj", "francetravail", "site"]);
    assert_eq!(jobmind_storage::candidatures::lister(c).unwrap().0.len(), 1);
    // La recherche d'avant : autour de sa ville (le genre par défaut)
    let r = offres::lister_recherches(c).unwrap();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].lieu, Lieu::Ville { nom: "Metz".into(), rayon: 30 });
    assert_eq!(r[0].derniere_fois, None);
}

/// L'archive : Adzuna puis Jooble ; deux offres annoncées par les deux sites ne font qu'une, avec ses deux sources ;
/// les autres restent séparées ; une recherche refaite ne réécrit rien ; une offre que le site ne donne plus reste.
#[test]
fn archive_et_doublons() {
    let b = base();
    let c = b.connexion();
    let az = adzuna::lire(200, &faux::reponse_adzuna(), false).unwrap();
    let jo = jooble::lire(200, &faux::reponse_jooble(), false).unwrap();
    let r = offres::ajouter_recherche(
        c,
        &NouvelleRecherche {
            nom: "Quai".into(),
            mots: "agent de quai".into(),
            lieu: Lieu::Ville { nom: "Metz".into(), rayon: 30 },
        },
        m(9),
    )
    .unwrap();
    let b1 = offres::ajouter_trouvees(c, "adzuna", &az, Some(r), m(10)).unwrap();
    assert_eq!((b1.nouvelles, b1.retrouvees, b1.fusionnees), (12, 0, 0));
    let b2 = offres::ajouter_trouvees(c, "jooble", &jo, Some(r), m(10)).unwrap();
    assert_eq!((b2.nouvelles, b2.retrouvees, b2.fusionnees), (7, 0, 2));
    let l = offres::lister(c).unwrap();
    assert_eq!(l.len(), 17);
    // L'agent de quai d'Ennery : une seule ligne, Adzuna et Jooble
    let quai: Vec<_> = l.iter().filter(|o| o.titre.starts_with("Agent de quai")).collect();
    assert_eq!(quai.len(), 2, "{quai:?}");
    let ennery = quai.iter().find(|o| o.lieu.starts_with("Ennery")).unwrap();
    assert_eq!(ennery.sources, ["adzuna", "jooble"]);
    assert_eq!(ennery.contrat, Some(Contrat::Cdi));
    assert_eq!((ennery.departement.as_deref(), ennery.region.as_deref()), (Some("57"), Some("grand_est")));
    assert_eq!(ennery.recherches, [r]);
    // L'autre agent de quai (Woippy, une autre entreprise) reste à part
    let woippy = quai.iter().find(|o| o.lieu.starts_with("Woippy")).unwrap();
    assert_eq!((woippy.sources.as_slice(), woippy.contrat), (&["jooble".to_string()][..], Some(Contrat::Interim)));
    // Le cariste : fusionné aussi ; une offre sans entreprise jamais
    assert!(l.iter().any(|o| o.titre == "Cariste CACES 3" && o.sources == ["adzuna", "jooble"]));
    assert!(l.iter().filter(|o| o.entreprise.is_empty()).all(|o| o.sources.len() == 1));
    // Ce qui se lit dans les offres
    let dev = l.iter().find(|o| o.titre.starts_with("Développeur")).unwrap();
    assert_eq!(dev.teletravail, Some(jobmind_core::offre::TeleOffre::Complet));
    assert_eq!((dev.departement.as_deref(), dev.contrat), (Some("54"), Some(Contrat::Cdi)));
    let apprenti = l.iter().find(|o| o.titre.starts_with("Apprenti")).unwrap();
    assert_eq!(apprenti.contrat, Some(Contrat::Alternance));
    // L'offre ouverte : le plus long texte, un lien par source
    let o = offres::lire(c, ennery.id).unwrap().unwrap();
    assert_eq!(o.liens.iter().map(|x| x.source.as_str()).collect::<Vec<_>>(), ["adzuna", "jooble"]);
    assert!(o.liens[0].url.starts_with("https://www.adzuna.fr/land/ad/"));
    assert_eq!(o.description_de, "adzuna");
    assert!(o.description.contains("Débutant accepté"));
    // Ouverte par le doublon : la même
    let doublon: i64 = c.query_row("SELECT id FROM offres WHERE doublon_de = ?1", [ennery.id], |x| x.get(0)).unwrap();
    assert_eq!(offres::lire(c, doublon).unwrap().unwrap().offre.id, ennery.id);
    // Refaite plus tard : tout est retrouvé, rien de nouveau, rien de réécrit
    let b3 = offres::ajouter_trouvees(c, "adzuna", &az, Some(r), m(15)).unwrap();
    assert_eq!((b3.nouvelles, b3.retrouvees), (0, 12));
    // Le site ne donne plus que 3 offres : les 17 restent gardées
    offres::ajouter_trouvees(c, "adzuna", &az[..3], None, m(16)).unwrap();
    assert_eq!(offres::lister(c).unwrap().len(), 17);
    // Une offre au texte abîmé ou piégé est gardée telle quelle, sans risque
    let mut piege = az[0].clone();
    piege.id_source = "piege".into();
    piege.titre = "Cariste'); DROP TABLE offres; --".into();
    piege.entreprise = "Robert'); --".into();
    piege.url = String::new();
    offres::ajouter_trouvees(c, "adzuna", &[piege], None, m(16)).unwrap();
    assert_eq!(offres::lister(c).unwrap().len(), 18);
}

#[test]
fn recherche_dans_le_texte_et_marques() {
    let b = base();
    let c = b.connexion();
    offres::ajouter_trouvees(c, "adzuna", &adzuna::lire(200, &faux::reponse_adzuna(), false).unwrap(), None, m(10))
        .unwrap();
    offres::ajouter_trouvees(c, "jooble", &jooble::lire(200, &faux::reponse_jooble(), false).unwrap(), None, m(10))
        .unwrap();
    let l = offres::lister(c).unwrap();
    let id = |t: &str| l.iter().find(|o| o.titre.starts_with(t)).unwrap().id;
    // Sans accents ni majuscules, le début d'un mot suffit ; les doublons ne comptent qu'une fois
    assert_eq!(offres::chercher_texte(c, "PRÉPARATEUR").unwrap(), [id("Préparateur")].into());
    assert!(offres::chercher_texte(c, "prepa").unwrap().len() > 2);
    let palettes = offres::chercher_texte(c, "palettes").unwrap();
    assert!(palettes.contains(&id("Agent de quai")), "{palettes:?}");
    assert_eq!(offres::chercher_texte(c, "chariot élévateur").unwrap(), [id("Cariste")].into());
    assert_eq!(offres::chercher_texte(c, "boulangerie metz").unwrap(), [id("Vendeur en boulangerie")].into());
    assert!(offres::chercher_texte(c, "astronaute").unwrap().is_empty());
    // Des textes qui ressemblent à la syntaxe de FTS5 : jamais d'erreur
    for t in ["\"", "a OR b", "NEAR(", "*", "-cdi", "x AND", "() ; DROP", "", "   "] {
        assert!(offres::chercher_texte(c, t).is_ok(), "{t}");
    }
    assert_eq!(offres::requete_texte("agent de quai"), Some("\"agent\"* \"de\"* \"quai\"*".into()));
    // Les marques
    let q = id("Agent de quai");
    assert_eq!(offres::basculer(c, q, Marque::Favori).unwrap(), Some(true));
    assert_eq!(offres::basculer(c, q, Marque::PlusTard).unwrap(), Some(true));
    assert_eq!(offres::basculer(c, q, Marque::Favori).unwrap(), Some(false));
    assert_eq!(offres::basculer(c, 9999, Marque::Favori).unwrap(), None);
    let avant = offres::nombre_nouvelles(c).unwrap();
    assert_eq!(avant, 17);
    assert!(offres::marquer_vue(c, q).unwrap());
    assert_eq!(offres::basculer(c, id("Magasinier"), Marque::PasPourMoi).unwrap(), Some(true));
    assert_eq!(offres::nombre_nouvelles(c).unwrap(), 15);
    let l = offres::lister(c).unwrap();
    let o = l.iter().find(|o| o.id == q).unwrap();
    assert!(!o.favori && o.plus_tard && o.vue);
    assert!(l.iter().find(|o| o.titre == "Magasinier").unwrap().cachee);
    // Une candidature rattachée
    c.execute("INSERT INTO candidatures (entreprise, poste, cree_le, change_le) VALUES ('A', 'B', 'x', 'x')", [])
        .unwrap();
    let k = c.last_insert_rowid();
    assert!(offres::lier_candidature(c, q, k).unwrap());
    assert_eq!(offres::lister(c).unwrap().iter().find(|o| o.id == q).unwrap().candidature, Some(k));
}

#[test]
fn recherches_gardees_et_demandes_du_jour() {
    let b = base();
    let c = b.connexion();
    let n = |nom: &str, lieu| NouvelleRecherche { nom: nom.into(), mots: nom.to_lowercase(), lieu };
    let a = offres::ajouter_recherche(c, &n("Cariste", Lieu::Ville { nom: "Metz".into(), rayon: 20 }), m(9)).unwrap();
    let d = jobmind_core::lieux::departement("54").unwrap();
    let z = offres::ajouter_recherche(c, &n("Vendeur", Lieu::Departement(d)), m(9)).unwrap();
    offres::ajouter_recherche(c, &n("Partout", Lieu::Partout), m(9)).unwrap();
    let l = offres::lister_recherches(c).unwrap();
    assert_eq!(l.iter().map(|x| x.nom.as_str()).collect::<Vec<_>>(), ["Cariste", "Vendeur", "Partout"]);
    assert_eq!(l[1].lieu, Lieu::Departement(d));
    assert!(offres::modifier_recherche(c, a, &n("Cariste de nuit", Lieu::Partout)).unwrap());
    offres::noter_recherche(c, a, m(11), 23, "23 offres, 5 nouvelles").unwrap();
    let l = offres::lister_recherches(c).unwrap();
    assert_eq!((l[0].nom.as_str(), l[0].lieu.clone()), ("Cariste de nuit", Lieu::Partout));
    assert_eq!((l[0].derniere_fois, l[0].dernier_nombre), (Some(m(11)), Some(23)));
    // Supprimée : ses offres restent, le lien part
    offres::ajouter_trouvees(c, "jooble", &jooble::lire(200, &faux::reponse_jooble(), false).unwrap(), Some(z), m(10))
        .unwrap();
    assert!(offres::supprimer_recherche(c, z).unwrap());
    assert!(!offres::supprimer_recherche(c, z).unwrap());
    let l = offres::lister(c).unwrap();
    assert_eq!(l.len(), 7);
    assert!(l.iter().all(|o| o.recherches.is_empty()));
    assert_eq!(offres::nombre_recherches(c).unwrap(), 2);
    // Les demandes du jour, par source
    let j = Jour::nouveau(2026, 11, 18).unwrap();
    assert_eq!(offres::appels(c, j, "adzuna").unwrap(), 0);
    assert_eq!(offres::compter_appel(c, j, "adzuna").unwrap(), 1);
    assert_eq!(offres::compter_appel(c, j, "adzuna").unwrap(), 2);
    assert_eq!(offres::compter_appel(c, j, "jooble").unwrap(), 1);
    assert_eq!(offres::appels(c, j.plus(1), "adzuna").unwrap(), 0);
}

/// Étape 5 : « Ajouter à Jobmind ». La page de LinkedIn d'exemple entre dans l'archive avec son nombre de candidats ;
/// un second clic plus tard la met à jour (le nombre et sa date) sans la dupliquer ; la même offre déjà venue de Jooble
/// est fusionnée avec elle ; une page sans nombre de candidats garde le dernier nombre vu.
#[test]
fn ajouter_une_page() {
    use jobmind_sources::page::{self, exemples};
    let b = base();
    let c = b.connexion();
    // La même offre, déjà donnée par Jooble
    let deja = jobmind_sources::OffreTrouvee {
        id_source: "j-1".into(),
        titre: "Cariste (H/F)".into(),
        entreprise: "Entrepôts du Val de Moselle SAS".into(),
        lieu: "Ennery (57)".into(),
        url: "https://jooble.org/desc/1".into(),
        description: "Cariste en CDI.".into(),
        ..Default::default()
    };
    offres::ajouter_trouvees(c, "jooble", &[deja], None, m(9)).unwrap();
    let premiere = offres::lister(c).unwrap()[0].id;
    let p = page::lire(exemples::PAGES[0].0, exemples::LINKEDIN_CONNECTE).unwrap();
    let n = p.candidats.as_ref().map(|x| (x.texte.as_str(), x.nombre.map(i64::from)));
    let a = offres::ajouter_page(c, p.site.code(), &p.offre, n, m(10)).unwrap();
    assert_eq!(a, offres::AjoutPage { id: premiere, nouvelle: true, fusionnee: true });
    let o = offres::lire(c, a.id).unwrap().unwrap();
    assert_eq!(o.offre.sources, ["jooble", "linkedin"]);
    // (le texte le plus long : celui de la page, entier)
    assert_eq!(o.description_de, "linkedin");
    assert!(o.description.contains("Charger et décharger les camions"));
    let vue = o.candidats.unwrap();
    assert_eq!((vue.site.as_str(), vue.texte.as_str(), vue.nombre), ("linkedin", "34 candidats", Some(34)));
    assert_eq!(vue.vu_le, Jour::nouveau(2026, 11, 18).unwrap());
    // Un second clic le lendemain : mis à jour, pas dupliqué
    let demain = Moment::nouveau(2026, 11, 19, 8, 0, 0).unwrap();
    let a2 = offres::ajouter_page(c, "linkedin", &p.offre, Some(("Plus de 100 candidats", Some(100))), demain).unwrap();
    assert_eq!(a2, offres::AjoutPage { id: premiere, nouvelle: false, fusionnee: false });
    assert_eq!(offres::lister(c).unwrap().len(), 1);
    let vue = offres::lire(c, premiere).unwrap().unwrap().candidats.unwrap();
    assert_eq!((vue.texte.as_str(), vue.vu_le), ("Plus de 100 candidats", Jour::nouveau(2026, 11, 19).unwrap()));
    // Un clic qui ne montre aucun nombre : le dernier vu reste
    let plus_tard = Moment::nouveau(2026, 11, 20, 8, 0, 0).unwrap();
    offres::ajouter_page(c, "linkedin", &p.offre, None, plus_tard).unwrap();
    let vue = offres::lire(c, premiere).unwrap().unwrap().candidats.unwrap();
    assert_eq!(vue.texte, "Plus de 100 candidats");
    let vues: i64 = c.query_row("SELECT COUNT(*) FROM offres_vues", [], |r| r.get(0)).unwrap();
    assert_eq!(vues, 3);
    // Une autre page, d'un autre site : une nouvelle offre, cherchable dans tout le texte
    let w = page::lire(exemples::PAGES[3].0, exemples::WTTJ).unwrap();
    let a3 = offres::ajouter_page(c, w.site.code(), &w.offre, None, m(11)).unwrap();
    assert!(a3.nouvelle && !a3.fusionnee && a3.id != premiere);
    assert!(offres::chercher_texte(c, "menuisier").unwrap().contains(&a3.id));
    let o = offres::lire(c, a3.id).unwrap().unwrap();
    assert_eq!(o.offre.salaire, "26 000 à 30 000 € par an");
    assert_eq!(o.offre.contrat, Some(Contrat::Cdi));
    assert_eq!(o.offre.departement.as_deref(), Some("57"));
    assert_eq!(o.candidats, None);
}
