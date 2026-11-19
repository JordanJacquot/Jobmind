//! L'envoi d'une candidature par mail (étape 10 du plan), sans réseau : l'adresse pour postuler lue dans le texte
//! d'une offre ([`trouver_adresse`]), l'adresse tapée par Jordan et l'objet vérifiés, et le rythme des envois (une
//! seule candidature à la fois, au moins [`ECART_S`] secondes entre deux envois, [`PLAFOND_JOUR`] par jour au plus :
//! Jobmind n'est pas un outil d'envoi en masse).
//!
//! La lecture de l'adresse est prudente : une adresse « ne pas répondre », celle d'un site d'emploi, une adresse dans
//! un lien, ou une offre qui dit de ne pas postuler par mail ne donnent rien ; deux adresses, celle « pour postuler »
//! passe avant celle « pour tout renseignement ». Les adresses masquées contre les robots (« rh [at] exemple.fr »,
//! « jobs arobase exemple point fr », « emploi @ exemple.fr ») sont remises au propre. Jordan relit toujours l'adresse
//! avant l'envoi et peut la changer.

/// Au moins tant de secondes entre deux envois (réussis ou non).
pub const ECART_S: i64 = 60;
/// Au plus tant d'envois par mail par jour.
pub const PLAFOND_JOUR: u32 = 20;
/// Le plus long objet.
pub const OBJET_MAX: usize = 200;

/// Une adresse trouvée dans une offre, avec le bout de phrase où elle est écrite (tel quel, pour la montrer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trouvee {
    pub adresse: String,
    pub extrait: String,
}

/// Une adresse qui peut partir : ASCII, une seule @, des caractères permis de chaque côté, un domaine avec un point.
pub fn adresse_valide(a: &str) -> bool {
    a.len() <= 254
        && a.is_ascii()
        && a.matches('@').count() == 1
        && a.split_once('@').is_some_and(|(l, d)| {
            !l.is_empty()
                && l.len() <= 64
                && !l.starts_with('.')
                && !l.ends_with('.')
                && !l.contains("..")
                && l.chars().all(|c| c.is_ascii_alphanumeric() || "._%+-'".contains(c))
                && d.contains('.')
                && d.len() >= 4
                && !d.starts_with(['.', '-'])
                && !d.ends_with(['.', '-'])
                && !d.contains("..")
                && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
                && d.rsplit('.')
                    .next()
                    .is_some_and(|tld| tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic()))
        })
}

/// Vérifie l'adresse tapée (ou corrigée) par Jordan : espaces, chevrons, guillemets et « mailto: » retirés, en
/// minuscules. L'erreur est une phrase simple.
pub fn verifier_destinataire(t: &str) -> Result<String, String> {
    let mut a = t.trim().trim_matches(|c: char| matches!(c, '<' | '>' | '"' | '\'' | ' ')).to_lowercase();
    if let Some(x) = a.strip_prefix("mailto:") {
        a = x.split('?').next().unwrap_or_default().trim().to_string();
    }
    if a.is_empty() {
        return Err("Écris l'adresse mail de l'entreprise, celle que donne l'offre pour postuler.".into());
    }
    if a.chars().any(|c| c.is_whitespace() || c == ',' || c == ';') && a.matches('@').count() > 1 {
        return Err("Une seule adresse à la fois : Jobmind envoie ta candidature à une seule personne.".into());
    }
    if !adresse_valide(&a) {
        return Err("Cette adresse ne semble pas juste. Elle s'écrit comme recrutement@entreprise.fr.".into());
    }
    if exclue(&a) {
        return Err("Cette adresse ne reçoit pas de candidatures (une adresse « ne pas répondre » ou celle d'un site \
                    d'emploi). Postule sur le site de l'offre."
            .into());
    }
    Ok(a)
}

