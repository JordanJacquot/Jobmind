//! La sauvegarde et la restauration (étape 12), sur des dossiers d'essai sous %TEMP% (jamais les vraies données) :
//! aller-retour complet (base et CV identiques, rien d'autre dans le zip), annulation, sauvegardes piégées ou abîmées
//! refusées sans rien écrire hors du dossier d'essai, sauvegardes automatiques et d'avant restauration, phrases.

use std::path::{Path, PathBuf};

use jobmind_storage::base::version_connue;
use jobmind_storage::profil::{self, GenreCv, NouveauCv};
use jobmind_storage::rusqlite::types::Value;
use jobmind_storage::sauvegarde::{self, ErreurSauvegarde, FICHIER_MAX, MANIFESTE};
use jobmind_storage::{Base, Moment};

/// Un dossier d'essai neuf sous %TEMP%, effacé à la fin.
struct Essai(PathBuf);

impl Essai {
    fn nouveau(nom: &str) -> Essai {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("JobmindEssai-{}-sauvegarde-{nom}-{n}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Essai(d)
    }

    /// Le dossier de données, un cran plus bas : un nom qui « remonte » (../x) tomberait dans le dossier d'essai,
    /// où on le chercherait.
    fn donnees(&self) -> PathBuf {
        let d = self.0.join("donnees");
        std::fs::create_dir_all(&d).unwrap();
        d
    }
}

impl Drop for Essai {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn moment() -> Moment {
    Moment::nouveau(2026, 11, 19, 14, 5, 7).unwrap()
}

/// Les noms d'un dossier, triés (rien s'il n'existe pas).
fn noms(d: &Path) -> Vec<String> {
    let mut l: Vec<String> = std::fs::read_dir(d)
        .map(|e| e.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    l.sort();
    l
}

/// Tout le contenu d'une base : chaque table, ses lignes triées.
fn vidage(b: &Base) -> Vec<(String, Vec<String>)> {
    let c = b.connexion();
    let mut r = c.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").unwrap();
    let tables: Vec<String> = r.query_map([], |l| l.get(0)).unwrap().map(Result::unwrap).collect();
    tables
        .into_iter()
        .map(|t| {
            let mut r = c.prepare(&format!("SELECT * FROM \"{t}\"")).unwrap();
            let n = r.column_count();
            let mut lignes: Vec<String> = r
                .query_map([], |l| {
                    Ok(format!("{:?}", (0..n).map(|i| l.get::<_, Value>(i).unwrap()).collect::<Vec<_>>()))
                })
                .unwrap()
                .map(Result::unwrap)
                .collect();
            lignes.sort();
            (t, lignes)
        })
        .collect()
}

/// Une base remplie à la main, deux CV, et tout ce qui ne doit jamais partir dans une sauvegarde.
fn remplir(d: &Path, graine: &str) -> Base {
    let (b, _) = Base::ouvrir(d, moment()).unwrap();
    let c = b.connexion();
    c.execute_batch(&format!(
        "INSERT INTO offres (titre, entreprise, lieu, description, ajoutee_le) \
             VALUES ('Magasinier {graine}', 'Dépôt', 'Lyon', 'Préparer les commandes', '2026-11-18T10:00:00');
         INSERT INTO offres (titre, entreprise, ajoutee_le, doublon_de) VALUES ('Magasinier (copie)', 'Dépôt', '2026-11-18T10:01:00', 1);
         INSERT INTO candidatures (offre_id, entreprise, poste, statut, envoyee_le, notes, cree_le, change_le) \
             VALUES (1, 'Dépôt', 'Magasinier', 'envoyee', '2026-11-18', 'Écrit « vite »', '2026-11-18T10:05:00', '2026-11-18T10:05:00');
         INSERT INTO candidatures (entreprise, poste, statut, cree_le, change_le) \
             VALUES ('Boulangerie {graine}', 'Vendeur', 'entretien', '2026-11-17T09:00:00', '2026-11-19T09:00:00');
         INSERT INTO evenements (candidature_id, quand, type, statut) VALUES (1, '2026-11-18T10:05:00', 'statut', 'envoyee');
         INSERT INTO profil (id, nom, ville, metiers) VALUES (1, 'Essai', 'Lyon', 'magasinier');
         INSERT INTO profil_style (titre, texte, ajoute_le) VALUES ('Lettre', 'Bonjour, je suis motivé.', '2026-11-18T10:00:00');
         INSERT INTO parametres (cle, valeur) VALUES ('relance_jours', '7');"
    ))
    .unwrap();
    for (octets, ext, genre) in [
        (format!("%PDF-1.4 cv {graine}").into_bytes(), "pdf", GenreCv::Pdf),
        (vec![0x50, 0x4B, 3, 4, 0, 255, 7], "docx", GenreCv::Docx),
    ] {
        let fichier = profil::copier_fichier(d, &octets, ext, moment()).unwrap();
        profil::ajouter_cv(
            c,
            &NouveauCv { nom: "Mon CV", fichier: Some(&fichier), genre, texte: "Magasinier", taille: Some(7) },
            moment(),
        )
        .unwrap();
    }
    // Ce qui ne part jamais : les secrets, le journal, la session des sites, les réglages, et ce qui traîne dans cv
    std::fs::write(d.join("cles.json"), b"{\"secret\":\"ne pas sauvegarder\"}").unwrap();
    std::fs::write(d.join("journal.txt"), b"journal").unwrap();
    std::fs::write(d.join("reglages.json"), b"{}").unwrap();
    std::fs::create_dir_all(d.join("panneau-sites")).unwrap();
    std::fs::write(d.join("panneau-sites").join("Cookies"), b"cookies").unwrap();
    std::fs::write(d.join("cv").join(".cache"), b"cache").unwrap();
    std::fs::create_dir_all(d.join("cv").join("sous")).unwrap();
    std::fs::write(d.join("cv").join("sous").join("x.pdf"), b"x").unwrap();
    b
}

/// Les fichiers de CV d'un dossier de données (fichiers ordinaires seulement), avec leurs octets.
fn fichiers_cv(d: &Path) -> Vec<(String, Vec<u8>)> {
    noms(&d.join("cv"))
        .into_iter()
        .filter(|n| d.join("cv").join(n).is_file())
        .map(|n| {
            let o = std::fs::read(d.join("cv").join(&n)).unwrap();
            (n, o)
        })
        .collect()
}

#[test]
fn aller_retour_complet() {
    let source = Essai::nouveau("source");
    let ds = source.donnees();
    let b = remplir(&ds, "a");
    let (zip, resume) = sauvegarde::faire(&b, &ds, moment()).unwrap();
    assert_eq!(resume.faite_le, moment());
    assert_eq!(resume.version_base, version_connue());
    assert_eq!((resume.candidatures, resume.offres, resume.cv), (2, 1, 2));
    // Le fichier temporaire de VACUUM INTO est effacé
    assert_eq!(noms(&ds.join("data")), ["jobmind.db"]);
    // Rien d'autre que le manifeste, la base et les deux CV
    let cv_source = fichiers_cv(&ds);
    let cv_sur: Vec<&(String, Vec<u8>)> = cv_source.iter().filter(|(n, _)| n.starts_with("cv-")).collect();
    assert_eq!(cv_sur.len(), 2);
    let mut attendus = vec![MANIFESTE.to_string(), "data/jobmind.db".to_string()];
    attendus.extend(cv_sur.iter().map(|(n, _)| format!("cv/{n}")));
    assert_eq!(sauvegarde::noms_des_parties(&zip).unwrap(), attendus);
    for interdit in ["cles.json", "journal.txt", "reglages.json", "panneau-sites", "Cookies", ".cache", "sous"] {
        assert!(!sauvegarde::noms_des_parties(&zip).unwrap().iter().any(|n| n.contains(interdit)), "{interdit}");
        assert!(!zip.windows(interdit.len()).any(|w| w == interdit.as_bytes()), "{interdit}");
    }
    assert!(!zip.windows(9).any(|w| w == b"ne pas sa"));
    // Écrite, puis relue dans un autre dossier de données vide
    let fichier = source.0.join(sauvegarde::nom_fichier(moment()));
    sauvegarde::ecrire(&fichier, &zip).unwrap();
    assert_eq!(noms(&source.0), ["Jobmind sauvegarde 2026-11-19.zip", "donnees"]);
    let cible = Essai::nouveau("cible");
    let dc = cible.donnees();
    let contenu = sauvegarde::lire_fichier(&fichier, &dc).unwrap();
    assert_eq!(contenu.resume, resume);
    // (le fichier temporaire de vérification est effacé)
    assert!(noms(&dc.join("data")).is_empty());
    sauvegarde::remplacer(&dc, &contenu).unwrap();
    assert!(sauvegarde::en_attente(&dc));
    sauvegarde::confirmer(&dc).unwrap();
    assert!(!sauvegarde::en_attente(&dc));
    assert_eq!(noms(&dc), ["cv", "data"]);
    assert_eq!(noms(&dc.join("data")), ["jobmind.db"]);
    let (b2, appliquees) = Base::ouvrir(&dc, moment()).unwrap();
    assert!(appliquees.is_empty());
    assert_eq!(vidage(&b2), vidage(&b));
    let cv_cible = fichiers_cv(&dc);
    assert_eq!(cv_cible.iter().collect::<Vec<_>>(), cv_sur);
    // Les CV de la base pointent bien vers les fichiers remis
    for cv in profil::lister_cv(b2.connexion()).unwrap() {
        let chemin = profil::chemin_fichier(&dc, cv.fichier.as_deref().unwrap()).unwrap();
        assert!(chemin.is_file(), "{chemin:?}");
    }
}

#[test]
fn remplacer_puis_annuler() {
    let source = Essai::nouveau("annuler-source");
    let ds = source.donnees();
    let b = remplir(&ds, "a");
    let (zip, _) = sauvegarde::faire(&b, &ds, moment()).unwrap();
    // Un dossier de données avec ses propres données (différentes)
    let cible = Essai::nouveau("annuler-cible");
    let dc = cible.donnees();
    let b2 = remplir(&dc, "b");
    std::fs::write(dc.join("cv").join("autre.pdf"), b"autre").unwrap();
    let vidage_avant = vidage(&b2);
    drop(b2);
    let cv_avant = fichiers_cv(&dc);
    let contenu = sauvegarde::lire(&zip, &dc).unwrap();
    sauvegarde::remplacer(&dc, &contenu).unwrap();
    assert!(dc.join("data").join("jobmind.db.avant").is_file());
    assert!(dc.join("cv.avant").is_dir());
    assert_ne!(fichiers_cv(&dc), cv_avant);
    // Une deuxième restauration attend que la première soit confirmée ou annulée
    assert!(matches!(sauvegarde::remplacer(&dc, &contenu), Err(ErreurSauvegarde::EnCours)));
    sauvegarde::annuler(&dc).unwrap();
    assert!(!sauvegarde::en_attente(&dc));
    assert_eq!(fichiers_cv(&dc), cv_avant);
    assert!(dc.join("cv").join("sous").join("x.pdf").is_file());
    let (b3, _) = Base::ouvrir(&dc, moment()).unwrap();
    assert_eq!(vidage(&b3), vidage_avant);
    assert_ne!(vidage(&b3), vidage(&b));
    drop(b3);
    for reste in ["cles.json", "journal.txt", "reglages.json", "panneau-sites"] {
        assert!(dc.join(reste).exists(), "{reste}");
    }
    assert_eq!(noms(&dc.join("data")), ["jobmind.db"]);

    // Dans un dossier vide : l'annulation ne laisse rien
    let vide = Essai::nouveau("annuler-vide");
    let dv = vide.donnees();
    sauvegarde::remplacer(&dv, &sauvegarde::lire(&zip, &dv).unwrap()).unwrap();
    assert!(dv.join("data").join("jobmind.db").is_file());
    sauvegarde::annuler(&dv).unwrap();
    assert_eq!(noms(&dv), ["data"]);
    assert!(noms(&dv.join("data")).is_empty());
    // (annuler ou confirmer sans restauration en cours ne change rien)
    sauvegarde::annuler(&dv).unwrap();
    sauvegarde::confirmer(&dv).unwrap();
    assert_eq!(noms(&dv), ["data"]);
}

// --- Sauvegardes refusées ---

/// Les octets d'une base de Jobmind (copie cohérente), et sa version.
fn octets_base(essai: &Essai, futur: bool) -> Vec<u8> {
    let d = essai.0.join(if futur { "base-future" } else { "base" });
    let (b, _) = Base::ouvrir(&d, moment()).unwrap();
    if futur {
        b.connexion()
            .execute(
                "INSERT INTO migrations (version, nom, appliquee_le) VALUES (?1, 'future', '2030-01-01T00:00:00')",
                [version_connue() + 1],
            )
            .unwrap();
    }
    drop(b);
    std::fs::read(Base::chemin_dans(&d)).unwrap()
}

fn manifeste(appli: &str, version: u32, base: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "appli": appli,
        "format": 1,
        "faite_le": moment().iso(),
        "version_base": version,
        "base": { "taille": base.len(), "crc32": crc32fast::hash(base) },
    }))
    .unwrap()
}

