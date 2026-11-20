//! Les offres dans la base (étape 4 du plan) : l'archive de chaque offre trouvée (gardée même si elle disparaît du
//! site), les doublons rattachés à la première venue, la recherche dans tout le texte (FTS5), favoris, « plus tard » et
//! « Pas pour moi » ; les recherches gardées ; le compte des demandes du jour à chaque source.
//!
//! Une offre déjà gardée (la même source, le même numéro) n'est jamais réécrite : seul le jour où elle a été retrouvée
//! change. Un doublon ([`jobmind_core::offre::meme_offre`]) garde sa ligne (l'archive de sa source) et pointe la
//! première (`doublon_de`) ; ce qui manquait à la première (sa position, son contrat, son salaire...) est pris chez lui.

use std::collections::HashSet;

use jobmind_core::Jour;
use jobmind_core::lecture::{Diplome, Experience};
use jobmind_core::lieux;
use jobmind_core::offre::{
    self, Comparee, Offre, TeleOffre, contrat_lu, departement_lu, empreinte, meme_offre, teletravail_lu,
};
use jobmind_core::recherche::{Contrat, GenreLieu, Lieu, RAYON_DEFAUT};
use jobmind_core::recherches::NouvelleRecherche;
use jobmind_sources::OffreTrouvee;
use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::lecture::{LectureGardee, lecture_de};
use crate::moment::Moment;

/// Ce qu'a donné l'archivage d'une réponse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bilan {
    /// Des offres jamais vues (doublons compris)
    pub nouvelles: usize,
    /// Déjà gardées : seulement retrouvées
    pub retrouvees: usize,
    /// Parmi les nouvelles, rattachées à une offre déjà gardée
    pub fusionnees: usize,
}

impl Bilan {
    pub fn ajouter(&mut self, b: Bilan) {
        self.nouvelles += b.nouvelles;
        self.retrouvees += b.retrouvees;
        self.fusionnees += b.fusionnees;
    }
}

fn vide_en_null(t: &str) -> Option<&str> {
    (!t.trim().is_empty()).then_some(t)
}

