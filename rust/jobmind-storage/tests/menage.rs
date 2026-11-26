//! Ce qu'un audit a trouvé (migration 17, 26/11/2026), sur des bases d'essai seulement (en mémoire, ou dans un dossier
//! d'essai sous %TEMP%, jamais les vraies données) : une base de la version 16 déjà remplie est mise à jour sans rien
//! perdre ; le ménage vide la réponse brute des vieilles offres sans rien changer d'autre ; la réponse de Claude
//! gardée est remise quand une version suivante des règles ne trouve plus rien.

use std::path::PathBuf;

use jobmind_core::lecture::{Diplome, Experience, LuPar, VERSION_REGLES};
use jobmind_sources::OffreTrouvee;
use jobmind_storage::lecture::{self, ResultatIa};
use jobmind_storage::offres::{self, JOURS_BRUT, Marque};
use jobmind_storage::rusqlite::Connection;
use jobmind_storage::rusqlite::types::Value;
use jobmind_storage::{Base, Moment};

/// Un dossier d'essai neuf sous %TEMP%, effacé à la fin.
struct Essai(PathBuf);

impl Essai {
    fn nouveau(nom: &str) -> Essai {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-menage-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Essai(d)
    }
}

impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn jour(mois: u32, j: u32) -> Moment {
    Moment::nouveau(2026, mois, j, 10, 0, 0).unwrap()
}

/// Un texte où les règles ne trouvent ni diplôme ni expérience, assez long pour être donné à Claude.
const LONG: &str = "Au sein de notre entrepôt de Metz, vous préparez les commandes de nos clients, vous chargez les \
                    camions et vous tenez votre zone propre. Horaires en journée, du lundi au vendredi, travail en \
                    équipe de cinq personnes avec un chef d'équipe présent.";

fn trouvee(id: &str, titre: &str, entreprise: &str, description: &str, brut: &str) -> OffreTrouvee {
    OffreTrouvee {
        id_source: id.into(),
        titre: titre.into(),
        entreprise: entreprise.into(),
        lieu: "Metz (57)".into(),
        description: description.into(),
        brut: brut.into(),
        ..Default::default()
    }
}