/// Un zip fait à la main : le manifeste, la base, puis les parties données.
fn zip_avec(base: &[u8], autres: &[(&str, &[u8])]) -> Vec<u8> {
    let m = manifeste("Jobmind", version_connue(), base);
    let mut parties: Vec<(&str, &[u8])> = vec![(MANIFESTE, &m), ("data/jobmind.db", base)];
    parties.extend_from_slice(autres);
    sauvegarde::fabriquer_zip(&parties, moment()).unwrap()
}

/// Le début de la ligne de cette partie dans le répertoire central.
fn ligne_centrale(zip: &[u8], nom: &str) -> usize {
    (0..zip.len() - 46)
        .find(|&i| {
            zip[i..i + 4] == [0x50, 0x4B, 1, 2] && {
                let n = u16::from_le_bytes([zip[i + 28], zip[i + 29]]) as usize;
                zip.get(i + 46..i + 46 + n) == Some(nom.as_bytes())
            }
        })
        .unwrap()
}

/// Change la ligne du répertoire central de cette partie, à ce décalage, sur 4 octets (16 : crc, 20 : taille
/// compressée, 24 : taille).
fn changer_central(zip: &mut [u8], nom: &str, decalage: usize, valeur: u32) {
    let i = ligne_centrale(zip, nom) + decalage;
    zip[i..i + 4].copy_from_slice(&valeur.to_le_bytes());
}

