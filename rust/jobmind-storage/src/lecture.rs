//! La lecture et le tri des offres dans la base (étape 6 du plan).
//!
//! - [`relire`] : chaque offre (la première de son groupe de doublons, avec le plus long texte du groupe) pas encore lue
//!   par la version actuelle des règles est lue ([`jobmind_core::lecture::lire`]) ; ce que Claude avait trouvé reste
//!   quand les règles ne trouvent rien, et sa réponse gardée est remise là où une version d'avant des règles avait
//!   posé une valeur que la version actuelle ne trouve plus ;
//! - [`renoter`] : chaque offre dont la note a été faite avec un autre « Ce que je cherche » ou un autre CV (une autre
//!   empreinte) est notée de nouveau ;
//! - [`a_lire_par_ia`], [`garder_ia`] : les offres que les règles n'ont pas su classer, données à Claude une seule
//!   fois chacune, et ce qu'il a rendu (le plafond du jour : [`ia_du_jour`]) ;
//! - [`a_recomprendre`], [`garder_comprise`] : les réponses de Claude gardées avant la migration 17 (seul leur texte
//!   était gardé), à comprendre de nouveau par l'appli, une seule fois, sans rien redemander à Claude.

use jobmind_core::Jour;
use jobmind_core::lecture::{self, Confiance, Diplome, Experience, LuPar, VERSION_REGLES};
use jobmind_core::offre::TeleOffre;
use jobmind_core::pertinence::{Niveau, Note};
use jobmind_core::recherche::Contrat;
use rusqlite::{Connection, OptionalExtension, params};

use crate::moment::Moment;

/// Le plus long texte du groupe de doublons de l'offre `o`.
const TEXTE_DU_GROUPE: &str = "(SELECT x.description FROM offres x WHERE x.id = o.id OR x.doublon_de = o.id \
     ORDER BY length(x.description) DESC, x.id LIMIT 1)";

/// Une offre à lire : son numéro, son titre, le plus long texte de son groupe, et qui avait lu son diplôme et son
/// expérience.
type ALire = (i64, String, String, Option<String>, Option<String>);

/// Lit les offres que la version actuelle des règles n'a pas encore lues. Rend leur nombre.
pub fn relire(c: &Connection) -> rusqlite::Result<usize> {
    let a_lire: Vec<ALire> = {
        let mut r = c.prepare(&format!(
            "SELECT o.id, o.titre, {TEXTE_DU_GROUPE}, o.diplome_lu_par, o.experience_lue_par FROM offres o \
             WHERE o.doublon_de IS NULL AND (o.lecture_version IS NULL OR o.lecture_version < ?1)"
        ))?;
        r.query_map([VERSION_REGLES], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?, x.get(3)?, x.get(4)?)))?
            .collect::<rusqlite::Result<_>>()?
    };
    if a_lire.is_empty() {
        return Ok(0);
    }
    let t = c.unchecked_transaction()?;
    for (id, titre, texte, d_par, e_par) in &a_lire {
        let l = lecture::lire(titre, texte);
        match &l.diplome {
            Some(d) => {
                t.execute(
                    "UPDATE offres SET diplome = ?2, diplome_lu_par = 'regles', diplome_preuve = ?3, \
                     diplome_confiance = ?4 WHERE id = ?1",
                    params![id, d.valeur.code(), d.preuve, d.confiance.code()],
                )?;
            }
            None => match garde_par_ia(&t, *id, Champ::Diplome)? {
                // (les règles ne trouvent rien, ou plus rien : la réponse de Claude gardée est remise)
                Some((valeur, preuve)) => {
                    t.execute(
                        "UPDATE offres SET diplome = ?2, diplome_lu_par = 'ia', diplome_preuve = ?3, \
                         diplome_confiance = 'moyenne' WHERE id = ?1",
                        params![id, valeur, preuve],
                    )?;
                }
                // (ce que Claude avait trouvé reste : une lecture d'avant la migration 17, pas encore comprise de
                // nouveau)
                None if d_par.as_deref() == Some("ia") => {}
                None => {
                    t.execute(
                        "UPDATE offres SET diplome = NULL, diplome_lu_par = NULL, diplome_preuve = NULL, \
                         diplome_confiance = NULL WHERE id = ?1",
                        [id],
                    )?;
                }
            },
        }
        match &l.experience {
            Some(e) => {
                t.execute(
                    "UPDATE offres SET experience = ?2, experience_lue_par = 'regles', experience_preuve = ?3, \
                     experience_confiance = ?4 WHERE id = ?1",
                    params![id, e.valeur.code(), e.preuve, e.confiance.code()],
                )?;
            }
            None => match garde_par_ia(&t, *id, Champ::Experience)? {
                Some((valeur, preuve)) => {
                    t.execute(
                        "UPDATE offres SET experience = ?2, experience_lue_par = 'ia', experience_preuve = ?3, \
                         experience_confiance = 'moyenne' WHERE id = ?1",
                        params![id, valeur, preuve],
                    )?;
                }
                None if e_par.as_deref() == Some("ia") => {}
                None => {
                    t.execute(
                        "UPDATE offres SET experience = NULL, experience_lue_par = NULL, experience_preuve = NULL, \
                         experience_confiance = NULL WHERE id = ?1",
                        [id],
                    )?;
                }
            },
        }
        let autres = (!l.autres.is_empty()).then(|| l.autres.join(", "));
        t.execute(
            "UPDATE offres SET autres_demandes = ?2, lecture_version = ?3, pertinence_signature = NULL WHERE id = ?1",
            params![id, autres, VERSION_REGLES],
        )?;
    }
    t.commit()?;
    Ok(a_lire.len())
}

