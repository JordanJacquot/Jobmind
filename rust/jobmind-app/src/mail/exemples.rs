//! Construction d'essai et tests (étape 7) : la fausse boîte Gmail, des mails bruts inventés (jamais de vrais).

use jobmind_storage::Moment;

/// L'adresse et le mot de passe d'application inventés que la fausse boîte attend (jamais de vrais).
pub const BOITE_ADRESSE: &str = "jordan.exemple@gmail.com";
pub const BOITE_MDP: &str = "essaiessaiessaii";
/// Ce fichier (`{ "adresse", "mdp", "envoyer" }`, les identifiants inventés ci-dessus) remplit « Ma boîte mail » comme
/// Jordan le ferait (et clique « Brancher ma boîte » si `envoyer`) ; supprimé aussitôt lu, rien de son contenu au
/// journal.
pub const BOITE_DEMANDE: &str = "boite.demande";

/// Un mail brut (RFC 5322) : texte seul, ou HTML et texte (multipart/alternative), en UTF-8.
pub fn mail_brut(id: &str, de: &str, objet: &str, date: &str, texte: &str, html: Option<&str>) -> Vec<u8> {
    let mut m = format!(
        "Message-ID: <{id}>\r\nFrom: {de}\r\nTo: Jordan Exemple <{BOITE_ADRESSE}>\r\nSubject: =?UTF-8?B?{}?=\r\nDate: \
         {date}\r\nMIME-Version: 1.0\r\n",
        base64(objet.as_bytes())
    );
    match html {
        None => {
            m.push_str("Content-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n");
            m.push_str(&texte.replace('\n', "\r\n"));
        }
        Some(h) => {
            m.push_str("Content-Type: multipart/alternative; boundary=\"jobmind-essai\"\r\n\r\n");
            m.push_str("--jobmind-essai\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n");
            m.push_str(&base64(texte.as_bytes()));
            m.push_str("\r\n--jobmind-essai\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n");
            m.push_str(&base64(h.as_bytes()));
            m.push_str("\r\n--jobmind-essai--\r\n");
        }
    }
    m.into_bytes()
}

/// Le base64 d'un texte (en lignes de 76 caractères).
fn base64(o: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for (i, c) in o.chunks(3).enumerate() {
        if i > 0 && i % 19 == 0 {
            s.push_str("\r\n");
        }
        let n =
            (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    s
}

/// Les mails de la fausse boîte : les alertes d'exemple des quatre sites, des réponses inventées aux candidatures
/// d'exemple (un refus, un entretien proposé, une demande d'informations), un mail à ranger, une publicité.
pub fn boite_exemples(maintenant: Moment) -> Vec<Vec<u8>> {
    const MOIS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let date = |h: u32| {
        format!(
            "{} {} {} {:02}:{:02}:00 +0100",
            maintenant.jour,
            MOIS[(maintenant.mois as usize).clamp(1, 12) - 1],
            maintenant.annee,
            h.min(maintenant.heure),
            if h < maintenant.heure { 15 } else { maintenant.minute.saturating_sub(1) }
        )
    };
    let mut l = Vec::new();
    for (i, a) in jobmind_sources::alertes::exemples::ALERTES.iter().enumerate() {
        let de = if a.de_nom.is_empty() { a.de.to_string() } else { format!("\"{}\" <{}>", a.de_nom, a.de) };
        let html = (!a.html.is_empty()).then_some(a.html);
        l.push(mail_brut(&format!("alerte-{i}@essai.jobmind"), &de, a.objet, &date(6), a.texte, html));
    }
    l.push(mail_brut(
        "refus-1@transports-moselle-fret.fr",
        "Service recrutement <recrutement@transports-moselle-fret.fr>",
        "Votre candidature au poste d'agent de quai",
        &date(7),
        "Bonjour,\n\nNous vous remercions de l'intérêt que vous portez à Transports Moselle Fret.\n\nAprès étude de votre \
         dossier, nous sommes au regret de vous informer que votre candidature n'a pas été retenue pour ce poste.\n\n\
         Nous conservons votre CV et ne manquerons pas de revenir vers vous si un autre poste se libère.\n\nCordialement,\n\
         Le service recrutement",
        None,
    ));
    l.push(mail_brut(
        "entretien-1@netea-services.fr",
        "Sophie Exemple <s.exemple@netea-services.fr>",
        "Re: Candidature agent d'entretien",
        &date(8),
        "Bonjour Jordan,\n\nMerci pour votre candidature au poste d'agent d'entretien.\n\nVotre profil a retenu notre \
         attention et nous souhaiterions vous rencontrer pour un entretien à notre agence de Metz. Seriez-vous \
         disponible jeudi à 10 h ?\n\nBien à vous,\nSophie Exemple\nNetéa Services",
        Some(
            "<p>Bonjour Jordan,</p><p>Merci pour votre candidature au poste d'agent d'entretien.</p><p>Votre profil a \
              retenu notre attention et nous souhaiterions vous rencontrer pour un entretien à notre agence de Metz. \
              Seriez-vous disponible jeudi à 10 h ?</p><script>alert(1)</script><p>Bien à vous,<br>Sophie Exemple<br>\
              Netéa Services</p>",
        ),
    ));
    l.push(mail_brut(
        "infos-1@residence-les-tilleuls.fr",
        "Résidence Les Tilleuls <accueil@residence-les-tilleuls.fr>",
        "Votre candidature : agent de service",
        &date(9),
        "Bonjour,\n\nNous avons bien reçu votre candidature pour le poste d'agent de service.\n\nMerci de nous transmettre \
         une copie de votre carte Vitale et vos disponibilités pour le mois prochain.\n\nCordialement,\nLa direction",
        None,
    ));
    l.push(mail_brut(
        "ranger-1@essai.jobmind",
        "Ressources humaines <rh.recrutement.est@gmail.com>",
        "Votre candidature",
        &date(10),
        "Bonjour,\n\nNous avons bien reçu votre candidature pour le poste de vendeur rayon outillage. Elle sera étudiée \
         avec attention.\n\nCordialement",
        None,
    ));
    l.push(mail_brut(
        "pub-1@brico-lorraine.fr",
        "Brico Lorraine <news@brico-lorraine.fr>",
        "Les promotions de la semaine",
        &date(11),
        "Perceuses et visseuses à prix réduit cette semaine dans tous nos magasins.",
        None,
    ));
    l
}