fn lire_central(zip: &[u8], nom: &str, decalage: usize) -> u32 {
    let i = ligne_centrale(zip, nom) + decalage;
    u32::from_le_bytes([zip[i], zip[i + 1], zip[i + 2], zip[i + 3]])
}

/// Lit ces octets dans un dossier de données neuf ; vérifie que rien n'a été écrit hors de son dossier `data`
/// (resté vide).
fn refus(octets: &[u8]) -> ErreurSauvegarde {
    let e = Essai::nouveau("refus");
    let d = e.donnees();
    let erreur = sauvegarde::lire(octets, &d).unwrap_err();
    assert_eq!(noms(&e.0), ["donnees"]);
    assert!(noms(&d).iter().all(|n| n == "data"), "{:?}", noms(&d));
    assert!(noms(&d.join("data")).is_empty(), "{:?}", noms(&d.join("data")));
    erreur
}

#[test]
fn noms_pieges_refuses() {
    let e = Essai::nouveau("noms");
    let base = octets_base(&e, false);
    // (le zip juste, d'abord : accepté)
    let juste = zip_avec(&base, &[("cv/cv-1.pdf", b"pdf")]);
    let d = e.donnees();
    assert_eq!(sauvegarde::lire(&juste, &d).unwrap().noms_cv().collect::<Vec<_>>(), ["cv-1.pdf"]);
    for nom in ["../x", "cv/../../x", "/abs", "C:/x", "C:\\x", "\\\\serveur\\x", "", "cles.json", "cv/sous/x.pdf"] {
        let z = zip_avec(&base, &[(nom, b"piege")]);
        assert!(matches!(refus(&z), ErreurSauvegarde::Piegee(_)), "{nom:?}");
    }
    // Doublons (aussi en changeant les majuscules : le même fichier sous Windows)
    for (a, b) in [("cv/a.pdf", "cv/a.pdf"), ("cv/A.pdf", "cv/a.pdf"), ("data/jobmind.db", "data/jobmind.db")] {
        let z = zip_avec(&base, &[(a, b"1"), (b, b"2")]);
        assert!(matches!(refus(&z), ErreurSauvegarde::Piegee(_)), "{a} {b}");
    }
    // Trop de parties
    let noms_cv: Vec<String> = (0..sauvegarde::PARTIES_MAX).map(|i| format!("cv/f{i}.txt")).collect();
    let parties: Vec<(&str, &[u8])> = noms_cv.iter().map(|n| (n.as_str(), b"x".as_slice())).collect();
    assert!(matches!(refus(&zip_avec(&base, &parties)), ErreurSauvegarde::Piegee(_)));
}