/// Un champ lu dans une offre.
#[derive(Clone, Copy)]
enum Champ {
    Diplome,
    Experience,
}

/// Ce que Claude a trouvé pour ce champ d'une offre, gardé avec sa lecture (migration 17) : le code de la valeur et sa
/// preuve, de la lecture la plus récente qui en dit quelque chose. Une valeur que Jobmind ne connaît pas (une base
/// abîmée) n'est pas rendue.
fn garde_par_ia(c: &Connection, offre: i64, champ: Champ) -> rusqlite::Result<Option<(String, String)>> {
    let sql = match champ {
        Champ::Diplome => {
            "SELECT diplome, COALESCE(diplome_preuve, '') FROM lectures_ia \
             WHERE offre_id = ?1 AND diplome IS NOT NULL ORDER BY id DESC LIMIT 1"
        }
        Champ::Experience => {
            "SELECT experience, COALESCE(experience_preuve, '') FROM lectures_ia \
             WHERE offre_id = ?1 AND experience IS NOT NULL ORDER BY id DESC LIMIT 1"
        }
    };
    let lu: Option<(String, String)> =
        c.prepare_cached(sql)?.query_row([offre], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
    Ok(lu.filter(|(v, _)| match champ {
        Champ::Diplome => Diplome::depuis_code(v).is_some(),
        Champ::Experience => Experience::depuis_code(v).is_some(),
    }))
}

/// Ce que la note d'une offre demande, lu dans la base.
#[derive(Debug, Clone, Default)]
pub struct ANoter {
    pub id: i64,
    pub titre: String,
    pub description: String,
    pub lieu: String,
    pub departement: Option<String>,
    pub region: Option<String>,
    pub point: Option<(f64, f64)>,
    pub contrat: Option<Contrat>,
    pub teletravail: Option<TeleOffre>,
    pub salaire: String,
    pub diplome: Option<Diplome>,
    pub experience: Option<Experience>,
}

/// Note les offres dont la note a été faite avec une autre empreinte ([`jobmind_core::pertinence::signature`]). Rend
/// leur nombre.
pub fn renoter(c: &Connection, signature: &str, noter: impl Fn(&ANoter) -> Note) -> rusqlite::Result<usize> {
    let a_noter: Vec<ANoter> = {
        let mut r = c.prepare(&format!(
            "SELECT o.id, o.titre, {TEXTE_DU_GROUPE}, o.lieu, o.departement, o.region, o.latitude, o.longitude, \
             o.contrat, o.teletravail, COALESCE(o.salaire, ''), o.diplome, o.experience FROM offres o \
             WHERE o.doublon_de IS NULL AND (o.pertinence_signature IS NULL OR o.pertinence_signature != ?1)"
        ))?;
        r.query_map([signature], |x| {
            let lat: Option<f64> = x.get(6)?;
            let lon: Option<f64> = x.get(7)?;
            Ok(ANoter {
                id: x.get(0)?,
                titre: x.get(1)?,
                description: x.get::<_, Option<String>>(2)?.unwrap_or_default(),
                lieu: x.get(3)?,
                departement: x.get(4)?,
                region: x.get(5)?,
                point: lat.zip(lon),
                contrat: Contrat::depuis_code(&x.get::<_, Option<String>>(8)?.unwrap_or_default()),
                teletravail: TeleOffre::depuis_code(&x.get::<_, Option<String>>(9)?.unwrap_or_default()),
                salaire: x.get(10)?,
                diplome: Diplome::depuis_code(&x.get::<_, Option<String>>(11)?.unwrap_or_default()),
                experience: Experience::depuis_code(&x.get::<_, Option<String>>(12)?.unwrap_or_default()),
            })
        })?
        .collect::<rusqlite::Result<_>>()?
    };
    if a_noter.is_empty() {
        return Ok(0);
    }
    let t = c.unchecked_transaction()?;
    for o in &a_noter {
        let n = noter(o);
        t.execute(
            "UPDATE offres SET pertinence = ?2, raison = ?3, hors_sujet = ?4, pertinence_signature = ?5 WHERE id = ?1",
            params![o.id, n.score.min(100), n.raison, n.niveau == Niveau::HorsSujet, signature],
        )?;
    }
    t.commit()?;
    Ok(a_noter.len())
}

/// Retire toutes les notes (« Ce que je cherche » n'est pas rempli : rien pour noter).
pub fn effacer_notes(c: &Connection) -> rusqlite::Result<usize> {
    c.execute(
        "UPDATE offres SET pertinence = NULL, raison = NULL, hors_sujet = 0, pertinence_signature = NULL \
         WHERE pertinence IS NOT NULL OR pertinence_signature IS NOT NULL OR hors_sujet = 1",
        [],
    )
}

/// Ce qui a été lu dans une offre, avec sa preuve, sa confiance et qui l'a lu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lu<T> {
    pub valeur: T,
    pub preuve: String,
    pub confiance: Confiance,
    pub par: LuPar,
}

