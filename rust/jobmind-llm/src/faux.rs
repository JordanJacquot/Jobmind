//! Un faux Claude, pour les tests et la construction d'essai de l'appli (option « faux », jamais livrée) : il répond à
//! une demande de candidature (étape 8) comme l'API Messages, sans jamais joindre la vraie API ni demander une vraie
//! clé. Sa réponse se construit à partir de la demande reçue : il reprend des lignes du CV envoyé (des passages qui y
//! sont vraiment) et ajoute exprès une phrase inventée (« encadré une équipe de six personnes ») que Jobmind doit
//! surligner « à vérifier ». Ses modes imitent les ennuis : un refus, trop de demandes, une clé refusée, une
//! surcharge, une réponse abîmée, un serveur lent. Étape 9 : une demande de réécriture (`<lettre_a_reecrire>`) reçoit
//! la lettre retouchée phrase par phrase ([`rendu_style`]), avec exprès un nombre ajouté ([`AJOUT_INVENTE`]) que Jobmind
//! doit refuser.
//!
//! [`ServeurClaude`] : un petit serveur HTTP local (127.0.0.1, un port libre) qui répond ainsi, pour les tests.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

/// Ce que fait le faux Claude.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModeFaux {
    #[default]
    Normal,
    /// `stop_reason` « refusal »
    Refus,
    /// 429 : trop de demandes
    Quota,
    /// 401 : la clé est refusée
    CleRefusee,
    /// 529 : surchargé
    Surcharge,
    /// Un JSON coupé au milieu
    Abime,
    /// Répond normalement, mais au bout de 6 secondes
    Lent,
}

impl ModeFaux {
    /// « refus », « quota », « cle », « surcharge », « abime », « lent » ; autre chose : normal.
    pub fn depuis(t: &str) -> ModeFaux {
        match t.trim() {
            "refus" => ModeFaux::Refus,
            "quota" => ModeFaux::Quota,
            "cle" => ModeFaux::CleRefusee,
            "surcharge" => ModeFaux::Surcharge,
            "abime" => ModeFaux::Abime,
            "lent" => ModeFaux::Lent,
            _ => ModeFaux::Normal,
        }
    }
}

/// La phrase inventée exprès par le faux Claude (Jobmind doit la surligner).
pub const PHRASE_INVENTEE: &str = "J'ai aussi encadré une équipe de six personnes.";

/// Une partie du texte envoyé (`<cv>...</cv>`).
fn section<'a>(t: &'a str, nom: &str) -> &'a str {
    let o = format!("<{nom}>\n");
    let f = format!("\n</{nom}>");
    match t.find(&o) {
        Some(i) => {
            let d = i + o.len();
            let e = t[d..].find(&f).map_or(t.len(), |x| d + x);
            &t[d..e]
        }
        None => "",
    }
}

fn mots(t: &str, n: usize) -> String {
    t.split_whitespace().take(n).collect::<Vec<_>>().join(" ")
}

fn phrase(t: &str, sur_moi: bool, extraits: &[String]) -> Value {
    json!({ "texte": t, "sur_moi": sur_moi, "extraits": extraits })
}