/// Vérifie l'objet : sur une ligne, sans tiret long (remplacé par un tiret court), 1 à 200 caractères.
pub fn verifier_objet(t: &str) -> Result<String, String> {
    let o: String = t
        .chars()
        .map(|c| {
            if c == '\u{2014}' || c == '\u{2013}' {
                '-'
            } else if c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if o.is_empty() {
        return Err("Écris l'objet du mail, par exemple « Candidature au poste de magasinier ».".into());
    }
    if o.chars().count() > OBJET_MAX {
        return Err(format!("L'objet est trop long : {OBJET_MAX} caractères au plus."));
    }
    Ok(o)
}

/// L'objet proposé quand Claude n'en a pas donné : « Candidature au poste de magasinier ».
pub fn objet_par_defaut(poste: &str) -> String {
    let p = poste.split_whitespace().collect::<Vec<_>>().join(" ");
    let p: String = p.chars().take(150).collect();
    if p.is_empty() { "Candidature".into() } else { format!("Candidature au poste de {p}") }
}

/// Peut-on envoyer maintenant ? `dernier` : le moment du dernier envoi tenté (en secondes), `du_jour` : les envois
/// du jour. Rien : oui ; sinon la phrase pour Jordan et les secondes à attendre (0 : plus aujourd'hui).
pub fn rythme(dernier: Option<i64>, maintenant: i64, du_jour: u32) -> Option<(String, i64)> {
    if du_jour >= PLAFOND_JOUR {
        return Some((
            format!(
                "Tu as déjà envoyé {PLAFOND_JOUR} candidatures par mail aujourd'hui. Jobmind en reste là pour \
                 aujourd'hui : reprends demain."
            ),
            0,
        ));
    }
    // (une horloge qui recule ne fait jamais attendre plus que l'écart)
    let reste = dernier.map_or(0, |d| (ECART_S - (maintenant - d)).min(ECART_S));
    if reste > 0 {
        let t = if reste > 1 { format!("{reste} secondes") } else { "une seconde".into() };
        return Some((format!("Une candidature vient de partir. Attends {t} avant d'envoyer la suivante."), reste));
    }
    None
}

// ---------------------------------------------------------------------------------------------------------------
// L'adresse lue dans une offre
// ---------------------------------------------------------------------------------------------------------------

/// Les débuts d'adresse qui ne reçoivent pas de réponse.
const MUETTES: [&str; 12] = [
    "noreply",
    "no-reply",
    "no.reply",
    "no_reply",
    "donotreply",
    "do-not-reply",
    "do.not.reply",
    "nepasrepondre",
    "ne-pas-repondre",
    "ne.pas.repondre",
    "ne_pas_repondre",
    "mailer-daemon",
];

/// Les domaines des sites d'emploi et des boîtes de messagerie des sites (jamais une adresse pour postuler).
const SITES: [&str; 14] = [
    "linkedin.com",
    "indeed.com",
    "indeed.fr",
    "francetravail.fr",
    "pole-emploi.fr",
    "welcometothejungle.com",
    "wttj.co",
    "adzuna.fr",
    "adzuna.com",
    "jooble.org",
    "hellowork.com",
    "apec.fr",
    "monster.fr",
    "example.com",
];

/// Une adresse qui ne reçoit pas de candidature.
fn exclue(a: &str) -> bool {
    let (l, d) = a.split_once('@').unwrap_or((a, ""));
    MUETTES.iter().any(|m| l.contains(m))
        || SITES.iter().any(|s| d == *s || d.ends_with(&format!(".{s}")))
        || l.starts_with("bounce")
}

/// Le texte en minuscules, avec les adresses masquées remises au propre : « [at] », « (at) », « {at} », « [@] »,
/// « (arobase) », « arobase » entre deux mots, et une @ entourée d'espaces deviennent « @ » ; « [dot] », « (point) »
/// deviennent « . ». Chaque caractère gardé note sa place dans le texte de départ (pour l'extrait).
fn remis_au_propre(texte: &str) -> (Vec<char>, Vec<usize>) {
    let src: Vec<char> = texte.chars().flat_map(char::to_lowercase).collect();
    let mut out: Vec<char> = Vec::with_capacity(src.len());
    let mut pos: Vec<usize> = Vec::with_capacity(src.len());
    let mot_a = |i: usize| -> Option<(char, usize)> {
        // « [ at ] », « (arobase) », « {dot} », « [@] » : le signe et la longueur
        let ouvre = src[i];
        let ferme = match ouvre {
            '[' => ']',
            '(' => ')',
            '{' => '}',
            '<' => '>',
            _ => return None,
        };
        let mut j = i + 1;
        while j < src.len() && src[j] == ' ' {
            j += 1;
        }
        let debut = j;
        while j < src.len() && (src[j].is_alphabetic() || src[j] == '@') {
            j += 1;
        }
        let mot: String = src[debut..j].iter().collect();
        while j < src.len() && src[j] == ' ' {
            j += 1;
        }
        if j >= src.len() || src[j] != ferme {
            return None;
        }
        let signe = match mot.as_str() {
            "at" | "arobase" | "arrobase" | "@" => '@',
            // (« prenom.nom (a) exemple.fr » : seulement entre deux espaces, jamais « salarié(a) »)
            "a" if ouvre == '('
                && i > 0
                && src[i - 1] == ' '
                && src.get(j + 1) == Some(&' ')
                && src.get(j + 2).is_some_and(|c| c.is_ascii_alphanumeric()) =>
            {
                '@'
            }
            "dot" | "point" => '.',
            _ => return None,
        };
        Some((signe, j + 1 - i))
    };
    let mut i = 0;
    while i < src.len() {
        if let Some((signe, n)) = mot_a(i) {
            while out.last() == Some(&' ') {
                out.pop();
                pos.pop();
            }
            out.push(signe);
            pos.push(i);
            i += n;
            while i < src.len() && src[i] == ' ' {
                i += 1;
            }
            continue;
        }
        // « arobase » entre deux mots
        if src[i] == ' ' {
            for (mot, signe) in [(" arobase ", '@'), (" arrobase ", '@')] {
                let m: Vec<char> = mot.chars().collect();
                if src.len() >= i + m.len() && src[i..i + m.len()] == m[..] {
                    out.push(signe);
                    pos.push(i);
                    i += m.len();
                    break;
                }
            }
            if i < src.len() && src[i] == ' ' {
                // une @ entourée d'espaces : « emploi @ exemple.fr »
                let mut j = i;
                while j < src.len() && src[j] == ' ' {
                    j += 1;
                }
                if j < src.len() && src[j] == '@' && out.last().is_some_and(|c| c.is_alphanumeric()) {
                    let mut k = j + 1;
                    while k < src.len() && src[k] == ' ' {
                        k += 1;
                    }
                    if k < src.len() && src[k].is_alphanumeric() && k - j <= 3 && j - i <= 2 {
                        out.push('@');
                        pos.push(j);
                        i = k;
                        continue;
                    }
                }
                out.push(' ');
                pos.push(i);
                i += 1;
            }
            continue;
        }
        if src[i] == '@' {
            // « emploi@ exemple.fr »
            out.push('@');
            pos.push(i);
            i += 1;
            let mut k = i;
            while k < src.len() && src[k] == ' ' {
                k += 1;
            }
            if k > i && k - i <= 2 && k < src.len() && src[k].is_alphanumeric() {
                i = k;
            }
            continue;
        }
        out.push(src[i]);
        pos.push(i);
        i += 1;
    }
    (out, pos)
}

fn car_local(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._%+-'".contains(c)
}

fn car_domaine(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '.' || c == '-'
}

/// Les mots qui annoncent l'adresse pour postuler, et ceux d'une adresse pour autre chose.
const POUR_POSTULER: [&str; 22] = [
    "candidature",
    "postuler",
    "postulez",
    "cv",
    "envoyez",
    "envoyer",
    "adressez",
    "adresser",
    "transmettez",
    "transmettre",
    "faites parvenir",
    "par mail",
    "par e-mail",
    "par email",
    "par courriel",
    "par mel",
    "recrutement",
    "lettre de motivation",
    "intéressé",
    "interessé",
    "a l'adresse",
    "à l'adresse",
];
const POUR_AUTRE: [&str; 9] = [
    "renseignement",
    "information",
    "question",
    "infos ",
    "réclamation",
    "service client",
    "clients",
    "commande",
    "devis",
];
/// Dites juste avant l'adresse : elle n'est pas pour postuler.
const PAS_PAR_MAIL: [&str; 4] = ["ne pas envoyer", "n'envoyez pas", "ne seront pas traitées", "ne sera pas traitée"];
/// Dites n'importe où dans l'offre : on ne postule pas par mail, aucune adresse n'est proposée.
const JAMAIS_PAR_MAIL: [&str; 11] = [
    "uniquement en ligne",
    "seulement en ligne",
    "exclusivement en ligne",
    "aucune candidature par mail",
    "pas de candidature par mail",
    "aucun cv par mail",
    "pas de cv par mail",
    "envoyé par mail ne sera",
    "envoyés par mail ne seront",
    "reçu par mail ne sera",
    "reçus par mail ne seront",
];

/// Les mots entiers de `t` (lettres, chiffres, apostrophes).
fn contient_mot(t: &str, mot: &str) -> bool {
    let mut depart = 0;
    while let Some(p) = t[depart..].find(mot) {
        let i = depart + p;
        let avant = t[..i].chars().next_back();
        let apres = t[i + mot.len()..].chars().next();
        if !avant.is_some_and(char::is_alphanumeric) && !apres.is_some_and(char::is_alphanumeric) {
            return true;
        }
        depart = i + mot.len();
    }
    false
}

/// L'adresse pour postuler lue dans le texte d'une offre (rien : aucune, ou aucune sûre).
pub fn trouver_adresse(texte: &str) -> Option<Trouvee> {
    let originaux: Vec<char> = texte.chars().flat_map(char::to_lowercase).collect();
    let vrais: Vec<char> = texte.chars().collect();
    let (t, pos) = remis_au_propre(texte);
    let mut candidates: Vec<(i32, usize, String, usize, usize)> = Vec::new();
    for (i, &c) in t.iter().enumerate() {
        if c != '@' {
            continue;
        }
        // La partie avant @
        let mut d = i;
        while d > 0 && car_local(t[d - 1]) {
            d -= 1;
        }
        // La partie après @ (« exemple point fr » accepté)
        let mut f = i + 1;
        let mut dom = String::new();
        loop {
            while f < t.len() && car_domaine(t[f]) {
                dom.push(t[f]);
                f += 1;
            }
            let suite: String = t[f..t.len().min(f + 8)].iter().collect();
            let sep = [" point ", " dot "].into_iter().find(|s| suite.starts_with(s));
            match sep {
                Some(s) if t.get(f + s.len()).is_some_and(|c| c.is_ascii_alphabetic()) && !dom.is_empty() => {
                    dom.push('.');
                    f += s.chars().count();
                }
                _ => break,
            }
        }
        let local: String = t[d..i].iter().collect::<String>().trim_start_matches(['.', '-', '\'']).to_string();
        let dom = dom.trim_end_matches(['.', '-']).to_string();
        let adresse = format!("{local}@{dom}");
        if !adresse_valide(&adresse) || exclue(&adresse) {
            continue;
        }
        // Dans un lien (« https://...@... », « .../@jobs ») : pas une adresse
        let avant: String = t[d.saturating_sub(12)..d].iter().collect();
        let avant_c = d.checked_sub(1).map(|k| t[k]);
        if avant_c == Some('/') || (avant_c == Some(':') && !avant.ends_with("mailto:")) {
            continue;
        }
        let mot_avant: String = t[..d].iter().rev().take_while(|c| !c.is_whitespace()).collect();
        if mot_avant.contains("//") || mot_avant.contains("www.") {
            continue;
        }
        if t.get(f).is_some_and(|c| *c == '/') {
            continue;
        }
        // Le contexte : la phrase d'avant et celle de l'adresse
        let ctx_debut = d.saturating_sub(160);
        // (les adresses du contexte ne comptent pas : « recrutement.metz@... » n'annonce pas la suivante)
        let contexte: String = t[ctx_debut..d]
            .iter()
            .collect::<String>()
            .split(' ')
            .map(|m| if m.contains('@') { "" } else { m })
            .collect::<Vec<_>>()
            .join(" ");
        // (seulement depuis le début de la phrase précédente, ou la ligne)
        let proche: String = {
            let c: Vec<char> = contexte.chars().collect();
            let mut coupe = 0;
            let mut points = 0;
            for k in (0..c.len()).rev() {
                if c[k] == '.' && c.get(k + 1).is_some_and(|x| x.is_whitespace()) || c[k] == '\n' {
                    points += 1;
                    if points == 2 {
                        coupe = k + 1;
                        break;
                    }
                }
            }
            c[coupe..].iter().collect()
        };
        // (la phrase de l'adresse seulement, depuis le dernier point ou la dernière ligne)
        let phrase: String = {
            let c: Vec<char> = proche.chars().collect();
            let k = (0..c.len())
                .rev()
                .find(|&k| c[k] == '\n' || c[k] == '.' && c.get(k + 1).is_some_and(|x| x.is_whitespace()))
                .map_or(0, |k| k + 1);
            c[k..].iter().collect()
        };
        let proche_court: String = phrase.chars().rev().take(90).collect::<Vec<_>>().into_iter().rev().collect();
        let mut note = 0i32;
        if POUR_POSTULER.iter().any(|m| contient_mot(&proche, m)) {
            note += 3;
        }
        if POUR_POSTULER.iter().any(|m| contient_mot(&proche_court, m)) {
            note += 1;
        }
        if POUR_AUTRE.iter().any(|m| proche_court.contains(m)) {
            note -= 4;
        }
        if PAS_PAR_MAIL.iter().any(|m| proche.contains(m)) {
            note -= 10;
        }
        let l = local.as_str();
        if ["rh", "drh", "recrutement", "recrute", "jobs", "job", "emploi", "emplois", "candidature", "candidatures"]
            .iter()
            .any(|m| {
                l.split(['.', '-', '_', '+']).any(|x| x == *m || x.starts_with("recrut") && m.starts_with("recrut"))
            })
        {
            note += 2;
        }
        if ["info", "infos", "contact", "accueil", "commercial", "commandes", "sav"].contains(&l) {
            note -= 1;
        }
        let debut_src = pos.get(d).copied().unwrap_or(0);
        let fin_src = pos.get(f.saturating_sub(1)).copied().map_or(originaux.len(), |p| p + 1);
        candidates.push((note, i, adresse, debut_src, fin_src));
    }
    // Toute l'offre dit de ne pas postuler par mail : rien
    let tout: String = t.iter().collect();
    if JAMAIS_PAR_MAIL.iter().any(|m| tout.contains(m))
        || candidates.iter().all(|c| c.0 < 0) && PAS_PAR_MAIL.iter().any(|m| tout.contains(m))
    {
        return None;
    }
    let meilleure = candidates.iter().filter(|c| c.0 > -4).max_by_key(|c| (c.0, std::cmp::Reverse(c.1)))?;
    let b = meilleure.4.min(vrais.len());
    let a = meilleure.3.min(b);
    let debut = (0..a).rev().find(|&k| matches!(vrais[k], '.' | '\n' | '!' | '?')).map_or(0, |k| k + 1);
    let fin = (b..vrais.len()).find(|&k| vrais[k] == '\n').unwrap_or(vrais.len());
    let mut extrait: String = vrais[debut..fin].iter().collect::<String>().trim().to_string();
    if extrait.chars().count() > 200 {
        extrait = vrais[a.saturating_sub(80)..(b + 40).min(vrais.len())].iter().collect::<String>().trim().to_string();
    }
    Some(Trouvee { adresse: meilleure.2.clone(), extrait })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(t: &str) -> Option<String> {
        trouver_adresse(t).map(|x| x.adresse)
    }

    #[test]
    fn adresses_simples() {
        assert_eq!(
            lu("Envoyez CV et lettre à recrutement@logipole-exemple.fr."),
            Some("recrutement@logipole-exemple.fr".into())
        );
        assert_eq!(lu("Candidature : RH@Exemple-Transport.FR"), Some("rh@exemple-transport.fr".into()));
        assert_eq!(lu("Postulez (jobs@exemple.fr) avant vendredi"), Some("jobs@exemple.fr".into()));
        assert_eq!(lu("Contact : <emploi@exemple.org>"), Some("emploi@exemple.org".into()));
        assert_eq!(lu("mailto:rh.metz2@groupe-exemple.fr"), Some("rh.metz2@groupe-exemple.fr".into()));
        assert_eq!(lu("Aucune adresse ici, postulez en ligne."), None);
    }

    #[test]
    fn adresses_masquees() {
        assert_eq!(
            lu("Écrivez à contact [at] exemple-transport.fr pour postuler"),
            Some("contact@exemple-transport.fr".into())
        );
        assert_eq!(lu("CV à rh(at)batiment-exemple.fr"), Some("rh@batiment-exemple.fr".into()));
        assert_eq!(lu("Candidatures : jobs arobase resto-exemple point fr"), Some("jobs@resto-exemple.fr".into()));
        assert_eq!(lu("Envoyez votre CV : emploi @ exemple.fr"), Some("emploi@exemple.fr".into()));
        assert_eq!(lu("CV : rh [at] exemple [dot] fr"), Some("rh@exemple.fr".into()));
    }

    #[test]
    fn prudence() {
        assert_eq!(lu("Ne répondez pas : noreply@exemple.fr"), None);
        assert_eq!(lu("Postulez sur https://www.exemple.fr/@carrieres/postuler"), None);
        assert_eq!(lu("Profil : https://jordan:secret@exemple.fr/offre"), None);
        assert_eq!(lu("Postulez sur LinkedIn ou écrivez à alertes@linkedin.com"), None);
        assert_eq!(
            lu("Merci de ne pas envoyer de CV par mail : postulez uniquement en ligne. Infos : contact@exemple.fr"),
            None
        );
        // Deux adresses : celle pour postuler
        assert_eq!(
            lu(
                "Pour tout renseignement : info@cabinet-exemple.fr. Candidatures (CV et lettre) à rh@cabinet-exemple.fr."
            ),
            Some("rh@cabinet-exemple.fr".into())
        );
        assert_eq!(
            lu("Candidatures à rh@cabinet-exemple.fr. Pour tout renseignement : info@cabinet-exemple.fr."),
            Some("rh@cabinet-exemple.fr".into())
        );
    }

    #[test]
    fn extrait_tel_quel() {
        let t = trouver_adresse("Poste en CDI.\nEnvoyez CV et lettre à Recrutement@Exemple.fr avant le 30.\nMerci.")
            .unwrap();
        assert_eq!(t.extrait, "Envoyez CV et lettre à Recrutement@Exemple.fr avant le 30.");
    }

    #[test]
    fn destinataire_tape() {
        assert_eq!(verifier_destinataire(" <RH@Exemple.fr> "), Ok("rh@exemple.fr".into()));
        assert_eq!(verifier_destinataire("mailto:rh@exemple.fr?subject=x"), Ok("rh@exemple.fr".into()));
        for mauvais in [
            "",
            "rh",
            "rh@",
            "rh@exemple",
            "rh@@exemple.fr",
            "rh@exemple.f",
            "jérôme@exemple.fr",
            "a b@exemple.fr",
            "rh@exemple.fr\r\nBcc: x@y.fr",
        ] {
            assert!(verifier_destinataire(mauvais).is_err(), "{mauvais}");
        }
        assert!(verifier_destinataire("a@exemple.fr, b@exemple.fr").unwrap_err().contains("Une seule adresse"));
        assert!(verifier_destinataire("noreply@exemple.fr").unwrap_err().contains("ne reçoit pas"));
    }

    #[test]
    fn objet_verifie() {
        assert_eq!(verifier_objet("  Candidature \u{2014} magasinier\n "), Ok("Candidature - magasinier".into()));
        assert!(verifier_objet("   ").is_err());
        assert!(verifier_objet(&"x".repeat(201)).is_err());
        assert_eq!(objet_par_defaut("Agent  de quai"), "Candidature au poste de Agent de quai");
        assert_eq!(objet_par_defaut(""), "Candidature");
    }

    #[test]
    fn le_rythme() {
        assert_eq!(rythme(None, 1000, 0), None);
        assert_eq!(rythme(Some(1000 - 60), 1000, 3), None);
        let (t, s) = rythme(Some(1000 - 15), 1000, 3).unwrap();
        assert_eq!(s, 45);
        assert!(t.contains("45 secondes"), "{t}");
        let (t, s) = rythme(None, 1000, PLAFOND_JOUR).unwrap();
        assert_eq!(s, 0);
        assert!(t.contains("demain"));
        // (une horloge qui recule ne bloque pas plus d'une minute)
        assert_eq!(rythme(Some(5000), 1000, 0).map(|x| x.1), Some(ECART_S));
    }

    #[test]
    fn aucun_tiret_long() {
        for t in [
            verifier_destinataire("").unwrap_err(),
            verifier_objet("").unwrap_err(),
            rythme(Some(1000), 1001, 0).unwrap().0,
            rythme(None, 1, PLAFOND_JOUR).unwrap().0,
        ] {
            assert!(!t.contains('\u{2014}') && !t.contains('\u{2013}'), "{t}");
        }
    }
}
