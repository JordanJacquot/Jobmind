//! Les offres des sites déjà analysées (migration 15, 0.2.0) : une offre lue en coulisse (lecture automatique ou
//! clic) est gardée ici, pour ne pas la relire à chaque lancement de Jobmind (dix minutes pour 300 offres). Elle est
//! reconnue par son site et son identifiant chez lui (jobmind_sources::page::cle_offre), comme une offre retirée.
//!
//! - Le contenu gardé est la page lue ([`PageLue`]), écrite en JSON à la main, avec le numéro de son format
//!   ([`FORMAT`]) : une ligne d'un autre format, ou illisible, est ignorée (l'offre sera relue), jamais une erreur.
//! - Une analyse sert [`JOURS_GARDEES`] jours : après, elle est ignorée puis effacée ([`effacer_anciennes`]).
//! - « Déjà vue » : le moment où la personne a ouvert l'offre elle-même (`vue_le`, [`marquer_vue`]). Il est gardé
//!   plus longtemps que le contenu ([`JOURS_VUES_GARDEES`]) : une offre restée en ligne plus d'un mois est relue, mais
//!   elle ne revient pas comme une offre jamais vue.

use jobmind_sources::OffreTrouvee;
use jobmind_sources::page::{Candidats, LuPar, PageLue, Site};
use jobmind_sources::texte::{DESCRIPTION_MAX, LIGNE_MAX};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::moment::Moment;

/// Le numéro du format du contenu gardé (à augmenter quand [`PageLue`] change : les anciennes lignes sont ignorées).
pub const FORMAT: i64 = 1;
/// Une analyse sert ce nombre de jours, puis elle est effacée.
pub const JOURS_GARDEES: i64 = 30;
/// « Déjà vue » est retenu ce nombre de jours (la ligne ne garde alors plus que la clé de l'offre et cette date).
pub const JOURS_VUES_GARDEES: i64 = 365;
/// Les tailles gardées, au plus : une adresse, la réponse brute du site (comme jobmind_sources::page), les zones.
const URL_MAX: usize = 2000;
const BRUT_MAX: usize = 60_000;
const ZONES_MAX: usize = 12;

/// Une offre analysée relue de la base.
#[derive(Debug, Clone, PartialEq)]
pub struct Analyse {
    /// Le code du site (« francetravail »)
    pub site: String,
    /// Son identifiant chez le site (sinon son adresse remise au propre)
    pub id_site: String,
    pub url: String,
    pub page: PageLue,
    pub analysee_le: String,
    /// Le moment où la personne l'a ouverte (vide : jamais)
    pub vue_le: String,
}

fn couper(t: &str, max: usize) -> String {
    t.chars().take(max).collect()
}

/// Le moment d'il y a `jours` jours, écrit comme dans la base (les dates s'y comparent comme des textes).
fn il_y_a(maintenant: Moment, jours: i64) -> String {
    format!(
        "{}T{:02}:{:02}:{:02}",
        maintenant.jour().plus(-jours).iso(),
        maintenant.heure,
        maintenant.minute,
        maintenant.seconde
    )
}

/// La page lue, en JSON (format [`FORMAT`]), ses textes coupés aux tailles du dépôt.
fn en_json(p: &PageLue) -> String {
    let o = &p.offre;
    json!({
        "site": p.site.code(),
        "lu_par": p.lu_par.code(),
        "site_entreprise": couper(&p.site_entreprise, URL_MAX),
        "candidats": p.candidats.as_ref().map(|c| json!({ "nombre": c.nombre, "texte": couper(&c.texte, LIGNE_MAX) })),
        "offre": {
            "id_source": couper(&o.id_source, URL_MAX),
            "titre": couper(&o.titre, LIGNE_MAX),
            "entreprise": couper(&o.entreprise, LIGNE_MAX),
            "lieu": couper(&o.lieu, LIGNE_MAX),
            "zones": o.zones.iter().take(ZONES_MAX).map(|z| couper(z, LIGNE_MAX)).collect::<Vec<_>>(),
            "url": couper(&o.url, URL_MAX),
            "description": couper(&o.description, DESCRIPTION_MAX),
            "contrat": couper(&o.contrat, LIGNE_MAX),
            "salaire": couper(&o.salaire, LIGNE_MAX),
            "publiee_le": couper(&o.publiee_le, LIGNE_MAX),
            "latitude": o.latitude,
            "longitude": o.longitude,
            "brut": couper(&o.brut, BRUT_MAX),
        },
    })
    .to_string()
}