/// Garde les offres d'une réponse (dans une transaction : tout ou rien), et note la recherche qui les a trouvées.
pub fn ajouter_trouvees(
    c: &Connection,
    source: &str,
    offres: &[OffreTrouvee],
    recherche: Option<i64>,
    maintenant: Moment,
) -> rusqlite::Result<Bilan> {
    let t = c.unchecked_transaction()?;
    let source_id: i64 = t.query_row("SELECT id FROM sources WHERE code = ?1", [source], |r| r.get(0))?;
    let quand = maintenant.iso();
    let mut b = Bilan::default();
    for o in offres {
        let deja: Option<(i64, Option<i64>)> = t
            .query_row(
                "SELECT id, doublon_de FROM offres WHERE source_id = ?1 AND id_source = ?2",
                params![source_id, o.id_source],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let principale = match deja {
            Some((id, doublon)) => {
                t.execute("UPDATE offres SET retrouvee_le = ?2 WHERE id = ?1", params![id, quand])?;
                b.retrouvees += 1;
                doublon.unwrap_or(id)
            }
            None => {
                let (id, rattachee) = inserer(&t, source, source_id, o, &quand)?;
                b.nouvelles += 1;
                if rattachee.is_some() {
                    b.fusionnees += 1;
                }
                rattachee.unwrap_or(id)
            }
        };
        if let Some(r) = recherche {
            t.execute(
                "INSERT OR IGNORE INTO offres_recherches (offre_id, recherche_id, trouvee_le) \
                 SELECT ?1, ?2, ?3 WHERE EXISTS (SELECT 1 FROM recherches WHERE id = ?2)",
                params![principale, r, quand],
            )?;
        }
    }
    t.commit()?;
    Ok(b)
}

/// Ajoute une offre jamais vue ; la rattache à la première venue si c'est un doublon. Rend son numéro et celui de la
/// première (s'il y en a une).
fn inserer(
    c: &Connection,
    source: &str,
    source_id: i64,
    o: &OffreTrouvee,
    quand: &str,
) -> rusqlite::Result<(i64, Option<i64>)> {
    let contrat = contrat_lu(&o.contrat, &o.titre, &o.description).map(Contrat::code);
    let tele = teletravail_lu(&o.titre, &o.description).map(TeleOffre::code);
    let dep = departement_lu(&o.lieu, &o.zones);
    let emp = empreinte(&o.entreprise, &o.titre, &o.lieu);
    c.execute(
        "INSERT INTO offres (source_id, id_source, url, titre, entreprise, lieu, departement, region, latitude, \
         longitude, contrat, teletravail, salaire, description, publiee_le, ajoutee_le, empreinte, brut, retrouvee_le) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?16)",
        params![
            source_id,
            o.id_source,
            vide_en_null(&o.url),
            o.titre,
            o.entreprise,
            o.lieu,
            dep.map(|d| d.code),
            dep.map(|d| d.region),
            o.latitude,
            o.longitude,
            contrat,
            tele,
            vide_en_null(&o.salaire),
            o.description,
            vide_en_null(&o.publiee_le),
            quand,
            emp,
            o.brut,
        ],
    )?;
    let id = c.last_insert_rowid();
    // Les offres déjà gardées qui pourraient être la même
    let mut r = c.prepare(
        "SELECT o.id, s.code, COALESCE(o.url, ''), o.entreprise, o.titre, o.lieu, o.description FROM offres o \
         JOIN sources s ON s.id = o.source_id WHERE o.doublon_de IS NULL AND o.id != ?1 \
         AND ((?2 IS NOT NULL AND o.empreinte = ?2) OR (?3 IS NOT NULL AND o.url = ?3)) ORDER BY o.id",
    )?;
    let candidates: Vec<(i64, String, String, String, String, String, String)> = r
        .query_map(params![id, emp, vide_en_null(&o.url)], |x| {
            Ok((x.get(0)?, x.get(1)?, x.get(2)?, x.get(3)?, x.get(4)?, x.get(5)?, x.get(6)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let moi = Comparee {
        source,
        url: &o.url,
        entreprise: &o.entreprise,
        titre: &o.titre,
        lieu: &o.lieu,
        description: &o.description,
    };
    let premiere = candidates.iter().find(|(_, s, u, e, t, l, d)| {
        meme_offre(&moi, &Comparee { source: s, url: u, entreprise: e, titre: t, lieu: l, description: d })
    });
    let Some((p, ..)) = premiere else { return Ok((id, None)) };
    c.execute("UPDATE offres SET doublon_de = ?2 WHERE id = ?1", params![id, p])?;
    // (ce qui manquait à la première)
    c.execute(
        "UPDATE offres SET latitude = COALESCE(latitude, ?2), longitude = COALESCE(longitude, ?3), \
         departement = COALESCE(departement, ?4), region = COALESCE(region, ?5), contrat = COALESCE(contrat, ?6), \
         teletravail = COALESCE(teletravail, ?7), salaire = COALESCE(salaire, ?8), \
         publiee_le = COALESCE(publiee_le, ?9), lecture_version = NULL, pertinence_signature = NULL WHERE id = ?1",
        params![
            p,
            o.latitude,
            o.longitude,
            dep.map(|d| d.code),
            dep.map(|d| d.region),
            contrat,
            tele,
            vide_en_null(&o.salaire),
            vide_en_null(&o.publiee_le)
        ],
    )?;
    Ok((id, Some(*p)))
}

/// Une liste de numéros écrite par `group_concat` (« 3,7 »).
fn numeros(t: Option<String>) -> Vec<i64> {
    t.unwrap_or_default().split(',').filter_map(|x| x.trim().parse().ok()).collect()
}

/// Les colonnes lues pour une offre de la liste, dans l'ordre de [`lire_offre`] (`o` : la première de son groupe).
const COLONNES: &str = "o.id, o.titre, o.entreprise, o.lieu, o.departement, o.region, o.latitude, o.longitude, \
    o.contrat, o.teletravail, COALESCE(o.salaire, ''), o.publiee_le, o.ajoutee_le, o.favori, o.plus_tard, \
    o.pas_pour_moi, o.vue, \
    (SELECT group_concat(s.code) FROM offres x JOIN sources s ON s.id = x.source_id \
     WHERE x.id = o.id OR x.doublon_de = o.id), \
    (SELECT group_concat(r.recherche_id) FROM offres_recherches r WHERE r.offre_id = o.id), \
    (SELECT MAX(k.id) FROM candidatures k WHERE k.offre_id = o.id), \
    o.diplome, o.experience, o.pertinence, COALESCE(o.raison, ''), o.hors_sujet, o.pas_hors_sujet";

fn lire_offre(r: &Row) -> rusqlite::Result<Offre> {
    let texte = |i: usize| -> rusqlite::Result<String> { Ok(r.get::<_, Option<String>>(i)?.unwrap_or_default()) };
    let ajoutee = Moment::lire_iso(&texte(12)?).map(|m| m.jour());
    let mut sources: Vec<String> = Vec::new();
    for s in texte(17)?.split(',').filter(|s| !s.is_empty()) {
        if !sources.iter().any(|x| x == s) {
            sources.push(s.to_string());
        }
    }
    Ok(Offre {
        id: r.get(0)?,
        titre: texte(1)?,
        entreprise: texte(2)?,
        lieu: texte(3)?,
        departement: r.get(4)?,
        region: r.get(5)?,
        latitude: r.get(6)?,
        longitude: r.get(7)?,
        contrat: Contrat::depuis_code(&texte(8)?),
        teletravail: TeleOffre::depuis_code(&texte(9)?),
        salaire: texte(10)?,
        publiee_le: Jour::lire_iso(&texte(11)?),
        ajoutee_le: ajoutee.unwrap_or(Jour { annee: 2026, mois: 1, jour: 1 }),
        sources,
        favori: r.get::<_, i64>(13)? == 1,
        plus_tard: r.get::<_, i64>(14)? == 1,
        cachee: r.get::<_, i64>(15)? == 1,
        vue: r.get::<_, i64>(16)? == 1,
        recherches: numeros(r.get(18)?),
        candidature: r.get(19)?,
        diplome: Diplome::depuis_code(&texte(20)?),
        experience: Experience::depuis_code(&texte(21)?),
        pertinence: r.get::<_, Option<i64>>(22)?.map(|n| n.clamp(0, 100) as u8),
        raison: texte(23)?,
        hors_sujet: r.get::<_, i64>(24)? == 1,
        pas_hors_sujet: r.get::<_, i64>(25)? == 1,
    })
}

/// Toutes les offres gardées (une par groupe de doublons), dans l'ordre de la liste.
pub fn lister(c: &Connection) -> rusqlite::Result<Vec<Offre>> {
    let mut r = c.prepare(&format!("SELECT {COLONNES} FROM offres o WHERE o.doublon_de IS NULL"))?;
    let mut l: Vec<Offre> = r.query_map([], lire_offre)?.collect::<rusqlite::Result<_>>()?;
    offre::trier(&mut l);
    Ok(l)
}

/// Le nombre d'offres jamais ouvertes (et pas cachées).
pub fn nombre_nouvelles(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row("SELECT COUNT(*) FROM offres WHERE doublon_de IS NULL AND vue = 0 AND pas_pour_moi = 0", [], |r| {
        r.get::<_, i64>(0)
    })
    .map(|n| n as usize)
}

/// Une source d'une offre ouverte : son code, l'adresse de l'offre chez elle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lien {
    pub source: String,
    pub url: String,
}

/// Une offre ouverte : la ligne de la liste, son texte, ses liens (un par source), le jour où elle a été gardée et
/// retrouvée pour la dernière fois.
#[derive(Debug, Clone, PartialEq)]
pub struct OffreComplete {
    pub offre: Offre,
    /// Le plus long texte du groupe (les sources n'en donnent que le début)
    pub description: String,
    /// La source de ce texte
    pub description_de: String,
    pub liens: Vec<Lien>,
    pub retrouvee_le: Option<Jour>,
    /// Étape 5 : le nombre de candidats que la page montrait au dernier « Ajouter à Jobmind »
    pub candidats: Option<VueCandidats>,
    /// Étape 6 : le diplôme et l'expérience lus, avec leur preuve
    pub lecture: LectureGardee,
    /// Étape 7 : le jour où une alerte mail l'a apportée (rien : venue d'ailleurs)
    pub venue_par_mail: Option<Jour>,
}

/// Le numéro de la première offre du groupe de celle-ci (elle-même si ce n'est pas un doublon).
pub fn principale(c: &Connection, id: i64) -> rusqlite::Result<Option<i64>> {
    c.query_row("SELECT COALESCE(doublon_de, id) FROM offres WHERE id = ?1", [id], |r| r.get(0)).optional()
}

/// Une offre ouverte (celle-ci ou la première de son groupe).
pub fn lire(c: &Connection, id: i64) -> rusqlite::Result<Option<OffreComplete>> {
    let Some(p) = principale(c, id)? else { return Ok(None) };
    let Some(o) =
        c.query_row(&format!("SELECT {COLONNES} FROM offres o WHERE o.id = ?1"), [p], lire_offre).optional()?
    else {
        return Ok(None);
    };
    let mut r = c.prepare(
        "SELECT s.code, COALESCE(x.url, ''), x.description, x.retrouvee_le FROM offres x \
         JOIN sources s ON s.id = x.source_id WHERE x.id = ?1 OR x.doublon_de = ?1 ORDER BY x.id",
    )?;
    let groupe: Vec<(String, String, String, Option<String>)> =
        r.query_map([p], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?, x.get(3)?)))?.collect::<rusqlite::Result<_>>()?;
    let (mut description, mut description_de) = (String::new(), String::new());
    let mut liens: Vec<Lien> = Vec::new();
    let mut retrouvee: Option<Jour> = None;
    for (source, url, texte, quand) in groupe {
        if texte.chars().count() > description.chars().count() {
            description = texte;
            description_de = source.clone();
        }
        if !liens.iter().any(|l| l.source == source && l.url == url) {
            liens.push(Lien { source, url });
        }
        let q = quand.as_deref().and_then(Moment::lire_iso).map(|m| m.jour());
        retrouvee = retrouvee.max(q);
    }
    let candidats = candidats(c, p)?;
    let lecture = lecture_de(c, p)?;
    let venue_par_mail = crate::mails::venue_par_mail(c, p)?;
    Ok(Some(OffreComplete {
        offre: o,
        description,
        description_de,
        liens,
        retrouvee_le: retrouvee,
        candidats,
        lecture,
        venue_par_mail,
    }))
}

/// Le nombre de candidats qu'une page montrait, avec le site et le jour (étape 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VueCandidats {
    /// Le code du site (« linkedin »)
    pub site: String,
    /// Ce que la page disait, en mots simples (« 34 candidats », « Plus de 100 candidats »)
    pub texte: String,
    pub nombre: Option<i64>,
    pub vu_le: Jour,
}

/// Le dernier nombre de candidats vu pour ce groupe d'offres (le plus récent clic qui en montrait un).
pub fn candidats(c: &Connection, principale: i64) -> rusqlite::Result<Option<VueCandidats>> {
    c.query_row(
        "SELECT COALESCE(v.site, ''), v.texte_candidats, v.nombre_candidats, v.vue_le FROM offres_vues v \
         JOIN offres x ON x.id = v.offre_id WHERE (x.id = ?1 OR x.doublon_de = ?1) AND v.texte_candidats IS NOT NULL \
         ORDER BY v.vue_le DESC, v.id DESC LIMIT 1",
        [principale],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<i64>>(2)?, r.get::<_, String>(3)?)),
    )
    .optional()
    .map(|x| {
        x.and_then(|(site, texte, nombre, quand)| {
            Moment::lire_iso(&quand).map(|m| VueCandidats { site, texte, nombre, vu_le: m.jour() })
        })
    })
}

