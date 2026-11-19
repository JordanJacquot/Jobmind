//! La réécriture dans le style de Jordan (étape 9), en deux temps : ses règles fixes d'abord ([`avec_regles`] : les
//! remplacements, puis le plancher), puis Claude guidé par ses propres textes, et ses règles une seconde fois.
//!
//! Rien d'inventé : Claude reçoit la lettre phrase par phrase, chacune avec son numéro, et rend chaque phrase réécrite
//! avec les numéros des phrases de départ qu'elle reprend (`depuis`) et, si elle parle de Jordan, les passages de son
//! CV qui la prouvent. [`assembler`] vérifie chaque phrase rendue ([`jobmind_core::preparation::verifier_reecrite`] :
//! aucun nombre, nom propre ou fait nouveau) :
//! - une phrase qui reprend une phrase « à vérifier » de la lettre de départ reste à vérifier (jamais blanchie) ;
//! - une phrase qui ajoute quelque chose (un nombre, un nom, un fait absents du CV, de l'offre et de la lettre de
//!   départ) ou un mot familier est refusée : les phrases de départ qu'elle reprenait sont remises telles quelles, et
//!   une note le dit à Jordan ; sans phrase de départ connue, elle reste mais surlignée « à vérifier » (un mot familier
//!   sans phrase de départ : retirée).
//!
//! La formule d'appel, la formule de politesse et la signature ne passent jamais par Claude : elles restent celles de la
//! lettre de départ (ou celles du plancher si elles manquent).

use jobmind_core::offre::normaliser;
use jobmind_core::preparation::{PHRASE_MAX, PhraseRendue, Raison, SourcesReecriture, propre, verifier_reecrite};

use crate::plancher::{self, Probleme};
use crate::regles::Regles;
use crate::texte::{Structure, structure};

/// Une phrase de la lettre de départ, et sa raison si elle est « à vérifier ».
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhraseDepart {
    pub texte: String,
    pub a_verifier: Option<String>,
}

/// La lettre de départ découpée.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Depart {
    pub appel: Option<String>,
    pub paragraphes: Vec<Vec<PhraseDepart>>,
    pub fin: Option<String>,
}

fn proche(a: &str, b: &str) -> bool {
    let (a, b) = (normaliser(a), normaliser(b));
    let (a, b) =
        (a.split_whitespace().collect::<Vec<_>>().join(" "), b.split_whitespace().collect::<Vec<_>>().join(" "));
    !a.is_empty() && !b.is_empty() && (a.contains(&b) || b.contains(&a))
}

impl Depart {
    /// La lettre découpée ; une phrase est à vérifier si elle contient une des phrases à vérifier données (texte et
    /// raison), ou y est contenue.
    pub fn depuis_lettre(lettre: &str, a_verifier: &[(String, String)]) -> Depart {
        let s = structure(lettre);
        let paragraphes = s
            .paragraphes
            .iter()
            .map(|p| {
                p.iter()
                    .map(|x| PhraseDepart {
                        texte: x.clone(),
                        a_verifier: a_verifier.iter().find(|(t, _)| proche(x, t)).map(|(_, r)| r.clone()),
                    })
                    .collect()
            })
            .collect();
        Depart { appel: s.appel, paragraphes, fin: s.fin }
    }

    /// Les phrases, dans l'ordre (la phrase n est la (n - 1)-ième).
    pub fn phrases(&self) -> Vec<&PhraseDepart> {
        self.paragraphes.iter().flatten().collect()
    }

    /// Aucune phrase.
    pub fn est_vide(&self) -> bool {
        self.paragraphes.iter().all(Vec::is_empty)
    }