#[test]
fn bombes_refusees() {
    let e = Essai::nouveau("bombes");
    let base = octets_base(&e, false);
    // 16 Mo de zéros : quelques kilo-octets compressés
    let zeros = vec![0u8; 16 * 1024 * 1024];
    let juste = zip_avec(&base, &[("cv/bombe.pdf", &zeros)]);
    assert!(juste.len() < 200_000 + base.len(), "{}", juste.len());
    // Taille annoncée énorme : refusée sans rien décompresser
    let mut z = juste.clone();
    changer_central(&mut z, "cv/bombe.pdf", 24, 3_000_000_000);
    assert!(matches!(refus(&z), ErreurSauvegarde::TropGrosse));
    // Taille annoncée plus grande que ce que deflate peut donner pour si peu (mais sous la limite d'un CV)
    let mut z = juste.clone();
    let compresse = lire_central(&z, "cv/bombe.pdf", 20);
    assert!(compresse * 1500 < sauvegarde::CV_MAX as u32, "{compresse}");
    changer_central(&mut z, "cv/bombe.pdf", 24, compresse * 1500);
    assert!(matches!(refus(&z), ErreurSauvegarde::Piegee(_)));
    // Taille annoncée mensongère (petite) : la décompression s'arrête à la taille annoncée
    let mut z = juste.clone();
    changer_central(&mut z, "cv/bombe.pdf", 24, 1000);
    assert!(matches!(refus(&z), ErreurSauvegarde::Abimee(_)));
    // Le total décompressé, réuni, limité (des parties chacune permise)
    let parties: Vec<String> = (0..60).map(|i| format!("cv/b{i}.pdf")).collect();
    let morceaux: Vec<(&str, &[u8])> = parties.iter().map(|n| (n.as_str(), b"petit".as_slice())).collect();
    let mut z = zip_avec(&base, &morceaux);
    for n in &parties {
        changer_central(&mut z, n, 24, 30 * 1024 * 1024);
    }
    assert!(matches!(refus(&z), ErreurSauvegarde::TropGrosse | ErreurSauvegarde::Piegee(_)));
}

