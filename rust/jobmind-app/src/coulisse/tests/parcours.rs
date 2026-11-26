//! 0.2.0 : « proche de ton parcours » sur les offres d'un site : le bandeau des cartes (en mots, jamais un chiffre),
//! la phrase de l'offre ouverte, le tri « Les plus proches de ton parcours d'abord », et le cas sans CV de bout en
//! bout (aucun bandeau, pas ce tri, la ligne qui invite à ajouter son CV, rien ne casse). Listes écrites à la main,
//! CV inventés.

use jobmind_core::parcours::{Parcours, lire as lire_cv};

use super::choix::TriSite;
use super::doublons_filtres::{carte, liste_de_pages};
use super::lecture_auto::{lignes, rangs};
use super::*;

const CV: &str = "Lucien Tartempion
EXPÉRIENCES
2019 - 2022 : Cariste chez Logi-Fiction
- Conduite de chariots élévateurs
- Chargement et déchargement des camions
2016 - 2019 : Préparateur de commandes - Entrepôts Imaginaires
FORMATION
2014 : CAP Agent d'entreposage
PERMIS
Permis B, CACES 1, 3 et 5";

/// Cinq offres analysées : le métier de la personne avec son CACES, un métier voisin, deux sans rapport, et le même
/// métier avec un diplôme hors de portée.
fn cinq_offres() -> Coulisse {
    liste_de_pages(&[
        (
            (
                "Comptable H/F",
                "Cabinet Chiffres",
                "57 - Metz",
                "CDI",
                "2026-11-18",
                "Tenue de la comptabilité de nos clients, déclarations de TVA. BTS comptabilité exigé, deux ans d'expérience.",
            ),
            None,
        ),
        (
            (
                "Magasinier H/F",
                "Pièces Auto Seille",
                "57 - Woippy",
                "CDD",
                "2026-11-15",
                "Réception des marchandises, tenue du magasin et service au comptoir. Débutant accepté.",
            ),
            None,
        ),
        (
            (
                "Cariste H/F",
                "Logipôle Lorraine",
                "57 - Metz",
                "CDI",
                "2026-11-12",
                "Chargement et déchargement des camions, rangement des palettes. CACES 3 exigé. Débutant accepté.",
            ),
            None,
        ),
        (
            (
                "Soudeur TIG",
                "Métal Moselle",
                "57 - Ennery",
                "Intérim",
                "2026-11-10",
                "Soudure sur inox à partir de plans, meulage et finition. Lecture de plans indispensable.",
            ),
            None,
        ),
        (
            (
                "Cariste chef d'équipe",
                "Logipôle Lorraine",
                "57 - Metz",
                "CDI",
                "2026-11-09",
                "Vous encadrez une équipe de caristes. Diplôme exigé : Bac+2 en logistique. CACES 3 obligatoire.",
            ),
            None,
        ),
    ])
}

fn bandeaux(c: &Coulisse) -> Vec<Value> {
    lignes(c).iter().map(|l| l["proche"]["niveau"].clone()).collect()
}

fn tris(c: &Coulisse) -> Vec<String> {
    c.vue(jour())["liste"]["filtres"]["tri"]["choix"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["v"].as_str().unwrap().to_string())
        .collect()
}