    /// Les phrases numérotées pour Claude (« [1] ... », une ligne vide entre deux paragraphes).
    pub fn numerotee(&self) -> String {
        let mut n = 0;
        self.paragraphes
            .iter()
            .map(|p| {
                p.iter()
                    .map(|x| {
                        n += 1;
                        format!("[{n}] {}", x.texte)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// La lettre entière.
    pub fn texte(&self) -> String {
        self.structure(self.paragraphes.iter().map(|p| p.iter().map(|x| x.texte.clone()).collect()).collect())
    }

    fn structure(&self, paragraphes: Vec<Vec<String>>) -> String {
        Structure { appel: self.appel.clone(), paragraphes, fin: self.fin.clone() }.texte()
    }
}

/// Une phrase rendue par Claude : son texte, si elle parle de Jordan, les passages cités, les phrases de départ
/// qu'elle reprend (leurs numéros).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PhraseNouvelle {
    pub texte: String,
    pub sur_moi: bool,
    pub extraits: Vec<String>,
    pub depuis: Vec<usize>,
}

/// Une phrase à vérifier de la version dans mon style : son texte, la raison, son code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AVerifier {
    pub texte: String,
    pub raison: String,
    pub code: String,
}

/// La version dans le style de Jordan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stylee {
    pub texte: String,
    pub a_verifier: Vec<AVerifier>,
    /// Ce que Jordan doit savoir (« Une phrase est remise comme avant : ... »)
    pub notes: Vec<String>,
    /// Les phrases refusées (remises comme avant, ou retirées)
    pub refusees: usize,
    /// Les changements faits par les règles
    pub changements: usize,
}

const HERITEE: &str = "Elle reprend une phrase à vérifier de ta lettre de départ.";

/// Une phrase passée par les règles puis le plancher. Rend le texte et le nombre de changements.
fn regler(t: &str, regles: &Regles) -> (String, usize) {
    let (x, n) = regles.appliquer(t);
    (plancher::corriger(&x), n)
}

/// La lettre assemblée, avec sa formule d'appel et sa formule de politesse.
fn finir(d: &Depart, paragraphes: Vec<Vec<String>>) -> String {
    let p: Vec<Vec<String>> = paragraphes.into_iter().filter(|x| !x.is_empty()).collect();
    plancher::assurer(&d.structure(p))
}

/// Le premier temps : seulement les règles de Jordan et le plancher, sans Claude (marche sans clé). Les phrases à
/// vérifier le restent.
pub fn avec_regles(d: &Depart, regles: &Regles) -> Stylee {
    let mut changements = 0;
    let mut a_verifier = Vec::new();
    let paragraphes: Vec<Vec<String>> = d
        .paragraphes
        .iter()
        .map(|p| {
            p.iter()
                .map(|x| {
                    let (t, n) = regler(&x.texte, regles);
                    changements += n;
                    if let Some(r) = &x.a_verifier {
                        a_verifier.push(AVerifier { texte: t.clone(), raison: r.clone(), code: "depart".into() });
                    }
                    t
                })
                .collect()
        })
        .collect();
    let mut notes = Vec::new();
    match changements {
        0 => notes.push("Tes règles n'ont rien trouvé à changer.".to_string()),
        1 => notes.push("Tes règles ont fait un changement.".to_string()),
        n => notes.push(format!("Tes règles ont fait {n} changements.")),
    }
    Stylee { texte: finir(d, paragraphes), a_verifier, notes, refusees: 0, changements }
}

/// La lettre de départ passée par les règles (le premier temps), gardée découpée : c'est elle que Claude réécrit, avec
/// les mêmes numéros.
pub fn depart_regle(d: &Depart, regles: &Regles) -> Depart {
    Depart {
        appel: d.appel.clone(),
        paragraphes: d
            .paragraphes
            .iter()
            .map(|p| {
                p.iter()
                    .map(|x| PhraseDepart { texte: regler(&x.texte, regles).0, a_verifier: x.a_verifier.clone() })
                    .collect()
            })
            .collect(),
        fin: d.fin.clone(),
    }
}

/// Ce que Jobmind a pour vérifier une réécriture : le CV et le profil, l'offre, les textes de Jordan.
#[derive(Debug, Clone, Copy)]
pub struct Sources<'a> {
    pub profil: &'a str,
    pub offre: &'a str,
    pub vocabulaire: &'a str,
}

/// Le second temps : la réécriture de Claude vérifiée, assemblée, passée par les règles et le plancher. Rien : aucune
/// phrase utilisable.
pub fn assembler(d: &Depart, rendu: &[Vec<PhraseNouvelle>], s: Sources, regles: &Regles) -> Option<Stylee> {
    let depart = d.phrases();
    let texte_depart = d.texte();
    let sr = SourcesReecriture { profil: s.profil, offre: s.offre, depart: &texte_depart, vocabulaire: s.vocabulaire };
    // (les phrases de départ déjà reprises par une phrase gardée, ou déjà remises)
    let mut couvertes = vec![false; depart.len()];
    let mut paragraphes: Vec<Vec<String>> = Vec::new();
    let mut a_verifier: Vec<AVerifier> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut refusees = 0;
    let mut changements = 0;
    let mut phrases = 0;
    for p in rendu {
        let mut para: Vec<String> = Vec::new();
        for x in p {
            let (t, n) = regler(&propre(&x.texte, PHRASE_MAX), regles);
            if t.is_empty() {
                continue;
            }
            let sources: Vec<usize> = x.depuis.iter().copied().filter(|i| (1..=depart.len()).contains(i)).collect();
            let heritee = sources.iter().find_map(|i| depart[i - 1].a_verifier.clone());
            let familier = plancher::mots_a_revoir(&t).into_iter().next();
            let verifiee = verifier_reecrite(
                &PhraseRendue { texte: t.clone(), sur_moi: x.sur_moi, extraits: x.extraits.clone() },
                sr,
            );
            let raison: Option<Raison> = verifiee.as_ref().and_then(|v| v.a_verifier.clone());
            let ajout = match (&familier, &raison) {
                (Some(Probleme::Familier(_, _, m)) | Some(Probleme::Tutoiement(_, _, m)), _) => {
                    Some(format!("un mot familier « {m} »"))
                }
                // (un mot nouveau peut n'être qu'un autre mot pour la même chose : surligné, pas refusé)
                (_, Some(r)) if heritee.is_none() && !matches!(r, Raison::MotAbsent(_)) => Some(r.ajout()),
                _ => None,
            };
            match ajout {
                None => {
                    for i in &sources {
                        couvertes[i - 1] = true;
                    }
                    if let Some(r) = heritee {
                        let raison = if r.is_empty() { HERITEE.to_string() } else { r };
                        a_verifier.push(AVerifier { texte: t.clone(), raison, code: "depart".into() });
                    } else if let Some(r @ Raison::MotAbsent(m)) = &raison {
                        let raison = format!(
                            "« {m} » n'était ni dans ta lettre ni dans ton CV : garde-le seulement si c'est vrai."
                        );
                        a_verifier.push(AVerifier { texte: t.clone(), raison, code: r.code().into() });
                    }
                    changements += n;
                    para.push(t);
                    phrases += 1;
                }
                Some(quoi) => {
                    refusees += 1;
                    let remises: Vec<usize> = sources.iter().copied().filter(|i| !couvertes[i - 1]).collect();
                    if !sources.is_empty() && remises.is_empty() {
                        notes.push(format!("Une phrase est retirée : la réécriture ajoutait {quoi}."));
                    } else if !sources.is_empty() {
                        for i in remises {
                            couvertes[i - 1] = true;
                            let pd = depart[i - 1];
                            if let Some(r) = &pd.a_verifier {
                                a_verifier.push(AVerifier {
                                    texte: pd.texte.clone(),
                                    raison: r.clone(),
                                    code: "depart".into(),
                                });
                            }
                            para.push(pd.texte.clone());
                            phrases += 1;
                        }
                        notes.push(format!("Une phrase est remise comme avant : la réécriture ajoutait {quoi}."));
                    } else if familier.is_some() {
                        notes.push(format!("Une phrase est retirée : elle contenait {quoi}."));
                    } else if let Some(r) = raison {
                        a_verifier.push(AVerifier { texte: t.clone(), raison: r.texte(), code: r.code().into() });
                        para.push(t);
                        phrases += 1;
                    }
                }
            }
        }
        if !para.is_empty() {
            paragraphes.push(para);
        }
    }
    if phrases == 0 {
        return None;
    }
    Some(Stylee { texte: finir(d, paragraphes), a_verifier, notes, refusees, changements })
}

#[cfg(test)]
mod tests;