/// Le JSON qu'aurait rendu Claude pour ce texte envoyé.
pub fn rendu(envoye: &str) -> Value {
    let offre = section(envoye, "offre");
    let cv = section(envoye, "cv");
    let consigne = !section(envoye, "consigne").trim().is_empty();
    let ligne = |k: &str| offre.lines().find_map(|l| l.strip_prefix(k)).map(str::trim).unwrap_or_default().to_string();
    let (titre, entreprise) = (ligne("Titre : "), ligne("Entreprise : "));
    let lignes: Vec<&str> = cv.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    // (les lignes d'expérience : « 2023 à 2025 : ... »)
    let experiences: Vec<String> = lignes
        .iter()
        .filter_map(|l| l.split_once(" : ").filter(|(a, _)| a.chars().any(|c| c.is_ascii_digit())).map(|(_, b)| b))
        .map(|b| mots(b, 12))
        .filter(|b| b.split_whitespace().count() >= 3)
        .collect();
    let diplome = lignes
        .iter()
        .find(|l| ["CAP", "BEP", "Bac", "BTS", "CACES", "Licence", "Master", "DUT"].iter().any(|d| l.contains(d)))
        .map(|l| mots(l, 10))
        .filter(|l| l.split_whitespace().count() >= 2);
    let nom = lignes
        .first()
        .filter(|l| (2..=4).contains(&l.split_whitespace().count()) && !l.chars().any(|c| c.is_ascii_digit()))
        .map(|l| (*l).to_string())
        .unwrap_or_default();
    let mut p2 = Vec::new();
    match experiences.first() {
        Some(e) => p2.push(phrase(&format!("J'ai travaillé comme {e}."), true, std::slice::from_ref(e))),
        None => p2.push(phrase(
            "J'ai une solide expérience en logistique.",
            true,
            &["expérience en logistique".to_string()],
        )),
    }
    if let (Some(e), false) = (experiences.get(1), consigne) {
        p2.push(phrase(&format!("Avant cela, j'ai travaillé comme {e}."), true, std::slice::from_ref(e)));
    }
    if let Some(d) = &diplome {
        p2.push(phrase(&format!("Je suis titulaire du {d}."), true, std::slice::from_ref(d)));
    }
    p2.push(phrase(PHRASE_INVENTEE, true, &["chef d'équipe de six personnes".to_string()]));
    let paragraphes = vec![
        vec![phrase(
            &format!(
                "Votre offre pour le poste de {} chez {} a retenu toute mon attention.",
                titre.to_lowercase(),
                entreprise
            ),
            false,
            &[],
        )],
        p2,
        vec![phrase("Je serais heureux de vous présenter mon parcours lors d'un entretien.", false, &[])],
    ];
    // Les questions : les phrases de l'offre qui finissent par « ? »
    let questions: Vec<Value> = offre
        .split(['\n', '.'])
        .map(str::trim)
        .filter(|l| l.ends_with('?') && l.split_whitespace().count() >= 3)
        .take(3)
        .map(|q| {
            let rep = match experiences.first() {
                Some(e) => {
                    phrase(&format!("Ce poste correspond à ce que j'ai fait : {e}."), true, std::slice::from_ref(e))
                }
                None => phrase("Ce métier me plaît depuis longtemps.", true, &[]),
            };
            json!({ "question": q, "extrait_offre": q, "reponse": [rep] })
        })
        .collect();
    let mut conseils: Vec<Value> = Vec::new();
    if let Some(e) = experiences.first() {
        conseils.push(json!({
            "genre": "mettre_en_avant",
            "texte": format!("Place en haut de ton CV ton expérience : {e}."),
            "extrait_cv": e, "extrait_offre": "",
        }));
    }
    if let Some(d) = &diplome {
        conseils.push(json!({
            "genre": "reformuler",
            "texte": format!("Écris « {d} » dans une rubrique Diplômes et habilitations, bien visible."),
            "extrait_cv": d, "extrait_offre": "",
        }));
    }
    if let Some(s) = offre
        .split(['\n', '.'])
        .map(str::trim)
        .find(|l| ["exig", "souhait", "maîtris", "obligatoire", "requis"].iter().any(|m| l.contains(m)))
    {
        conseils.push(json!({
            "genre": "absent",
            "texte": format!("L'offre demande : « {} ».", mots(s, 10)),
            "extrait_cv": "", "extrait_offre": mots(s, 10),
        }));
    }
    json!({
        "objet": format!("Candidature au poste de {}", titre.to_lowercase()),
        "appel": "Madame, Monsieur,",
        "paragraphes": paragraphes,
        "politesse": "Je vous prie d'agréer, Madame, Monsieur, l'expression de mes salutations distinguées.",
        "signature": nom,
        "questions": questions,
        "conseils_cv": conseils,
    })
}