#[test]
fn zips_abimes_refuses() {
    let e = Essai::nouveau("abimes");
    let base = octets_base(&e, false);
    let juste = zip_avec(&base, &[("cv/cv-1.pdf", "un CV assez long pour être compressé ".repeat(50).as_bytes())]);
    // Pas un zip, vide, coupé
    assert!(matches!(refus(b"bonjour, je ne suis pas un zip"), ErreurSauvegarde::PasUnZip));
    assert!(matches!(refus(b""), ErreurSauvegarde::PasUnZip));
    assert!(matches!(refus(&juste[..juste.len() / 2]), ErreurSauvegarde::Abimee(_)));
    assert!(matches!(refus(&juste[..juste.len() - 1]), ErreurSauvegarde::Abimee(_)));
    // Somme de contrôle fausse
    let mut z = juste.clone();
    changer_central(&mut z, "cv/cv-1.pdf", 16, 0xDEAD_BEEF);
    assert!(matches!(refus(&z), ErreurSauvegarde::Abimee(_)));
    // Un octet de la base changé : son empreinte ne colle plus
    let mut changee = base.clone();
    changee[200] ^= 0xFF;
    let m = manifeste("Jobmind", version_connue(), &base);
    let z = sauvegarde::fabriquer_zip(&[(MANIFESTE, &m), ("data/jobmind.db", &changee)], moment()).unwrap();
    assert!(matches!(refus(&z), ErreurSauvegarde::Abimee(_)));
    // Manifeste d'une autre appli, ou absent
    let m = manifeste("Paymind", version_connue(), &base);
    let z = sauvegarde::fabriquer_zip(&[(MANIFESTE, &m), ("data/jobmind.db", &base)], moment()).unwrap();
    assert!(matches!(refus(&z), ErreurSauvegarde::PasJobmind));
    let z = sauvegarde::fabriquer_zip(&[("data/jobmind.db", &base)], moment()).unwrap();
    assert!(matches!(refus(&z), ErreurSauvegarde::PasJobmind));
    let z = sauvegarde::fabriquer_zip(&[(MANIFESTE, b"pas du json")], moment()).unwrap();
    assert!(matches!(refus(&z), ErreurSauvegarde::PasJobmind));
    // Base absente
    let z =
        sauvegarde::fabriquer_zip(&[(MANIFESTE, &manifeste("Jobmind", version_connue(), &base))], moment()).unwrap();
    assert!(matches!(refus(&z), ErreurSauvegarde::Abimee(_)));
    // Base abîmée : l'en-tête SQLite, puis des octets au hasard (le fichier temporaire de vérification est effacé)
    let mut abimee = b"SQLite format 3\0".to_vec();
    let mut x: u32 = 12345;
    for _ in 0..16_384 {
        x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        abimee.push((x >> 16) as u8);
    }
    assert!(matches!(refus(&zip_avec(&abimee, &[])), ErreurSauvegarde::Abimee(_)));
    // Pas une base SQLite du tout
    assert!(matches!(refus(&zip_avec(b"bonjour", &[])), ErreurSauvegarde::Abimee(_)));
}

#[test]
fn base_plus_recente_refusee() {
    let e = Essai::nouveau("future");
    let future = octets_base(&e, true);
    // Le manifeste dit la version connue, la base en a une de plus
    let connue = version_connue();
    assert!(matches!(
        refus(&zip_avec(&future, &[])),
        ErreurSauvegarde::PlusRecente { trouvee, connue: c } if trouvee == connue + 1 && c == connue
    ));
    // Le manifeste lui-même annonce une version plus récente
    let m = manifeste("Jobmind", connue + 1, &future);
    let z = sauvegarde::fabriquer_zip(&[(MANIFESTE, &m), ("data/jobmind.db", &future)], moment()).unwrap();
    assert!(matches!(refus(&z), ErreurSauvegarde::PlusRecente { .. }));
    // Un format de sauvegarde plus récent
    let base = octets_base(&e, false);
    let m = serde_json::to_vec(&serde_json::json!({ "appli": "Jobmind", "format": 2 })).unwrap();
    let z = sauvegarde::fabriquer_zip(&[(MANIFESTE, &m), ("data/jobmind.db", &base)], moment()).unwrap();
    let erreur = refus(&z);
    assert!(matches!(erreur, ErreurSauvegarde::FormatRecent(2)));
    assert!(erreur.message().contains("plus récente"));
    // Une base d'une version plus ancienne (tables de moins) se lit : l'appli la migrera en l'ouvrant
    let vieille = e.0.join("vieille");
    std::fs::create_dir_all(vieille.join("data")).unwrap();
    {
        let b = Base::en_memoire_jusqua(connue - 1, moment()).unwrap();
        let chemin = Base::chemin_dans(&vieille);
        b.connexion().execute("VACUUM INTO ?1", [chemin.to_str().unwrap()]).unwrap();
    }
    let octets = std::fs::read(Base::chemin_dans(&vieille)).unwrap();
    let m = manifeste("Jobmind", connue - 1, &octets);
    let z = sauvegarde::fabriquer_zip(&[(MANIFESTE, &m), ("data/jobmind.db", &octets)], moment()).unwrap();
    let d = e.donnees();
    assert_eq!(sauvegarde::lire(&z, &d).unwrap().resume.version_base, connue - 1);
}

#[test]
fn fichiers_refuses() {
    let e = Essai::nouveau("fichiers");
    let d = e.donnees();
    // Plus gros que FICHIER_MAX : refusé sans être lu (un fichier creux, rien n'est écrit sur le disque)
    let gros = e.0.join("gros.zip");
    std::fs::File::create(&gros).unwrap().set_len(FICHIER_MAX + 1).unwrap();
    assert!(matches!(sauvegarde::lire_fichier(&gros, &d), Err(ErreurSauvegarde::TropGrosse)));
    std::fs::remove_file(&gros).unwrap();
    // Vide
    let vide = e.0.join("vide.zip");
    std::fs::write(&vide, b"").unwrap();
    assert!(matches!(sauvegarde::lire_fichier(&vide, &d), Err(ErreurSauvegarde::PasUnZip)));
    // Absent
    assert!(matches!(sauvegarde::lire_fichier(&e.0.join("absent.zip"), &d), Err(ErreurSauvegarde::Disque(_))));
    // Un dossier
    assert!(matches!(sauvegarde::lire_fichier(&e.0, &d), Err(ErreurSauvegarde::PasUnZip)));
}