/// Toutes les lignes d'une table, triées, avec seulement ces colonnes.
fn lignes(c: &Connection, table: &str, colonnes: &str) -> Vec<String> {
    let mut r = c.prepare(&format!("SELECT {colonnes} FROM \"{table}\"")).unwrap();
    let n = r.column_count();
    let mut l: Vec<String> = r
        .query_map([], |x| {
            let mut s = Vec::new();
            for i in 0..n {
                s.push(format!("{:?}", x.get::<_, Value>(i)?));
            }
            Ok(s.join("|"))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    l.sort();
    l
}

/// Les tables d'une base (sans celles de la recherche dans tout le texte ni la table des migrations), et leurs
/// colonnes.
fn tables(c: &Connection) -> Vec<(String, String)> {
    let mut r = c
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'offres_fts%' \
             AND name NOT LIKE 'sqlite_%' AND name != 'migrations' ORDER BY name",
        )
        .unwrap();
    let noms: Vec<String> = r.query_map([], |x| x.get(0)).unwrap().map(Result::unwrap).collect();
    noms.into_iter()
        .map(|t| {
            let mut r = c.prepare(&format!("SELECT name FROM pragma_table_info('{t}') ORDER BY cid")).unwrap();
            let cols: Vec<String> = r.query_map([], |x| x.get(0)).unwrap().map(Result::unwrap).collect();
            (t, cols.iter().map(|x| format!("\"{x}\"")).collect::<Vec<_>>().join(", "))
        })
        .collect()
}

/// Tout le contenu d'une base, table par table, avec les colonnes données.
fn vidage(c: &Connection, tables: &[(String, String)]) -> Vec<(String, Vec<String>)> {
    tables.iter().map(|(t, cols)| (t.clone(), lignes(c, t, cols))).collect()
}

/// Remplit une base de la version 16 comme le faisait Jobmind avant la migration 17 : des offres (avec leur réponse
/// brute), un doublon, une candidature rattachée à son offre, des marques, une page vue, une lecture de Claude (seul
/// son texte était gardé), un paramètre.
fn remplir_version_16(c: &Connection) -> (i64, i64) {
    let gros = format!("{{\"page\":\"{}\"}}", "b".repeat(60_000));
    offres::ajouter_trouvees(
        c,
        "francetravail",
        &[
            trouvee("1", "Agent de tri", "Poste", LONG, &gros),
            trouvee("2", "Comptable", "Cabinet", "BTS comptabilité exigé, 3 ans d'expérience.", "{\"id\":2}"),
        ],
        None,
        jour(6, 1),
    )
    .unwrap();
    offres::ajouter_page(
        c,
        "indeed",
        &trouvee("a", "Agent de tri", "Poste", "Un résumé.", "{\"a\":1}"),
        None,
        jour(6, 2),
    )
    .unwrap();
    let l = offres::lister(c).unwrap();
    let tri = l.iter().find(|o| o.titre == "Agent de tri").unwrap().id;
    let compta = l.iter().find(|o| o.titre == "Comptable").unwrap().id;
    assert_eq!(offres::basculer(c, tri, Marque::Favori).unwrap(), Some(true));
    c.execute(
        "INSERT INTO candidatures (offre_id, entreprise, poste, cree_le, change_le) \
         VALUES (?1, 'Poste', 'Agent de tri', '2026-06-03T10:00:00', '2026-06-03T10:00:00')",
        [tri],
    )
    .unwrap();
    c.execute("INSERT INTO parametres (cle, valeur) VALUES ('alerte_jours', '14')", []).unwrap();
    // (la lecture de Claude comme l'écrivait la version 16 : sa réponse seulement)
    c.execute(
        "INSERT INTO lectures_ia (offre_id, lue_le, modele, etat, jetons_entree, jetons_sortie, cout_millioniemes, \
         reponse) VALUES (?1, '2026-06-03T11:00:00', 'claude-haiku-5-5', 'ok', 900, 60, 120, '{\"diplome\":\"aucun\"}')",
        [tri],
    )
    .unwrap();
    c.execute("UPDATE offres SET lue_par_ia = 1 WHERE id = ?1", [tri]).unwrap();
    (tri, compta)
}

/// Une base de la version 16 déjà remplie, dans un fichier : ouverte par cette version, elle passe à la version 17
/// sans rien perdre (chaque ligne de chaque table, colonne par colonne), gagne l'index des candidatures par offre et
/// les colonnes de la réponse de Claude, et reste saine.
#[test]
fn une_base_de_la_version_16_remplie_est_mise_a_jour_sans_rien_perdre() {
    let e = Essai::nouveau("version16");
    let ancienne = Base::en_memoire_jusqua(16, jour(6, 1)).unwrap();
    assert_eq!(ancienne.version().unwrap(), 16);
    let (tri, _) = remplir_version_16(ancienne.connexion());
    let avant_tables = tables(ancienne.connexion());
    let avant = vidage(ancienne.connexion(), &avant_tables);
    let trouve_avant = offres::chercher_texte(ancienne.connexion(), "entrepôt").unwrap();
    // La base de la version 16 écrite dans un fichier, comme celle d'une personne qui met Jobmind à jour
    let fichier = Base::chemin_dans(&e.0);
    std::fs::create_dir_all(fichier.parent().unwrap()).unwrap();
    ancienne.connexion().execute("VACUUM INTO ?1", [fichier.to_str().unwrap()]).unwrap();
    drop(ancienne);

    let (b, appliquees) = Base::ouvrir(&e.0, jour(11, 26)).unwrap();
    assert_eq!(appliquees, vec![17]);
    assert_eq!(b.version().unwrap(), 17);
    let c = b.connexion();
    // Rien de perdu ni de changé : les mêmes lignes, dans les colonnes d'avant
    assert_eq!(vidage(c, &avant_tables), avant);
    assert_eq!(offres::chercher_texte(c, "entrepôt").unwrap(), trouve_avant);
    assert!(!trouve_avant.is_empty());
    let brut: i64 = c.query_row("SELECT length(brut) FROM offres WHERE id = ?1", [tri], |r| r.get(0)).unwrap();
    assert!(brut > 60_000);
    // L'index des candidatures par offre : il existe, et la recherche s'en sert
    let plan: Vec<String> = c
        .prepare("EXPLAIN QUERY PLAN SELECT id FROM candidatures WHERE offre_id = ?1")
        .unwrap()
        .query_map([tri], |r| r.get(3))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(plan.iter().any(|p| p.contains("candidatures_offre")), "{plan:?}");
    // L'ancienne lecture de Claude : toujours là, pas encore comprise de nouveau
    let (comprise, d): (i64, Option<String>) = c
        .query_row("SELECT comprise, diplome FROM lectures_ia WHERE offre_id = ?1", [tri], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((comprise, d), (0, None));
    let sain: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0)).unwrap();
    assert_eq!(sain, "ok");
    let liens: i64 = c.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r.get(0)).unwrap();
    assert_eq!(liens, 0);
    drop(b);
    // Rouverte : plus rien à appliquer
    let (b, appliquees) = Base::ouvrir(&e.0, jour(11, 26)).unwrap();
    assert!(appliquees.is_empty());
    assert_eq!(vidage(b.connexion(), &avant_tables), avant);
}

