//! L'adresse pour postuler mesurée sur des offres écrites à part, sans les règles sous les yeux
//! (`adresses_jamais_vues.tsv` : le nom, le texte de l'offre avec ses retours à la ligne écrits « \n », l'adresse
//! attendue ou rien), chacun écrit par un agent qui n'a jamais vu les règles. Le premier : 17 justes sur 17 du premier
//! coup ; le second (plus dur, écrit ensuite) : 12 sur 15 avant retouche des règles (une offre « questions à un ancien
//! responsable, candidature en ligne seulement », « (a) » pour l'arobase, deux adresses selon le lieu), 15 sur 15
//! depuis. Une adresse mal lue est montrée ; le test
//! demande au moins 80 % de justes, et jamais une adresse inventée (une adresse proposée doit être écrite dans
//! l'offre, à des accents ou à un masquage près).

use jobmind_core::envoi::trouver_adresse;

fn texte(t: &str) -> String {
    let mut r = String::new();
    let mut c = t.chars();
    while let Some(x) = c.next() {
        if x == '\\' {
            match c.next() {
                Some('n') => r.push('\n'),
                Some(y) => r.push(y),
                None => {}
            }
        } else {
            r.push(x);
        }
    }
    r
}

fn mesurer(jeu: &str) -> (usize, usize) {
    let (mut justes, mut total) = (0, 0);
    for l in jeu.lines().filter(|l| !l.trim().is_empty()) {
        let c: Vec<&str> = l.split('\t').collect();
        assert_eq!(c.len(), 3, "{l}");
        let t = texte(c[1]);
        let attendu = c[2].trim();
        let lu = trouver_adresse(&t);
        let adresse = lu.as_ref().map(|x| x.adresse.as_str()).unwrap_or("");
        total += 1;
        if adresse == attendu {
            justes += 1;
        } else {
            println!("mal lu : {} => {adresse:?} (attendu {attendu:?})", c[0]);
        }
        if let Some(x) = &lu {
            // (rien d'inventé : la partie avant @ est écrite dans l'offre)
            let local = x.adresse.split('@').next().unwrap_or_default();
            assert!(t.to_lowercase().contains(local), "{} : {} n'est pas dans l'offre", c[0], x.adresse);
            assert!(!x.extrait.is_empty() && t.contains(&x.extrait), "{} : extrait {:?}", c[0], x.extrait);
        }
    }
    println!("{justes} justes sur {total}");
    (justes, total)
}

#[test]
fn adresses_jamais_vues() {
    let (j, t) = mesurer(include_str!("adresses_jamais_vues.tsv"));
    assert!(t >= 8);
    assert!(j * 100 >= t * 80, "{j} justes sur {t}");
}

#[test]
fn adresses_jamais_vues_2() {
    let (j, t) = mesurer(include_str!("adresses_jamais_vues_2.tsv"));
    assert!(t >= 8);
    assert!(j * 100 >= t * 80, "{j} justes sur {t}");
}