/// Étape 9 : ce que le faux Claude ajoute exprès en réécrivant (un nombre absent du CV : Jobmind doit refuser la phrase
/// et remettre celle de départ).
pub const AJOUT_INVENTE: &str = " pendant quatre ans";

/// Le plus long passage (3 à 12 mots) de la phrase qui est mot pour mot dans le CV.
fn extrait_du_cv(phrase: &str, cv: &str) -> Option<String> {
    let m: Vec<&str> = phrase.trim_end_matches('.').split_whitespace().collect();
    for n in (3..=12.min(m.len())).rev() {
        for w in m.windows(n) {
            let x = w.join(" ");
            if jobmind_core::lecture::citation_presente(cv, &x) {
                return Some(x);
            }
        }
    }
    None
}

/// Étape 9 : le JSON qu'aurait rendu Claude pour une réécriture (la lettre numérotée de `<lettre_a_reecrire>`). Il
/// reprend chaque phrase en la retouchant un peu, cite le CV pour celles qui parlent de Jordan, et ajoute exprès un
/// nombre ([`AJOUT_INVENTE`]) à la première d'entre elles.
pub fn rendu_style(envoye: &str) -> Value {
    let lettre = section(envoye, "lettre_a_reecrire");
    let cv = section(envoye, "cv");
    let forte = envoye.contains("<intensite>forte</intensite>");
    let mut ajoute = false;
    let paragraphes: Vec<Value> = lettre
        .split("\n\n")
        .map(|p| {
            let l: Vec<Value> = p
                .lines()
                .filter_map(|ligne| {
                    let (n, t) = ligne.trim().strip_prefix('[')?.split_once("] ")?;
                    let n: u64 = n.parse().ok()?;
                    let mut t = t
                        .replace("a retenu toute mon attention", "m'intéresse beaucoup")
                        .replace("de vous présenter mon parcours", "d'en parler avec vous")
                        .replace("une équipe de six personnes", "six personnes");
                    if forte {
                        t = t.replace("J'ai travaillé comme", "J'ai été");
                    }
                    let extrait = extrait_du_cv(&t, cv);
                    let sur_moi = extrait.is_some() || t.starts_with("J'ai");
                    if extrait.is_some() && !ajoute && t.ends_with('.') {
                        ajoute = true;
                        t = format!("{}{AJOUT_INVENTE}.", t.trim_end_matches('.'));
                    }
                    let extraits: Vec<String> = match extrait {
                        Some(e) => vec![e],
                        None if sur_moi => vec!["chef d'équipe de six personnes".to_string()],
                        None => Vec::new(),
                    };
                    Some(json!({ "texte": t, "depuis": [n], "sur_moi": sur_moi, "extraits": extraits }))
                })
                .collect();
            Value::Array(l)
        })
        .filter(|p| p.as_array().is_some_and(|a| !a.is_empty()))
        .collect();
    json!({ "paragraphes": paragraphes })
}