/// Le ménage : seule la réponse brute des offres pas retrouvées depuis plus de 90 jours est vidée. Aucune offre n'est
/// retirée, et tout le reste de chaque offre (son texte, la recherche, ses marques, sa candidature) ne change pas.
#[test]
fn le_menage_vide_la_reponse_brute_des_vieilles_offres_seulement() {
    let (b, _) = Base::en_memoire(jour(6, 1)).unwrap();
    let c = b.connexion();
    let (tri, compta) = remplir_version_16(c);
    // Une offre récente, et une vieille offre retrouvée récemment
    offres::ajouter_trouvees(
        c,
        "francetravail",
        &[trouvee("3", "Vendeur", "Brico", "Vente.", "{\"id\":3}")],
        None,
        jour(11, 20),
    )
    .unwrap();
    offres::ajouter_trouvees(c, "francetravail", &[trouvee("2", "Comptable", "Cabinet", "", "")], None, jour(11, 1))
        .unwrap();
    let sans_brut = "\"id\", \"titre\", \"entreprise\", \"lieu\", \"description\", \"url\", \"favori\", \"plus_tard\", \
                     \"pas_pour_moi\", \"doublon_de\", \"ajoutee_le\", \"retrouvee_le\", \"empreinte\", \"lue_par_ia\"";
    let avant_offres = lignes(c, "offres", sans_brut);
    let toutes = tables(c);
    let autres: Vec<(String, String)> = toutes.into_iter().filter(|(t, _)| t != "offres").collect();
    let avant_autres = vidage(c, &autres);
    let avec_brut = |c: &Connection| -> Vec<String> {
        c.prepare("SELECT titre || ' ' || id_source FROM offres WHERE brut IS NOT NULL ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(avec_brut(c), ["Agent de tri 1", "Comptable 2", "Agent de tri a", "Vendeur 3"]);

    // Avant les 90 jours (le 30 août : 90 jours après le 1er juin) : rien ne change
    assert_eq!(JOURS_BRUT, 90);
    assert_eq!(offres::menage(c, Moment::nouveau(2026, 8, 30, 23, 59, 59).unwrap()).unwrap(), 0);
    assert_eq!(avec_brut(c).len(), 4);
    // Le lendemain : l'offre du 1er juin, pas son doublon du 2 juin
    assert_eq!(offres::menage(c, Moment::nouveau(2026, 8, 31, 0, 0, 0).unwrap()).unwrap(), 1);
    assert_eq!(avec_brut(c), ["Comptable 2", "Agent de tri a", "Vendeur 3"]);
    // Le 26 novembre : le doublon aussi ; l'offre retrouvée le 1er novembre et l'offre récente gardent la leur
    assert_eq!(offres::menage(c, jour(11, 26)).unwrap(), 1);
    assert_eq!(avec_brut(c), ["Comptable 2", "Vendeur 3"]);
    // Une seconde fois : plus rien à faire
    assert_eq!(offres::menage(c, jour(11, 26)).unwrap(), 0);

    // Aucune offre retirée, rien d'autre de changé
    assert_eq!(lignes(c, "offres", sans_brut), avant_offres);
    assert_eq!(vidage(c, &autres), avant_autres);
    let l = offres::lister(c).unwrap();
    assert_eq!(l.len(), 3);
    assert!(l.iter().find(|o| o.id == tri).unwrap().favori);
    let x = offres::lire(c, tri).unwrap().unwrap();
    assert_eq!(x.description, LONG);
    assert_eq!(x.liens.len(), 2);
    assert!(offres::chercher_texte(c, "entrepôt").unwrap().contains(&tri));
    assert!(offres::lire(c, compta).unwrap().is_some());
    // L'offre allégée se met encore à jour : retrouvée sur sa page, sa réponse brute du jour est gardée
    offres::ajouter_page(
        c,
        "francetravail",
        &trouvee("1", "Agent de tri", "Poste", LONG, "{\"neuf\":1}"),
        None,
        jour(11, 26),
    )
    .unwrap();
    assert_eq!(avec_brut(c), ["Agent de tri 1", "Comptable 2", "Vendeur 3"]);
    assert_eq!(offres::menage(c, jour(11, 27)).unwrap(), 0);
}

fn resultat(id: i64, diplome: Option<(Diplome, &str)>, experience: Option<(Experience, &str)>) -> ResultatIa {
    ResultatIa {
        offre_id: id,
        modele: "claude-haiku-5-5".into(),
        etat: "ok",
        jetons_entree: 900,
        jetons_sortie: 60,
        cout_millioniemes: 120,
        reponse: "{}".into(),
        diplome: diplome.map(|(d, p)| (d, p.to_string())),
        experience: experience.map(|(e, p)| (e, p.to_string())),
    }
}

/// Fait comme si une version d'avant des règles avait lu l'offre et posé, à tort, ce diplôme.
fn regles_d_avant(c: &Connection, id: i64, diplome: &str) {
    c.execute(
        "UPDATE offres SET diplome = ?2, diplome_lu_par = 'regles', diplome_preuve = 'chef d''équipe', \
         diplome_confiance = 'moyenne', lecture_version = ?3 WHERE id = ?1",
        (id, diplome, VERSION_REGLES - 1),
    )
    .unwrap();
}

/// Une version d'avant des règles avait posé une valeur fausse : celle de Claude n'a pas pu être écrite par-dessus.
/// Quand la version actuelle des règles ne trouve plus rien à cet endroit, la réponse de Claude gardée est remise :
/// l'offre ne reste pas « non dit », et Claude n'est pas appelé une seconde fois.
#[test]
fn la_reponse_de_claude_gardee_est_remise_quand_les_regles_ne_trouvent_plus_rien() {
    let (b, _) = Base::en_memoire(jour(11, 19)).unwrap();
    let c = b.connexion();
    offres::ajouter_trouvees(
        c,
        "francetravail",
        &[trouvee("1", "Agent de tri", "Poste", LONG, "{}")],
        None,
        jour(11, 19),
    )
    .unwrap();
    let id = offres::lister(c).unwrap()[0].id;
    regles_d_avant(c, id, "bac2");
    // Claude lit l'offre (son expérience n'était pas dite) : il trouve aussi le diplôme, que les règles avaient déjà
    let r = resultat(
        id,
        Some((Diplome::Aucun, "vous préparez les commandes")),
        Some((Experience::Souhaitee, "travail en équipe de cinq personnes")),
    );
    lecture::garder_ia(c, &r, jour(11, 20)).unwrap();
    let x = lecture::lecture_de(c, id).unwrap();
    let d = x.diplome.unwrap();
    // (jamais par-dessus les règles, tant qu'elles disent quelque chose)
    assert_eq!((d.valeur, d.par), (Diplome::Bac2, LuPar::Regles));
    assert_eq!(x.experience.unwrap().par, LuPar::Ia);
    assert!(x.par_ia);
    // La version actuelle des règles relit l'offre et ne trouve plus de diplôme : celui de Claude est remis
    assert_eq!(lecture::relire(c).unwrap(), 1);
    let x = lecture::lecture_de(c, id).unwrap();
    let d = x.diplome.unwrap();
    assert_eq!((d.valeur, d.par, d.preuve.as_str()), (Diplome::Aucun, LuPar::Ia, "vous préparez les commandes"));
    let e = x.experience.unwrap();
    assert_eq!((e.valeur, e.par), (Experience::Souhaitee, LuPar::Ia));
    // Claude n'est pas appelé de nouveau, et rien n'est à comprendre de nouveau
    assert!(lecture::a_lire_par_ia(c, 10).unwrap().is_empty());
    assert!(lecture::a_recomprendre(c, 10).unwrap().is_empty());
    assert_eq!(lecture::ia_du_jour(c, jour(11, 20).jour()).unwrap(), (1, 120));
    // Relue encore (une autre version, un texte changé) : toujours là
    c.execute("UPDATE offres SET lecture_version = NULL", []).unwrap();
    assert_eq!(lecture::relire(c).unwrap(), 1);
    assert_eq!(lecture::lecture_de(c, id).unwrap().diplome.unwrap().valeur, Diplome::Aucun);
}

/// Contre-épreuve : quand les règles trouvent une valeur, c'est elle qui est montrée, jamais celle de Claude ; et une
/// offre que Claude n'a jamais lue redevient simplement « non dit ».
#[test]
fn les_regles_passent_toujours_devant_claude() {
    let (b, _) = Base::en_memoire(jour(11, 19)).unwrap();
    let c = b.connexion();
    offres::ajouter_trouvees(
        c,
        "francetravail",
        &[
            trouvee("1", "Comptable", "Cabinet", "BTS comptabilité exigé, 3 ans d'expérience.", "{}"),
            trouvee("2", "Agent de tri", "Poste", LONG, "{}"),
        ],
        None,
        jour(11, 19),
    )
    .unwrap();
    let l = offres::lister(c).unwrap();
    let compta = l.iter().find(|o| o.titre == "Comptable").unwrap().id;
    let tri = l.iter().find(|o| o.titre == "Agent de tri").unwrap().id;
    lecture::relire(c).unwrap();
    lecture::garder_ia(c, &resultat(compta, Some((Diplome::Bac5, "BTS comptabilité exigé")), None), jour(11, 20))
        .unwrap();
    c.execute("UPDATE offres SET lecture_version = NULL", []).unwrap();
    regles_d_avant(c, tri, "bac2");
    assert_eq!(lecture::relire(c).unwrap(), 2);
    let d = lecture::lecture_de(c, compta).unwrap().diplome.unwrap();
    assert_eq!((d.valeur, d.par), (Diplome::Bac2, LuPar::Regles));
    // (aucune lecture de Claude pour celle-ci : la valeur fausse est retirée, rien n'est inventé)
    let x = lecture::lecture_de(c, tri).unwrap();
    assert!(x.diplome.is_none() && x.experience.is_none() && !x.par_ia);
    assert_eq!(lecture::a_lire_par_ia(c, 10).unwrap().len(), 1);
}

/// Une lecture de Claude faite avant la migration 17 (seul son texte était gardé) : elle est rendue une seule fois
/// pour être comprise de nouveau, ce qu'elle disait est posé là où rien n'est dit, et gardé pour les relectures
/// suivantes.
#[test]
fn une_ancienne_reponse_de_claude_est_comprise_de_nouveau_une_seule_fois() {
    let mut b = Base::en_memoire_jusqua(16, jour(6, 1)).unwrap();
    let (tri, compta) = remplir_version_16(b.connexion());
    // (la version 2 des règles avait posé un diplôme faux sur l'offre, et Claude en avait trouvé un autre)
    regles_d_avant(b.connexion(), tri, "bac2");
    // (une lecture illisible : rien à comprendre)
    b.connexion()
        .execute(
            "INSERT INTO lectures_ia (offre_id, lue_le, modele, etat, reponse) \
             VALUES (?1, '2026-06-03T11:00:00', 'claude-haiku-5-5', 'illisible', 'surcharge')",
            [compta],
        )
        .unwrap();
    assert_eq!(b.migrer_tout(jour(11, 26)).unwrap(), vec![17]);
    let c = b.connexion();
    // Les règles actuelles ne trouvent plus de diplôme : l'offre est « non dit », et Claude ne la relira pas
    lecture::relire(c).unwrap();
    assert!(lecture::lecture_de(c, tri).unwrap().diplome.is_none());
    assert!(lecture::a_lire_par_ia(c, 10).unwrap().iter().all(|(id, ..)| *id != tri));
    // L'ancienne réponse est rendue, avec le titre et le plus long texte de l'offre
    let a = lecture::a_recomprendre(c, 10).unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!((a[0].offre_id, a[0].titre.as_str(), a[0].texte.as_str()), (tri, "Agent de tri", LONG));
    assert_eq!(a[0].reponse, "{\"diplome\":\"aucun\"}");
    assert!(lecture::a_recomprendre(c, 0).unwrap().is_empty());
    // L'appli l'a comprise : « aucun diplôme », posé sur l'offre
    c.execute("UPDATE offres SET pertinence_signature = 'empreinte'", []).unwrap();
    let d = (Diplome::Aucun, "vous préparez les commandes".to_string());
    assert!(lecture::garder_comprise(c, a[0].lecture_id, Some(&d), None).unwrap());
    let x = lecture::lecture_de(c, tri).unwrap();
    let lu = x.diplome.unwrap();
    assert_eq!((lu.valeur, lu.par, lu.preuve.as_str()), (Diplome::Aucun, LuPar::Ia, "vous préparez les commandes"));
    assert!(x.experience.is_none());
    // Sa note sera refaite
    let s: Option<String> =
        c.query_row("SELECT pertinence_signature FROM offres WHERE id = ?1", [tri], |r| r.get(0)).unwrap();
    assert_eq!(s, None);
    // Une seule fois : plus rien à comprendre, et la même lecture donnée de nouveau ne change rien
    assert!(lecture::a_recomprendre(c, 10).unwrap().is_empty());
    let autre = (Diplome::Bac5, "x".to_string());
    assert!(!lecture::garder_comprise(c, a[0].lecture_id, Some(&autre), None).unwrap());
    assert!(!lecture::garder_comprise(c, 9999, Some(&autre), None).unwrap());
    assert_eq!(lecture::lecture_de(c, tri).unwrap().diplome.unwrap().valeur, Diplome::Aucun);
    // Les règles relues plus tard : la réponse gardée reste
    c.execute("UPDATE offres SET lecture_version = NULL", []).unwrap();
    lecture::relire(c).unwrap();
    assert_eq!(lecture::lecture_de(c, tri).unwrap().diplome.unwrap().par, LuPar::Ia);
    // Le compte du jour et le coût ne bougent pas : Claude n'a pas été appelé
    assert_eq!(lecture::ia_du_jour(c, jour(6, 3).jour()).unwrap(), (2, 120));
}

/// Une ancienne réponse qui ne disait rien (ou incompréhensible) : notée comprise, l'offre ne change pas.
#[test]
fn une_ancienne_reponse_qui_ne_dit_rien() {
    let mut b = Base::en_memoire_jusqua(16, jour(6, 1)).unwrap();
    let (tri, _) = remplir_version_16(b.connexion());
    b.migrer_tout(jour(11, 26)).unwrap();
    let c = b.connexion();
    lecture::relire(c).unwrap();
    let a = lecture::a_recomprendre(c, 10).unwrap();
    assert_eq!(a.len(), 1);
    assert!(!lecture::garder_comprise(c, a[0].lecture_id, None, None).unwrap());
    assert!(lecture::a_recomprendre(c, 10).unwrap().is_empty());
    let x = lecture::lecture_de(c, tri).unwrap();
    assert!(x.diplome.is_none() && x.experience.is_none() && x.par_ia);
}
