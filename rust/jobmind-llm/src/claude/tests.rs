//! Le client de Claude contre un faux serveur local (127.0.0.1, un port pris au hasard) : jamais la vraie API, jamais
//! une vraie clé (« sk-ant-essai... » est inventée).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::lecture;

const CLE: &str = "sk-ant-essai-0000000000000000000000000000";

/// Un faux serveur qui répond une fois : ce statut, ce corps, après cette attente. Rend son adresse et la demande reçue.
fn faux(statut: u16, corps: &str, attente: Duration) -> (String, Arc<Mutex<String>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = format!("http://{}/v1/messages", l.local_addr().unwrap());
    let recue = Arc::new(Mutex::new(String::new()));
    let r = Arc::clone(&recue);
    let corps = corps.to_string();
    std::thread::spawn(move || {
        let Ok((mut s, _)) = l.accept() else { return };
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut tout = Vec::new();
        let mut tampon = [0u8; 4096];
        // (les en-têtes, puis le corps d'après Content-Length)
        loop {
            let n = s.read(&mut tampon).unwrap_or(0);
            if n == 0 {
                break;
            }
            tout.extend_from_slice(&tampon[..n]);
            let t = String::from_utf8_lossy(&tout).to_string();
            if let Some(fin) = t.find("\r\n\r\n") {
                let long = t[..fin]
                    .lines()
                    .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|x| x.trim().to_string()))
                    .and_then(|x| x.parse::<usize>().ok())
                    .unwrap_or(0);
                if tout.len() >= fin + 4 + long {
                    break;
                }
            }
        }
        *r.lock().unwrap() = String::from_utf8_lossy(&tout).to_string();
        std::thread::sleep(attente);
        let raison = match statut {
            200 => "OK",
            401 => "Unauthorized",
            429 => "Too Many Requests",
            _ => "Erreur",
        };
        let rep = format!(
            "HTTP/1.1 {statut} {raison}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corps}",
            corps.len()
        );
        let _ = s.write_all(rep.as_bytes());
    });
    (adresse, recue)
}

fn reponse_ok(json_rendu: &str) -> String {
    json!({
        "id": "msg_essai",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-5-5",
        "content": [{ "type": "text", "text": json_rendu }],
        "stop_reason": "end_turn",
        "usage": { "input_tokens": 812, "output_tokens": 64 },
    })
    .to_string()
}

const OFFRE: &str = "Magasinier H/F. Vous rangez les pièces détachées et préparez les envois. Une première \
                     expérience en magasin est souhaitée. Poste en journée.";

fn appeler(adresse: &str, delai: Duration) -> Result<Reponse, ErreurLlm> {
    Claude::nouveau(adresse, CLE, false, delai).repondre(&lecture::demande("Magasinier", OFFRE))
}

#[test]
fn une_reponse_juste_et_la_demande_envoyee() {
    let rendu = r#"{"diplome":"non_dit","diplome_citation":"","experience":"souhaitee","experience_mois":0,"experience_citation":"Une première expérience en magasin est souhaitée"}"#;
    let (a, recue) = faux(200, &reponse_ok(rendu), Duration::ZERO);
    let r = appeler(&a, Duration::from_secs(5)).unwrap();
    assert_eq!((r.jetons_entree, r.jetons_sortie), (812, 64));
    let lu = lecture::comprendre(&lecture::texte_envoye("Magasinier", OFFRE), &r.texte).unwrap();
    assert_eq!(lu.experience.unwrap().0, jobmind_core::lecture::Experience::Souhaitee);
    // La demande : les en-têtes de l'API, le petit modèle, le schéma, la réflexion coupée
    let d = recue.lock().unwrap().clone();
    let bas = d.to_ascii_lowercase();
    assert!(bas.starts_with("post /v1/messages "), "{d}");
    assert!(bas.contains(&format!("x-api-key: {CLE}")));
    assert!(bas.contains("anthropic-version: 2026-10-21"));
    let corps: Value = serde_json::from_str(&d[d.find("\r\n\r\n").unwrap() + 4..]).unwrap();
    assert_eq!(corps["model"], "claude-haiku-5-5");
    assert_eq!(corps["output_config"]["format"]["type"], "json_schema");
    assert_eq!(corps["output_config"]["effort"], "low");
    assert_eq!(corps["thinking"]["type"], "disabled");
    assert_eq!(corps["messages"][0]["role"], "user");
    assert!(corps["messages"][0]["content"].as_str().unwrap().contains("pièces détachées"));
}