/// La réponse de l'API à ce corps de demande, dans ce mode : (statut, corps, attente).
pub fn repondre(corps: &str, mode: ModeFaux) -> (u16, String, Duration) {
    let erreur = |t: &str| json!({ "type": "error", "error": { "type": t, "message": "faux Claude" } }).to_string();
    let message = |contenu: Value, fin: &str| {
        json!({
            "id": "msg_faux",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-5-5",
            "content": contenu,
            "stop_reason": fin,
            "usage": { "input_tokens": corps.len() / 4, "output_tokens": 1800 },
        })
        .to_string()
    };
    let envoye = serde_json::from_str::<Value>(corps)
        .ok()
        .and_then(|v| v.pointer("/messages/0/content").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    match mode {
        ModeFaux::Quota => (429, erreur("rate_limit_error"), Duration::ZERO),
        ModeFaux::CleRefusee => (401, erreur("authentication_error"), Duration::ZERO),
        ModeFaux::Surcharge => (529, erreur("overloaded_error"), Duration::ZERO),
        ModeFaux::Refus => (200, message(json!([]), "refusal"), Duration::ZERO),
        ModeFaux::Abime => (
            200,
            message(
                json!([{ "type": "text", "text": "{\"objet\": \"Candidature\", \"paragraphes\": [[{\"texte\": \"Bonj" }]),
                "end_turn",
            ),
            Duration::ZERO,
        ),
        ModeFaux::Normal | ModeFaux::Lent => {
            let texte = if envoye.contains("<lettre_a_reecrire>") {
                rendu_style(&envoye).to_string()
            } else {
                rendu(&envoye).to_string()
            };
            let attente = if mode == ModeFaux::Lent { Duration::from_secs(6) } else { Duration::ZERO };
            (
                200,
                message(json!([{ "type": "thinking", "thinking": "" }, { "type": "text", "text": texte }]), "end_turn"),
                attente,
            )
        }
    }
}

/// Un petit serveur HTTP local (127.0.0.1, un port libre) qui répond comme [`repondre`] ; arrêté quand il est lâché.
pub struct ServeurClaude {
    port: u16,
    fin: Arc<AtomicBool>,
    /// Le mode, changeable pendant qu'il tourne
    pub mode: Arc<Mutex<ModeFaux>>,
    /// Les corps reçus
    pub recus: Arc<Mutex<Vec<String>>>,
}

impl ServeurClaude {
    pub fn demarrer(mode: ModeFaux) -> std::io::Result<ServeurClaude> {
        let ecoute = TcpListener::bind("127.0.0.1:0")?;
        let port = ecoute.local_addr()?.port();
        let fin = Arc::new(AtomicBool::new(false));
        let mode = Arc::new(Mutex::new(mode));
        let recus = Arc::new(Mutex::new(Vec::new()));
        let (f, m, r) = (Arc::clone(&fin), Arc::clone(&mode), Arc::clone(&recus));
        std::thread::Builder::new().name("faux-claude".into()).spawn(move || {
            for c in ecoute.incoming() {
                if f.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(c) = c else { continue };
                let (m, r) = (Arc::clone(&m), Arc::clone(&r));
                let _ = std::thread::Builder::new().name("faux-claude-client".into()).spawn(move || servir(c, &m, &r));
            }
        })?;
        Ok(ServeurClaude { port, fin, mode, recus })
    }

    /// « http://127.0.0.1:<port>/v1/messages »
    pub fn adresse(&self) -> String {
        format!("http://127.0.0.1:{}/v1/messages", self.port)
    }
}

impl Drop for ServeurClaude {
    fn drop(&mut self) {
        self.fin.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.port).into(), Duration::from_millis(200));
    }
}

fn servir(mut s: TcpStream, mode: &Mutex<ModeFaux>, recus: &Mutex<Vec<String>>) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let mut tout = Vec::new();
    let mut tampon = [0u8; 8192];
    let corps = loop {
        let n = s.read(&mut tampon).unwrap_or(0);
        if n == 0 {
            return;
        }
        tout.extend_from_slice(&tampon[..n]);
        if let Some(fin) = tout.windows(4).position(|w| w == b"\r\n\r\n") {
            let tete = String::from_utf8_lossy(&tout[..fin]).to_ascii_lowercase();
            let long = tete
                .lines()
                .find_map(|l| l.strip_prefix("content-length:").map(|x| x.trim().to_string()))
                .and_then(|x| x.parse::<usize>().ok())
                .unwrap_or(0)
                .min(4_000_000);
            if tout.len() >= fin + 4 + long {
                break String::from_utf8_lossy(&tout[fin + 4..fin + 4 + long]).into_owned();
            }
        }
    };
    if let Ok(mut l) = recus.lock() {
        l.push(corps.clone());
    }
    let m = mode.lock().map(|m| *m).unwrap_or_default();
    let (statut, rep, attente) = repondre(&corps, m);
    if !attente.is_zero() {
        std::thread::sleep(attente);
    }
    let t = format!(
        "HTTP/1.1 {statut} Faux\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{rep}",
        rep.len()
    );
    let _ = s.write_all(t.as_bytes());
    let _ = s.flush();
}