/// Ce qu'a donné « Ajouter à Jobmind » (étape 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AjoutPage {
    /// La première offre du groupe (celle que la liste montre)
    pub id: i64,
    /// Jamais vue de ce site : ajoutée (sinon : mise à jour)
    pub nouvelle: bool,
    /// Ajoutée, mais rattachée à une offre déjà gardée (la même, venue d'une autre source)
    pub fusionnee: bool,
}

/// Garde l'offre lue sur une page (« Ajouter à Jobmind ») dans l'archive, dans une transaction : la même table que
/// les autres sources (`source` : le code du site), la même fusion des doublons. Une offre déjà gardée de
/// ce site (le même numéro) est mise à jour : son texte s'il est plus long aujourd'hui, ce qui lui manquait, le jour où
/// elle a été revue. Chaque clic note ce que la page montrait (le nombre de candidats, s'il y en a un) dans
/// `offres_vues`.
pub fn ajouter_page(
    c: &Connection,
    source: &str,
    o: &OffreTrouvee,
    candidats: Option<(&str, Option<i64>)>,
    maintenant: Moment,
) -> rusqlite::Result<AjoutPage> {
    let t = c.unchecked_transaction()?;
    let source_id: i64 = t.query_row("SELECT id FROM sources WHERE code = ?1", [source], |r| r.get(0))?;
    let quand = maintenant.iso();
    let deja: Option<(i64, Option<i64>)> = t
        .query_row(
            "SELECT id, doublon_de FROM offres WHERE source_id = ?1 AND id_source = ?2",
            params![source_id, o.id_source],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let (ligne, ajout) = match deja {
        Some((id, doublon)) => {
            let contrat = contrat_lu(&o.contrat, &o.titre, &o.description).map(Contrat::code);
            let tele = teletravail_lu(&o.titre, &o.description).map(TeleOffre::code);
            t.execute(
                "UPDATE offres SET retrouvee_le = ?2, \
                 description = CASE WHEN length(?3) > length(description) THEN ?3 ELSE description END, \
                 entreprise = CASE WHEN entreprise = '' THEN ?4 ELSE entreprise END, \
                 lieu = CASE WHEN lieu = '' THEN ?5 ELSE lieu END, \
                 salaire = COALESCE(salaire, ?6), url = COALESCE(url, ?7), publiee_le = COALESCE(publiee_le, ?8), \
                 contrat = COALESCE(contrat, ?9), teletravail = COALESCE(teletravail, ?10), brut = ?11 WHERE id = ?1",
                params![
                    id,
                    quand,
                    o.description,
                    o.entreprise,
                    o.lieu,
                    vide_en_null(&o.salaire),
                    vide_en_null(&o.url),
                    vide_en_null(&o.publiee_le),
                    contrat,
                    tele,
                    o.brut
                ],
            )?;
            (id, AjoutPage { id: doublon.unwrap_or(id), nouvelle: false, fusionnee: false })
        }
        None => {
            let (id, rattachee) = inserer(&t, source, source_id, o, &quand)?;
            (id, AjoutPage { id: rattachee.unwrap_or(id), nouvelle: true, fusionnee: rattachee.is_some() })
        }
    };
    let (texte, nombre) = match candidats {
        Some((t, n)) => (Some(t), n),
        None => (None, None),
    };
    t.execute(
        "INSERT INTO offres_vues (offre_id, vue_le, site, nombre_candidats, texte_candidats) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![ligne, quand, source, nombre, texte],
    )?;
    // (étape 6 : le texte a pu changer, la première du groupe est relue)
    t.execute("UPDATE offres SET lecture_version = NULL, pertinence_signature = NULL WHERE id = ?1", [ajout.id])?;
    t.commit()?;
    Ok(ajout)
}

/// La requête FTS5 d'un texte tapé : chaque mot (8 au plus) doit commencer un mot de l'offre. Rien sans mot.
pub fn requete_texte(t: &str) -> Option<String> {
    let mots: Vec<String> = t
        .split(|c: char| !c.is_alphanumeric())
        .filter(|m| !m.is_empty())
        .take(8)
        .map(|m| format!("\"{}\"*", m.chars().take(40).collect::<String>()))
        .collect();
    (!mots.is_empty()).then(|| mots.join(" "))
}

/// Les offres (les premières de leur groupe) dont le titre, l'entreprise, le lieu ou le texte contient tous les mots
/// tapés (sans tenir compte des accents ni des majuscules).
pub fn chercher_texte(c: &Connection, t: &str) -> rusqlite::Result<HashSet<i64>> {
    let Some(q) = requete_texte(t) else { return Ok(HashSet::new()) };
    let mut r = c.prepare(
        "SELECT DISTINCT COALESCE(o.doublon_de, o.id) FROM offres_fts f JOIN offres o ON o.id = f.rowid \
         WHERE offres_fts MATCH ?1",
    )?;
    r.query_map([q], |x| x.get(0))?.collect()
}

/// Une marque d'offre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marque {
    Favori,
    PlusTard,
    PasPourMoi,
    /// Étape 6 : « Pas hors sujet »
    PasHorsSujet,
}