#[test]
fn une_citation_inventee_par_le_serveur_est_rejetee() {
    let rendu = r#"{"diplome":"bac2","diplome_citation":"BTS logistique exigé","experience":"duree","experience_mois":24,"experience_citation":"deux ans d'expérience en magasin"}"#;
    let (a, _) = faux(200, &reponse_ok(rendu), Duration::ZERO);
    let r = appeler(&a, Duration::from_secs(5)).unwrap();
    let lu = lecture::comprendre(&lecture::texte_envoye("Magasinier", OFFRE), &r.texte).unwrap();
    assert!(lu.citation_inventee && lu.diplome.is_none() && lu.experience.is_none());
}

#[test]
fn json_abime() {
    let (a, _) = faux(200, "{\"content\": [", Duration::ZERO);
    assert!(matches!(appeler(&a, Duration::from_secs(5)), Err(ErreurLlm::Illisible(_))));
    // Un JSON juste dont le texte n'est pas le JSON demandé
    let (a, _) = faux(200, &reponse_ok("Je ne sais pas."), Duration::ZERO);
    let r = appeler(&a, Duration::from_secs(5)).unwrap();
    assert!(matches!(lecture::comprendre(OFFRE, &r.texte), Err(ErreurLlm::Illisible(_))));
}

#[test]
fn les_erreurs_de_l_api() {
    let erreur = |t: &str| json!({ "type": "error", "error": { "type": t, "message": "x" } }).to_string();
    for (statut, genre, attendu) in [
        (401, "authentication_error", "cle_refusee"),
        (403, "permission_error", "cle_refusee"),
        (429, "rate_limit_error", "limite"),
        (529, "overloaded_error", "surcharge"),
        (503, "overloaded_error", "surcharge"),
        (500, "api_error", "reseau"),
        (400, "invalid_request_error", "refus"),
    ] {
        let (a, _) = faux(statut, &erreur(genre), Duration::ZERO);
        let e = appeler(&a, Duration::from_secs(5)).unwrap_err();
        assert_eq!(e.code(), attendu, "{statut} {e}");
        // Jamais la clé dans une erreur
        assert!(!format!("{e} {e:?} {}", e.message()).contains("sk-ant"), "{e}");
    }
    // Refus et réponse coupée
    let refus = json!({ "content": [], "stop_reason": "refusal" }).to_string();
    assert_eq!(lire_reponse(200, &refus).unwrap_err().code(), "refus");
    let coupee = json!({ "content": [{ "type": "text", "text": "{" }], "stop_reason": "max_tokens" }).to_string();
    assert_eq!(lire_reponse(200, &coupee).unwrap_err().code(), "illisible");
    // Le texte vient des blocs « text », même après un autre bloc
    let deux = json!({ "content": [{ "type": "thinking", "thinking": "" }, { "type": "text", "text": "{}" }], "stop_reason": "end_turn" });
    assert_eq!(lire_reponse(200, &deux.to_string()).unwrap().texte, "{}");
}

#[test]
fn le_delai_depasse() {
    let (a, _) = faux(200, &reponse_ok("{}"), Duration::from_secs(4));
    let debut = std::time::Instant::now();
    assert_eq!(appeler(&a, Duration::from_secs(1)).unwrap_err(), ErreurLlm::Delai);
    assert!(debut.elapsed() < Duration::from_secs(3));
}

#[test]
fn sans_cle_et_sans_serveur() {
    let c = Claude::nouveau("http://127.0.0.1:9/v1/messages", "", false, Duration::from_secs(2));
    assert_eq!(c.repondre(&lecture::demande("x", "y")).unwrap_err(), ErreurLlm::CleManquante);
    // (personne n'écoute sur le port 9)
    let c = Claude::nouveau("http://127.0.0.1:9/v1/messages", CLE, false, Duration::from_secs(2));
    assert!(matches!(c.repondre(&lecture::demande("x", "y")), Err(ErreurLlm::Reseau(_) | ErreurLlm::Delai)));
    // https seulement : une adresse en http est refusée sans rien envoyer
    let c = Claude::nouveau("http://127.0.0.1:9/v1/messages", CLE, true, Duration::from_secs(2));
    assert!(matches!(c.repondre(&lecture::demande("x", "y")), Err(ErreurLlm::Reseau(_))));
    assert!(!format!("{c:?}").contains("sk-ant"));
    assert_eq!(ADRESSE, "https://api.anthropic.com/v1/messages");
}