/// Avec la fiche du CV : le bandeau en mots sur les cartes proches, rien sur les autres ; la phrase dans l'offre
/// ouverte ; le tri par parcours ; aucun chiffre de note nulle part.
#[test]
fn bandeau_phrase_et_tri_avec_un_cv() {
    let fiche = lire_cv(CV);
    assert!(fiche.utilisable(), "{fiche:?}");
    let mut c = cinq_offres();
    c.poser_parcours(Some(&fiche), jour());
    let v = c.vue(jour());
    assert_eq!(bandeaux(&c), [Value::Null, json!("peu"), json!("tres"), Value::Null, json!("peu")]);
    assert_eq!(carte(&c, 2)["proche"], json!({ "niveau": "tres", "nom": "Très proche de ton parcours" }));
    assert_eq!(carte(&c, 1)["proche"]["nom"], "Un peu proche de ton parcours");
    assert_eq!(v["liste"]["parcours"], Value::Null, "avec une fiche, pas d'invitation");
    // (le bandeau ne porte que des mots : ni note, ni rang, ni raison sur la carte)
    for l in lignes(&c) {
        let p = &l["proche"];
        assert!(p.is_null() || p.as_object().unwrap().keys().all(|k| k == "niveau" || k == "nom"), "{p}");
        assert!(!l.to_string().contains("rang"), "{l}");
    }
    // Le tri : proposé, choisi, les plus proches d'abord (à égalité l'ordre du site, les offres loin à la fin)
    assert_eq!(tris(&c), ["site", "recentes", "pres", "parcours"]);
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4]);
    let f = c.geste(None, TRI, &json!({ "tri": "parcours" }), m(9, 0));
    assert!(f.change && f.journal.as_deref().unwrap().ends_with("tri parcours)"), "{f:?}");
    assert_eq!(c.choix.tri, TriSite::Parcours);
    assert_eq!(rangs(&c), [2, 4, 1, 0, 3]);
    assert_eq!(c.vue(jour())["liste"]["filtres"]["tri"]["ici"], "parcours");
    // L'offre ouverte en grand dit pourquoi, en une phrase
    c.geste(None, OUVRIR, &json!({ "i": 2 }), m(9, 1));
    c.poser_parcours(Some(&fiche), jour());
    let v = c.vue(jour());
    assert_eq!(
        v["fiche"]["proche"],
        json!({
            "niveau": "tres",
            "nom": "Très proche de ton parcours",
            "raison": "Tu as été cariste 3 ans, et tu as le CACES 3 demandé.",
        })
    );
    c.geste(None, OUVRIR, &json!({ "i": 4 }), m(9, 2));
    c.poser_parcours(Some(&fiche), jour());
    let v = c.vue(jour());
    assert_eq!(
        v["fiche"]["proche"]["raison"],
        "Tu as été cariste 3 ans, et tu as le CACES 3 demandé, mais l'offre demande un Bac+2."
    );
    // (une offre loin du parcours : aucune ligne, pour ne pas décourager)
    c.geste(None, OUVRIR, &json!({ "i": 3 }), m(9, 3));
    c.poser_parcours(Some(&fiche), jour());
    assert_eq!(c.vue(jour())["fiche"]["proche"], Value::Null);
    sans_tiret_long(&c.vue(jour()));
    // Une offre retirée remontrée n'a pas de bandeau
    c.geste(None, RETOUR, &Value::Null, m(9, 4));
    let adresse = super::lecture_auto::url(&c, 2);
    c.retirees.insert(cle(&adresse));
    c.voir_retirees = true;
    assert_eq!(carte(&c, 2)["retiree"], true);
    assert_eq!(carte(&c, 2)["proche"], Value::Null);
    // L'état reste léger : rien que la page de cartes, et quelques dizaines d'octets de plus par carte
    assert!(c.vue(jour()).to_string().len() < 12_000);
}

/// La fiche corrigée change les bandeaux tout de suite : une ligne retirée ne compte plus, une ligne ajoutée compte.
#[test]
fn la_fiche_corrigee_change_les_bandeaux() {
    let mut fiche = lire_cv(CV);
    let mut c = cinq_offres();
    c.poser_parcours(Some(&fiche), jour());
    assert_eq!(bandeaux(&c)[3], Value::Null);
    fiche.metiers.push(jobmind_core::parcours::metier_saisi("Soudeur, 4 ans"));
    fiche.metiers.retain(|m| m.nom != "Cariste");
    fiche.permis.clear();
    c.poser_parcours(Some(&fiche), jour());
    assert_eq!(bandeaux(&c)[3], json!("tres"));
    assert_eq!(c.offres_lues[3].proche.as_ref().unwrap().raison, "Tu as été soudeur 4 ans.");
    assert_ne!(bandeaux(&c)[2], json!("tres"), "plus cariste, plus de CACES");
    // (la même fiche posée de nouveau : rien n'est recalculé, rien ne change)
    let avant = c.vue(jour());
    c.poser_parcours(Some(&fiche), jour());
    assert_eq!(c.vue(jour()), avant);
}