/// La lecture gardée d'une offre.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LectureGardee {
    pub diplome: Option<Lu<Diplome>>,
    pub experience: Option<Lu<Experience>>,
    /// « Permis B, CACES 3 »
    pub autres: String,
    /// Lue par les règles (pas encore : rien n'est dit)
    pub lue: bool,
    /// Déjà donnée à Claude
    pub par_ia: bool,
}

/// La lecture gardée d'une offre (la première de son groupe).
pub fn lecture_de(c: &Connection, id: i64) -> rusqlite::Result<LectureGardee> {
    let r = c
        .query_row(
            "SELECT diplome, diplome_lu_par, diplome_preuve, diplome_confiance, experience, experience_lue_par, \
             experience_preuve, experience_confiance, COALESCE(autres_demandes, ''), lecture_version, lue_par_ia \
             FROM offres WHERE id = ?1",
            [id],
            |x| {
                let s =
                    |i: usize| -> rusqlite::Result<String> { Ok(x.get::<_, Option<String>>(i)?.unwrap_or_default()) };
                let diplome = Diplome::depuis_code(&s(0)?).map(|valeur| Lu {
                    valeur,
                    par: LuPar::depuis_code(&s(1).unwrap_or_default()).unwrap_or(LuPar::Regles),
                    preuve: s(2).unwrap_or_default(),
                    confiance: Confiance::depuis_code(&s(3).unwrap_or_default()).unwrap_or(Confiance::Moyenne),
                });
                let experience = Experience::depuis_code(&s(4)?).map(|valeur| Lu {
                    valeur,
                    par: LuPar::depuis_code(&s(5).unwrap_or_default()).unwrap_or(LuPar::Regles),
                    preuve: s(6).unwrap_or_default(),
                    confiance: Confiance::depuis_code(&s(7).unwrap_or_default()).unwrap_or(Confiance::Moyenne),
                });
                Ok(LectureGardee {
                    diplome,
                    experience,
                    autres: s(8)?,
                    lue: x.get::<_, Option<i64>>(9)?.is_some(),
                    par_ia: x.get::<_, i64>(10)? == 1,
                })
            },
        )
        .optional()?;
    Ok(r.unwrap_or_default())
}

/// Le texte d'une offre est assez long pour que Claude y trouve quelque chose (un simple résumé n'en dit pas
/// assez).
pub const TEXTE_MIN_IA: usize = 200;

const FLOUES: &str = "o.doublon_de IS NULL AND o.pas_pour_moi = 0 AND o.lue_par_ia = 0 \
    AND o.lecture_version IS NOT NULL AND (o.diplome IS NULL OR o.experience IS NULL) \
    AND NOT (o.hors_sujet = 1 AND o.pas_hors_sujet = 0)";