impl Marque {
    fn colonne(self) -> &'static str {
        match self {
            Marque::Favori => "favori",
            Marque::PlusTard => "plus_tard",
            Marque::PasPourMoi => "pas_pour_moi",
            Marque::PasHorsSujet => "pas_hors_sujet",
        }
    }
}

/// Pose ou retire une marque (sur la première du groupe). Rend la nouvelle valeur (rien : l'offre n'existe pas).
pub fn basculer(c: &Connection, id: i64, m: Marque) -> rusqlite::Result<Option<bool>> {
    let Some(p) = principale(c, id)? else { return Ok(None) };
    let col = m.colonne();
    c.execute(&format!("UPDATE offres SET {col} = 1 - {col} WHERE id = ?1"), [p])?;
    c.query_row(&format!("SELECT {col} FROM offres WHERE id = ?1"), [p], |r| r.get::<_, i64>(0)).map(|v| Some(v == 1))
}

/// Pose une marque à cette valeur, sans basculer (sur la première du groupe). Rend vrai si elle a changé (rien :
/// l'offre n'existe pas).
pub fn poser(c: &Connection, id: i64, m: Marque, oui: bool) -> rusqlite::Result<Option<bool>> {
    let Some(p) = principale(c, id)? else { return Ok(None) };
    let col = m.colonne();
    let n =
        c.execute(&format!("UPDATE offres SET {col} = ?1 WHERE id = ?2 AND {col} <> ?1"), params![i64::from(oui), p])?;
    Ok(Some(n > 0))
}