/// Le site d'un code gardé (« francetravail », « site » pour un autre site) ; rien pour un code inconnu.
fn site_du_code(code: &str) -> Option<Site> {
    Site::CONNUS.iter().copied().chain([Site::Autre]).find(|s| s.code() == code)
}

/// La page lue relue de son JSON ; rien si ce n'en est pas une (le site, la façon de lire, le titre et le texte de
/// l'offre doivent y être).
fn depuis_json(t: &str) -> Option<PageLue> {
    let v: Value = serde_json::from_str(t).ok()?;
    let site = site_du_code(v.get("site")?.as_str()?)?;
    let lu_par = match v.get("lu_par")?.as_str()? {
        "donnees" => LuPar::Donnees,
        "regles" => LuPar::Regles,
        "texte" => LuPar::Texte,
        _ => return None,
    };
    let o = v.get("offre")?.as_object()?;
    let texte = |k: &str, max: usize| couper(o.get(k).and_then(Value::as_str).unwrap_or_default(), max);
    let nombre = |k: &str| o.get(k).and_then(Value::as_f64).filter(|x| x.is_finite());
    o.get("titre")?.as_str()?;
    o.get("description")?.as_str()?;
    let zones = o
        .get("zones")
        .and_then(Value::as_array)
        .map(|z| z.iter().filter_map(Value::as_str).take(ZONES_MAX).map(|x| couper(x, LIGNE_MAX)).collect())
        .unwrap_or_default();
    let candidats = v.get("candidats").filter(|c| c.is_object()).map(|c| Candidats {
        nombre: c.get("nombre").and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()),
        texte: couper(c.get("texte").and_then(Value::as_str).unwrap_or_default(), LIGNE_MAX),
    });
    Some(PageLue {
        site,
        offre: OffreTrouvee {
            id_source: texte("id_source", URL_MAX),
            titre: texte("titre", LIGNE_MAX),
            entreprise: texte("entreprise", LIGNE_MAX),
            lieu: texte("lieu", LIGNE_MAX),
            zones,
            url: texte("url", URL_MAX),
            description: texte("description", DESCRIPTION_MAX),
            contrat: texte("contrat", LIGNE_MAX),
            salaire: texte("salaire", LIGNE_MAX),
            publiee_le: texte("publiee_le", LIGNE_MAX),
            latitude: nombre("latitude"),
            longitude: nombre("longitude"),
            brut: texte("brut", BRUT_MAX),
        },
        candidats,
        lu_par,
        site_entreprise: couper(v.get("site_entreprise").and_then(Value::as_str).unwrap_or_default(), URL_MAX),
    })
}

/// Écrit l'analyse d'une offre, ou remplace celle d'avant (sa date d'analyse devient `maintenant` ; « Déjà vue »
/// reste comme il était).
pub fn ecrire(
    c: &Connection,
    site: &str,
    id_site: &str,
    url: &str,
    page: &PageLue,
    maintenant: Moment,
) -> rusqlite::Result<()> {
    c.execute(
        "INSERT INTO offres_analysees (site, id_site, url, format, contenu, analysee_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT (site, id_site) DO UPDATE SET url = excluded.url, format = excluded.format, \
         contenu = excluded.contenu, analysee_le = excluded.analysee_le",
        params![site, id_site, couper(url.trim(), URL_MAX), FORMAT, en_json(page), maintenant.iso()],
    )?;
    Ok(())
}

/// Les analyses gardées pour ces clés (le code du site, l'identifiant chez lui), dans l'ordre demandé. Une clé sans
/// analyse, une analyse d'un autre format, illisible, ou de plus de [`JOURS_GARDEES`] jours : rien pour elle.
pub fn lire(c: &Connection, cles: &[(String, String)], maintenant: Moment) -> rusqlite::Result<Vec<Analyse>> {
    let limite = il_y_a(maintenant, JOURS_GARDEES);
    let mut r = c.prepare(
        "SELECT url, format, contenu, analysee_le, vue_le FROM offres_analysees WHERE site = ?1 AND id_site = ?2",
    )?;
    let mut trouvees = Vec::new();
    for (site, id_site) in cles {
        let ligne: Option<(String, i64, String, String, String)> = r
            .query_row(params![site, id_site], |l| Ok((l.get(0)?, l.get(1)?, l.get(2)?, l.get(3)?, l.get(4)?)))
            .optional()?;
        let Some((url, format, contenu, analysee_le, vue_le)) = ligne else { continue };
        if format != FORMAT || analysee_le < limite {
            continue;
        }
        let Some(page) = depuis_json(&contenu) else { continue };
        trouvees.push(Analyse { site: site.clone(), id_site: id_site.clone(), url, page, analysee_le, vue_le });
    }
    Ok(trouvees)
}