// --- Automatiques ---

#[test]
fn automatiques_une_par_jour_sept_gardees() {
    let e = Essai::nouveau("auto");
    let d = e.donnees();
    let b = remplir(&d, "a");
    let dossier = d.join(sauvegarde::DOSSIER);
    std::fs::create_dir_all(&dossier).unwrap();
    // Des fichiers qui ne sont pas des sauvegardes automatiques : jamais effacés
    let autres =
        ["auto-notes.txt", "Jobmind sauvegarde 2026-10-01.zip", "auto-2026-11-1.zip", "auto-2026-11-01.zip.part"];
    for n in autres {
        std::fs::write(dossier.join(n), b"a garder").unwrap();
    }
    for jour in 1..=9 {
        let m = Moment::nouveau(2026, 11, jour, 9, 0, 0).unwrap();
        let chemin = sauvegarde::automatique(&b, &d, m).unwrap().unwrap();
        assert_eq!(chemin, dossier.join(format!("auto-2026-11-{jour:02}.zip")));
        // Une seule par jour
        assert_eq!(sauvegarde::automatique(&b, &d, Moment::nouveau(2026, 11, jour, 18, 0, 0).unwrap()).unwrap(), None);
    }
    let l = sauvegarde::automatiques(&d);
    assert_eq!(l.len(), sauvegarde::AUTO_GARDEES);
    assert_eq!(l.first().unwrap().0, "auto-2026-11-09.zip");
    assert_eq!(l.last().unwrap().0, "auto-2026-11-03.zip");
    assert_eq!(l.first().unwrap().1, jobmind_core::Jour::nouveau(2026, 11, 9).unwrap());
    for n in autres {
        assert_eq!(std::fs::read(dossier.join(n)).unwrap(), b"a garder", "{n}");
    }
    assert_eq!(noms(&dossier).len(), 7 + autres.len());
    // Chacune est une vraie sauvegarde
    let autre = Essai::nouveau("auto-relue");
    let r = sauvegarde::lire_fichier(&dossier.join("auto-2026-11-09.zip"), &autre.donnees()).unwrap();
    assert_eq!((r.resume.candidatures, r.resume.cv), (2, 2));
    // Et le dossier data ne garde aucun fichier temporaire
    assert_eq!(noms(&d.join("data")), ["jobmind.db"]);
}

#[test]
fn avant_restauration_trois_gardees() {
    let e = Essai::nouveau("avant");
    let d = e.donnees();
    let b = remplir(&d, "a");
    let dossier = d.join(sauvegarde::DOSSIER);
    std::fs::create_dir_all(&dossier).unwrap();
    std::fs::write(dossier.join("avant-restauration-a-garder.zip"), b"a garder").unwrap();
    std::fs::write(dossier.join("auto-2026-11-19.zip"), b"auto").unwrap();
    for s in 0..5 {
        let m = Moment::nouveau(2026, 11, 19, 14, 5, s * 10).unwrap();
        let chemin = sauvegarde::avant_restauration(&b, &d, m).unwrap();
        assert_eq!(chemin, dossier.join(format!("avant-restauration-2026-11-19-1405{:02}.zip", s * 10)));
    }
    assert_eq!(
        noms(&dossier),
        [
            "auto-2026-11-19.zip",
            "avant-restauration-2026-11-19-140520.zip",
            "avant-restauration-2026-11-19-140530.zip",
            "avant-restauration-2026-11-19-140540.zip",
            "avant-restauration-a-garder.zip",
        ]
    );
}

#[test]
fn phrases_simples() {
    let erreurs = [
        ErreurSauvegarde::PasUnZip,
        ErreurSauvegarde::PasJobmind,
        ErreurSauvegarde::Abimee("x".into()),
        ErreurSauvegarde::PlusRecente { trouvee: 13, connue: 12 },
        ErreurSauvegarde::FormatRecent(2),
        ErreurSauvegarde::TropGrosse,
        ErreurSauvegarde::DonneesTropGrosses,
        ErreurSauvegarde::Piegee("x".into()),
        ErreurSauvegarde::EnCours,
        ErreurSauvegarde::Disque(std::io::Error::other("x")),
        ErreurSauvegarde::Base("x".into()),
    ];
    for e in &erreurs {
        let m = e.message();
        assert!(m.ends_with('.'), "{m}");
        assert!(m.len() < 200, "{m}");
        assert!(!m.contains('\u{2013}') && !m.contains('\u{2014}'), "{m}");
        assert!(!e.to_string().contains('\u{2013}') && !e.to_string().contains('\u{2014}'), "{e}");
        for technique in ["zip", "SQLite", "crc", "manifeste", "base"] {
            assert!(!m.contains(technique), "{m}");
        }
    }
    assert_eq!(ErreurSauvegarde::PasUnZip.message(), "Ce fichier n'est pas une sauvegarde de Jobmind.");
    assert_eq!(ErreurSauvegarde::Abimee(String::new()).message(), "Ce fichier de sauvegarde est abîmé.");
    assert_eq!(
        ErreurSauvegarde::PlusRecente { trouvee: 13, connue: 12 }.message(),
        "Cette sauvegarde vient d'une version plus récente de Jobmind. Mets Jobmind à jour, puis réessaie."
    );
}