/// Sans CV, de bout en bout : les cartes s'affichent comme avant, aucun bandeau, le tri par parcours n'est ni proposé
/// ni accepté, une ligne invite à ajouter son CV, l'offre ouverte n'a pas de phrase. Pareil avec un CV illisible ou
/// presque vide, avec sa phrase à lui. Rien ne casse.
#[test]
fn sans_cv_rien_ne_casse() {
    let vide = Parcours::default();
    let presque = lire_cv("Jean Exemple\n06 00 00 00 00\nBonjour");
    assert!(!presque.utilisable());
    let illisible = lire_cv("\u{fffd}\u{fffd}\u{fffd} \u{0} ????");
    for (fiche, invite) in [
        (None, "Ajoute ton CV dans Mon profil pour voir les offres qui te ressemblent."),
        (
            Some(&vide),
            "Jobmind n'a pas réussi à lire ton parcours dans ton CV. Complète-le dans Mon profil pour voir les offres qui te ressemblent.",
        ),
        (
            Some(&presque),
            "Jobmind n'a pas réussi à lire ton parcours dans ton CV. Complète-le dans Mon profil pour voir les offres qui te ressemblent.",
        ),
        (
            Some(&illisible),
            "Jobmind n'a pas réussi à lire ton parcours dans ton CV. Complète-le dans Mon profil pour voir les offres qui te ressemblent.",
        ),
    ] {
        let mut c = cinq_offres();
        // (avant que le programme ne pose quoi que ce soit : comme sans CV)
        assert_eq!(
            c.vue(jour())["liste"]["parcours"]["invite"],
            "Ajoute ton CV dans Mon profil pour voir les offres qui te ressemblent."
        );
        c.poser_parcours(fiche, jour());
        let v = c.vue(jour());
        assert_eq!(rangs(&c), [0, 1, 2, 3, 4], "toutes les cartes, dans l'ordre du site");
        assert!(bandeaux(&c).iter().all(Value::is_null), "aucun bandeau : {:?}", bandeaux(&c));
        assert_eq!(v["liste"]["parcours"], json!({ "invite": invite, "bouton": "Ouvrir Mon profil" }));
        assert_eq!(v["liste"]["sous"], "5 offres analysées");
        assert_eq!(tris(&c), ["site", "recentes", "pres"], "le tri par parcours n'est pas proposé");
        // (demandé quand même, par une vieille page : refusé, rien ne change)
        assert_eq!(c.geste(None, TRI, &json!({ "tri": "parcours" }), m(9, 0)), Fait::default());
        assert_eq!(c.choix.tri, TriSite::Site);
        // Les autres tris et les filtres marchent comme avant
        assert!(c.geste(None, TRI, &json!({ "tri": "recentes" }), m(9, 0)).change);
        assert_eq!(rangs(&c), [0, 1, 2, 3, 4]);
        // L'offre ouverte : pas de phrase
        c.geste(None, OUVRIR, &json!({ "i": 2 }), m(9, 1));
        c.poser_parcours(fiche, jour());
        let v = c.vue(jour());
        assert_eq!(v["fiche"]["titre"], "Cariste H/F");
        assert_eq!(v["fiche"]["proche"], Value::Null);
        assert!(c.offres_lues.iter().all(|f| f.proche.is_none()));
        sans_tiret_long(&v);
    }
    // Sans aucune liste, sans aucune offre : rien non plus
    let mut c = Coulisse::default();
    c.poser_parcours(None, jour());
    c.poser_parcours(Some(&lire_cv(CV)), jour());
    assert!(c.vue(jour())["liste"].is_null());
}

/// Le CV retiré pendant que le tri par parcours est choisi : le tri retombe sur l'ordre du site, les bandeaux s'en
/// vont, l'invitation revient. Et le lendemain, les offres sont comparées de nouveau (un poste en cours a un jour de
/// plus).
#[test]
fn le_cv_retire_et_le_jour_qui_change() {
    let fiche = lire_cv(CV);
    let mut c = cinq_offres();
    c.poser_parcours(Some(&fiche), jour());
    assert!(c.geste(None, TRI, &json!({ "tri": "parcours" }), m(9, 0)).change);
    assert_eq!(rangs(&c)[0], 2);
    c.poser_parcours(None, jour());
    assert_eq!(c.choix.tri, TriSite::Site);
    assert_eq!(rangs(&c), [0, 1, 2, 3, 4]);
    assert!(bandeaux(&c).iter().all(Value::is_null));
    assert!(c.vue(jour())["liste"]["parcours"]["invite"].is_string());
    // Le CV revient : les bandeaux aussi
    c.poser_parcours(Some(&fiche), jour());
    assert_eq!(bandeaux(&c)[2], json!("tres"));
    let demain = jour().plus(1);
    c.offres_lues[2].proche = None;
    c.poser_parcours(Some(&fiche), demain);
    assert!(c.offres_lues.iter().all(|f| f.proche.is_some()));
    // Une nouvelle recherche remet le tri, comme les autres choix
    c.choix.tri = TriSite::Parcours;
    c.choix.remettre();
    assert_eq!(c.choix.tri, TriSite::Site);
}