/// Les offres floues (les règles n'ont pas trouvé le diplôme ou l'expérience, pas encore données à Claude, ni
/// cachées ni hors sujet), les plus pertinentes d'abord : leur numéro, leur titre et le plus long texte du groupe.
pub fn a_lire_par_ia(c: &Connection, max: usize) -> rusqlite::Result<Vec<(i64, String, String)>> {
    let mut r = c.prepare(&format!(
        "SELECT o.id, o.titre, {TEXTE_DU_GROUPE} AS t FROM offres o WHERE {FLOUES} AND length(t) >= ?1 \
         ORDER BY COALESCE(o.pertinence, -1) DESC, o.id DESC LIMIT ?2"
    ))?;
    r.query_map(params![TEXTE_MIN_IA as i64, max as i64], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)))?.collect()
}

/// Le nombre d'offres floues que Claude pourrait lire.
pub fn nombre_floues(c: &Connection) -> rusqlite::Result<usize> {
    c.query_row(
        &format!("SELECT COUNT(*) FROM offres o WHERE {FLOUES} AND length({TEXTE_DU_GROUPE}) >= ?1"),
        [TEXTE_MIN_IA as i64],
        |x| x.get::<_, i64>(0),
    )
    .map(|n| n as usize)
}

/// Ce que Claude a rendu pour une offre, déjà vérifié (une citation absente de l'offre : rien gardé).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultatIa {
    pub offre_id: i64,
    pub modele: String,
    /// « ok », « citation_inventee », « illisible »
    pub etat: &'static str,
    pub jetons_entree: u32,
    pub jetons_sortie: u32,
    pub cout_millioniemes: u64,
    /// La réponse telle quelle (coupée), pour le journal de la base
    pub reponse: String,
    pub diplome: Option<(Diplome, String)>,
    pub experience: Option<(Experience, String)>,
}

/// Garde ce que Claude a rendu : la lecture notée (une seule fois par offre), et ce qu'il a trouvé seulement là où
/// les règles n'avaient rien trouvé (« lu par Claude », confiance moyenne). Ce qu'il a trouvé est aussi gardé avec sa
/// lecture, même là où les règles ont déjà une valeur : si une version suivante des règles ne trouve plus rien à cet
/// endroit, [`relire`] remet la réponse de Claude, sans la lui redemander.
pub fn garder_ia(c: &Connection, r: &ResultatIa, maintenant: Moment) -> rusqlite::Result<()> {
    let t = c.unchecked_transaction()?;
    let reponse: String = r.reponse.chars().take(4000).collect();
    t.execute(
        "INSERT INTO lectures_ia (offre_id, lue_le, modele, etat, jetons_entree, jetons_sortie, cout_millioniemes, \
         reponse, diplome, diplome_preuve, experience, experience_preuve, comprise) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1)",
        params![
            r.offre_id,
            maintenant.iso(),
            r.modele,
            r.etat,
            r.jetons_entree,
            r.jetons_sortie,
            i64::try_from(r.cout_millioniemes).unwrap_or(i64::MAX),
            reponse,
            r.diplome.as_ref().map(|(d, _)| d.code()),
            r.diplome.as_ref().map(|(_, p)| p),
            r.experience.as_ref().map(|(e, _)| e.code()),
            r.experience.as_ref().map(|(_, p)| p),
        ],
    )?;
    t.execute("UPDATE offres SET lue_par_ia = 1, pertinence_signature = NULL WHERE id = ?1", [r.offre_id])?;
    poser_ia(&t, r.offre_id, r.diplome.as_ref(), r.experience.as_ref())?;
    t.commit()
}

/// Pose ce que Claude a trouvé sur une offre, seulement là où rien n'est dit (jamais par-dessus les règles). Rend le
/// nombre de champs remplis.
fn poser_ia(
    c: &Connection,
    offre: i64,
    diplome: Option<&(Diplome, String)>,
    experience: Option<&(Experience, String)>,
) -> rusqlite::Result<usize> {
    let mut n = 0;
    if let Some((d, preuve)) = diplome {
        n += c.execute(
            "UPDATE offres SET diplome = ?2, diplome_lu_par = 'ia', diplome_preuve = ?3, diplome_confiance = 'moyenne' \
             WHERE id = ?1 AND diplome IS NULL",
            params![offre, d.code(), preuve],
        )?;
    }
    if let Some((e, preuve)) = experience {
        n += c.execute(
            "UPDATE offres SET experience = ?2, experience_lue_par = 'ia', experience_preuve = ?3, \
             experience_confiance = 'moyenne' WHERE id = ?1 AND experience IS NULL",
            params![offre, e.code(), preuve],
        )?;
    }
    Ok(n)
}