// --- Audit : schéma, confirmation, restes, base abîmée ---

/// Une base avec un déclencheur, une vue ou une table de plus que les migrations est refusée (un déclencheur
/// s'exécuterait à chaque écriture de Jobmind).
#[test]
fn base_avec_objets_inconnus_refusee() {
    let e = Essai::nouveau("schema");
    for (i, sql) in [
        "CREATE TRIGGER piege AFTER INSERT ON candidatures BEGIN DELETE FROM offres; END",
        "CREATE VIEW espion AS SELECT * FROM profil",
        "CREATE TABLE intruse (x)",
        "CREATE INDEX intrus ON offres (titre)",
        // (un déclencheur connu, mais changé)
        "DROP TRIGGER offres_fts_ajout; CREATE TRIGGER offres_fts_ajout AFTER INSERT ON offres BEGIN DELETE FROM cv; END",
    ]
    .iter()
    .enumerate()
    {
        let d = e.0.join(format!("b{i}"));
        let (b, _) = Base::ouvrir(&d, moment()).unwrap();
        b.connexion().execute_batch(sql).unwrap();
        drop(b);
        let octets = std::fs::read(Base::chemin_dans(&d)).unwrap();
        assert!(matches!(refus(&zip_avec(&octets, &[])), ErreurSauvegarde::Piegee(_)), "{sql}");
    }
    // (la base telle que les migrations la font reste acceptée)
    let base = octets_base(&e, false);
    let d = e.donnees();
    assert!(sauvegarde::lire(&zip_avec(&base, &[]), &d).is_ok());
}

/// Jobmind s'arrête pendant la confirmation, après la marque effacée et avant les restes : au démarrage, la
/// restauration n'est PAS défaite (avant : les restes .avant la faisaient annuler), et les restes sont effacés.
#[test]
fn confirmation_interrompue_pas_defaite() {
    let source = Essai::nouveau("conf-source");
    let ds = source.donnees();
    let b = remplir(&ds, "a");
    let (zip, _) = sauvegarde::faire(&b, &ds, moment()).unwrap();
    let cible = Essai::nouveau("conf-cible");
    let dc = cible.donnees();
    drop(remplir(&dc, "b"));
    sauvegarde::remplacer(&dc, &sauvegarde::lire(&zip, &dc).unwrap()).unwrap();
    // (l'arrêt : seule la marque est partie)
    std::fs::remove_file(dc.join("restauration-en-cours.txt")).unwrap();
    assert!(dc.join("data").join("jobmind.db.avant").is_file());
    assert!(!sauvegarde::en_attente(&dc));
    sauvegarde::annuler(&dc).unwrap();
    let (b2, _) = Base::ouvrir(&dc, moment()).unwrap();
    assert_eq!(vidage(&b2), vidage(&b), "la restauration est gardée");
    drop(b2);
    let effaces = sauvegarde::nettoyer_restes(&dc);
    assert_eq!(effaces.len(), 2, "{effaces:?}");
    assert_eq!(noms(&dc.join("data")), ["jobmind.db"]);
    assert!(!dc.join("cv.avant").exists());
    // Une confirmation normale : propre, sans rien à dire au journal
    let cible2 = Essai::nouveau("conf-cible2");
    let d2 = cible2.donnees();
    drop(remplir(&d2, "c"));
    sauvegarde::remplacer(&d2, &sauvegarde::lire(&zip, &d2).unwrap()).unwrap();
    assert_eq!(sauvegarde::confirmer_bilan(&d2).unwrap(), None);
    assert!(!sauvegarde::en_attente(&d2));
    assert_eq!(noms(&d2.join("data")), ["jobmind.db"]);
}

