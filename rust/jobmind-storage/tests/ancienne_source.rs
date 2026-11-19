//! Test de compatibilité : Jooble a été retiré le 19/11/2026, Adzuna le 20/11/2026 (décisions de Jordan). Une base et un
//! fichier de clés écrits par une version d'avant peuvent encore contenir les clés de Jooble et d'Adzuna, leurs sources
//! de la migration 4, des offres venues d'elles et leurs compteurs d'appels. Tout s'ouvre et marche : les clés des
//! sources retirées sont effacées du fichier sans erreur et sans perdre les autres, les offres restent lisibles comme
//! des offres ordinaires, avec leur lien vers l'annonce (et se regroupent avec les mêmes offres de France Travail), les
//! compteurs sont ignorés. Dossier d'essai sous %TEMP%, jamais les vraies données.

use std::path::PathBuf;

use jobmind_sources::{OffreTrouvee, faux};
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

/// Deux offres telles que Jooble les donnait (la première est aussi chez Adzuna et France Travail, écrite autrement).
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

/// Des offres telles qu'Adzuna les donnait : les offres inventées, avec le lien d'Adzuna vers l'annonce.
fn offres_adzuna() -> Vec<OffreTrouvee> {
    faux::offres_inventees()
        .into_iter()
        .map(|o| OffreTrouvee { url: format!("https://www.adzuna.fr/land/ad/{}?se=essai", o.id_source), ..o })
        .collect()
}

#[test]
fn une_ancienne_base_avec_des_offres_et_des_cles_de_sources_retirees_s_ouvre_et_marche() {
    let e = Essai::nouveau("ancienne-source");
    // Ce qu'une version d'avant a laissé : des offres de Jooble et d'Adzuna, leurs demandes du jour comptées, leurs clés
    {
        let (b, _) = Base::ouvrir(&e.0, m(9)).unwrap();
        let c = b.connexion();
        offres::ajouter_trouvees(c, "jooble", &offres_jooble(), None, m(9)).unwrap();
        offres::ajouter_trouvees(c, "adzuna", &offres_adzuna()[..3], None, m(9)).unwrap();
        for source in ["jooble", "adzuna"] {
            c.execute("INSERT INTO appels (jour, source, nombre) VALUES ('2026-11-18', ?1, 3)", [source]).unwrap();
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

    // Les clés : celles de Jooble et d'Adzuna sont effacées du fichier, sans erreur, et la clé Claude reste
    let (mut cles, journal) = Cles::ouvrir(&e.0, Box::new(Melange));
    let journal = journal.unwrap();
    assert_eq!(journal, "Clés : clé d'une source retirée effacée");
    assert!(!journal.contains("1a2b") && !journal.contains("0123"));
    assert_eq!(cles.etat(Cle::Claude), Some(&EtatCle::Gardee("se termine par ntee".into())));
    assert!(!fichier.with_extension("json.abime").exists());
    let texte = std::fs::read_to_string(&fichier).unwrap();
    assert!(!texte.contains("jooble") && !texte.contains("adzuna"), "{texte}");
    for code in ["adzuna_id", "adzuna_cle", "jooble"] {
        assert!(Cle::depuis_code(code).is_none());
    }
    // Une clé gardée ensuite : les autres restent
    cles.garder(Cle::Claude, "sk-ant-api03-Nouvelle").unwrap();
    let (cles, journal) = Cles::ouvrir(&e.0, Box::new(Melange));
    assert!(journal.is_none(), "{journal:?}");
    assert_eq!(cles.lire(Cle::Claude).as_deref(), Some("sk-ant-api03-Nouvelle"));

    // La base se rouvre sans migration de plus, et les offres de Jooble et d'Adzuna se lisent comme les autres
    let (b, appliquees) = Base::ouvrir(&e.0, m(10)).unwrap();
    assert!(appliquees.is_empty(), "{appliquees:?}");
    let c = b.connexion();
    let l = offres::lister(c).unwrap();
    assert_eq!(l.len(), 2 + 3 - 1);
    let quai = l.iter().find(|o| o.titre.starts_with("Agent de quai")).unwrap();
    assert_eq!(quai.sources, ["jooble", "adzuna"]);
    let o = offres::lire(c, quai.id).unwrap().unwrap();
    assert_eq!(o.liens.len(), 2);
    assert!(o.liens[0].url.starts_with("https://jooble.org/desc/"));
    assert!(o.liens[1].url.starts_with("https://www.adzuna.fr/land/ad/"));
    let cariste = l.iter().find(|o| o.titre == "Cariste CACES 3").unwrap();
    assert_eq!(cariste.sources, ["adzuna"]);
    assert_eq!(offres::lire(c, cariste.id).unwrap().unwrap().description_de, "adzuna");
    assert!(offres::chercher_texte(c, "boulangerie").unwrap().len() == 1);
    assert_eq!(offres::basculer(c, cariste.id, offres::Marque::Favori).unwrap(), Some(true));
    // Les mêmes offres lues sur France Travail : regroupées avec les anciennes, rien ne plante
    let bilan = offres::ajouter_trouvees(c, "francetravail", &faux::offres_inventees(), None, m(11)).unwrap();
    assert_eq!((bilan.nouvelles, bilan.fusionnees), (12, 3));
    let l = offres::lister(c).unwrap();
    assert_eq!(l.len(), 4 + 12 - 3);
    let quai = l.iter().find(|o| o.titre.starts_with("Agent de quai") && o.lieu.starts_with("Ennery")).unwrap();
    assert_eq!(quai.sources, ["jooble", "adzuna", "francetravail"]);
    assert!(l.iter().find(|o| o.titre == "Cariste CACES 3").unwrap().favori);
    assert!(offres::marquer_vue(c, quai.id).unwrap());
}