/// Une réponse de Claude gardée avant la migration 17 : seul son texte était gardé, pas ce qu'elle disait. L'appli la
/// comprend de nouveau (comme le jour de la lecture : `jobmind_llm::lecture::comprendre`, avec le titre et le texte de
/// l'offre), puis donne le résultat à [`garder_comprise`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ARecomprendre {
    /// Le numéro de la lecture (table `lectures_ia`)
    pub lecture_id: i64,
    pub offre_id: i64,
    pub titre: String,
    /// Le plus long texte du groupe de l'offre, comme celui donné à Claude
    pub texte: String,
    /// La réponse de Claude, telle quelle
    pub reponse: String,
}

/// Les réponses de Claude pas encore comprises de nouveau (les plus anciennes d'abord, `max` au plus). Une réponse
/// notée illisible n'a rien à dire : elle n'est pas rendue. Après la première fois, la liste est vide : chaque
/// nouvelle lecture ([`garder_ia`]) garde tout de suite ce qu'elle dit.
pub fn a_recomprendre(c: &Connection, max: usize) -> rusqlite::Result<Vec<ARecomprendre>> {
    let mut r = c.prepare(&format!(
        "SELECT l.id, o.id, o.titre, COALESCE({TEXTE_DU_GROUPE}, ''), COALESCE(l.reponse, '') FROM lectures_ia l \
         JOIN offres o ON o.id = l.offre_id WHERE l.comprise = 0 AND l.etat != 'illisible' ORDER BY l.id LIMIT ?1"
    ))?;
    r.query_map([i64::try_from(max).unwrap_or(i64::MAX)], |x| {
        Ok(ARecomprendre {
            lecture_id: x.get(0)?,
            offre_id: x.get(1)?,
            titre: x.get(2)?,
            texte: x.get(3)?,
            reponse: x.get(4)?,
        })
    })?
    .collect()
}

/// Garde ce qu'une ancienne réponse de Claude disait (rien : elle ne disait rien, ou elle n'a pas pu être comprise),
/// la note comprise (elle ne sera plus rendue par [`a_recomprendre`]), et le pose sur son offre là où rien n'est dit
/// aujourd'hui (jamais par-dessus les règles). Vrai si l'offre a changé (sa note sera refaite). Une lecture inconnue
/// ou déjà comprise : rien, et faux.
pub fn garder_comprise(
    c: &Connection,
    lecture_id: i64,
    diplome: Option<&(Diplome, String)>,
    experience: Option<&(Experience, String)>,
) -> rusqlite::Result<bool> {
    let t = c.unchecked_transaction()?;
    let offre: Option<i64> = t
        .query_row("SELECT offre_id FROM lectures_ia WHERE id = ?1 AND comprise = 0", [lecture_id], |r| r.get(0))
        .optional()?;
    let Some(offre) = offre else { return Ok(false) };
    t.execute(
        "UPDATE lectures_ia SET diplome = ?2, diplome_preuve = ?3, experience = ?4, experience_preuve = ?5, \
         comprise = 1 WHERE id = ?1",
        params![
            lecture_id,
            diplome.map(|(d, _)| d.code()),
            diplome.map(|(_, p)| p),
            experience.map(|(e, _)| e.code()),
            experience.map(|(_, p)| p),
        ],
    )?;
    let poses = poser_ia(&t, offre, diplome, experience)?;
    if poses > 0 {
        t.execute("UPDATE offres SET pertinence_signature = NULL WHERE id = ?1", [offre])?;
    }
    t.commit()?;
    Ok(poses > 0)
}

/// Les lectures de Claude faites ce jour-là, et leur coût en millionièmes de dollar.
pub fn ia_du_jour(c: &Connection, jour: Jour) -> rusqlite::Result<(u32, u64)> {
    c.query_row(
        "SELECT COUNT(*), COALESCE(SUM(cout_millioniemes), 0) FROM lectures_ia WHERE substr(lue_le, 1, 10) = ?1",
        [jour.iso()],
        |x| Ok((x.get::<_, i64>(0)?.max(0) as u32, x.get::<_, i64>(1)?.max(0) as u64)),
    )
}