/// Les restes d'un arrêt brutal sont effacés au démarrage ; jamais la base, cles.json ni une vraie sauvegarde.
#[test]
fn restes_d_un_arret_brutal_effaces() {
    let e = Essai::nouveau("restes");
    let d = e.donnees();
    let data = d.join("data");
    let sauv = d.join("sauvegardes");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&sauv).unwrap();
    std::fs::create_dir_all(d.join("cv.nouveau")).unwrap();
    std::fs::write(d.join("cv.nouveau").join("cv-1.pdf"), b"x").unwrap();
    let restes = [
        data.join("sauvegarde-123-456789-0.db"),
        data.join("sauvegarde-123-456789-0.db-journal"),
        data.join("verification-9-8-7.db"),
        data.join("verification-9-8-7.db.ecriture-9-8-8"),
        data.join("jobmind.db.nouvelle"),
        sauv.join("auto-2026-11-19.zip.ecriture-12-34-5"),
        d.join("restauration-en-cours.txt.ecriture-1-2-3"),
    ];
    let gardes = [
        data.join("jobmind.db"),
        data.join("jobmind.db-wal"),
        data.join("sauvegarde-notes.db"),
        data.join("verification.db"),
        data.join("cles.json"),
        data.join("cles.json.nouveau"),
        data.join("cles.json.ecriture-1-2-3"),
        d.join("cles.json"),
        d.join("cles.json.nouveau"),
        d.join("journal.txt"),
        sauv.join("auto-2026-11-19.zip"),
        sauv.join("avant-restauration-2026-11-19-140507.zip"),
        sauv.join("Jobmind sauvegarde 2026-11-19.zip"),
        sauv.join("base-abimee-2026-11-19-140507.db"),
        sauv.join("photo.ecriture-pas-un-reste"),
        sauv.join("photo.ecriture-"),
    ];
    for f in restes.iter().chain(&gardes) {
        std::fs::write(f, b"x").unwrap();
    }
    let effaces = sauvegarde::nettoyer_restes(&d);
    for f in &restes {
        assert!(!f.exists(), "{f:?}");
        assert!(effaces.contains(f), "{f:?}");
    }
    assert!(!d.join("cv.nouveau").exists());
    for f in &gardes {
        assert!(f.is_file(), "{f:?}");
    }
    // Une restauration en cours : sa base préparée n'est pas touchée (annuler s'en occupe)
    std::fs::write(d.join("restauration-en-cours.txt"), b"base=1\ncv=1\n").unwrap();
    std::fs::write(data.join("jobmind.db.nouvelle"), b"x").unwrap();
    std::fs::write(data.join("jobmind.db.avant"), b"x").unwrap();
    assert!(sauvegarde::nettoyer_restes(&d).is_empty());
    assert!(data.join("jobmind.db.nouvelle").is_file() && data.join("jobmind.db.avant").is_file());
}

/// Une sauvegarde remise quand la base est abîmée (ou absente) : sans base ouverte, la base abîmée copiée telle
/// quelle dans sauvegardes\, puis la remise ordinaire (annulable, puis confirmée).
#[test]
fn remise_avec_une_base_abimee() {
    let source = Essai::nouveau("abimee-source");
    let ds = source.donnees();
    let b = remplir(&ds, "a");
    let (zip, _) = sauvegarde::faire(&b, &ds, moment()).unwrap();
    let cible = Essai::nouveau("abimee-cible");
    let dc = cible.donnees();
    std::fs::create_dir_all(dc.join("data")).unwrap();
    std::fs::write(dc.join("data").join("jobmind.db"), b"SQLite format 3\0 tout abime").unwrap();
    assert!(Base::ouvrir(&dc, moment()).is_err());
    std::fs::write(dc.join("data").join("jobmind.db-wal"), b"wal").unwrap();
    // (SQLite a pu effacer un -wal en échouant : le compagnon est écrit ensuite)
    let contenu = sauvegarde::lire(&zip, &dc).unwrap();
    // Annulée : la base abîmée revient telle quelle
    let copie = sauvegarde::remplacer_base_abimee(&dc, &contenu, moment()).unwrap().unwrap();
    assert_eq!(copie, dc.join("sauvegardes").join("base-abimee-2026-11-19-140507.db"));
    assert_eq!(std::fs::read(&copie).unwrap(), b"SQLite format 3\0 tout abime");
    assert_eq!(std::fs::read(dc.join("sauvegardes").join("base-abimee-2026-11-19-140507.db-wal")).unwrap(), b"wal");
    assert!(sauvegarde::en_attente(&dc));
    assert!(matches!(sauvegarde::remplacer_base_abimee(&dc, &contenu, moment()), Err(ErreurSauvegarde::EnCours)));
    sauvegarde::annuler(&dc).unwrap();
    assert_eq!(std::fs::read(dc.join("data").join("jobmind.db")).unwrap(), b"SQLite format 3\0 tout abime");
    // Gardée : la base de la sauvegarde s'ouvre, la copie de la base abîmée reste
    let m2 = Moment::nouveau(2026, 11, 19, 15, 0, 0).unwrap();
    sauvegarde::remplacer_base_abimee(&dc, &contenu, m2).unwrap().unwrap();
    let (b2, _) = Base::ouvrir(&dc, m2).unwrap();
    assert_eq!(vidage(&b2), vidage(&b));
    drop(b2);
    sauvegarde::confirmer(&dc).unwrap();
    assert_eq!(noms(&dc.join("data")), ["jobmind.db"]);
    assert!(copie.is_file());
    // Sans base du tout : pas de copie, la remise se fait
    let vide = Essai::nouveau("abimee-vide");
    let dv = vide.donnees();
    assert_eq!(sauvegarde::remplacer_base_abimee(&dv, &contenu, moment()).unwrap(), None);
    sauvegarde::confirmer(&dv).unwrap();
    assert!(Base::ouvrir(&dv, moment()).is_ok());
}
