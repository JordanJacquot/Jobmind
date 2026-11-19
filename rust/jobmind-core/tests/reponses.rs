//! Le statut proposé mesuré sur des réponses écrites à part, sans les règles sous les yeux
//! (`reponses_jamais_vues*.tsv` : l'objet, le texte, ce que dit la réponse). Le premier jeu a donné 14 justes sur 20
//! avant que des tournures y soient ajoutées aux règles, le second (écrit ensuite) 5 sur 10 ; tous deux sont justes
//! depuis (les règles mesurées une troisième fois sur la vraie boîte de Jordan diront le vrai chiffre). Une
//! réponse mal lue est montrée ; le test demande au moins 70 % de justes, et jamais un refus ou une embauche annoncés
//! à tort (les pires erreurs : Jordan confirme toujours, mais ce serait une fausse alerte).

use jobmind_core::mails::{Proposition, proposer};

fn mesurer(jeu: &str) -> (usize, usize) {
    let (mut justes, mut total, mut graves) = (0, 0, Vec::new());
    for l in jeu.lines().filter(|l| !l.trim().is_empty()) {
        let c: Vec<&str> = l.split('\t').collect();
        let attendu = Proposition::depuis_code(c[2].trim());
        let lu = proposer(c[0], c[1]).proposition.map(|(p, _)| p);
        total += 1;
        if lu == attendu {
            justes += 1;
        } else {
            println!("mal lu : {} => {lu:?} (attendu {attendu:?})", c[1]);
            if matches!(lu, Some(Proposition::Refus | Proposition::Positive)) {
                graves.push(c[1].to_string());
            }
        }
    }
    println!("{justes} justes sur {total}");
    assert!(graves.is_empty(), "annoncés à tort : {graves:?}");
    (justes, total)
}

#[test]
fn reponses_jamais_vues() {
    let (j, t) = mesurer(include_str!("reponses_jamais_vues.tsv"));
    assert!(j * 100 >= t * 70, "{j} justes sur {t}");
}

#[test]
fn reponses_jamais_vues_2() {
    let (j, t) = mesurer(include_str!("reponses_jamais_vues_2.tsv"));
    assert!(j * 100 >= t * 70, "{j} justes sur {t}");
}
