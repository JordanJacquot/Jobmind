//! Le score naturel mesuré sur des lettres jamais vues (`tests\jamais_vus`) : 12 écrites à la manière d'une IA, 12 à la
//! manière de vraies personnes, écrites à part par un agent qui n'a jamais lu les règles, APRÈS qu'elles ont été fixées
//! (19/11/2026). Le résultat est affiché (`cargo test -p jobmind-humanizer --test jamais_vus -- --nocapture`) ; le test
//! demande seulement que les lettres d'IA aient en moyenne un score plus bas que les lettres humaines.

use jobmind_humanizer::{Regles, score};

fn lettres(debut: &str) -> Vec<(String, String)> {
    let d = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("jamais_vus");
    let mut l: Vec<(String, String)> = std::fs::read_dir(&d)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(debut) && n.ends_with(".txt")))
        .map(|p| {
            let t = std::fs::read_to_string(&p).unwrap();
            (p.file_name().unwrap().to_string_lossy().into_owned(), t.trim_start_matches('\u{feff}').to_string())
        })
        .collect();
    l.sort();
    l
}

#[test]
fn score_sur_des_lettres_jamais_vues() {
    let r = Regles::default();
    let mut moyennes = Vec::new();
    for genre in ["ia_", "humain_"] {
        let l = lettres(genre);
        assert!(l.len() >= 10, "{genre} : {} lettres", l.len());
        let mut total = 0u32;
        for (nom, t) in &l {
            let s = score(t, &r);
            total += u32::from(s.note);
            println!("{nom:12} {:3}  {:18} {}", s.note, s.mot, s.principales().join(" | "));
        }
        let m = f64::from(total) / l.len() as f64;
        println!("{genre} moyenne : {m:.1}");
        moyennes.push(m);
    }
    assert!(moyennes[0] < moyennes[1], "{moyennes:?}");
}
