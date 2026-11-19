//! Test de compatibilité : Jooble a été retiré le 19/11/2026 (décision de Jordan). Une base et un fichier de clés
//! écrits par une version d'avant peuvent encore contenir une clé Jooble, la source « jooble » de la migration 4, des
//! offres venues de Jooble et des compteurs d'appels Jooble. Tout s'ouvre et marche : la clé Jooble est laissée de côté
//! sans erreur et sans perdre les autres, les offres restent lisibles comme des offres ordinaires (et se regroupent
//! avec les mêmes offres d'Adzuna), les compteurs sont ignorés. Dossier d'essai sous %TEMP%, jamais les vraies données.

use std::path::PathBuf;

use jobmind_core::Jour;
use jobmind_sources::{OffreTrouvee, adzuna, faux};
use jobmind_storage::cles::{Cles, Coffre, EtatCle};
use jobmind_storage::{Base, Moment, offres};

use jobmind_core::cle::Cle;

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

/// Un faux coffre (le vrai, DPAPI, est dans jobmind-app) : chaque octet mélangé.
struct Melange;
impl Coffre for Melange {
    fn fermer(&self, clair: &[u8]) -> Result<Vec<u8>, String> {
        Ok(clair.iter().map(|b| b ^ 0x5A).collect())
    }
    fn ouvrir(&self, chiffre: &[u8]) -> Result<Vec<u8>, String> {
        Ok(chiffre.iter().map(|b| b ^ 0x5A).collect())
    }
}

fn hex(clair: &str) -> String {
    clair.bytes().map(|b| format!("{:02x}", b ^ 0x5A)).collect()
}

fn m(h: u32) -> Moment {
    Moment::nouveau(2026, 11, 18, h, 0, 0).unwrap()
}

/// Deux offres telles que Jooble les donnait (la première est aussi chez Adzuna, écrite autrement).
fn offres_jooble() -> Vec<OffreTrouvee> {
    let o = |id: &str, titre: &str, entreprise: &str, lieu: &str, texte: &str| OffreTrouvee {
        id_source: id.into(),
        titre: titre.into(),
        entreprise: entreprise.into(),
        lieu: lieu.into(),
        url: format!("https://jooble.org/desc/{id}"),
        description: texte.into(),
        contrat: "CDI".into(),
        publiee_le: "2026-11-17".into(),
        brut: format!(r#"{{"id":{id},"title":"{titre}"}}"#),
        ..Default::default()
    };
    vec![
        o(
            "-7100000000000000001",
            "Agent de quai (H/F)",
            "ENTREPOTS DU VAL DE MOSELLE",
            "Ennery (57)",
            "...Chargement et déchargement des camions, contrôle des palettes à l'arrivée...",
        ),
        o(
            "-7100000000000000005",
            "Vendeur en boulangerie",
            "Fournil de la Cathédrale",
            "Metz",
            "Vente. Débutant accepté.",
        ),
    ]
}

#[test]
fn une_ancienne_base_avec_des_offres_et_une_cle_jooble_s_ouvre_et_marche() {
    let e = Essai::nouveau("ancienne-source");
    // Ce qu'une version d'avant a laissé : des offres de Jooble, ses demandes du jour comptées, sa clé gardée
    {
        let (b, _) = Base::ouvrir(&e.0, m(9)).unwrap();
        let c = b.connexion();
        offres::ajouter_trouvees(c, "jooble", &offres_jooble(), None, m(9)).unwrap();
        for _ in 0..3 {
            offres::compter_appel(c, Jour::nouveau(2026, 11, 18).unwrap(), "jooble").unwrap();
        }
    }
    let fichier = Cles::chemin_dans(&e.0);
    std::fs::write(
        &fichier,
        format!(
            r#"{{"version":1,"cles":{{"adzuna_id":"{}","jooble":"{}","adzuna_cle":"{}","claude":"{}"}}}}"#,
            hex("1a2b3c4d"),
            hex("aaaabbbb-cccc-dddd-eeee-ffff00001111"),
            hex("0123456789abcdef0123456789abcdef"),
            hex("sk-ant-api03-Inventee"),
        ),
    )
    .unwrap();

    // Les clés : la clé Jooble est laissée de côté, sans erreur, et les autres sont toutes là
    let (mut cles, journal) = Cles::ouvrir(&e.0, Box::new(Melange));
    assert!(journal.is_none(), "{journal:?}");
    assert_eq!(cles.lire(Cle::AdzunaId).as_deref(), Some("1a2b3c4d"));
    assert_eq!(cles.lire(Cle::AdzunaCle).as_deref(), Some("0123456789abcdef0123456789abcdef"));
    assert_eq!(cles.etat(Cle::Claude), Some(&EtatCle::Gardee("se termine par ntee".into())));
    assert!(!fichier.with_extension("json.abime").exists());
    // Une clé gardée ensuite : les autres restent ; l'ancienne clé Jooble n'est plus réécrite
    cles.garder(Cle::Claude, "sk-ant-api03-Nouvelle").unwrap();
    let (cles, _) = Cles::ouvrir(&e.0, Box::new(Melange));
    assert_eq!(cles.lire(Cle::AdzunaId).as_deref(), Some("1a2b3c4d"));
    assert_eq!(cles.lire(Cle::Claude).as_deref(), Some("sk-ant-api03-Nouvelle"));
    let texte = std::fs::read_to_string(&fichier).unwrap();
    assert!(!texte.contains("jooble"), "{texte}");

    // La base se rouvre sans migration de plus, et les offres de Jooble se lisent comme les autres
    let (b, appliquees) = Base::ouvrir(&e.0, m(10)).unwrap();
    assert!(appliquees.is_empty(), "{appliquees:?}");
    let c = b.connexion();
    let l = offres::lister(c).unwrap();
    assert_eq!(l.len(), 2);
    let quai = l.iter().find(|o| o.titre.starts_with("Agent de quai")).unwrap();
    assert_eq!(quai.sources, ["jooble"]);
    let o = offres::lire(c, quai.id).unwrap().unwrap();
    assert_eq!(o.liens.len(), 1);
    assert!(o.liens[0].url.starts_with("https://jooble.org/desc/"));
    assert!(offres::chercher_texte(c, "boulangerie").unwrap().len() == 1);
    // Les compteurs de Jooble ne gênent pas ceux d'Adzuna
    let jour = Jour::nouveau(2026, 11, 18).unwrap();
    assert_eq!(offres::appels(c, jour, "adzuna").unwrap(), 0);
    // Une recherche chez Adzuna : la même offre se regroupe avec l'ancienne, rien ne plante
    let az = adzuna::lire(200, &faux::reponse_adzuna(), false).unwrap();
    let bilan = offres::ajouter_trouvees(c, "adzuna", &az, None, m(11)).unwrap();
    assert_eq!((bilan.nouvelles, bilan.fusionnees), (12, 1));
    let l = offres::lister(c).unwrap();
    assert_eq!(l.len(), 2 + 12 - 1);
    let quai = l.iter().find(|o| o.titre.starts_with("Agent de quai") && o.lieu.starts_with("Ennery")).unwrap();
    assert_eq!(quai.sources, ["jooble", "adzuna"]);
    assert!(offres::marquer_vue(c, quai.id).unwrap());
}