/// Note l'offre comme ouverte. Faux si elle n'existe pas.
pub fn marquer_vue(c: &Connection, id: i64) -> rusqlite::Result<bool> {
    let Some(p) = principale(c, id)? else { return Ok(false) };
    c.execute("UPDATE offres SET vue = 1 WHERE id = ?1", [p])?;
    Ok(true)
}

/// Rattache une candidature à son offre.
pub fn lier_candidature(c: &Connection, offre_id: i64, candidature_id: i64) -> rusqlite::Result<bool> {
    let Some(p) = principale(c, offre_id)? else { return Ok(false) };
    Ok(c.execute("UPDATE candidatures SET offre_id = ?1 WHERE id = ?2", params![p, candidature_id])? == 1)
}

// ---------------------------------------------------------------------------------------------------------------
// Les recherches gardées
// ---------------------------------------------------------------------------------------------------------------

/// Une recherche gardée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RechercheGardee {
    pub id: i64,
    pub nom: String,
    pub mots: String,
    pub lieu: Lieu,
}

/// Les recherches gardées, dans l'ordre où elles ont été créées (une ligne illisible est laissée).
pub fn lister_recherches(c: &Connection) -> rusqlite::Result<Vec<RechercheGardee>> {
    let mut r = c.prepare(
        "SELECT id, nom, mots_cles, genre_lieu, lieu, rayon_km, departement, region FROM recherches ORDER BY id",
    )?;
    let l = r.query_map([], |x| {
        let genre: String = x.get(3)?;
        let ville: String = x.get(4)?;
        let rayon: i64 = x.get(5)?;
        let dep: String = x.get(6)?;
        let reg: String = x.get(7)?;
        let lieu = match GenreLieu::depuis_code(&genre) {
            Some(GenreLieu::Ville) if !ville.is_empty() => {
                Some(Lieu::Ville { nom: ville, rayon: u32::try_from(rayon).unwrap_or(RAYON_DEFAUT).max(1) })
            }
            Some(GenreLieu::Departement) => lieux::departement(&dep).map(Lieu::Departement),
            Some(GenreLieu::Region) => lieux::region(&reg).map(Lieu::Region),
            Some(GenreLieu::Partout) => Some(Lieu::Partout),
            _ => None,
        };
        Ok(lieu.map(|lieu| RechercheGardee {
            id: x.get(0).unwrap_or_default(),
            nom: x.get(1).unwrap_or_default(),
            mots: x.get(2).unwrap_or_default(),
            lieu,
        }))
    })?;
    Ok(l.filter_map(|x| x.ok().flatten()).collect())
}