/// Note que la personne a ouvert cette offre (« Déjà vue »), même si son analyse n'est pas gardée : la ligne est alors
/// posée sans contenu (format 0, jamais relue comme une analyse). La date de la première fois reste. Vrai si l'offre
/// vient d'être notée.
pub fn marquer_vue(c: &Connection, site: &str, id_site: &str, url: &str, maintenant: Moment) -> rusqlite::Result<bool> {
    let n = c.execute(
        "INSERT INTO offres_analysees (site, id_site, url, analysee_le, vue_le) VALUES (?1, ?2, ?3, ?4, ?4) \
         ON CONFLICT (site, id_site) DO UPDATE SET vue_le = excluded.vue_le WHERE offres_analysees.vue_le = ''",
        params![site, id_site, couper(url.trim(), URL_MAX), maintenant.iso()],
    )?;
    Ok(n == 1)
}

/// Les clés (le code du site, l'identifiant chez lui) des offres que la personne a ouvertes.
pub fn vues(c: &Connection) -> rusqlite::Result<Vec<(String, String)>> {
    let mut r = c.prepare("SELECT site, id_site FROM offres_analysees WHERE vue_le <> ''")?;
    r.query_map([], |l| Ok((l.get(0)?, l.get(1)?)))?.collect()
}

/// Le ménage : les analyses de plus de [`JOURS_GARDEES`] jours sont effacées. Celles d'une offre ouverte par la
/// personne depuis moins de [`JOURS_VUES_GARDEES`] jours gardent leur ligne, vidée de son contenu, pour « Déjà vue ».
/// Rend le nombre d'analyses effacées.
pub fn effacer_anciennes(c: &Connection, maintenant: Moment) -> rusqlite::Result<usize> {
    let (limite, limite_vues) = (il_y_a(maintenant, JOURS_GARDEES), il_y_a(maintenant, JOURS_VUES_GARDEES));
    // (une date vide, jamais vue, est avant toutes les autres)
    let effacees =
        c.execute("DELETE FROM offres_analysees WHERE analysee_le < ?1 AND vue_le < ?2", params![limite, limite_vues])?;
    let videes = c.execute(
        "UPDATE offres_analysees SET format = 0, contenu = '' WHERE analysee_le < ?1 AND format <> 0",
        params![limite],
    )?;
    Ok(effacees + videes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Base;

    fn m(jour: u32, h: u32) -> Moment {
        Moment::nouveau(2026, 11, jour, h, 0, 0).unwrap()
    }

    fn page(titre: &str) -> PageLue {
        PageLue {
            site: Site::FranceTravail,
            offre: OffreTrouvee {
                id_source: "123ABC".into(),
                titre: titre.into(),
                entreprise: "Entrepôts de Moselle".into(),
                lieu: "57 - Metz".into(),
                zones: vec!["France".into(), "Moselle".into()],
                url: "https://candidat.francetravail.fr/offres/recherche/detail/123ABC".into(),
                description: "Conduite de chariot.\nDébutant accepté, « sans diplôme ».".into(),
                contrat: "CDI".into(),
                salaire: "1 900 € par mois".into(),
                publiee_le: "2026-11-18".into(),
                latitude: Some(49.5),
                longitude: None,
                brut: "{\"page\":\"x\"}".into(),
            },
            candidats: Some(Candidats { nombre: Some(34), texte: "34 candidats".into() }),
            lu_par: LuPar::Donnees,
            site_entreprise: "https://www.exemple.fr/".into(),
        }
    }

    fn cles(l: &[&str]) -> Vec<(String, String)> {
        l.iter().map(|id| ("francetravail".to_string(), id.to_string())).collect()
    }

    const URL: &str = "https://candidat.francetravail.fr/offres/recherche/detail/123ABC";

    #[test]
    fn la_migration_15_sur_une_base_de_la_version_14() {
        let mut b = Base::en_memoire_jusqua(14, m(19, 9)).unwrap();
        assert_eq!(b.migrer_tout(m(19, 10)).unwrap(), vec![15, 16, 17]);
        assert!(lire(b.connexion(), &cles(&["123ABC"]), m(19, 10)).unwrap().is_empty());
        assert!(vues(b.connexion()).unwrap().is_empty());
    }

    /// Une analyse écrite se relit pareille (tous ses champs), elle est remplacée par une seconde écriture, et les
    /// clés sans analyse ne donnent rien.
    #[test]
    fn ecrire_relire_remplacer() {
        let (b, _) = Base::en_memoire(m(19, 9)).unwrap();
        let c = b.connexion();
        let p = page("Cariste H/F");
        ecrire(c, "francetravail", "123ABC", URL, &p, m(19, 9)).unwrap();
        let l = lire(c, &cles(&["inconnue", "123ABC"]), m(19, 10)).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].page, p);
        assert_eq!((l[0].id_site.as_str(), l[0].url.as_str()), ("123ABC", URL));
        assert_eq!((l[0].analysee_le.as_str(), l[0].vue_le.as_str()), ("2026-11-19T09:00:00", ""));
        // Sans candidats, sans coordonnées, lue par les règles : pareil
        let mut q = page("Cariste de nuit");
        q.candidats = None;
        q.offre.latitude = None;
        q.lu_par = LuPar::Regles;
        ecrire(c, "francetravail", "123ABC", URL, &q, m(20, 9)).unwrap();
        let l = lire(c, &cles(&["123ABC"]), m(20, 10)).unwrap();
        assert_eq!((l.len(), &l[0].page, l[0].analysee_le.as_str()), (1, &q, "2026-11-20T09:00:00"));
        let n: i64 = c.query_row("SELECT COUNT(*) FROM offres_analysees", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
    }

    /// Ce qui est écrit est coupé aux tailles du dépôt.
    #[test]
    fn les_textes_sont_coupes() {
        let (b, _) = Base::en_memoire(m(19, 9)).unwrap();
        let c = b.connexion();
        let mut p = page(&"t".repeat(5000));
        p.offre.description = "é".repeat(DESCRIPTION_MAX + 500);
        p.offre.brut = "b".repeat(BRUT_MAX + 500);
        p.offre.zones = (0..40).map(|i| format!("zone {i}")).collect();
        ecrire(c, "francetravail", "123ABC", &"u".repeat(5000), &p, m(19, 9)).unwrap();
        let l = lire(c, &cles(&["123ABC"]), m(19, 9)).unwrap();
        let o = &l[0].page.offre;
        assert_eq!(o.titre.chars().count(), LIGNE_MAX);
        assert_eq!(o.description.chars().count(), DESCRIPTION_MAX);
        assert_eq!(o.brut.chars().count(), BRUT_MAX);
        assert_eq!(o.zones.len(), ZONES_MAX);
        assert_eq!(l[0].url.chars().count(), URL_MAX);
    }

    /// Une ligne d'un autre format, un contenu illisible, incomplet ou d'un site inconnu : ignorés, sans erreur.
    #[test]
    fn un_autre_format_ou_un_contenu_illisible_est_ignore() {
        let (b, _) = Base::en_memoire(m(19, 9)).unwrap();
        let c = b.connexion();
        let p = page("Cariste H/F");
        for id in ["A", "B", "C", "D", "E"] {
            ecrire(c, "francetravail", id, URL, &p, m(19, 9)).unwrap();
        }
        c.execute("UPDATE offres_analysees SET format = ?1 WHERE id_site = 'A'", params![FORMAT + 1]).unwrap();
        c.execute("UPDATE offres_analysees SET contenu = 'pas du JSON' WHERE id_site = 'B'", []).unwrap();
        c.execute("UPDATE offres_analysees SET contenu = '{\"site\":\"francetravail\"}' WHERE id_site = 'C'", [])
            .unwrap();
        c.execute(
            "UPDATE offres_analysees SET contenu = REPLACE(contenu, '\"site\":\"francetravail\"', '\"site\":\"inconnu\"') \
             WHERE id_site = 'D'",
            [],
        )
        .unwrap();
        let l = lire(c, &cles(&["A", "B", "C", "D", "E"]), m(19, 10)).unwrap();
        assert_eq!(l.iter().map(|a| a.id_site.as_str()).collect::<Vec<_>>(), ["E"]);
        // (la ligne d'un autre format est remplacée par l'analyse suivante)
        ecrire(c, "francetravail", "A", URL, &p, m(19, 11)).unwrap();
        assert_eq!(lire(c, &cles(&["A"]), m(19, 11)).unwrap().len(), 1);
    }

    /// Après 30 jours une analyse est ignorée, puis effacée par le ménage ; une plus récente reste.
    #[test]
    fn les_analyses_de_plus_de_30_jours_sont_ignorees_puis_effacees() {
        let (b, _) = Base::en_memoire(m(1, 9)).unwrap();
        let c = b.connexion();
        let p = page("Cariste H/F");
        let vieux = Moment::nouveau(2026, 10, 20, 9, 0, 0).unwrap();
        let limite = Moment::nouveau(2026, 10, 26, 9, 0, 0).unwrap();
        ecrire(c, "francetravail", "VIEILLE", URL, &p, vieux).unwrap();
        ecrire(c, "francetravail", "LIMITE", URL, &p, limite).unwrap();
        ecrire(c, "francetravail", "RECENTE", URL, &p, m(20, 9)).unwrap();
        // Le 25/11 à 9 h : celle du 20/10 a plus de 30 jours, celle du 26/10 à 9 h en a 30 tout juste
        let l = lire(c, &cles(&["VIEILLE", "LIMITE", "RECENTE"]), m(25, 9)).unwrap();
        assert_eq!(l.iter().map(|a| a.id_site.as_str()).collect::<Vec<_>>(), ["LIMITE", "RECENTE"]);
        assert_eq!(effacer_anciennes(c, m(25, 9)).unwrap(), 1);
        assert_eq!(effacer_anciennes(c, m(25, 9)).unwrap(), 0);
        let n: i64 = c.query_row("SELECT COUNT(*) FROM offres_analysees", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
        assert_eq!(JOURS_GARDEES, 30);
    }

    /// « Déjà vue » : noté une fois (la première date reste), gardé quand l'analyse est remplacée, et retenu après le
    /// ménage des 30 jours (la ligne est vidée, l'offre sera relue) ; effacé au bout d'un an.
    #[test]
    fn deja_vue_est_garde_plus_longtemps_que_l_analyse() {
        let (b, _) = Base::en_memoire(m(1, 9)).unwrap();
        let c = b.connexion();
        let p = page("Cariste H/F");
        let vieux = Moment::nouveau(2026, 10, 1, 9, 0, 0).unwrap();
        ecrire(c, "francetravail", "A", URL, &p, vieux).unwrap();
        assert!(marquer_vue(c, "francetravail", "A", URL, vieux).unwrap());
        assert!(!marquer_vue(c, "francetravail", "A", URL, m(2, 9)).unwrap(), "une seule fois");
        // (une offre vue dont l'analyse n'est pas gardée : sa ligne est posée, sans contenu)
        assert!(marquer_vue(c, "indeed", "abc", "https://fr.indeed.com/viewjob?jk=abc", m(2, 9)).unwrap());
        let mut v = vues(c).unwrap();
        v.sort();
        assert_eq!(v, [("francetravail".to_string(), "A".to_string()), ("indeed".to_string(), "abc".to_string())]);
        assert!(lire(c, &[("indeed".into(), "abc".into())], m(2, 10)).unwrap().is_empty(), "pas une analyse");
        // L'analyse remplacée : toujours vue, à sa première date
        ecrire(c, "francetravail", "A", URL, &p, Moment::nouveau(2026, 10, 2, 9, 0, 0).unwrap()).unwrap();
        let l = lire(c, &cles(&["A"]), Moment::nouveau(2026, 10, 3, 9, 0, 0).unwrap()).unwrap();
        assert_eq!(l[0].vue_le, "2026-10-01T09:00:00");
        // Le ménage, plus de 30 jours après : l'analyse est vidée (ignorée), « Déjà vue » reste
        assert_eq!(effacer_anciennes(c, m(25, 9)).unwrap(), 1);
        assert!(lire(c, &cles(&["A"]), m(25, 9)).unwrap().is_empty());
        assert_eq!(vues(c).unwrap().len(), 2);
        let taille: i64 =
            c.query_row("SELECT LENGTH(contenu) FROM offres_analysees WHERE id_site = 'A'", [], |r| r.get(0)).unwrap();
        assert_eq!(taille, 0);
        // Relue ensuite : son analyse revient, toujours vue
        ecrire(c, "francetravail", "A", URL, &p, m(25, 10)).unwrap();
        assert_eq!(lire(c, &cles(&["A"]), m(25, 11)).unwrap()[0].vue_le, "2026-10-01T09:00:00");
        // Plus d'un an après : tout est effacé
        let tard = Moment::nouveau(2027, 12, 1, 9, 0, 0).unwrap();
        assert_eq!(effacer_anciennes(c, tard).unwrap(), 2);
        assert!(vues(c).unwrap().is_empty());
    }
}