/// Le lieu en colonnes : (genre, ville, rayon, département, région).
fn colonnes_lieu(l: &Lieu) -> (&'static str, String, i64, &'static str, &'static str) {
    match l {
        Lieu::Ville { nom, rayon } => ("ville", nom.clone(), i64::from(*rayon), "", ""),
        Lieu::Departement(d) => ("departement", String::new(), 0, d.code, ""),
        Lieu::Region(r) => ("region", String::new(), 0, "", r.code),
        Lieu::Partout => ("partout", String::new(), 0, "", ""),
    }
}

pub fn ajouter_recherche(c: &Connection, n: &NouvelleRecherche, maintenant: Moment) -> rusqlite::Result<i64> {
    let (g, v, r, d, reg) = colonnes_lieu(&n.lieu);
    c.execute(
        "INSERT INTO recherches (nom, mots_cles, genre_lieu, lieu, rayon_km, departement, region, cree_le) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![n.nom, n.mots, g, v, r, d, reg, maintenant.iso()],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn modifier_recherche(c: &Connection, id: i64, n: &NouvelleRecherche) -> rusqlite::Result<bool> {
    let (g, v, r, d, reg) = colonnes_lieu(&n.lieu);
    Ok(c.execute(
        "UPDATE recherches SET nom = ?2, mots_cles = ?3, genre_lieu = ?4, lieu = ?5, rayon_km = ?6, \
         departement = ?7, region = ?8 WHERE id = ?1",
        params![id, n.nom, n.mots, g, v, r, d, reg],
    )? == 1)
}

/// Supprime une recherche gardée (ses offres restent gardées).
pub fn supprimer_recherche(c: &Connection, id: i64) -> rusqlite::Result<bool> {
    let t = c.unchecked_transaction()?;
    t.execute("DELETE FROM offres_recherches WHERE recherche_id = ?1", [id])?;
    let n = t.execute("DELETE FROM recherches WHERE id = ?1", [id])?;
    t.commit()?;
    Ok(n == 1)
}

pub fn nombre_recherches(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row("SELECT COUNT(*) FROM recherches", [], |r| r.get::<_, i64>(0)).map(|n| n as usize)
}
