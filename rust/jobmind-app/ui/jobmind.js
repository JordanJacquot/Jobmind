// L'accueil de Jobmind : la maquette C validée par Jordan (maquettes\proposition-c.html, « les dossiers ») : trois
// onglets, Candidatures, Offres et Mon profil, chacun avec une liste fine à gauche et le dossier ouvert à droite. Le
// kit (kit.js) l'appelle à la place de ses tuiles (bloc accueil.perso de l'état, voir « perso » en tête de kit.js) ;
// il garde l'en-tête, le bas, les flèches, A et B (Entrée et Échap) et les sons.
//
// Rien n'est calculé ici : Rust (src\ecran.rs) envoie les onglets, leurs listes et leurs dossiers, textes déjà écrits
// en français. Tout texte reçu est échappé (esc du kit), chaque id et chaque icône vérifiés. Ce fichier ne parle
// jamais à Rust lui-même : chaque geste passe par le kit (action), et Rust vérifie tout de nouveau (src\suivi.rs).
//
// - un onglet (clic, A ou Entrée) le montre ; sa liste à gauche, son dossier à droite ;
// - Candidatures (étape 2) : la liste groupée (À faire maintenant, En attente, Terminées), chaque ligne avec sa
//   pastille de statut et sa ligne d'action, le petit bilan du mois en bas ; une ligne ouvre son dossier (action
//   'c-ouvrir') : le statut à changer par un menu (Entretien : sa date, son heure, son lieu), « où on en est », la
//   prochaine action et ses boutons (« Copier un message », « C'est relancé », « Classer en Sans réponse »), la frise,
//   les pilules « Ce que j'ai envoyé », « L'offre », « Notes » (gardées en quittant le champ), « Modifier » et
//   « Supprimer » (jamais au premier clic) ;
// - « Ajouter une candidature » ouvre le formulaire à droite ; « Enregistrer » envoie tout à Rust ('c-enregistrer'),
//   qui répond par l'ordre { kit: 'accueil', enregistre: id } (le formulaire se ferme sur le dossier) ou par une
//   bulle qui dit ce qui ne va pas (le formulaire reste ouvert). Un brouillon fermé par Retour est gardé ;
// - les dates (la date d'envoi, celle de l'entretien) : le champ JJ/MM/AAAA, et un petit calendrier d'un mois (repris
//   de Paymind) ouvert par l'agenda du champ ou un clic dedans ; un clic sur un jour remplit le champ. Rust vérifie ;
// - Mon profil (étape 3) : Mon CV (importer un fichier : 'cv-importer', Rust ouvre la boîte « Ouvrir » de Windows ;
//   coller ; corriger le texte lu ; le CV principal ; supprimer), Ce que je cherche (postes, lieu, contrats, télétravail,
//   diplôme, débutant, salaire : 'recherche-enregistrer'), Mon style (ajouter, lire, supprimer un texte), Mes clés
//   (donner, remplacer, effacer : une clé tapée n'est gardée ici que le temps de l'envoyer) ; Réglages montre le délai
//   avant relance ('reglage-relance') ;
// - Offres (étape 4) : la barre des filtres ('o-filtres'), la recherche choisie et « Chercher maintenant »
//   ('o-chercher' : Rust interroge Adzuna sur un fil à part), la liste ('o-ouvrir'), l'offre ouverte
//   (favori, plus tard, « Pas pour moi », « Voir sur le site », la mention « Adzuna », « J'ai postulé »), Mes recherches
//   ('o-recherche', 'o-recherche-supprimer', 'o-depuis-profil') ; Mes clés : « Essayer la clé » ('cle-essayer') ;
// - les sites en coulisse (étape 5 bis) : en haut de la liste des offres, la liste déroulante « Où chercher »
//   (LinkedIn, France Travail, Indeed, Welcome to the Jungle, ou Adzuna avec mes offres gardées :
//   'cs-source') ; un site choisi, « Chercher sur LinkedIn » ('cs-chercher'),
//   ses résultats lus par Rust en coulisse ('cs-ouvrir', « Page suivante » : 'cs-suivante'), l'offre ouverte
//   (« Ajouter à Jobmind » : 'cs-ajouter', « Voir la page du site » : 'cs-voir') ; la carte de connexion de Jobmind
//   ('cs-connecter') ; la petite fenêtre du site, où Rust pose la vraie page (sa place : 'cs-zone') ; Réglages :
//   « Me déconnecter de tous les sites » ('cs-deconnecter') ;
// - Retour (Échap, B) ferme d'abord ce qui est ouvert (menu, question, formulaire), revient ensuite au premier
//   onglet, puis à la console de Mindset ;
// - les ordres de l'appli : { kit: 'accueil', onglet: id }, { rubrique: id }, { candidature: id } (le dossier
//   annoncé par une bulle), { ajouter: true }, { modifier: true }, { enregistre: id } (le formulaire enregistré),
//   { fait: id } (un statut changé : le menu ou le petit formulaire de l'entretien se referment), { statut: 'menu' }
//   et { statut: 'entretien' } (le menu des statuts, le petit formulaire de l'entretien du dossier ouvert) ; pour les
//   captures, { calendrier: true } (le calendrier de la date ouverte) et { cliquer: data-k } (un clic sur cet élément).
(() => {
  'use strict';
  const reduit = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const idSur = (t) => typeof t === 'string' && /^[a-z]{1,30}$/.test(t);
  const codeSur = (t) => typeof t === 'string' && /^[a-z_]{1,30}$/.test(t);
  const numSur = (n) => Number.isInteger(n) && n > 0 && n < 1e12;
  const texteSur = (t) => (typeof t === 'string' ? t : '');
  const svg = (corps, w = 1.9) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${corps}</svg>`;
  // Les icônes que Rust peut demander (une autre n'est pas dessinée)
  const ICONES = {
    dossier: svg('<path d="M3.5 7.5a2 2 0 0 1 2-2h4l2 2.2h7a2 2 0 0 1 2 2V18a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/><path d="M9 14l2.2 2.2L15.5 12"/>'),
    loupe: svg('<circle cx="11" cy="11" r="6.5"/><path d="M20 20l-4.2-4.2"/>'),
    profil: svg('<circle cx="12" cy="8.5" r="3.8"/><path d="M4.5 20c1.2-3.8 4-5.6 7.5-5.6s6.3 1.8 7.5 5.6"/>'),
    cv: svg('<path d="M6.5 3.5h7.5l4.5 4.5v12a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1v-15.5a1 1 0 0 1 1-1z"/><path d="M14 3.5V8h4.5M9 12.5h6M9 16h6"/>'),
    plume: svg('<path d="M19.5 4.5c-6 .5-10.5 4.5-12 11.5l-1.5 4"/><path d="M7.5 16c3 0 6.5-1.5 8.5-5M12 8.5c2.5-.2 4.5-.8 7.5-4"/>'),
    cible: svg('<circle cx="12" cy="12" r="8"/><circle cx="12" cy="12" r="4"/><circle cx="12" cy="12" r=".6" fill="currentColor"/>'),
    reglages: svg('<circle cx="12" cy="12" r="3"/><path d="M12 3.5v2.2M12 18.3v2.2M3.5 12h2.2M18.3 12h2.2M6 6l1.6 1.6M16.4 16.4L18 18M6 18l1.6-1.6M16.4 7.6L18 6"/>'),
    relance: svg('<path d="M20 11a8 8 0 0 0-14.3-4.9L4 8"/><path d="M4 3.5V8h4.5"/><path d="M4 13a8 8 0 0 0 14.3 4.9L20 16"/><path d="M20 20.5V16h-4.5"/>'),
    horloge: svg('<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>'),
    agenda: svg('<rect x="3.5" y="5" width="17" height="15" rx="2.5"/><path d="M3.5 10h17M8 3v4M16 3v4M8.5 14h2M13.5 14h2"/>'),
    coche: svg('<path d="M5 12.5l4.5 4.5L19 7.5"/>', 2.3),
    copie: svg('<rect x="8.5" y="8.5" width="11" height="11" rx="2.2"/><path d="M15.5 8.5V6.2a1.7 1.7 0 0 0-1.7-1.7H6.2a1.7 1.7 0 0 0-1.7 1.7v7.6a1.7 1.7 0 0 0 1.7 1.7h2.3"/>'),
    plus: svg('<path d="M12 5v14M5 12h14"/>', 2.2),
    crayon: svg('<path d="M4 20h4L19 9a2.8 2.8 0 0 0-4-4L4 16z"/><path d="M13.5 6.5l4 4"/>'),
    poubelle: svg('<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>'),
    lien: svg('<path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1"/><path d="M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"/>'),
    bas: svg('<path d="M6 9l6 6 6-6"/>', 2.2),
    gauche: svg('<path d="M15 5l-7 7 7 7"/>', 2.4),
    droite: svg('<path d="M9 5l7 7-7 7"/>', 2.4),
    cle: svg('<circle cx="8" cy="15" r="4.2"/><path d="M11 12l8.5-8.5M16 7l2.5 2.5M14 9l2 2"/>'),
    importer: svg('<path d="M12 15V4M7.5 8.5L12 4l4.5 4.5"/><path d="M4.5 14v4a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-4"/>'),
    coller: svg('<rect x="5.5" y="4.5" width="13" height="16" rx="2.2"/><path d="M9 4.5V3.8a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v.7M9 10h6M9 13.5h6M9 17h3.5"/>'),
    etoile: svg('<path d="M12 3.8l2.5 5.1 5.6.8-4 3.9 1 5.6L12 16.6l-5 2.6 1-5.6-4-3.9 5.6-.8z"/>'),
    attention: svg('<path d="M12 4l9 16H3z"/><path d="M12 10v4.5M12 17.2v.1"/>'),
    cadenas: svg('<rect x="5" y="10.5" width="14" height="10" rx="2.2"/><path d="M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5"/>'),
    oeil: svg('<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/>'),
    ext: svg('<path d="M13.5 4.5h6v6M19.5 4.5l-8 8"/><path d="M17.5 13.5v4.5a1.5 1.5 0 0 1-1.5 1.5H6a1.5 1.5 0 0 1-1.5-1.5V8A1.5 1.5 0 0 1 6 6.5h4.5"/>'),
    cacher: svg('<path d="M3.5 3.5l17 17"/><path d="M10 6a9.6 9.6 0 0 1 2-.2c6 0 9.5 6.2 9.5 6.2a16 16 0 0 1-2.6 3.3M6.6 6.9C4 8.6 2.5 12 2.5 12S6 18.5 12 18.5c1.6 0 3-.4 4.3-1.1"/><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2"/>'),
    lieu: svg('<path d="M12 21s-7-6.2-7-11.5a7 7 0 0 1 14 0C19 14.8 12 21 12 21z"/><circle cx="12" cy="9.5" r="2.5"/>'),
    envoi: svg('<path d="M4 12l16-7.5-6 16-2.8-6.7z"/><path d="M11.2 13.8L20 4.5"/>'),
    croix: svg('<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>', 2.2),
    globe: svg('<circle cx="12" cy="12" r="8.5"/><path d="M3.5 12h17M12 3.5c2.4 2.5 3.5 5.3 3.5 8.5s-1.1 6-3.5 8.5c-2.4-2.5-3.5-5.3-3.5-8.5s1.1-6 3.5-8.5z"/>'),
    recharger: svg('<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>'),
    mail: svg('<rect x="3.5" y="5.5" width="17" height="13" rx="2.5"/><path d="M4.5 7.5l7.5 5.5 7.5-5.5"/>'),
  };
  const icone = (n) => (typeof n === 'string' && Object.hasOwn(ICONES, n) ? ICONES[n] : '');
  // La couleur de chaque statut, par son rôle dans le thème (jamais en dur : elle suit le thème de la suite)
  const COULEURS = { envoyee: 'var(--P)', relancee: 'var(--T)', entretien: 'var(--S)', positive: 'var(--vert)', negative: 'var(--rouge)', sans_reponse: 'var(--pale)', abandonnee: 'var(--pale)' };
  const couleurStatut = (c) => (Object.hasOwn(COULEURS, c) ? COULEURS[c] : 'var(--pale)');
  // Les champs du formulaire, dans l'ordre
  const CHAMPS = ['entreprise', 'poste', 'ville', 'date', 'source', 'cv', 'lien', 'envoye', 'notes'];
  const LONGUEURS = { entreprise: 120, poste: 120, ville: 80, date: 12, source: 60, cv: 120, lien: 500, envoye: 8000, notes: 4000 };

  let P = null;           // le dernier accueil reçu
  let O = null;           // les outils du kit (esc, son, action...)
  let hote = null;        // #ecran
  let onglet = null;      // l'onglet montré
  const rubrique = {};    // la rubrique ouverte, par onglet
  // Le suivi : ce qui ne vit que dans la page (oublié en fermant)
  let vueC = 'dossier';   // 'dossier' ou 'formulaire' (à droite)
  let form = null;        // le formulaire ouvert : { id (rien : nouvelle), v: { champs } }
  let brouillon = null;   // une nouvelle candidature commencée puis fermée par Retour
  const modifs = {};      // les changements d'une candidature existante fermés par Retour, par candidature
  let ficheVue = null;    // le dossier dessiné en dernier (un autre : ses petites choses se referment)
  let pilule = 'envoye';  // la pilule montrée dans le dossier
  let menu = false;       // le menu des statuts ouvert
  let rdv = null;         // le petit formulaire de l'entretien : { date, heure, lieu }
  let cal = null;         // le petit calendrier ouvert : { ou: 'f' (la date d'envoi) ou 'e' (l'entretien), a, m (0 à 11) }
  let confirme = false;   // « Supprimer ? » demandé
  let choisie = null;     // la ligne choisie à l'instant (avant que Rust ne renvoie le dossier)
  const notes = {};       // les notes tapées, par candidature (gardées si un état arrive pendant qu'on tape)
  let envoiJusqua = 0;    // (un double clic n'envoie pas deux fois)

  const onglets = () => (Array.isArray(P?.onglets) ? P.onglets.filter((o) => idSur(o?.id)) : []);
  const ongletDe = (id) => onglets().find((o) => o.id === id) || onglets()[0] || null;
  const rubriquesDe = (o) => (Array.isArray(o?.rubriques) ? o.rubriques.filter((r) => idSur(r?.id)) : []);
  const suiviDe = () => { const o = onglets().find((x) => x.id === 'candidatures'); return o?.suivi && typeof o.suivi === 'object' ? o.suivi : null; };
  const textesC = () => { const t = suiviDe()?.textes; return t && typeof t === 'object' ? t : {}; };
  const T = (k) => texteSur(textesC()[k]);
  const fiche = () => { const f = suiviDe()?.fiche; return f && typeof f === 'object' && numSur(f.id) ? f : null; };
  // Étape 7 : les mails à ranger (et celui ouvert à droite), le choix de la candidature fait pour chacun
  const mailsDe = () => { const m = suiviDe()?.mails; return m && typeof m === 'object' && !Array.isArray(m) ? m : {}; };
  let mailVu = null;      // le mail à ranger ouvert à droite (vueC === 'mail')
  const mrChoix = {};     // la candidature choisie pour chaque mail à ranger
  // La rubrique ouverte d'un onglet : celle choisie, sinon la première (Mon profil s'ouvre sur Mon CV)
  function rubriqueDe(o) {
    const l = rubriquesDe(o);
    return l.find((r) => r.id === rubrique[o.id]) || l[0] || null;
  }

  // ---------------------------------------------------------------- dessiner
  function ongletsHTML(actif) {
    const { esc } = O;
    return onglets().map((o) => {
      const n = Number.isInteger(o.nombre) && o.nombre > 0 ? `<span class="nb">${o.nombre}</span>` : '';
      const oui = o.id === actif.id;
      return `<button type="button" class="jm-onglet f${oui ? ' actif' : ''}" role="tab" aria-selected="${oui}" data-perso="onglet" data-id="${o.id}" data-k="o:${o.id}">${esc(texteSur(o.nom))}${n}</button>`;
    }).join('');
  }

  function listeHTML(o, choisieR) {
    const { esc } = O;
    const l = rubriquesDe(o);
    const lignes = l.map((r) => `<button type="button" class="jm-ligne f${r === choisieR ? ' sel' : ''}" data-perso="rubrique" data-id="${r.id}" data-k="r:${r.id}" aria-current="${r === choisieR}">
        <span class="jm-l-ic">${icone(r.icone)}</span><span class="jm-l-txt"><b>${esc(texteSur(r.nom))}</b>${r.sous ? `<span>${esc(texteSur(r.sous))}</span>` : ''}</span></button>`).join('');
    const vide = !l.length && o.vide ? `<p class="jm-vide-l">${esc(texteSur(o.vide))}</p>` : '';
    return `<div class="jm-liste-t">${esc(texteSur(o.liste))}</div>${lignes}${vide}`;
  }

  function pastille(code, nom, gros) {
    return `<span class="jm-st${gros ? ' gros' : ''}" style="--c:${couleurStatut(code)}">${O.esc(texteSur(nom))}</span>`;
  }

  // La liste des candidatures : le bouton d'ajout, les groupes, le bilan
  function listeCandidaturesHTML(o, s) {
    const { esc } = O;
    const groupes = Array.isArray(s.groupes) ? s.groupes : [];
    const ouverte = vueC === 'dossier' ? (choisie ?? s.ouverte) : null;
    let h = `<div class="jm-liste-tete"><span class="jm-liste-t">${esc(texteSur(o.liste))}</span>
      <span class="jm-liste-bts">${bilanDe().titre ? `<button type="button" class="jm-bilan-bt f${vueC === 'bilan' ? ' on' : ''}" data-perso="bilan" data-k="c:bilanh" title="${esc(texteSur(bilanDe().titre))}" aria-label="${esc(texteSur(bilanDe().titre))}">${icone('cible')}<span>Bilan</span></button>` : ''}<button type="button" class="jm-ajout f" data-perso="ajouter" data-k="c:ajouter" title="${esc(T('ajouter'))}">${icone('plus')}<span>${esc(T('ajouter_court'))}</span></button></span></div>`;
    // (étape 7 : les mails que Jobmind n'a pas su ranger, en tête)
    const mx = mailsDe(), tm = tx(mx), ar = liste(mx.a_ranger).filter((x) => numSur(x?.id));
    if (ar.length) {
      h += `<div class="jm-groupe-t">${esc(tm('titre'))} <span>${ar.length}</span></div>`;
      for (const x of ar) {
        const sel = vueC === 'mail' && x.id === mailVu;
        h += `<button type="button" class="jm-cand jm-mail-l f${sel ? ' sel' : ''}" data-perso="mail" data-id="${x.id}" data-k="ml:${x.id}" aria-current="${sel}">
          <span class="jm-c1"><b>${esc(texteSur(x.nom))}</b><span class="jm-st" style="--c:var(--T)">${icone('mail')}</span></span>
          <span class="jm-c2">${esc(texteSur(x.sous))}</span>${x.recu_le ? `<span class="jm-c3">${esc(tm('recu'))} le ${esc(texteSur(x.recu_le))}</span>` : ''}</button>`;
      }
    }
    for (const g of groupes) {
      const lignes = Array.isArray(g?.lignes) ? g.lignes.filter((x) => numSur(x?.id)) : [];
      if (!lignes.length) continue;
      h += `<div class="jm-groupe-t">${esc(texteSur(g.nom))} <span>${lignes.length}</span></div>`;
      for (const x of lignes) {
        const sel = x.id === ouverte;
        h += `<button type="button" class="jm-cand f${sel ? ' sel' : ''}${x.fini === true ? ' fini' : ''}" data-perso="cand" data-id="${x.id}" data-k="c:${x.id}" aria-current="${sel}">
          <span class="jm-c1"><b>${esc(texteSur(x.nom))}</b>${codeSur(x.statut) ? pastille(x.statut, x.statut_nom) : ''}</span>
          <span class="jm-c2">${esc(texteSur(x.sous))}</span>${x.action ? `<span class="jm-c3">${esc(texteSur(x.action))}</span>` : ''}${x.reponse === true ? `<span class="jm-c3 jm-c-rep">${icone('mail')}${esc(tm('reponse'))}</span>` : ''}</button>`;
      }
    }
    if (!groupes.length && !ar.length && o.vide) h += `<p class="jm-vide-l">${esc(texteSur(o.vide))}</p>`;
    const bilan = Array.isArray(s.bilan) ? s.bilan.filter((t) => typeof t === 'string') : [];
    // (étape 12 : le petit bilan ouvre « Mon bilan », le tableau de bord, à droite)
    if (bilan.length) {
      const sel = vueC === 'bilan';
      h += `<button type="button" class="jm-bilan f${sel ? ' sel' : ''}" data-perso="bilan" data-k="c:bilan" aria-current="${sel}">${bilan.map((t, i) => (i ? `<span>${esc(t)}</span>` : `<b>${esc(t)}</b>`)).join('')}<em>${icone('cible')}${esc(T('voir_bilan') || 'Voir mon bilan')}</em></button>`;
    }
    return h;
  }

  // ---------------------------------------------------------------- Mon bilan (étape 12)
  // Le tableau de bord, dessiné ici en HTML simple d'après les chiffres que Rust a calculés (src\bord.rs) : six
  // chiffres, les envois des huit dernières semaines en barres, la part de chaque statut, les offres nouvelles par
  // source, et l'export. Aucune bibliothèque de dessin.
  const bilanDe = () => objet(suiviDe()?.tableau);
  function bilanHTML() {
    const { esc } = O, b = bilanDe();
    const nb = (x) => (Number.isInteger(x) && x >= 0 ? x : 0);
    const part = (x, max) => Math.max(0, Math.min(max, nb(x)));
    const cartes = liste(b.cartes).map((c) => `<div class="jm-bc${['bien', 'attention'].includes(c?.ton) ? ` ${c.ton}` : ''}"><b>${esc(texteSur(c?.nombre))}</b><span>${esc(texteSur(c?.texte))}</span></div>`).join('');
    const barres = liste(b.barres).map((x) => `<div class="jm-bb" title="${esc(texteSur(x?.texte))} : ${nb(x?.nombre)}"><div class="jm-bb-p"><span class="jm-bb-n">${nb(x?.nombre) || ''}</span><i style="height:${Math.max(part(x?.part, 100) * 0.8, nb(x?.nombre) ? 4 : 0)}%"></i></div><span class="jm-bb-t">${esc(texteSur(x?.texte))}</span></div>`).join('');
    const st = liste(b.statuts).filter((x) => codeSur(x?.id));
    const bande = st.map((x) => `<i style="width:${part(x.part, 1000) / 10}%;--c:${couleurStatut(x.id)}"></i>`).join('');
    const legende = st.map((x) => `<span><i style="--c:${couleurStatut(x.id)}"></i>${esc(texteSur(x.nom))} <b>${nb(x.nombre)}</b></span>`).join('');
    const sources = liste(b.sources).map((x) => `<div class="jm-bs"><span>${esc(texteSur(x?.nom))}</span><i><u style="width:${part(x?.part, 100)}%"></u></i><b>${nb(x?.nombre)}</b></div>`).join('');
    return `<div class="jm-d jm-bilan-d${anime()}"><h1>${esc(texteSur(b.titre))}</h1><div class="jm-d-sous">${esc(texteSur(b.sous))}</div>
      <div class="jm-bcs">${cartes}</div>
      <div class="jm-h2">${esc(texteSur(b.barres_titre))}</div><div class="jm-bbs" role="img" aria-label="${esc(texteSur(b.barres_titre))}">${barres}</div>
      ${st.length ? `<div class="jm-h2">${esc(texteSur(b.statuts_titre))}</div><div class="jm-bande" role="img" aria-label="${esc(texteSur(b.statuts_titre))}">${bande}</div><div class="jm-legende">${legende}</div>` : ''}
      <div class="jm-h2">${esc(texteSur(b.offres_titre))}</div><p class="jm-petit jm-b-of">${esc(texteSur(b.offres_texte))}</p>${sources ? `<div class="jm-bss">${sources}</div>` : ''}
      <div class="jm-btns jm-b-bas">${bouton('bx-csv', 'bx:csv', texteSur(b.csv), 'importer', 'second')}${bouton('bx-pdf', 'bx:pdf', texteSur(b.pdf), 'cv', 'second')}${bouton('bilan-fermer', 'bx:fermer', texteSur(b.fermer), 'gauche', 'discret')}</div></div>`;
  }
  function ouvrirBilan() {
    if (vueC !== 'bilan') O.anime = !reduit();
    vueC = 'bilan'; menu = false; rdv = null; confirme = false; cal = null;
    dessinerTout('c:bilan');
  }

  function dossierVideHTML(d, avecAjout) {
    const { esc } = O;
    if (!d || typeof d !== 'object') return '';
    return `<div class="jm-vide${O.anime ? ' entree' : ''}">
        <div class="jm-v-ic plasma-bord">${icone(d.icone)}</div>
        <h1>${esc(texteSur(d.titre))}</h1>
        ${d.texte ? `<p>${esc(texteSur(d.texte))}</p>` : ''}
        ${avecAjout ? `<button type="button" class="jm-bt principal grand f" data-perso="ajouter" data-k="v:ajouter">${icone('plus')}${esc(T('ajouter'))}</button>` : ''}
        ${d.note ? `<p class="jm-note">${esc(texteSur(d.note))}</p>` : ''}
      </div>`;
  }

  // Un bouton du dossier
  const bouton = (perso, k, texte, ic, style, extra = '') => `<button type="button" class="jm-bt${style ? ` ${style}` : ''} f" data-perso="${perso}" data-k="${k}"${extra}>${icone(ic)}<span>${O.esc(texte)}</span></button>`;

  function menuStatutsHTML(f, s) {
    const { esc } = O;
    const statuts = Array.isArray(s.statuts) ? s.statuts.filter((x) => codeSur(x?.id)) : [];
    return `<div class="jm-menu" role="menu">${statuts.map((x) => `<button type="button" role="menuitemradio" aria-checked="${x.id === f.statut}" class="jm-menu-l f${x.id === f.statut ? ' ici' : ''}" data-perso="statut-choix" data-id="${x.id}" data-k="m:${x.id}" style="--c:${couleurStatut(x.id)}"><i></i>${esc(texteSur(x.nom))}</button>`).join('')}</div>`;
  }

  // ---------------------------------------------------------------- le petit calendrier des dates (repris de Paymind)
  // La date d'envoi du formulaire ('f') et celle de l'entretien ('e') : le champ JJ/MM/AAAA reste (taper reste
  // possible) ; l'agenda à sa droite, ou un clic dans le champ, ouvre le mois dessous. Un clic sur un jour écrit
  // « 05/01/2027 » dans le champ. Les jours qui se cliquent, les mois et aujourd'hui viennent de Rust (textes.calendrier),
  // qui vérifie de nouveau la date en enregistrant : c'est sa vérification qui compte.
  const ISO = /^\d{4}-\d{2}-\d{2}$/;
  const deux = (n) => String(n).padStart(2, '0');
  const textesCal = () => { const k = textesC().calendrier; return k && typeof k === 'object' ? k : {}; };
  const prefixe = (ou) => (ou === 'e' ? 'e' : 'f');
  // Un jour tapé, pour le montrer seulement (comme Jour::lire_saisie de Rust) : « 2/11/2026 », « 02.11.26 », « 2026-11-02 »
  function isoDe(t) {
    const s = String(t || '').trim();
    let a, m, j;
    if (ISO.test(s)) [a, m, j] = s.split('-').map(Number);
    else {
      const p = s.split(/[/.-]/).map((x) => x.trim());
      if (p.length !== 3 || !p.every((x) => /^\d+$/.test(x)) || p[0].length > 2 || p[1].length > 2 || !(p[2].length === 2 || p[2].length === 4)) return '';
      j = +p[0]; m = +p[1]; a = p[2].length === 2 ? 2000 + +p[2] : +p[2];
    }
    if (a < 1900 || a > 9999 || m < 1 || m > 12 || j < 1 || j > new Date(Date.UTC(a, m, 0)).getUTCDate()) return '';
    return `${a}-${deux(m)}-${deux(j)}`;
  }
  const saisieDe = (iso) => `${iso.slice(8, 10)}/${iso.slice(5, 7)}/${iso.slice(0, 4)}`;
  function bornesCal(ou) {
    const b = textesCal()[ou === 'e' ? 'entretien' : 'envoi'] || {};
    return { min: ISO.test(b.min || '') ? b.min : '1900-01-01', max: ISO.test(b.max || '') ? b.max : '9999-12-31' };
  }
  const valeurCal = (ou) => (ou === 'e' ? rdv?.date : form?.v?.date);
  // (le calendrier n'est montré qu'avec son champ : le formulaire ouvert, ou le petit formulaire de l'entretien)
  const calOuvert = (ou) => !!cal && cal.ou === ou && (ou === 'e' ? !!rdv : vueC === 'formulaire' && !!form);

  // Le champ de la date : le texte, et l'agenda qui ouvre le calendrier
  function champDate(ou, label, attrs, val, req) {
    const { esc } = O;
    const k = textesCal(), ouvert = calOuvert(ou), p = prefixe(ou);
    return `<div class="jm-champ jm-date${ouvert ? ' ouvert' : ''}"><span>${label}</span><div class="jm-date-l">
      <input class="f" ${attrs} data-champ="date" data-k="${p}:date" maxlength="12" autocomplete="off" spellcheck="false" placeholder="${esc(T('i_date'))}" value="${esc(val || '')}"${req ? ' aria-required="true"' : ''}>
      <button type="button" class="jm-cal-b f" data-perso="cal-ouvrir" data-ou="${ou}" data-k="${p}:cal" aria-label="${esc(texteSur(k.ouvrir))}" title="${esc(texteSur(k.ouvrir))}" aria-expanded="${ouvert}">${icone('agenda')}</button></div></div>`;
  }

  // Le mois montré : les jours de ses bornes se cliquent, le jour du champ et aujourd'hui sont marqués
  function calendrierHTML(ou) {
    const { esc } = O;
    const k = textesCal(), mois = Array.isArray(k.mois) ? k.mois : [];
    const { min, max } = bornesCal(ou), choisi = isoDe(valeurCal(ou)), auj = ISO.test(k.aujourdhui || '') ? k.aujourdhui : '';
    const { a, m } = cal, cle = `${a}-${deux(m + 1)}`, nomMois = texteSur(mois[m]);
    const decale = (new Date(Date.UTC(a, m, 1)).getUTCDay() + 6) % 7, n = new Date(Date.UTC(a, m + 1, 0)).getUTCDate();
    let jours = '<span class="jm-cal-vide"></span>'.repeat(decale);
    for (let j = 1; j <= n; j += 1) {
      const d = `${cle}-${deux(j)}`, cls = `${d === choisi ? ' oui' : ''}${d === auj ? ' auj' : ''}`;
      jours += d >= min && d <= max
        ? `<button type="button" class="jm-cal-d f${cls}" data-perso="cal-jour" data-ou="${ou}" data-v="${d}" data-k="cal:${d}" aria-label="${j} ${esc(nomMois)} ${a}" aria-pressed="${d === choisi}">${j}</button>`
        : `<span class="jm-cal-d non${cls}">${j}</span>`;
    }
    const fleche = (sens, ok, ic, aide) => (ok
      ? `<button type="button" class="jm-cal-fl f" data-perso="cal-mois" data-ou="${ou}" data-v="${sens}" data-k="cal:${sens < 0 ? 'avant' : 'apres'}" aria-label="${esc(texteSur(aide))}" title="${esc(texteSur(aide))}">${icone(ic)}</button>`
      : `<span class="jm-cal-fl non" aria-hidden="true">${icone(ic)}</span>`);
    const titre = texteSur(ou === 'e' ? k.titre_entretien : k.titre_envoi);
    const tete = (Array.isArray(k.jours) ? k.jours : []).map((j) => `<span class="jm-cal-j">${esc(texteSur(j))}</span>`).join('');
    // (il glisse en s'ouvrant seulement, pas à chaque mois changé)
    const entree = cal.neuf === true && !reduit(); cal.neuf = false;
    return `<div class="jm-cal${entree ? ' entree' : ''}" role="group" aria-label="${esc(titre)}">
      <div class="jm-cal-titre">${esc(titre)}</div>
      <div class="jm-cal-tete">${fleche(-1, cle > min.slice(0, 7), 'gauche', k.avant)}<b>${esc(nomMois)} ${a}</b>${fleche(1, cle < max.slice(0, 7), 'droite', k.apres)}</div>
      <div class="jm-cal-grille">${tete}${jours}</div>
      <div class="jm-cal-bas">${bouton('cal-fermer', 'cal:fermer', texteSur(k.fermer), '', 'discret', ` data-ou="${ou}"`)}</div></div>`;
  }

  // Ouvre le calendrier sur le mois du jour du champ (aujourd'hui s'il est vide, faux ou hors des bornes) ; le choix va
  // sur ce jour, sauf ouvert par un clic dans le champ (on peut continuer à taper)
  function ouvrirCal(ou, focusK) {
    const { min, max } = bornesCal(ou), k = textesCal();
    let d = isoDe(valeurCal(ou));
    if (!d || d < min || d > max) {
      const auj = ISO.test(k.aujourdhui || '') ? k.aujourdhui : min;
      d = auj < min ? min : auj > max ? max : auj;
    }
    cal = { ou, a: +d.slice(0, 4), m: +d.slice(5, 7) - 1, neuf: true };
    menu = false; confirme = false;
    dessinerTout(focusK || `cal:${d}`);
    montrerCal();
  }

  // Le champ et son calendrier, montrés ensemble dans le dossier (le champ d'abord, si tout ne tient pas)
  function montrerCal() {
    const box = hote.querySelector('.jm-dossier'), c = hote.querySelector('.jm-cal'), ch = hote.querySelector('.jm-date.ouvert');
    if (!box || !c || !ch) return;
    const b = box.getBoundingClientRect(), haut = ch.getBoundingClientRect().top - 8, bas = c.getBoundingClientRect().bottom + 8;
    let dy = 0;
    if (bas > b.bottom) dy = bas - b.bottom;
    if (haut - dy < b.top) dy = haut - b.top;
    if (dy) box.scrollTop += dy;
  }

  function fermerCal() {
    const p = prefixe(cal?.ou);
    cal = null;
    dessinerTout(`${p}:cal`);
  }

  // Ce qui est tapé dans le champ, calendrier ouvert : il suit (le mois du jour tapé, ce jour marqué), sans tout redessiner
  function suivreCal(ou) {
    if (!calOuvert(ou)) return;
    const d = isoDe(valeurCal(ou)), { min, max } = bornesCal(ou);
    if (d && d >= min && d <= max) cal = { ou, a: +d.slice(0, 4), m: +d.slice(5, 7) - 1 };
    const el = hote.querySelector('.jm-cal');
    if (el) el.outerHTML = calendrierHTML(ou);
  }

  function rdvHTML() {
    const { esc } = O;
    const champ = (k, label, indice, val, large) => `<label class="jm-champ${large ? ' large' : ''}"><span>${esc(label)}</span><input class="f" data-perso="rdv" data-champ="${k}" data-k="e:${k}" maxlength="${k === 'lieu' ? 120 : 12}" autocomplete="off" spellcheck="false" placeholder="${esc(indice)}" value="${esc(rdv[k] || '')}"></label>`;
    const date = champDate('e', esc(T('entretien_date')), `data-perso="rdv" aria-label="${esc(T('entretien_date'))}"`, rdv.date, false);
    return `<div class="jm-rdv"><b class="jm-rdv-t">${icone('agenda')}${esc(T('entretien_titre'))}</b>
      <div class="jm-rdv-champs">${date}${champ('heure', T('entretien_heure'), T('i_heure'), rdv.heure)}
        ${calOuvert('e') ? `<div class="jm-cal-place">${calendrierHTML('e')}</div>` : ''}${champ('lieu', T('entretien_lieu'), T('i_lieu'), rdv.lieu, true)}</div>
      <div class="jm-btns">${bouton('rdv-ok', 'e:ok', T('enregistrer'), 'coche', 'principal')}${bouton('rdv-non', 'e:non', T('annuler'), '', 'second')}</div></div>`;
  }

  function panneauHTML(f) {
    const { esc } = O;
    // (étape 7 : les mails reçus, en texte seulement)
    if (pilule === 'mails') {
      return liste(f.mails).filter((m) => numSur(m?.id)).map((m) => `<div class="jm-mail-r"><div class="jm-meta"><b>${esc(texteSur(m.objet))}</b><span>${esc(texteSur(m.de))}${m.recu_le ? `, ${esc(texteSur(m.recu_le))}` : ''}</span></div><div class="jm-lettre petit" data-brut>${esc(texteSur(m.texte))}</div></div>`).join('');
    }
    if (pilule === 'offre') {
      const o = f.offre || {};
      if (!o.lien) return `<p class="jm-panneau-vide">${esc(texteSur(o.vide))}</p>`;
      return `<div class="jm-meta">${esc(texteSur(o.texte))}</div><div class="jm-lien-l"><span class="jm-lien" data-brut>${esc(texteSur(o.lien))}</span>${bouton('copier', 'p:lien', T('copier_lien'), 'lien', 'discret', ' data-quoi="lien"')}</div>`;
    }
    if (pilule === 'notes') {
      const val = Object.hasOwn(notes, f.id) ? notes[f.id] : texteSur(f.notes);
      return `<textarea class="jm-notes f" data-perso="notes" data-k="p:notes" maxlength="4000" spellcheck="true" placeholder="${esc(T('i_notes'))}" aria-label="${esc(T('p_notes'))}">${esc(val)}</textarea><p class="jm-petit">${esc(T('notes_aide'))}</p>`;
    }
    // (étape 10 : les mails partis depuis Jobmind, chacun avec ce qui est parti exactement)
    const envois = liste(f.envois).filter((x) => numSur(x?.id));
    if (envois.length) {
      return envois.map((x, i) => `<div class="jm-envoye-m"><div class="jm-meta"><b>${esc(texteSur(x.titre))}</b>${i === 0 ? bouton('copier', 'p:envoye', T('copier'), 'copie', 'discret', ' data-quoi="envoye"') : ''}</div>
        <div class="jm-envoye-l">${liste(x.lignes).filter((l) => Array.isArray(l) && l.length === 2).map((l) => `<span>${esc(texteSur(l[0]))}</span><b data-brut>${esc(texteSur(l[1]))}</b>`).join('')}</div>
        <div class="jm-lettre" data-brut>${esc(texteSur(x.texte))}</div>${texteSur(x.message_id) ? `<p class="jm-petit jm-envoye-id" data-brut>${esc(x.message_id)}</p>` : ''}</div>`).join('');
    }
    const e = f.envoye || {};
    if (!e.texte) return `<div class="jm-meta">${esc(texteSur(e.meta))}</div><p class="jm-panneau-vide">${esc(texteSur(e.vide))}</p>`;
    return `<div class="jm-meta"><span>${esc(texteSur(e.meta))}</span>${bouton('copier', 'p:envoye', T('copier'), 'copie', 'discret', ' data-quoi="envoye"')}</div><div class="jm-lettre" data-brut>${esc(texteSur(e.texte))}</div>`;
  }

  function dossierHTML(f, s) {
    const { esc } = O;
    const a = f.action && typeof f.action === 'object' ? f.action : null;
    const c = f.calme && typeof f.calme === 'object' ? f.calme : null;
    let h = `<div class="jm-d${O.anime ? ' entree' : ''}"><div class="jm-d-tete"><div class="jm-d-titres"><h1>${esc(texteSur(f.titre))}</h1><div class="jm-d-sous">${esc(texteSur(f.sous))}</div></div>
      <div class="jm-st-place"><button type="button" class="jm-st gros bouton f" data-perso="statut-menu" data-k="d:statut" aria-haspopup="menu" aria-expanded="${menu}" title="${esc(T('statut'))}" style="--c:${couleurStatut(f.statut)}">${esc(texteSur(f.statut_nom))}${icone('bas')}</button>${menu ? menuStatutsHTML(f, s) : ''}</div></div>`;
    if (rdv) h += rdvHTML();
    h += `<p class="jm-ou">${esc(texteSur(f.ou))}</p><p class="jm-ou-sous">${esc(texteSur(f.ou_sous))}</p>`;
    if (a) {
      const bts = (Array.isArray(a.boutons) ? a.boutons : []).filter((b) => codeSur(b?.id)).map((b) => bouton(`act-${b.id === 'sans_reponse' ? 'sans' : b.id}`, `a:${b.id}`, texteSur(b.texte), b.icone, b.principal ? 'principal' : '')).join('');
      h += `<div class="jm-action"><span class="jm-a-ic">${icone(a.icone)}</span><div class="jm-a-txt"><b>${esc(texteSur(a.titre))}</b><span>${esc(texteSur(a.texte))}</span></div><div class="jm-a-bts">${bts}</div></div>`;
    } else if (c) {
      h += `<p class="jm-calme">${icone(c.icone)}<span>${esc(texteSur(c.texte))}</span></p>`;
    }
    // (étape 7 : le statut que dit une réponse reçue ; Jordan confirme d'un clic, jamais changé seul)
    const pr = objet(f.proposition);
    if (numSur(pr.mail)) {
      const bts = `${texteSur(pr.oui) ? bouton('m-oui', 'pr:oui', texteSur(pr.oui), 'coche', 'principal', ` data-id="${pr.mail}"`) : ''}${bouton('m-non', 'pr:non', texteSur(pr.non), '', 'second', ` data-id="${pr.mail}"`)}`;
      h += `<div class="jm-action jm-prop" style="--c:${couleurStatut(codeSur(pr.statut) ? pr.statut : 'relancee')}"><span class="jm-a-ic">${icone('mail')}</span><div class="jm-a-txt"><b>${esc(texteSur(pr.titre))}</b><span>${esc(texteSur(pr.texte))}</span><span class="jm-citation">${esc(texteSur(pr.citation))} <em>${esc(texteSur(pr.par))}</em></span></div><div class="jm-a-bts">${bts}</div></div>`;
    }
    // (étape 8 : une candidature liée à une offre : sa candidature préparée)
    const pp = objet(f.preparer);
    if (numSur(pp.offre)) h += `<div class="jm-action jm-pr-a"><span class="jm-a-ic">${icone('plume')}</span><div class="jm-a-txt"><b>${esc(texteSur(pp.titre))}</b><span>${esc(texteSur(pp.texte))}</span></div><div class="jm-a-bts">${bouton('pr-dossier', 'd:prep', texteSur(pp.bouton), 'droite', pp.prete === true ? '' : 'principal', ` data-id="${pp.offre}"`)}</div></div>`;
    const frise = Array.isArray(f.frise) ? f.frise : [];
    if (frise.length) h += `<div class="jm-frise">${frise.map((e, i) => `${i ? '<b class="jm-fl"></b>' : ''}<span><i></i>${esc(texteSur(e?.texte))} <em>${esc(texteSur(e?.date))}</em></span>`).join('')}</div>`;
    const pil = [['envoye', T('p_envoye')], ['offre', T('p_offre')], ['notes', T('p_notes')]];
    const nm = liste(f.mails).length;
    if (nm) pil.push(['mails', `${T('p_mails')} (${nm})`]);
    h += `<div class="jm-pilules" role="tablist">${pil.map(([id, nom]) => `<button type="button" role="tab" aria-selected="${pilule === id}" class="jm-pil f${pilule === id ? ' actif' : ''}" data-perso="pilule" data-id="${id}" data-k="p:${id}">${esc(nom)}</button>`).join('')}</div>`;
    h += `<div class="jm-panneau">${panneauHTML(f)}</div>`;
    if (confirme) {
      h += `<div class="jm-suppr"><p>${esc(texteSur(f.supprimer))}</p><div class="jm-btns">${bouton('suppr-oui', 'd:oui', T('oui_supprimer'), 'poubelle', 'danger')}${bouton('suppr-non', 'd:non', T('non'), '', 'second')}</div></div>`;
    } else {
      h += `<div class="jm-d-bas">${bouton('modifier', 'd:modifier', T('modifier'), 'crayon', 'discret')}${bouton('supprimer', 'd:supprimer', T('supprimer'), 'poubelle', 'discret danger')}</div>`;
    }
    return `${h}</div>`;
  }

  function formulaireHTML(s) {
    const { esc } = O;
    const v = form.v;
    const champ = (k, req, large) => {
      const lab = `${esc(T(k))}${req ? '' : ` <small>${esc(T('facultatif'))}</small>`}`;
      return `<label class="jm-champ${large ? ' large' : ''}"><span>${lab}</span><input class="f" data-perso="champ" data-champ="${k}" data-k="f:${k}" maxlength="${LONGUEURS[k]}" autocomplete="off" spellcheck="false" placeholder="${esc(T(`i_${k}`))}" value="${esc(v[k] || '')}"${req ? ' aria-required="true"' : ''}></label>`;
    };
    const zone = (k, haut) => `<label class="jm-champ large"><span>${esc(T(k))} <small>${esc(T('facultatif'))}</small></span><textarea class="f${haut ? ' haut' : ''}" data-perso="champ" data-champ="${k}" data-k="f:${k}" maxlength="${LONGUEURS[k]}" spellcheck="true" placeholder="${esc(T(`i_${k}`))}">${esc(v[k] || '')}</textarea></label>`;
    const moyens = (Array.isArray(s.moyens) ? s.moyens : []).filter((m) => codeSur(m?.id));
    const pastilles = moyens.map((m) => `<button type="button" role="radio" aria-checked="${v.moyen === m.id}" class="jm-puce f${v.moyen === m.id ? ' oui' : ''}" data-perso="moyen" data-id="${m.id}" data-k="f:moyen:${m.id}">${esc(texteSur(m.nom))}</button>`).join('');
    return `<form class="jm-form${O.anime ? ' entree' : ''}" novalidate>
      <h1>${esc(form.id ? T('modifier_titre') : T('nouvelle'))}</h1>${form.id ? '' : `<p class="jm-intro">${esc(T('intro'))}</p>`}
      <div class="jm-grille">${champ('entreprise', true)}${champ('poste', true)}${champ('ville')}${champDate('f', esc(T('date')), `data-perso="champ" aria-label="${esc(T('date'))}"`, v.date, true)}
        ${calOuvert('f') ? `<div class="jm-cal-place">${calendrierHTML('f')}</div>` : ''}
        <div class="jm-champ large"><span>${esc(T('moyen'))}</span><div class="jm-puces" role="radiogroup" aria-label="${esc(T('moyen'))}">${pastilles}</div></div>
        ${champ('source')}${champ('cv')}${champ('lien', false, true)}${zone('envoye', true)}${zone('notes')}</div>
      <p class="jm-err" role="alert"></p>
      <div class="jm-btns">${bouton('enregistrer', 'f:enregistrer', T('enregistrer'), 'coche', 'principal')}${bouton('annuler', 'f:annuler', T('annuler'), '', 'second')}</div></form>`;
  }

  // Le dossier de droite de l'onglet Candidatures
  // Étape 7 : un mail à ranger ouvert : son texte (jamais son HTML), la candidature à choisir, « Rien à voir »
  function mailRangerHTML(m) {
    const { esc } = O, t = tx(mailsDe());
    const choix = liste(mailsDe().choix).filter((x) => numSur(x?.id));
    const ici = numSur(mrChoix[m.id]) ? mrChoix[m.id] : (numSur(m.suggestion) ? m.suggestion : 0);
    const options = `<option value="0"${ici ? '' : ' selected'}>${esc(t('choisir'))}</option>${choix.map((x) => `<option value="${x.id}"${x.id === ici ? ' selected' : ''}>${esc(texteSur(x.nom))}</option>`).join('')}`;
    return `<div class="jm-d${O.anime ? ' entree' : ''}"><div class="jm-d-tete"><div class="jm-d-titres"><h1>${esc(texteSur(m.objet))}</h1><div class="jm-d-sous">${esc(t('de'))} ${esc(texteSur(m.de))}${m.recu_le ? `, ${esc(texteSur(m.recu_le))}` : ''}</div></div>
      <span class="jm-st gros" style="--c:var(--T)">${esc(t('titre'))}</span></div>
      <p class="jm-ou">${esc(t('intro'))}</p>${m.raison ? `<p class="jm-ou-sous">${esc(texteSur(m.raison))}</p>` : ''}
      <div class="jm-action jm-ranger"><span class="jm-a-ic">${icone('dossier')}</span><div class="jm-a-txt"><b>${esc(t('ranger_dans'))}</b>
        <select class="f jm-select" data-perso="m-choix" data-id="${m.id}" data-k="mr:choix" aria-label="${esc(t('ranger_dans'))}">${options}</select></div>
        <div class="jm-a-bts">${bouton('m-ranger', 'mr:ranger', t('ranger'), 'coche', 'principal', ` data-id="${m.id}"`)}${bouton('m-rien', 'mr:rien', t('rien'), 'poubelle', 'discret danger', ` data-id="${m.id}"`)}</div></div>
      <div class="jm-lettre jm-mail-t" data-brut>${esc(texteSur(m.texte) || t('vide_texte'))}</div>
      ${m.pieces ? `<p class="jm-petit">${esc(texteSur(m.pieces))}</p>` : ''}</div>`;
  }

  function droiteCandidatures(o, s) {
    if (vueC === 'formulaire' && form) return formulaireHTML(s);
    if (vueC === 'bilan' && bilanDe().titre) return bilanHTML();
    if (vueC === 'mail') {
      const m = objet(mailsDe().ouvert);
      if (numSur(m.id)) return mailRangerHTML(m);
    }
    const f = fiche();
    if (f) return dossierHTML(f, s);
    const vide = !(Array.isArray(s.groupes) && s.groupes.length);
    if (vide) return dossierVideHTML(o.dossier, true);
    return `<p class="jm-choisir">${O.esc(T('choisir'))}</p>`;
  }

  // Les réglages (Mon profil > Réglages) : le délai avant relance
  function reglagesHTML(d) {
    const { esc } = O;
    const r = d.reglage;
    const v = Number.isInteger(r.valeur) ? r.valeur : 14, min = Number.isInteger(r.min) ? r.min : 3, max = Number.isInteger(r.max) ? r.max : 60;
    const choix = (Array.isArray(r.choix) ? r.choix : []).filter((n) => Number.isInteger(n) && n >= min && n <= max);
    return `<div class="jm-reglages${O.anime ? ' entree' : ''}"><h1>${esc(texteSur(d.titre))}</h1>
      <div class="jm-carte-r"><div class="jm-r-tete"><span class="jm-a-ic">${icone('relance')}</span><div><b>${esc(texteSur(r.titre))}</b><p>${esc(texteSur(r.texte))}</p></div></div>
        <div class="jm-r-reglage"><div class="jm-r-valeur">
          <button type="button" class="jm-r-pm f" data-perso="delai" data-v="${v - 1}" data-k="g:moins" aria-label="${esc(texteSur(r.moins))}"${v <= min ? ' disabled' : ''}>−</button>
          <span class="jm-r-n" aria-live="polite"><b>${v}</b> ${esc(texteSur(r.unite))}</span>
          <button type="button" class="jm-r-pm f" data-perso="delai" data-v="${v + 1}" data-k="g:plus" aria-label="${esc(texteSur(r.plus))}"${v >= max ? ' disabled' : ''}>+</button></div>
          <div class="jm-puces">${choix.map((n) => `<button type="button" class="jm-puce f${n === v ? ' oui' : ''}" data-perso="delai" data-v="${n}" data-k="g:${n}" aria-pressed="${n === v}">${n} ${esc(texteSur(r.unite))}</button>`).join('')}</div></div></div>
      ${bordReglagesHTML(objet(d.bord))}
      ${sitesReglageHTML(d.sites)}
      ${d.note ? `<p class="jm-note gauche">${esc(texteSur(d.note))}</p>` : ''}</div>`;
  }

  // Réglages (étape 12) : chercher de nouvelles offres toute seule (le rythme), exporter, sauvegarder et remettre une
  // sauvegarde (jamais au premier clic : Rust lit et vérifie le fichier, puis la question)
  function bordReglagesHTML(b) {
    const { esc } = O;
    if (!b.rafraichir) return '';
    const r = objet(b.rafraichir), x = objet(b.export), s = objet(b.sauvegarde), a = objet(s.attente), boite = b.boite === true;
    const v = Number.isInteger(r.valeur) ? r.valeur : 6;
    const choix = liste(r.choix).filter((c) => Number.isInteger(c?.heures)).map((c) => `<button type="button" class="jm-puce f${c.heures === v ? ' oui' : ''}" data-perso="rythme" data-v="${c.heures}" data-k="rr:${c.heures}" aria-pressed="${c.heures === v}">${esc(texteSur(c.texte))}</button>`).join('');
    const an = objet(b.annonce);
    const annonce = texteSur(an.texte) ? `<div class="jm-annonce ${an.genre === 'erreur' ? 'erreur' : 'ok'}">${icone(an.genre === 'erreur' ? 'attention' : 'coche')}<span>${esc(texteSur(an.texte))}</span></div>` : '';
    const question = texteSur(a.titre)
      ? `<div class="jm-suppr jm-rs-q"><p><b>${esc(texteSur(a.titre))}</b></p><p class="jm-rs-f">${esc(texteSur(a.fichier))}. ${esc(texteSur(a.detail))}</p><p>${esc(texteSur(a.texte))}</p>
          <div class="jm-btns">${bouton('rs-oui', 'rs:oui', texteSur(a.oui), 'coche', 'danger')}${bouton('rs-non', 'rs:non', texteSur(a.non), '', 'second')}</div></div>`
      : '';
    const off = boite ? ' disabled' : '';
    return `<div class="jm-carte-r"><div class="jm-r-tete"><span class="jm-a-ic">${icone('recharger')}</span><div><b>${esc(texteSur(r.titre))}</b><p>${esc(texteSur(r.texte))}</p></div></div>
        <div class="jm-r-reglage"><div class="jm-puces">${choix}</div>${r.dernier ? `<p class="jm-petit">${esc(texteSur(r.dernier))}</p>` : ''}</div></div>
      <div class="jm-carte-r"><div class="jm-r-tete"><span class="jm-a-ic">${icone('cv')}</span><div><b>${esc(texteSur(x.titre))}</b><p>${esc(texteSur(x.texte))}</p></div></div>
        <div class="jm-btns jm-r-bt">${bouton('rx-csv', 'rx:csv', texteSur(x.csv), 'importer', 'second', off)}${bouton('rx-pdf', 'rx:pdf', texteSur(x.pdf), 'cv', 'second', off)}</div></div>
      <div class="jm-carte-r"><div class="jm-r-tete"><span class="jm-a-ic">${icone('dossier')}</span><div><b>${esc(texteSur(s.titre))}</b><p>${esc(texteSur(s.texte))}</p></div></div>
        <p class="jm-r-cles">${icone('cadenas')}<span>${esc(texteSur(s.cles))}</span></p>
        <div class="jm-btns jm-r-bt">${bouton('rs-sauver', 'rs:sauver', texteSur(s.sauver), 'importer', 'principal', off)}${bouton('rs-restaurer', 'rs:restaurer', texteSur(s.restaurer), 'relance', 'second', off)}</div>
        ${question}${annonce ? `<div class="jm-r-an">${annonce}</div>` : ''}<p class="jm-petit jm-r-auto">${esc(texteSur(s.auto))}</p></div>`;
  }

  // Réglages (étapes 5 et 5 bis) : la session des sites, et « Me déconnecter de tous les sites » (jamais au premier
  // clic : une question d'abord)
  function sitesReglageHTML(s) {
    const { esc } = O;
    if (!s || typeof s !== 'object' || !texteSur(s.titre)) return '';
    const bas = sitesConfirme
      ? `<div class="jm-suppr"><p>${esc(texteSur(s.question))}</p><div class="jm-btns">${bouton('cs-decon-oui', 'pn:oui', texteSur(s.oui), 'coche', 'danger')}${bouton('cs-decon-non', 'pn:non', texteSur(s.non), '', 'second')}</div></div>`
      : `<div class="jm-btns">${bouton('cs-deconnecter', 'pn:decon', texteSur(s.bouton), 'cadenas', 'second')}</div>`;
    return `<div class="jm-carte-r jm-r-sites"><div class="jm-r-tete"><span class="jm-a-ic">${icone('globe')}</span><div><b>${esc(texteSur(s.titre))}</b><p>${esc(texteSur(s.texte))}</p></div></div>${bas}</div>`;
  }

  // ---------------------------------------------------------------- les sites en coulisse (étape 5 bis)
  // LinkedIn, Indeed, Welcome to the Jungle et France Travail sont lus par Rust en coulisse (une vue des sites chargée
  // mais invisible) ; cette page n'en montre que ce que Rust a lu, dans le design de Jobmind : la liste des résultats,
  // l'offre ouverte (P.coulisse). Deux cartes par-dessus l'onglet :
  // - la connexion (« Me connecter ») : le mail et le mot de passe tapés ici ne vivent que dans cnx (jamais dans
  //   l'état, jamais redessinés en HTML : la case du mot de passe est remplie par sa propriété), envoyés une fois à
  //   Rust ('cs-connecter'), puis oubliés aussitôt ;
  // - la petite fenêtre du site (P.coulisse.petite) : un bandeau de Jobmind et, dessous, la zone laissée vide
  //   (.jm-pf-site) où Rust pose la vraie page du site ; sa place lui est envoyée à chaque changement ('cs-zone').
  let sitesConfirme = false;  // « Me déconnecter de tous les sites ? » demandé
  let cnx = null;             // la carte de connexion ouverte : { site, mail, mdp }
  let pfZone = null;          // la dernière zone envoyée
  let pfObs = null;           // qui regarde la taille de la zone
  let pfMinuteur = 0;
  let carteVue = null;        // la carte dessinée la fois d'avant (« pf » ou « cx ») : seule une nouvelle carte s'anime
  const cs = () => objet(P?.coulisse);
  const tC = () => { const t = objet(cs().textes); return (k, site) => texteSur(t[k]).replace('{site}', site || ''); };
  const siteDe = (id) => liste(cs().sites).find((s) => s?.id === id && codeSurP(id)) || null;
  const sourceSite = () => (codeSurP(cs().source) ? siteDe(cs().source) : null);
  const petiteOuverte = () => !!(cs().petite && typeof cs().petite === 'object');

  // « Où chercher » : la liste déroulante des sources de l'onglet Offres, en haut de la colonne de gauche. Fermée, elle
  // montre le choix (et son point vert s'il est connecté) ; ouverte (srcMenu), ses cinq choix « Chercher sur ... » :
  // les quatre sites (LinkedIn, France Travail, Indeed, Welcome to the Jungle), puis Adzuna (mes offres
  // gardées). Les flèches choisissent dans la liste, Entrée (A) prend le choix, Échap (B) la referme sans rien changer.
  // (le data-k de chaque choix garde le rang de Rust : « cs:s0 » Adzuna, « cs:s1 » LinkedIn, etc.)
  let srcMenu = false;
  const ORDRE_SITES = ['linkedin', 'francetravail', 'indeed', 'wttj'];
  function sourcesListe() {
    const t = tC(), sites = liste(cs().sites).filter((s) => codeSurP(s?.id));
    if (!sites.length) return [];
    const l = sites.map((s, i) => ({ id: s.id, nom: texteSur(s.nom), connecte: s.connecte === true, k: `cs:s${i + 1}` }));
    const rang = (s) => { const r = ORDRE_SITES.indexOf(s.id); return r < 0 ? 99 : r; };
    l.sort((a, b) => rang(a) - rang(b));
    return [...l, { id: '', nom: t('mes_offres'), connecte: false, k: 'cs:s0', note: t('mes_offres_note') }];
  }
  function sourcesHTML() {
    const { esc } = O, t = tC(), src = cs().source || '', l = sourcesListe();
    if (!l.length) return '';
    const ici = l.find((s) => s.id === src) || l[l.length - 1];
    const point = (s) => `<i class="jm-point${s.connecte ? ' ok' : ''}" aria-hidden="true"></i>`;
    const menuH = srcMenu
      ? `<div class="jm-srcs-m" role="listbox" aria-label="${esc(t('ou_t'))}">${l.map((s) => {
        const oui = s === ici;
        const droite = s.connecte ? `<span class="jm-srcs-cx">${point(s)}${esc(t('connecte'))}</span>` : '';
        const note = s.note ? `<small>${esc(s.note)}</small>` : '';
        return `<button type="button" role="option" aria-selected="${oui}" class="jm-srcs-l f${oui ? ' ici' : ''}" data-perso="cs-source" data-id="${s.id}" data-k="${s.k}">${icone(oui ? 'coche' : 'loupe')}<span class="jm-srcs-n">${esc(t('chercher', s.nom))}${note}</span>${droite}</button>`;
      }).join('')}</div>`
      : '';
    return `<div class="jm-srcs"><span class="jm-srcs-t">${esc(t('ou_t'))}</span>
      <button type="button" class="jm-srcs-b f${srcMenu ? ' on' : ''}" data-perso="cs-ou" data-k="cs:ou" aria-haspopup="listbox" aria-expanded="${srcMenu}">${ici.id ? point(ici) : icone('loupe')}<span class="jm-srcs-n">${esc(ici.nom)}</span>${ici.connecte ? `<span class="jm-srcs-cx">${esc(t('connecte'))}</span>` : ''}${icone('bas')}</button>${menuH}</div>`;
  }
  // Ouvre ou ferme « Où chercher » ; ouverte, la main va au choix en cours
  function basculerSources(ouvrir) {
    srcMenu = ouvrir;
    O.son?.tic?.();
    const ici = sourcesListe().find((s) => s.id === (cs().source || '')) || sourcesListe().at(-1);
    dessinerTout(ouvrir && ici ? ici.k : 'cs:ou');
  }
  // Dans « Où chercher » ouverte : haut et bas passent d'un choix à l'autre (sans en sortir), gauche et droite ne font rien
  function toucheSources(e) {
    if (!srcMenu || !['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) return;
    const els = [...hote.querySelectorAll('.jm-srcs-m .jm-srcs-l')];
    if (!els.length) return;
    const i = els.indexOf(document.activeElement);
    if (i < 0 && e.target?.dataset?.perso !== 'cs-ou') return;
    e.preventDefault(); e.stopPropagation();
    let j = i;
    if (e.key === 'ArrowDown') j = i < 0 ? 0 : (i + 1) % els.length;
    else if (e.key === 'ArrowUp') j = i < 0 ? els.length - 1 : (i - 1 + els.length) % els.length;
    else if (e.key === 'Home') j = 0;
    else if (e.key === 'End') j = els.length - 1;
    else return;
    els[j].focus({ preventScroll: true });
    els[j].scrollIntoView({ block: 'nearest' });
  }

  // Une phrase de Rust (une erreur, une lecture qui casse, une connexion faite), avec son bouton
  function annonceSitesHTML(a, site) {
    const { esc } = O, t = tC();
    if (!a || !texteSur(a.texte)) return '';
    const g = ['ok', 'attention', 'erreur'].includes(a.genre) ? a.genre : 'ok';
    const b = { voir: ['cs-voir', 'cs:a-voir', t('voir'), 'ext'], connexion: ['cs-connexion', 'cs:a-cx', t('connecter'), 'cadenas'], page_site: ['cs-page-site', 'cs:a-site', t('page_site'), 'globe'] }[a.bouton];
    const offre = numSur(a.offre) ? bouton('cs-offre', 'cs:a-offre', t('voir_offre'), 'droite', 'discret', ` data-id="${a.offre}"`) : '';
    return `<div class="jm-annonce ${g}" role="status">${icone(g === 'ok' ? 'coche' : 'attention')}<div><span>${esc(texteSur(a.texte))}</span>${b ? bouton(b[0], b[1], b[2], b[3], 'discret', ` data-id="${site?.id || ''}"`) : ''}${offre}</div></div>`;
  }

  // La colonne de gauche quand un site est choisi : la recherche, « Chercher sur LinkedIn », la connexion, la liste
  function listeSitesHTML(x, s) {
    const { esc } = O, t = tC(), c = cs(), to = tO(x);
    const recherches = liste(x.recherches).filter((r) => numSur(r?.id));
    let h = `<div class="jm-rch">${sourcesHTML()}`;
    if (recherches.length) {
      h += `<div class="jm-rch-l"><label class="jm-rch-choix"><span>${esc(to('mes_recherches'))}</span><select class="f" data-perso="of-recherche" data-k="or:choix">${recherches.map((r) => `<option value="${r.id}"${r.id === x.recherche ? ' selected' : ''}>${esc(texteSur(r.nom))}</option>`).join('')}</select></label></div>`;
      if (x.recherche_sous) h += `<p class="jm-rch-sous">${esc(texteSur(x.recherche_sous))}</p>`;
    } else {
      h += `<p class="jm-rch-sous">${esc(to('sans_recherche'))}</p><div class="jm-btns serre">${bouton('of-nouvelle', 'or:nouvelle', to('nouvelle'), 'plus', '')}</div>`;
    }
    const occupe = typeof c.attente === 'string' && c.attente;
    h += `<button type="button" class="jm-bt principal jm-rch-go f" data-perso="cs-chercher" data-id="${s.id}" data-k="cs:chercher"${occupe || !recherches.length ? ' disabled' : ''}>${icone('loupe')}<span>${esc(t('chercher', s.nom))}</span></button>`;
    if (occupe) h += `<div class="jm-attente petite" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(occupe)}</span></div>`;
    h += annonceSitesHTML(c.annonce, s);
    const oui = s.connecte === true;
    // (une annonce qui propose déjà « Me connecter » remplace la ligne d'état tant qu'elle est là : une seule carte)
    const remplace = !oui && texteSur(c.annonce?.texte) && c.annonce.bouton === 'connexion';
    if (!remplace) h += `<div class="jm-cx-etat${oui ? ' ok' : ''}"><i class="jm-point${oui ? ' ok' : ''}" aria-hidden="true"></i><span>${esc(texteSur(s.etat))}</span>${oui ? '' : bouton('cs-connexion', 'cs:connecter', t('connecter'), 'cadenas', 'discret', ` data-id="${s.id}"`)}</div>`;
    h += basculesHTML(to);
    h += '</div>';
    const l = c.liste && typeof c.liste === 'object' && c.liste.site === s.id ? c.liste : null;
    if (l && liste(l.lignes).length) {
      h += `<div class="jm-liste-tete"><span class="jm-liste-t">${esc(texteSur(l.titre))}</span></div><p class="jm-rch-sous jm-cs-sous">${esc(texteSur(l.sous))}</p>`;
      for (const r of liste(l.lignes)) {
        if (!Number.isInteger(r?.i) || r.i < 0 || r.i > 999) continue;
        const sel = r.sel === true;
        h += `<button type="button" class="jm-cand jm-off f${sel ? ' sel' : ''}" data-perso="cs-ouvrir" data-i="${r.i}" data-k="csl:${r.i}" aria-current="${sel}">
          <span class="jm-c1"><b>${esc(texteSur(r.titre))}</b></span>
          <span class="jm-c2">${esc(texteSur(r.sous))}</span><span class="jm-c3 pale">${esc(texteSur(r.l3))}</span>${texteSur(r.lu) ? `<span class="jm-c4"><span>${esc(r.lu)}</span></span>` : ''}</button>`;
      }
      if (l.suivante === true) h += `<div class="jm-btns centre jm-cs-suite">${bouton('cs-suivante', 'cs:suivante', t('suivante'), 'droite', 'second')}</div>`;
    }
    return h;
  }

  // L'offre d'un site lue en coulisse, comme une offre de l'onglet Offres
  function ficheSiteHTML(f, s) {
    const { esc } = O, t = tC();
    let h = `<div class="jm-d jm-of${O.anime ? ' entree' : ''}"><div class="jm-d-tete"><div class="jm-d-titres"><h1>${esc(texteSur(f.titre))}</h1><div class="jm-d-sous">${esc(texteSur(f.sous))}</div></div></div>`;
    h += `<div class="jm-o-src"><span>${esc(texteSur(f.source))}</span></div>`;
    if (texteSur(f.candidats)) h += `<p class="jm-o-double jm-o-cand">${icone('profil')}<span>${esc(texteSur(f.candidats))}</span></p>`;
    // (étape 8 : « L'offre » et « Ma candidature »)
    const p = prDe(f);
    if (p) {
      h += pilulesOffreHTML(f, p);
      if (ofPil === 'cand') return `${h}${candidatureHTML(p)}</div>`;
    }
    h += `<div class="jm-infos">${liste(f.infos).map((i) => `<div><span>${esc(texteSur(i?.nom))}</span><b>${esc(texteSur(i?.valeur))}</b></div>`).join('')}</div>`;
    h += luHTML(f.lecture, tO(offresDe()));
    const voir = bouton('cs-voir', 'cs:voir', t('voir'), 'ext', '', ` data-id="${s.id}"`);
    if (numSur(f.ajoutee)) {
      h += `<div class="jm-action calme-o"><span class="jm-a-ic">${icone('coche')}</span><div class="jm-a-txt"><b>${esc(t('ajoutee'))}</b></div><div class="jm-a-bts">${bouton('cs-offre', 'cs:offre', t('voir_offre'), 'dossier', '', ` data-id="${f.ajoutee}"`)}</div></div><div class="jm-btns">${voir}</div>`;
    } else {
      h += `<div class="jm-btns jm-o-bts">${bouton('cs-ajouter', 'cs:ajouter', t('ajouter'), 'plus', 'principal')}${voir}</div>`;
    }
    const texte = texteSur(f.texte);
    h += `<h2 class="jm-h2">${esc(t('texte_t'))}</h2>`;
    h += texte ? `<div class="jm-o-texte" data-brut>${esc(texte)}</div>` : `<p class="jm-panneau-vide">${esc(t('texte_vide'))}</p>`;
    return `${h}</div>`;
  }

  // La droite quand un site est choisi : l'offre ouverte, son chargement, ou ce que fait ce site dans Jobmind
  function droiteSitesHTML(s) {
    const { esc } = O, t = tC(), c = cs();
    if (c.attente_offre === true) {
      return `<div class="jm-vide"><div class="jm-v-ic plasma-bord">${icone('globe')}</div><div class="jm-attente" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(texteSur(c.attente))}</span></div></div>`;
    }
    const f = c.fiche && typeof c.fiche === 'object' && c.fiche.site === s.id ? c.fiche : null;
    if (f) return ficheSiteHTML(f, s);
    const l = c.liste && typeof c.liste === 'object' && c.liste.site === s.id && liste(c.liste.lignes).length;
    return `<div class="jm-vide${anime()}"><div class="jm-v-ic plasma-bord">${icone('globe')}</div><h1>${esc(t('vide_t', s.nom))}</h1>
      <p>${esc(l ? t('choisir') : t('vide', s.nom))}</p>
      ${s.connecte === true ? '' : `<p class="jm-note">${esc(t('vide_note'))}</p><div class="jm-btns centre">${bouton('cs-connexion', 'cs:connecter2', t('connecter'), 'cadenas', 'second', ` data-id="${s.id}"`)}</div>`}</div>`;
  }

  // La carte de connexion de Jobmind (par-dessus l'onglet)
  function connexionHTML() {
    const { esc } = O, t = tC(), s = siteDe(cnx?.site);
    if (!s) return '';
    return `<div class="jm-modale${carteVue === 'cx' ? '' : ' entree'}" role="dialog" aria-modal="true" aria-labelledby="jm-cx-t"><form class="jm-modale-boite" novalidate>
      <div class="jm-modale-tete"><span class="jm-a-ic">${icone('cadenas')}</span><h2 id="jm-cx-t">${esc(t('cx_titre', s.nom))}</h2></div>
      <p class="jm-intro">${esc(t('cx_intro', s.nom))}</p>
      <div class="jm-grille un">
        <label class="jm-champ"><span>${esc(t('cx_mail'))}</span><input class="f" type="email" data-perso="cs-cx-champ" data-champ="mail" data-k="cx:mail" maxlength="200" autocomplete="off" spellcheck="false" value="${esc(cnx.mail || '')}"></label>
        <label class="jm-champ"><span>${esc(t('cx_mdp'))}</span><input class="f" type="password" data-perso="cs-cx-champ" data-champ="mdp" data-k="cx:mdp" maxlength="300" autocomplete="off" spellcheck="false"></label>
      </div>
      <p class="jm-err" role="alert"></p>
      <div class="jm-btns">${bouton('cs-cx-ok', 'cx:ok', t('cx_ok'), 'cadenas', 'principal')}${bouton('cs-cx-non', 'cx:non', t('cx_annuler'), '', 'second')}</div>
      <div class="jm-cx-autre"><span>${esc(t('cx_autre'))}</span>${bouton('cs-page-site', 'cx:site', t('page_site'), 'globe', 'discret', ` data-id="${s.id}"`)}</div>
    </form></div>`;
  }

  // Étape 11 : à côté de la page du site (LinkedIn, ou Jordan qui fait lui-même), ses textes à copier et « C'est envoyé »
  function coteHTML(c) {
    if (!c || typeof c !== 'object') return '';
    const { esc } = O, qs = liste(c.questions).filter((q) => texteSur(q?.reponse));
    let h = `<aside class="jm-pf-cote" aria-label="${esc(texteSur(c.titre))}"><b class="jm-pf-ct">${esc(texteSur(c.titre))}</b><p class="jm-petit">${esc(texteSur(c.texte))}</p>`;
    if (texteSur(c.lettre)) h += `<div class="jm-pf-bloc"><div class="jm-pf-bt"><span>${esc(texteSur(c.lettre_t))}</span>${bouton('po-copier', 'pf:c-lettre', texteSur(c.copier), 'copie', 'discret', ' data-quoi="lettre"')}</div><div class="jm-pf-txt-l" data-brut>${esc(c.lettre)}</div></div>`;
    if (qs.length) h += `<b class="jm-pf-ct petit">${esc(texteSur(c.questions_t))}</b>${qs.map((q, n) => `<div class="jm-pf-bloc"><div class="jm-pf-bt"><span>${esc(texteSur(q.question))}</span>${bouton('po-copier', `pf:c-q${n}`, texteSur(c.copier), 'copie', 'discret', ` data-quoi="q" data-n="${n}"`)}</div><div class="jm-pf-txt-l" data-brut>${esc(q.reponse)}</div></div>`).join('')}`;
    h += `<div class="jm-pf-fin">${bouton('po-fait', 'pf:fait', texteSur(c.fait), 'coche', 'principal')}</div></aside>`;
    return h;
  }

  // La petite fenêtre du site : le bandeau de Jobmind, et la place de la vraie page
  function petiteHTML() {
    const { esc } = O, t = tC(), p = objet(cs().petite);
    const lecture = p.lecture === true;
    const ajouter = p.ajouter === true
      ? `<button type="button" class="jm-bt principal f" data-perso="cs-ajouter-page" data-k="pf:ajouter"${lecture ? ' disabled' : ''}>${lecture ? '<span class="jm-roue" aria-hidden="true"></span>' : icone('plus')}<span>${esc(lecture ? t('lecture') : t('ajouter'))}</span></button>`
      : '';
    return `<div class="jm-pf${carteVue === 'pf' ? '' : ' entree'}" role="dialog" aria-modal="true" aria-labelledby="jm-pf-t"><div class="jm-pf-cadre">
      <div class="jm-pf-bandeau"><span class="jm-a-ic">${icone(p.ajouter === true || p.cote ? 'globe' : 'attention')}</span>
        <div class="jm-pf-txt"><b id="jm-pf-t">${esc(texteSur(p.titre))}</b><span>${esc(texteSur(p.texte))}</span></div>
        <div class="jm-pf-bts">${ajouter}${bouton('cs-fermer', 'pf:fermer', t('fermer'), 'croix', 'second')}</div></div>
      ${annonceSitesHTML(p.annonce, null).replace('class="jm-annonce', 'class="jm-pf-annonce jm-annonce')}
      ${texteSur(p.apres) ? `<p class="jm-pf-apres">${esc(p.apres)}</p>` : ''}
      <div class="jm-pf-corps"><div class="jm-pf-site" aria-hidden="true"><span class="jm-roue" aria-hidden="true"></span></div>${coteHTML(p.cote)}</div>
    </div></div>`;
  }

  // Les cartes par-dessus l'onglet (après chaque dessin)
  function cartesSites() {
    const jm = hote.querySelector('#jm');
    if (!jm) return;
    if (cnx && !siteDe(cnx.site)) cnx = null;
    if (petiteOuverte()) { cnx = null; jm.insertAdjacentHTML('beforeend', petiteHTML()); carteVue = 'pf'; }
    else if (cnx) {
      jm.insertAdjacentHTML('beforeend', connexionHTML());
      carteVue = 'cx';
      // (le mot de passe tapé reste dans sa case, posé par sa propriété : jamais écrit dans le HTML)
      const m = hote.querySelector('[data-k="cx:mdp"]'); if (m) m.value = cnx.mdp || '';
    } else carteVue = null;
    suivreZone();
  }

  // La zone laissée à la page du site : envoyée à Rust (en pixels de la page) à chaque changement de taille ou de place
  function suivreZone() {
    const el = hote?.querySelector('.jm-pf-site');
    if (!el) { if (pfObs) { pfObs.disconnect(); pfObs = null; } pfZone = null; return; }
    if (!pfObs) pfObs = new ResizeObserver(() => { clearTimeout(pfMinuteur); pfMinuteur = setTimeout(envoyerZone, 40); });
    pfObs.disconnect();
    pfObs.observe(el);
    requestAnimationFrame(envoyerZone);
  }
  function envoyerZone() {
    const el = hote?.querySelector('.jm-pf-site');
    if (!el || !petiteOuverte()) return;
    const r = el.getBoundingClientRect(), d = (n) => Math.round(n * 2) / 2;
    const z = { x: d(r.left), y: d(r.top), l: d(r.width), h: d(r.height) };
    if (z.l < 40 || z.h < 40) return;
    if (pfZone && pfZone.x === z.x && pfZone.y === z.y && pfZone.l === z.l && pfZone.h === z.h) return;
    pfZone = z;
    O.action('cs-zone', z);
  }
  addEventListener('resize', () => { clearTimeout(pfMinuteur); pfMinuteur = setTimeout(envoyerZone, 40); });

  // Ouvre la carte de connexion d'un site
  function ouvrirConnexion(id) {
    if (!siteDe(id)) return;
    cnx = { site: id, mail: cnx?.site === id ? cnx.mail : '', mdp: '' };
    dessinerTout('cx:mail');
  }
  function fermerConnexion() {
    const id = cnx?.site;
    if (cnx) cnx.mdp = '';
    cnx = null;
    dessinerTout(id ? 'cs:connecter' : undefined);
  }
  // « Me connecter » : le mail et le mot de passe partent une fois à Rust, puis sont oubliés ici
  function envoyerConnexion() {
    if (!cnx) return;
    const t = tC(), mail = String(cnx.mail || '').trim(), mdp = String(cnx.mdp || '');
    const vide = !mail ? 'mail' : (!mdp ? 'mdp' : null);
    if (vide) {
      const el = hote.querySelector(`[data-k="cx:${vide}"]`);
      el?.closest('.jm-champ')?.classList.add('faux');
      const err = hote.querySelector('.jm-modale .jm-err'); if (err) err.textContent = t(vide === 'mail' ? 'cx_vide_mail' : 'cx_vide_mdp');
      el?.focus({ preventScroll: false });
      O.son?.retour?.();
      return;
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    O.son?.ok?.();
    const site = cnx.site;
    hote.querySelectorAll('.jm-modale input').forEach((el) => { el.value = ''; });
    cnx.mdp = ''; cnx = null;
    O.action('cs-connecter', { site, mail: mail.slice(0, 200), mdp: mdp.slice(0, 300) });
    dessinerTout('cs:chercher');
  }

  // Un élément des sites (ou des réglages de leur session) ; vrai s'il en était un
  function validerSites(el, d) {
    switch (d.perso) {
      case 'cs-ou': basculerSources(!srcMenu); return true;
      case 'cs-source': {
        if (d.id !== '' && !codeSurP(d.id)) return true;
        srcMenu = false;
        if ((d.id || '') === (cs().source || '')) { O.son?.retour?.(); dessinerTout('cs:ou'); return true; }
        O.son?.bascule?.(); dessinerTout('cs:ou'); O.anime = !reduit(); O.action('cs-source', { site: d.id || '' });
        return true;
      }
      case 'cs-chercher': {
        if (el.disabled || !codeSurP(d.id)) return true;
        const x = offresDe();
        O.son?.ok?.(); O.action('cs-chercher', { site: d.id, recherche: numSur(x?.recherche) ? x.recherche : 0 });
        return true;
      }
      case 'cs-suivante': O.son?.ok?.(); O.action('cs-suivante'); return true;
      case 'cs-ouvrir': { const i = Number(d.i); if (Number.isInteger(i) && i >= 0) { O.son?.ok?.(); O.anime = !reduit(); O.action('cs-ouvrir', { i }); } return true; }
      case 'cs-ajouter': if (!el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('cs-ajouter'); } return true;
      case 'cs-voir': O.son?.ok?.(); O.action('cs-voir'); return true;
      case 'cs-offre': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); O.action('cs-offre', { id }); } return true; }
      case 'cs-connexion': O.son?.ok?.(); ouvrirConnexion(codeSurP(d.id) ? d.id : cs().source); return true;
      case 'cs-cx-champ': {   // (Entrée : la case suivante, puis « Me connecter »)
        if (d.champ === 'mail') hote.querySelector('[data-k="cx:mdp"]')?.focus({ preventScroll: false });
        else envoyerConnexion();
        return true;
      }
      case 'cs-cx-ok': envoyerConnexion(); return true;
      case 'cs-cx-non': O.son?.retour?.(); fermerConnexion(); return true;
      case 'cs-page-site': { const id = codeSurP(d.id) ? d.id : cnx?.site; if (cnx) cnx.mdp = ''; cnx = null; if (codeSurP(id)) { O.son?.ok?.(); O.action('cs-page-site', { site: id }); } return true; }
      case 'cs-fermer': O.son?.retour?.(); O.action('cs-fermer'); return true;
      case 'cs-ajouter-page': if (!el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('cs-ajouter-page'); } return true;
      case 'cs-deconnecter': O.son?.tic?.(); sitesConfirme = true; dessinerTout('pn:non'); hote.querySelector('.jm-r-sites .jm-suppr')?.scrollIntoView({ block: 'nearest', behavior: reduit() ? 'auto' : 'smooth' }); return true;
      case 'cs-decon-non': O.son?.retour?.(); sitesConfirme = false; dessinerTout('pn:decon'); return true;
      case 'cs-decon-oui': sitesConfirme = false; el.disabled = true; O.son?.ok?.(); O.action('cs-deconnecter'); return true;
      default: return false;
    }
  }

  // Ce qui se tape dans la carte de connexion : gardé ici seulement, jamais dans l'état
  function saisieSites(el, d) {
    if (d.perso !== 'cs-cx-champ' || !cnx || !['mail', 'mdp'].includes(d.champ)) return false;
    cnx[d.champ] = String(el.value);
    el.closest('.jm-champ')?.classList.remove('faux');
    const err = hote.querySelector('.jm-modale .jm-err'); if (err) err.textContent = '';
    return true;
  }

  function droiteRubrique(r, o) {
    const d = r ? r.dossier : o.dossier;
    if (d && typeof d === 'object' && d.reglage && typeof d.reglage === 'object') return reglagesHTML(d);
    if (d && typeof d === 'object' && typeof d.vue === 'string') { const h = droiteProfil(d); if (h !== null) return h; }
    return dossierVideHTML(d, false);
  }

  // Ce qui est montré change de dossier : ses petites choses ouvertes se referment
  function suivreFiche() {
    const f = fiche(), id = f ? f.id : null;
    if (id !== ficheVue) { ficheVue = id; pilule = 'envoye'; menu = false; rdv = null; confirme = false; if (cal?.ou === 'e') cal = null; }
    if (cal && !calOuvert(cal.ou)) cal = null;
    // (étape 7 : le mail à ranger montré est celui que Rust a ouvert ; plus aucun : le dossier revient)
    if (vueC === 'mail') { const m = objet(mailsDe().ouvert); if (numSur(m.id)) mailVu = m.id; else { vueC = 'dossier'; mailVu = null; } }
    if (choisie !== null && (id === choisie || !suiviDe()?.groupes?.some((g) => g?.lignes?.some((x) => x?.id === choisie)))) choisie = null;
  }

  function dessinerTout(focusK) {
    const o = ongletDe(onglet);
    if (!o) { hote.innerHTML = ''; return; }
    onglet = o.id;
    // (ce qui est en cours de frappe et la place dans les panneaux, gardés)
    const actif = document.activeElement, kActif = actif?.closest?.('#jm [data-k]')?.dataset.k;
    const curseur = actif && /^(INPUT|TEXTAREA)$/.test(actif.tagName) ? [actif.selectionStart, actif.selectionEnd] : null;
    const hautD = hote.querySelector('.jm-dossier')?.scrollTop || 0, hautL = hote.querySelector('.jm-liste')?.scrollTop || 0;
    const memeOnglet = hote.querySelector('#jm')?.dataset.onglet === o.id;
    let gauche, droite, barre = '', recap = '';
    const s = o.id === 'candidatures' ? suiviDe() : null;
    const x = o.id === 'offres' ? offresDe() : null;
    if (s) {
      suivreFiche();
      gauche = listeCandidaturesHTML(o, s);
      droite = droiteCandidatures(o, s);
    } else if (x) {
      suivreOffres(x);
      // (étape 5 bis : un site choisi, ses résultats lus en coulisse ; sinon mes offres gardées et leurs filtres)
      const site = sourceSite();
      // (étape 10 : le récapitulatif de l'envoi par mail prend toute la place sous les onglets)
      // (étape 11 : le récapitulatif de l'envoi sur le site, de même)
      const po = poOuvert();
      const e = !po && enOuvert() ? enDe() : null;
      if (po) {
        recap = poRecapHTML(po);
        const cle = `${po.offre}:${po.ouvert.phase}`;
        if (poVu !== cle) {
          poVu = cle;
          if (!focusK) focusK = po.ouvert.phase === 'parti' ? 'po:dossier' : (po.ouvert.phase === 'incertain' ? 'po:fait' : 'po:envoyer');
        }
      } else poVu = null;
      if (e) {
        recap = recapHTML(e);
        if (enVu !== e.offre || enFini !== enIssue(e)) {
          if (enVu !== e.offre) enSaisie = null;
          enVu = e.offre; enFini = enIssue(e);
          if (!focusK) focusK = enFini === 'parti' ? 'en:dossier' : (enSaisieDe(e).a ? 'en:envoyer' : 'en:a');
        }
      } else enVu = null;
      if (recap) { gauche = ''; droite = ''; } else if (site) {
        gauche = listeSitesHTML(x, site);
        droite = droiteSitesHTML(site);
      } else {
        barre = barreOffresHTML(x);
        gauche = listeOffresHTML(o, x);
        droite = droiteOffres(x);
      }
    } else {
      const r = rubriqueDe(o);
      gauche = listeHTML(o, r);
      droite = droiteRubrique(r, o);
    }
    hote.innerHTML = `<div class="jm${O.anime ? ' entree' : ''}" id="jm" data-onglet="${o.id}">
        <nav class="jm-onglets" role="tablist" aria-label="Jobmind">${ongletsHTML(o)}</nav>${recap ? `<div class="jm-envoi-zone">${recap}</div>` : `${barre}
        <div class="jm-corps${x ? ' offres' : ''}">
          <aside class="jm-liste${s || x ? ' suivi' : ''}" aria-label="${O.esc(texteSur(o.liste))}">${gauche}</aside>
          <section class="jm-dossier" aria-live="polite">${droite}</section>
        </div>`}
      </div>`;
    O.anime = false;
    // (étape 5 bis : la carte de connexion, ou la petite fenêtre du site, par-dessus l'onglet)
    cartesSites();
    remplirBoite();
    if (memeOnglet) {
      const d = hote.querySelector('.jm-dossier'), l = hote.querySelector('.jm-liste');
      if (d) d.scrollTop = hautD;
      if (l) l.scrollTop = hautL;
    }
    const k = focusK || kActif;
    const cible = k && hote.querySelector(`[data-k="${CSS.escape(k)}"]`);
    (cible || hote.querySelector('.jm-onglet.actif'))?.focus({ preventScroll: true });
    if (cible && curseur && /^(INPUT|TEXTAREA)$/.test(cible.tagName) && k === kActif) { try { cible.setSelectionRange(curseur[0], curseur[1]); } catch (_) { /* (rien) */ } }
  }

  // Montre un onglet (noté au journal par Rust) ; le choix reste sur l'onglet
  function montrer(id, focusK) {
    const o = ongletDe(id);
    if (!o) return;
    srcMenu = false;   // (« Où chercher » se referme)
    if (o.id !== onglet) {
      onglet = o.id;
      O.anime = !reduit();
      O.action?.('onglet', o.id);
    }
    dessinerTout(focusK || `o:${o.id}`);
  }

  // Ouvre une rubrique de l'onglet montré (Mon profil)
  function ouvrirRubrique(id, focusK) {
    const o = ongletDe(onglet);
    if (!o || !rubriquesDe(o).some((r) => r.id === id)) return;
    if (rubrique[o.id] !== id) { rubrique[o.id] = id; O.anime = !reduit(); }
    dessinerTout(focusK || `r:${id}`);
  }

  // ---------------------------------------------------------------- le suivi
  // Ouvre le dossier d'une candidature (Rust renvoie l'état avec son dossier)
  function ouvrirCandidature(id) {
    vueC = 'dossier';
    if (fiche()?.id !== id) { choisie = id; O.anime = !reduit(); O.action('c-ouvrir', { id }); }
    dessinerTout(`c:${id}`);
  }

  // Étape 7 : ouvre un mail à ranger à droite (Rust renvoie l'état avec son texte)
  function ouvrirMail(id) {
    if (vueC !== 'mail' || mailVu !== id) O.anime = !reduit();
    vueC = 'mail'; mailVu = id; menu = false; rdv = null; confirme = false; cal = null;
    if (objet(mailsDe().ouvert).id !== id) O.action('m-ouvrir', { id });
    dessinerTout(`ml:${id}`);
  }

  function ouvrirFormulaire(f) {
    const t = textesC();
    if (f) {
      const c = f.champs && typeof f.champs === 'object' ? f.champs : {};
      const base = Object.fromEntries([...CHAMPS, 'moyen'].map((k) => [k, texteSur(c[k])]));
      form = { id: f.id, base, v: modifs[f.id] ? { ...modifs[f.id] } : { ...base } };
    } else {
      form = { id: null, v: brouillon ? { ...brouillon } : { ...Object.fromEntries(CHAMPS.map((k) => [k, ''])), date: texteSur(t.aujourdhui), moyen: 'mail' } };
    }
    vueC = 'formulaire'; menu = false; rdv = null; confirme = false; cal = null;
    O.anime = !reduit();
    dessinerTout('f:entreprise');
  }

  // Ferme le formulaire ; une nouvelle candidature commencée est gardée en brouillon (sauf après l'enregistrement).
  // Une candidature existante changée puis fermée par Retour garde aussi ses changements : « Modifier » les retrouve.
  function fermerFormulaire(garder) {
    if (form && !form.id) brouillon = garder && CHAMPS.some((k) => k !== 'date' && String(form.v[k] || '').trim()) ? { ...form.v } : null;
    if (form?.id) {
      const change = [...CHAMPS, 'moyen'].some((k) => String(form.v[k] || '') !== String(form.base?.[k] || ''));
      if (garder && change) { modifs[form.id] = { ...form.v }; window.kit?.message?.('Tes changements sont gardés : « Modifier » les retrouve.'); } else delete modifs[form.id];
    }
    form = null; vueC = 'dossier'; cal = null;
    const f = fiche();
    dessinerTout(f ? `c:${f.id}` : 'c:ajouter');
  }

  function enregistrer() {
    if (!form) return;
    const err = hote.querySelector('.jm-err');
    for (const k of ['entreprise', 'poste']) {
      if (!String(form.v[k] || '').trim()) {
        const el = hote.querySelector(`[data-k="f:${k}"]`);
        el?.closest('.jm-champ')?.classList.add('faux');
        if (err) err.textContent = k === 'entreprise' ? "Écris le nom de l'entreprise." : 'Écris le poste.';
        el?.focus({ preventScroll: false });
        O.son?.retour?.();
        return;
      }
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    const v = Object.fromEntries([...CHAMPS, 'moyen'].map((k) => [k, String(form.v[k] || '').slice(0, LONGUEURS[k] || 30)]));
    if (form.id) v.id = form.id;
    O.son?.ok?.();
    O.action('c-enregistrer', v);
  }

  // Copie un texte (le presse-papiers de la page ; sinon l'ancienne façon), puis dit « Copié » sur le bouton
  function copierBrut(texte, el, apres) {
    const fini = () => {
      const s = el?.querySelector('span');
      if (s) { const avant = s.textContent; s.textContent = T('copie') || 'Copié'; el.classList.add('fait'); setTimeout(() => { if (s.isConnected) { s.textContent = avant; el.classList.remove('fait'); } }, 1600); }
      O.son?.ok?.();
      apres?.();
    };
    const ancienne = () => {
      const z = document.createElement('textarea'); z.value = texte; z.setAttribute('readonly', ''); z.style.cssText = 'position:fixed;left:-9999px;top:0;opacity:0';
      document.body.appendChild(z); z.select();
      let ok = false; try { ok = document.execCommand('copy'); } catch (_) { ok = false; }
      z.remove(); if (ok) fini();
      el?.focus({ preventScroll: true });
    };
    try { navigator.clipboard.writeText(texte).then(fini, ancienne); } catch (_) { ancienne(); }
  }
  // Copie un texte du dossier ouvert (noté au journal par Rust)
  function copier(texte, el, quoi) {
    copierBrut(texte, el, () => { const f = fiche(); if (f) O.action('c-copie', { id: f.id, quoi }); });
  }

  // Les notes tapées : envoyées en quittant le champ, si elles ont changé
  function garderNotes(el) {
    const f = fiche(); if (!f) return;
    const v = el.value;
    if (v === texteSur(f.notes)) { delete notes[f.id]; return; }
    notes[f.id] = v;
    O.action('c-notes', { id: f.id, notes: v.slice(0, 4000) });
  }

  // Un élément du suivi (clic, A ou Entrée) ; vrai s'il en était un
  function validerSuivi(el, d) {
    const f = fiche();
    switch (d.perso) {
      case 'cand': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); ouvrirCandidature(id); } return true; }
      case 'ajouter': O.son?.ok?.(); ouvrirFormulaire(null); return true;
      // Étape 12 : Mon bilan, l'export
      case 'bilan': O.son?.ok?.(); ouvrirBilan(); return true;
      case 'bilan-fermer': O.son?.retour?.(); vueC = 'dossier'; dessinerTout('c:bilan'); return true;
      case 'bx-csv': case 'bx-pdf': case 'rx-csv': case 'rx-pdf':
        if (!el.disabled) { O.son?.ok?.(); O.action(d.perso.endsWith('csv') ? 'x-csv' : 'x-pdf'); }
        return true;
      case 'rythme': { const h = Number(d.v); if (Number.isInteger(h) && h >= 0 && h <= 24) { O.son?.bascule?.(); O.action('r-rythme', { heures: h }); } return true; }
      case 'rs-sauver': if (!el.disabled) { O.son?.ok?.(); O.action('s-sauver'); } return true;
      case 'rs-restaurer': if (!el.disabled) { O.son?.ok?.(); O.action('s-restaurer'); } return true;
      case 'rs-oui': if (!el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('s-oui'); } return true;
      case 'rs-non': O.son?.retour?.(); O.action('s-non'); return true;
      case 'champ': {   // (Entrée dans un champ d'une ligne : le champ suivant)
        if (el.tagName !== 'INPUT') return true;
        const els = [...hote.querySelectorAll('.jm-form .f')], i = els.indexOf(el);
        els[i + 1]?.focus({ preventScroll: false });
        return true;
      }
      case 'moyen':
        if (!form || !codeSur(d.id)) return true;
        form.v.moyen = d.id; O.son?.bascule?.();
        el.parentElement.querySelectorAll('.jm-puce').forEach((b) => { const oui = b === el; b.classList.toggle('oui', oui); b.setAttribute('aria-checked', String(oui)); });
        return true;
      case 'enregistrer': enregistrer(); return true;
      case 'annuler': O.son?.retour?.(); fermerFormulaire(false); return true;
      case 'pilule': if (['envoye', 'offre', 'notes', 'mails'].includes(d.id) && pilule !== d.id) { pilule = d.id; O.son?.tic?.(); dessinerTout(`p:${d.id}`); } return true;
      // Étape 7 : les mails à ranger, et le statut proposé par une réponse
      case 'mail': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); ouvrirMail(id); } return true; }
      case 'm-choix': return true;
      case 'm-ranger': {
        const id = Number(d.id), k = Number(mrChoix[id] || objet(mailsDe().ouvert).suggestion);
        if (!numSur(id)) return true;
        if (!numSur(k)) { O.son?.retour?.(); hote.querySelector('[data-k="mr:choix"]')?.focus({ preventScroll: false }); return true; }
        if (!el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('m-ranger', { id, candidature: k }); }
        return true;
      }
      case 'm-rien': { const id = Number(d.id); if (numSur(id) && !el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('m-rien', { id }); } return true; }
      case 'm-oui': case 'm-non': { const id = Number(d.id); if (numSur(id) && !el.disabled) { el.disabled = true; O.son?.ok?.(); O.action(d.perso, { id }); } return true; }
      case 'notes': return true;
      case 'copier': {
        if (!f) return true;
        const t = d.quoi === 'lien' ? texteSur(f.offre?.lien) : texteSur(f.envoye?.texte);
        if (t) copier(t, el, d.quoi === 'lien' ? 'lien' : 'envoye');
        return true;
      }
      case 'act-copier': if (f && f.relance) copier(texteSur(f.relance), el, 'relance'); return true;
      case 'act-relancee': if (f) { O.son?.ok?.(); O.action('c-relancee', { id: f.id }); } return true;
      case 'act-sans': if (f) { O.son?.ok?.(); O.action('c-sans-reponse', { id: f.id }); } return true;
      case 'act-entretien':
        if (!f) return true;
        O.son?.ok?.(); rdv = { ...(f.entretien || {}) }; menu = false; dessinerTout('e:date'); return true;
      case 'statut-menu': if (!f) return true; menu = !menu; O.son?.tic?.(); dessinerTout(menu ? `m:${f.statut}` : 'd:statut'); return true;
      case 'statut-choix': {
        if (!f || !codeSur(d.id)) return true;
        menu = false;
        if (d.id === 'entretien') { O.son?.ok?.(); rdv = { ...(f.entretien || {}) }; dessinerTout('e:date'); return true; }
        O.son?.ok?.(); dessinerTout('d:statut');
        if (d.id !== f.statut) O.action('c-statut', { id: f.id, statut: d.id });
        return true;
      }
      case 'rdv': {   // (Entrée : le champ suivant, puis Enregistrer)
        const els = [...hote.querySelectorAll('.jm-rdv .f')], i = els.indexOf(el);
        els[i + 1]?.focus({ preventScroll: false });
        return true;
      }
      case 'rdv-ok':
        if (!f || !rdv) return true;
        O.son?.ok?.();
        O.action('c-statut', { id: f.id, statut: 'entretien', date: String(rdv.date || '').slice(0, 12), heure: String(rdv.heure || '').slice(0, 12), lieu: String(rdv.lieu || '').slice(0, 120) });
        return true;
      case 'rdv-non': O.son?.retour?.(); rdv = null; dessinerTout('d:statut'); return true;
      case 'modifier': if (f) { O.son?.ok?.(); ouvrirFormulaire(f); } return true;
      case 'supprimer': O.son?.ok?.(); confirme = true; dessinerTout('d:non'); hote.querySelector('.jm-suppr')?.scrollIntoView({ block: 'nearest' }); return true;
      case 'suppr-non': O.son?.retour?.(); confirme = false; dessinerTout('d:supprimer'); return true;
      case 'suppr-oui': if (f && confirme) { confirme = false; el.disabled = true; O.action('c-supprimer', { id: f.id }); } return true;
      case 'cal-ouvrir': {
        const ou = d.ou === 'e' ? 'e' : 'f';
        if (ou === 'e' ? !rdv : !form) return true;
        O.son?.tic?.();
        if (calOuvert(ou)) fermerCal(); else ouvrirCal(ou);
        return true;
      }
      case 'cal-mois': {
        if (!cal || !calOuvert(cal.ou)) return true;
        const sens = Number(d.v) < 0 ? -1 : 1, t = new Date(Date.UTC(cal.a, cal.m + sens, 1));
        cal = { ou: cal.ou, a: t.getUTCFullYear(), m: t.getUTCMonth() };
        O.son?.tic?.();
        const k = el.dataset.k;
        dessinerTout(k);
        // (au bout des bornes, la flèche n'est plus là : le choix va sur le premier jour du mois)
        if (!hote.querySelector(`[data-k="${CSS.escape(k)}"]`)) hote.querySelector('.jm-cal .jm-cal-d.f, .jm-cal .f')?.focus({ preventScroll: true });
        return true;
      }
      case 'cal-jour': {
        const ou = d.ou === 'e' ? 'e' : 'f', { min, max } = bornesCal(ou), iso = String(d.v || '');
        if (!calOuvert(ou) || !ISO.test(iso) || iso < min || iso > max) return true;
        if (ou === 'e') rdv.date = saisieDe(iso);
        else {
          form.v.date = saisieDe(iso);
          const err = hote.querySelector('.jm-err'); if (err) err.textContent = '';
        }
        O.son?.ok?.();
        cal = null;
        dessinerTout(`${prefixe(ou)}:cal`);
        return true;
      }
      case 'cal-fermer': O.son?.retour?.(); if (cal) fermerCal(); return true;
      case 'delai': { const n = Number(d.v); if (Number.isInteger(n) && !el.disabled) { O.son?.bascule?.(); O.action('reglage-relance', n); } return true; }
      default: return validerProfil(el, d);
    }
  }

  // Ce qui se tape : gardé dans la page (un nouvel état qui arrive ne l'efface pas)
  function surSaisie(e) {
    const el = e.target, d = el.dataset || {};
    // (étape 7 : la candidature choisie pour un mail à ranger)
    if (d.perso === 'm-choix') { const id = Number(d.id), k = Number(el.value); if (numSur(id)) mrChoix[id] = numSur(k) ? k : 0; return; }
    if (d.perso === 'champ' && form && CHAMPS.includes(d.champ)) {
      form.v[d.champ] = el.value;
      el.closest('.jm-champ')?.classList.remove('faux');
      const err = hote.querySelector('.jm-err'); if (err) err.textContent = '';
      if (d.champ === 'date') suivreCal('f');
    } else if (d.perso === 'rdv' && rdv && ['date', 'heure', 'lieu'].includes(d.champ)) {
      rdv[d.champ] = el.value;
      if (d.champ === 'date') suivreCal('e');
    } else if (d.perso === 'notes') {
      const f = fiche(); if (f) notes[f.id] = el.value;
    } else if (d.perso === 'p-champ') {
      saisieProfil(el, d);
    } else if (d.perso === 'cs-cx-champ') {
      saisieSites(el, d);
    } else if (d.perso === 'pr-zone') {
      if (prEdition) prEdition.texte = el.value;
    } else if (d.perso === 'pr-consigne') {
      prConsigne = el.value;
    } else if (d.perso === 'po-champ') {
      const p = poDe(), i = Number(d.i); if (p && Number.isInteger(i)) { poSaisieDe(p).v[i] = el.value; poBas(); }
    } else if (d.perso === 'en-champ' && ['a', 'objet'].includes(d.champ)) {
      const e = enDe(); if (e) { enSaisieDe(e)[d.champ] = el.value; }
    } else if (typeof d.perso === 'string' && d.perso.startsWith('of-')) {
      saisieOffres(el, d);
    }
  }

  // Dans une zone de texte, Entrée (retour à la ligne), Retour arrière, gauche, droite, début et fin restent à la zone
  // (le kit les prendrait pour la manette). Haut et bas aussi, sauf au bord : le haut au début du texte (ou sur la
  // première ligne quand le curseur ne peut plus monter) et le bas à la fin (ou sur la dernière ligne) passent à
  // l'élément d'au-dessus ou d'en dessous. Échap sort d'abord de la zone (« zone libre » : elle reste choisie, les
  // flèches et la croix de la manette passent à côté, Entrée ou une lettre y fait retaper) ; un second Échap fait
  // l'action habituelle (Retour). Pendant qu'on tape, la souris ne déplace pas le choix.
  const sorties = new WeakSet();   // les touches renvoyées au kit pour sortir d'une zone (à laisser passer)
  function libre(z, oui) {
    z.classList.toggle('jm-libre', oui);
    if (oui) { try { z.setSelectionRange(z.selectionEnd, z.selectionEnd); } catch (_) { /* (rien) */ } }
  }
  function sortirZone(z, cle) {
    if (document.activeElement !== z) return;
    const e = new KeyboardEvent('keydown', { key: cle, bubbles: true, cancelable: true });
    sorties.add(e); z.dispatchEvent(e);
  }
  function toucheZone(e) {
    const z = e.target;
    if (z.classList.contains('jm-libre')) {
      // (zone libre : Entrée y fait retaper, les flèches et Échap vont au kit, ce qui s'écrit fait retaper)
      if (e.key === 'Enter') { e.preventDefault(); e.stopPropagation(); libre(z, false); return; }
      if (e.key.startsWith('Arrow') || e.key === 'Escape' || e.key === 'Tab' || e.ctrlKey || e.altKey || e.metaKey) return;
      if (e.key.length === 1 || e.key === 'Backspace' || e.key === 'Delete') { libre(z, false); e.stopPropagation(); }
      return;
    }
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); libre(z, true); O.son?.tic?.(); return; }
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.stopPropagation();
      if (e.shiftKey || z.selectionStart !== z.selectionEnd) return;
      const haut = e.key === 'ArrowUp', pos = z.selectionStart, v = z.value;
      if (haut ? pos === 0 : pos >= v.length) { e.preventDefault(); sortirZone(z, e.key); return; }
      // (sur la première ou la dernière ligne : si le curseur ne bouge pas, c'est le bord de la zone)
      if (haut ? v.lastIndexOf('\n', pos - 1) < 0 : v.indexOf('\n', pos) < 0) setTimeout(() => { if (z.selectionStart === pos && z.selectionEnd === pos) sortirZone(z, e.key); });
      return;
    }
    if (['ArrowLeft', 'ArrowRight', 'Enter', 'Backspace', 'Home', 'End'].includes(e.key)) e.stopPropagation();
  }
  function surTouche(e) {
    if (sorties.has(e)) return;
    toucheSources(e);
    if (e.target.tagName === 'TEXTAREA') toucheZone(e);
    // (une liste à choisir : ses flèches choisissent dans la liste)
    if (e.target.tagName === 'SELECT' && ['ArrowUp', 'ArrowDown', 'Home', 'End', 'Enter', 'Backspace'].includes(e.key)) e.stopPropagation();
  }
  function surSurvol(e) {
    const a = document.activeElement;
    if (a?.tagName === 'TEXTAREA' && !a.classList.contains('jm-libre') && a !== e.target) e.stopPropagation();
  }
  function surSortie(e) {
    const el = e.target;
    if (el.tagName === 'TEXTAREA') libre(el, false);
    if (el.dataset?.perso === 'notes') garderNotes(el);
  }
  // Un clic dans le champ d'une date ouvre aussi son calendrier (le curseur reste dans le champ)
  function surClic(e) {
    const el = e.target, d = el.dataset || {};
    if (el.tagName === 'TEXTAREA' && el.classList.contains('jm-libre')) libre(el, false);
    if (el.tagName !== 'INPUT' || d.champ !== 'date' || (d.perso !== 'champ' && d.perso !== 'rdv')) return;
    const ou = d.perso === 'rdv' ? 'e' : 'f';
    if ((ou === 'e' ? rdv : form) && !calOuvert(ou)) ouvrirCal(ou, d.k);
  }
  let ecoute = null;
  function ecouter(h) {
    if (ecoute === h) return;
    ecoute = h;
    h.addEventListener('click', surClic);
    h.addEventListener('input', surSaisie);
    h.addEventListener('keydown', surTouche);
    h.addEventListener('pointerover', surSurvol);
    // (« Où chercher » ouverte : un clic à côté la referme)
    h.addEventListener('pointerdown', (e) => { if (srcMenu && !e.target.closest?.('.jm-srcs')) { srcMenu = false; dessinerTout('cs:ou'); } });
    h.addEventListener('focusout', surSortie);
    h.addEventListener('submit', (e) => e.preventDefault());
  }

  // ---------------------------------------------------------------- Mon profil (étape 3)
  // Les rubriques Mon CV, Ce que je cherche, Mon style et Mes clés. Ce qui se tape reste ici tant que ce n'est pas
  // enregistré (un nouvel état ne l'efface pas) ; chaque envoi passe par le kit, et Rust vérifie tout (src\profil.rs).
  // Une clé tapée n'est gardée ici que le temps de l'envoyer : jamais dans l'état, jamais redessinée.
  const P_LONGUEURS = { nom: 80, texte: 30000, postes: 800, ville: 60, salaire: 12, titre: 80, stexte: 8000, cle: 300 };
  let cvMode = 'voir';     // Mon CV : 'voir' (le CV ouvert) ou 'coller' (un nouveau CV collé)
  const cvEdits = {};      // le nom et le texte corrigés, par CV : { nom, texte }
  let cvColle = null;      // le CV en train d'être collé : { nom, texte }
  let cvConfirme = false;  // « Supprimer ce CV ? »
  let rch = null;          // Ce que je cherche, en cours de saisie (une copie des valeurs reçues)
  let styleForm = null;    // le texte en train d'être ajouté : { titre, texte }
  let styleConfirme = null; // le texte dont la suppression est demandée
  let cleForm = null;      // le service dont la clé se tape : 'adzuna' ou 'claude'
  let cleSaisie = {};      // ce qui est tapé dans ce formulaire (vidé dès l'envoi ou l'annulation)
  let cleConfirme = null;  // le service dont l'effacement est demandé
  const objet = (x) => (x && typeof x === 'object' && !Array.isArray(x) ? x : {});
  const liste = (x) => (Array.isArray(x) ? x : []);
  const tx = (d) => { const t = objet(d.textes); return (k) => texteSur(t[k]); };
  const deptSur = (t) => typeof t === 'string' && /^(\d{2,3}|2[AB])$/.test(t);
  const codeSurP = (t) => typeof t === 'string' && /^[a-z0-9_]{1,40}$/.test(t);
  const anime = () => (O.anime ? ' entree' : '');
  const pChamp = (zone, champ, k, val, attrs = '') => `data-perso="p-champ" data-zone="${zone}" data-champ="${champ}" data-k="${k}"${attrs} value="${O.esc(val || '')}"`;
  const rubriqueOuverte = (id) => onglet === 'profil' && rubriqueDe(ongletDe('profil'))?.id === id;

  // -------------------------------------------- Mon CV
  function cvVideHTML(d, imp) {
    const { esc } = O, t = tx(d), v = objet(d.vide);
    return `<div class="jm-vide${anime()}">
        <div class="jm-v-ic plasma-bord">${icone('cv')}</div>
        <h1>${esc(texteSur(v.titre))}</h1><p>${esc(texteSur(v.texte))}</p>
        ${imp ? attenteHTML(imp) : ''}
        <div class="jm-btns centre">${bouton('cv-importer', 'cv:importer', t('importer'), 'importer', 'principal grand', imp ? ' disabled' : '')}${bouton('cv-coller', 'cv:coller', t('coller'), 'coller', 'second grand')}</div>
        <p class="jm-note">${esc(t('formats'))} ${esc(texteSur(v.note))}</p></div>`;
  }

  const attenteHTML = (imp) => `<div class="jm-attente" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${O.esc(texteSur(imp.texte))}</span></div>`;

  function cvCollerHTML(d) {
    const { esc } = O, t = tx(d), v = cvColle || { nom: '', texte: '' };
    return `<form class="jm-p${anime()}" novalidate><h1>${esc(t('coller_titre'))}</h1><p class="jm-intro">${esc(t('coller_intro'))}</p>
      <div class="jm-grille un">
        <label class="jm-champ"><span>${esc(t('nom'))}</span><input class="f" ${pChamp('colle', 'nom', 'cv:nom', v.nom, ` maxlength="${P_LONGUEURS.nom}" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_nom'))}"`)}></label>
        <label class="jm-champ"><span>${esc(t('texte'))}</span><textarea class="f jm-cv-texte" data-perso="p-champ" data-zone="colle" data-champ="texte" data-k="cv:texte" maxlength="${P_LONGUEURS.texte}" spellcheck="true" placeholder="${esc(t('i_texte'))}">${esc(v.texte || '')}</textarea></label></div>
      <p class="jm-err" role="alert"></p>
      <div class="jm-btns">${bouton('cv-enregistrer', 'cv:enreg', t('enregistrer'), 'coche', 'principal')}${bouton('cv-annuler', 'cv:annuler', t('annuler'), '', 'second')}</div></form>`;
  }

  function cvHTML(d) {
    const { esc } = O, t = tx(d);
    const cvs = liste(d.cvs).filter((c) => numSur(c?.id));
    const imp = d.import && typeof d.import === 'object' ? d.import : null;
    if (cvMode === 'coller') return cvCollerHTML(d);
    if (!cvs.length) return cvVideHTML(d, imp);
    const c = objet(d.ouvert), plein = d.plein === true;
    let h = `<div class="jm-p${anime()}"><h1>${esc(texteSur(d.titre))}</h1><p class="jm-intro">${esc(texteSur(d.intro))}</p>
      <div class="jm-outils">${bouton('cv-importer', 'cv:importer', t('importer'), 'importer', '', imp || plein ? ' disabled' : '')}${bouton('cv-coller', 'cv:coller', t('coller'), 'coller', '', plein ? ' disabled' : '')}<span>${esc(plein ? t('plein') : t('formats'))}</span></div>`;
    if (imp) h += attenteHTML(imp);
    if (cvs.length > 1) {
      h += `<div class="jm-cvs" role="tablist" aria-label="${esc(t('mes_cv'))}">${cvs.map((x) => {
        const oui = x.id === c.id;
        return `<button type="button" role="tab" aria-selected="${oui}" class="jm-cvc f${oui ? ' sel' : ''}" data-perso="cv-ouvrir" data-id="${x.id}" data-k="cvl:${x.id}">${x.principal === true ? icone('etoile') : ''}<b>${esc(texteSur(x.nom))}</b><span>${esc(texteSur(x.sous))}</span></button>`;
      }).join('')}</div>`;
    }
    if (!numSur(c.id)) return `${h}</div>`;
    const e = Object.hasOwn(cvEdits, c.id) ? cvEdits[c.id] : { nom: texteSur(c.nom), texte: texteSur(c.texte) };
    h += `<div class="jm-cv"><div class="jm-cv-tete"><span class="jm-a-ic">${icone('cv')}</span><div class="jm-cv-t"><b>${esc(texteSur(c.nom))}</b><span>${esc([texteSur(c.sous), texteSur(c.mots)].filter(Boolean).join(', '))}</span></div>
        ${c.principal === true ? `<span class="jm-badge">${icone('etoile')}${esc(t('principal'))}</span>` : bouton('cv-principal', 'cv:principal', t('rendre_principal'), 'etoile', 'discret')}</div>
      <p class="jm-petit">${esc(texteSur(c.fichier))}</p>
      ${c.sans_texte === true ? `<p class="jm-alerte">${icone('attention')}<span>${esc(t('sans_texte'))}</span></p>` : ''}
      <div class="jm-grille un">
        <label class="jm-champ"><span>${esc(t('nom'))}</span><input class="f" ${pChamp('cv', 'nom', 'cv:nom', e.nom, ` maxlength="${P_LONGUEURS.nom}" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_nom'))}"`)}></label>
        <label class="jm-champ"><span>${esc(t('texte'))}</span><textarea class="f jm-cv-texte" data-perso="p-champ" data-zone="cv" data-champ="texte" data-k="cv:texte" maxlength="${P_LONGUEURS.texte}" spellcheck="true" placeholder="${esc(t('i_texte'))}">${esc(e.texte)}</textarea></label></div>
      <p class="jm-petit">${esc(t('aide'))}</p>
      <p class="jm-err" role="alert"></p>
      <div class="jm-btns">${bouton('cv-enregistrer', 'cv:enreg', t('enregistrer'), 'coche', 'principal')}</div>`;
    h += cvConfirme
      ? `<div class="jm-suppr"><p>${esc(texteSur(c.question))}</p><div class="jm-btns">${bouton('cv-suppr-oui', 'cv:oui', t('oui'), 'poubelle', 'danger')}${bouton('cv-suppr-non', 'cv:non', t('non'), '', 'second')}</div></div>`
      : `<div class="jm-d-bas">${bouton('cv-supprimer', 'cv:suppr', t('supprimer'), 'poubelle', 'discret danger')}</div>`;
    return `${h}</div></div>`;
  }

  // Le CV montré (pour l'enregistrer, le supprimer...)
  const cvDossier = () => objet(rubriquesDe(ongletDe('profil')).find((r) => r.id === 'cv')?.dossier);
  const styleDossier = () => objet(rubriquesDe(ongletDe('profil')).find((r) => r.id === 'style')?.dossier);

  function cvEnregistrer() {
    const d = cvDossier(), err = hote.querySelector('.jm-err');
    const colle = cvMode === 'coller', c = objet(d.ouvert);
    const v = colle ? (cvColle || {}) : (Object.hasOwn(cvEdits, c.id) ? cvEdits[c.id] : { nom: texteSur(c.nom), texte: texteSur(c.texte) });
    for (const k of ['nom', 'texte']) {
      if (!String(v[k] || '').trim()) {
        hote.querySelector(`[data-k="cv:${k}"]`)?.closest('.jm-champ')?.classList.add('faux');
        if (err) err.textContent = k === 'nom' ? 'Donne un nom à ce CV.' : 'Colle le texte de ton CV.';
        hote.querySelector(`[data-k="cv:${k}"]`)?.focus({ preventScroll: false });
        O.son?.retour?.();
        return;
      }
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    const m = { nom: String(v.nom).slice(0, P_LONGUEURS.nom), texte: String(v.texte).slice(0, P_LONGUEURS.texte) };
    if (!colle && numSur(c.id)) m.id = c.id;
    O.son?.ok?.();
    O.action('cv-enregistrer', m);
  }

  // -------------------------------------------- Ce que je cherche
  function rchDe(d) {
    if (!rch) {
      const v = objet(d.valeurs);
      rch = {
        postes: texteSur(v.postes), lieu: codeSurP(v.lieu) ? v.lieu : 'ville', ville: texteSur(v.ville),
        rayon: Number.isInteger(v.rayon) ? v.rayon : 30, departement: deptSur(v.departement) ? v.departement : '',
        region: codeSurP(v.region) ? v.region : '', contrats: liste(v.contrats).filter(codeSurP),
        teletravail: codeSurP(v.teletravail) ? v.teletravail : 'peu_importe', sans_diplome: v.sans_diplome === true,
        debutant: v.debutant === true, salaire: texteSur(v.salaire),
      };
    }
    return rch;
  }

  function puce(perso, k, nom, oui, extra = '', role = 'radio') {
    const etat = role === 'radio' ? `role="radio" aria-checked="${oui}"` : `aria-pressed="${oui}"`;
    return `<button type="button" ${etat} class="jm-puce f${oui ? ' oui' : ''}" data-perso="${perso}" data-k="${k}"${extra}>${role === 'radio' ? '' : icone(oui ? 'coche' : 'plus')}${O.esc(texteSur(nom))}</button>`;
  }

  function rechercheHTML(d) {
    const { esc } = O, t = tx(d), r = rchDe(d);
    const choixListe = (champ, k, l, val, sur) => `<select class="f" data-perso="p-champ" data-zone="rc" data-champ="${champ}" data-k="${k}" aria-label="${esc(t(champ))}">
        <option value="">${esc(t('choisir'))}</option>${liste(l).filter((x) => sur(x?.id)).map((x) => `<option value="${x.id}"${x.id === val ? ' selected' : ''}>${esc(texteSur(x.nom))}</option>`).join('')}</select>`;
    const lieux = liste(d.lieux).filter((x) => codeSurP(x?.id));
    let ou = '';
    if (r.lieu === 'ville') {
      ou = `<label class="jm-champ"><span>${esc(t('ville'))}</span><input class="f" ${pChamp('rc', 'ville', 'rc:ville', r.ville, ` maxlength="${P_LONGUEURS.ville}" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_ville'))}"`)}></label>
        <div class="jm-champ"><span>${esc(t('rayon'))}</span><div class="jm-puces" role="radiogroup" aria-label="${esc(t('rayon'))}">${liste(d.rayons).filter(Number.isInteger).map((n) => puce('rc-rayon', `rc:r-${n}`, `${n} ${t('km')}`, n === r.rayon, ` data-v="${n}"`)).join('')}</div></div>`;
    } else if (r.lieu === 'departement') {
      ou = `<label class="jm-champ large"><span>${esc(t('departement'))}</span>${choixListe('departement', 'rc:dep', d.departements, r.departement, deptSur)}</label>`;
    } else if (r.lieu === 'region') {
      ou = `<label class="jm-champ large"><span>${esc(t('region'))}</span>${choixListe('region', 'rc:reg', d.regions, r.region, codeSurP)}</label>`;
    }
    const contrats = liste(d.liste_contrats).filter((x) => codeSurP(x?.id));
    const teles = liste(d.liste_teletravail).filter((x) => codeSurP(x?.id));
    return `<form class="jm-p${anime()}" novalidate><h1>${esc(texteSur(d.titre))}</h1><p class="jm-intro">${esc(texteSur(d.intro))}</p>
      ${d.enregistree === true ? `<p class="jm-deja">${icone('coche')}<span>${esc(t('deja'))}</span></p>` : ''}
      <div class="jm-grille">
        <label class="jm-champ large"><span>${esc(t('postes'))}</span><textarea class="f petit" data-perso="p-champ" data-zone="rc" data-champ="postes" data-k="rc:postes" maxlength="${P_LONGUEURS.postes}" spellcheck="true" placeholder="${esc(t('i_postes'))}">${esc(r.postes)}</textarea><small class="jm-aide">${esc(t('postes_aide'))}</small></label>
        <div class="jm-champ large"><span>${esc(t('lieu'))}</span><div class="jm-puces" role="radiogroup" aria-label="${esc(t('lieu'))}">${lieux.map((x, i) => puce('rc-lieu', `rc:l${i}`, x.nom, x.id === r.lieu, ` data-id="${x.id}"`)).join('')}</div></div>
        ${ou}
        <div class="jm-champ large"><span>${esc(t('contrats'))}</span><div class="jm-puces">${contrats.map((x, i) => puce('rc-contrat', `rc:c${i}`, x.nom, r.contrats.includes(x.id), ` data-id="${x.id}"`, 'bascule')).join('')}</div><small class="jm-aide">${esc(t('contrats_aide'))}</small></div>
        <div class="jm-champ large"><span>${esc(t('teletravail'))}</span><div class="jm-puces" role="radiogroup" aria-label="${esc(t('teletravail'))}">${teles.map((x, i) => puce('rc-tele', `rc:t${i}`, x.nom, x.id === r.teletravail, ` data-id="${x.id}"`)).join('')}</div></div>
        <div class="jm-champ large"><span>${esc(t('avant'))}</span><div class="jm-puces">${puce('rc-case', 'rc:diplome', t('sans_diplome'), r.sans_diplome, ' data-id="sans_diplome"', 'bascule')}${puce('rc-case', 'rc:debutant', t('debutant'), r.debutant, ' data-id="debutant"', 'bascule')}</div></div>
        <label class="jm-champ"><span>${esc(t('salaire'))} <small>${esc(t('facultatif'))}</small></span><input class="f" inputmode="numeric" ${pChamp('rc', 'salaire', 'rc:salaire', r.salaire, ` maxlength="${P_LONGUEURS.salaire}" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_salaire'))}"`)}><small class="jm-aide">${esc(t('salaire_aide'))}</small></label>
      </div>
      <p class="jm-err" role="alert"></p>
      <div class="jm-btns">${bouton('rc-enregistrer', 'rc:enreg', t('enregistrer'), 'coche', 'principal')}</div></form>`;
  }

  function rechercheEnregistrer() {
    if (!rch) return;
    const err = hote.querySelector('.jm-err');
    // (sans poste : tous les métiers du lieu choisi ; ni poste ni lieu : refusé, comme dans Rust)
    if (!String(rch.postes || '').trim() && rch.lieu === 'partout') {
      hote.querySelector('[data-k="rc:postes"]')?.closest('.jm-champ')?.classList.add('faux');
      if (err) err.textContent = 'Écris un poste, ou choisis un lieu.';
      hote.querySelector('[data-k="rc:postes"]')?.focus({ preventScroll: false });
      O.son?.retour?.();
      return;
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    O.son?.ok?.();
    O.action('recherche-enregistrer', {
      postes: String(rch.postes).slice(0, P_LONGUEURS.postes), lieu: rch.lieu, ville: String(rch.ville).slice(0, P_LONGUEURS.ville),
      rayon: rch.rayon, departement: rch.departement, region: rch.region, contrats: rch.contrats.slice(0, 20),
      teletravail: rch.teletravail, sans_diplome: rch.sans_diplome, debutant: rch.debutant, salaire: String(rch.salaire).slice(0, P_LONGUEURS.salaire),
    });
  }

  // -------------------------------------------- Mon style
  // Deux pilules (étape 9) : « Mes textes » (étape 3) et « Mes règles » : le plancher professionnel (montré, il ne se
  // coupe pas), la longueur des phrases, l'intensité par défaut, ses expressions à éviter, les formules toutes faites
  // et les mots trop recherchés (coupés ou remis), « Tout remettre comme au départ ». Rust vérifie tout (src\style.rs).
  let stPil = 'textes';                         // la pilule de Mon style : 'textes' ou 'regles'
  let stAjout = { motif: '', remplacement: '' }; // l'expression en train d'être ajoutée
  let stZero = false;                           // « Tout remettre comme au départ ? »

  function pilulesStyleHTML(d) {
    const { esc } = O, r = objet(d.regles), t = (k) => texteSur(objet(r.textes)[k]);
    if (!Object.keys(r).length) return '';
    return `<div class="jm-pilules jm-st-pil" role="tablist">${[['textes', t('p_textes')], ['regles', t('p_regles')]].map(([id, nom]) => `<button type="button" role="tab" aria-selected="${stPil === id}" class="jm-pil f${stPil === id ? ' actif' : ''}" data-perso="sty-pil" data-id="${id}" data-k="sp:${id}">${esc(nom)}</button>`).join('')}</div>`;
  }

  function reglesHTML(d) {
    const { esc } = O, r = objet(d.regles), t = (k) => texteSur(objet(r.textes)[k]);
    const max = Number.isInteger(r.phrase_max) ? r.phrase_max : 25;
    let h = `<div class="jm-p${anime()}">${pilulesStyleHTML(d)}<h1>${esc(t('titre'))}</h1><p class="jm-intro">${esc(t('intro'))}</p>`;
    h += `<h2 class="jm-h2">${esc(t('plancher_t'))}</h2><p class="jm-petit">${esc(t('plancher_s'))}</p><ul class="jm-plancher">${liste(r.plancher).map((x) => `<li>${icone('cadenas')}<span>${esc(texteSur(x))}</span></li>`).join('')}</ul>`;
    h += `<h2 class="jm-h2">${esc(t('longueur_t'))}</h2><div class="jm-st-long">${bouton('sty-moins', 'sr:moins', t('moins'), 'gauche', 'second', max <= r.phrase_min ? ' disabled' : '')}<span><b>${max}</b> ${esc(t('longueur'))}</span>${bouton('sty-plus', 'sr:plus', t('plus'), 'droite', 'second', max >= r.phrase_plus ? ' disabled' : '')}</div>`;
    const ints = liste(r.intensites).filter((x) => codeSur(x?.code));
    const ic = ints.find((x) => x.code === r.intensite);
    h += `<h2 class="jm-h2">${esc(t('intensite_t'))}</h2><div class="jm-segments" role="radiogroup">${ints.map((x) => `<button type="button" role="radio" aria-checked="${x.code === r.intensite}" class="jm-seg f${x.code === r.intensite ? ' actif' : ''}" data-perso="sty-int" data-id="${x.code}" data-k="sr:i-${x.code}">${esc(texteSur(x.nom))}</button>`).join('')}</div>${ic ? `<p class="jm-petit">${esc(texteSur(ic.texte))}</p>` : ''}`;
    for (const g of liste(r.groupes)) {
      const l = liste(g?.liste);
      const actives = l.filter((x) => x?.active === true).length;
      h += `<h2 class="jm-h2">${esc(texteSur(g?.titre))}${l.length ? ` <small>${actives} / ${l.length}</small>` : ''}</h2>`;
      if (g?.id === 'perso') {
        h += `<div class="jm-st-ajout"><label class="jm-champ"><span>${esc(t('motif'))}</span><input class="f" ${pChamp('sr', 'motif', 'sr:motif', stAjout.motif, ` maxlength="60" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_motif'))}"`)}></label>
          <label class="jm-champ"><span>${esc(t('remplacement'))} <small>${esc(t('facultatif'))}</small></span><input class="f" ${pChamp('sr', 'remplacement', 'sr:rempl', stAjout.remplacement, ` maxlength="60" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_remplacement'))}"`)}></label>
          ${bouton('sty-ajouter', 'sr:ajouter', t('ajouter'), 'plus', 'principal')}</div><p class="jm-err" role="alert"></p>`;
        if (!l.length) h += `<p class="jm-petit">${esc(t('aucune_perso'))}</p>`;
      }
      h += `<div class="jm-regles">${l.map((x) => {
        const perso = numSur(x?.id);
        const cle = perso ? `data-id="${x.id}"` : `data-code="${esc(texteSur(x?.code))}"`;
        const k = perso ? `sr:p-${x.id}` : `sr:${esc(texteSur(x?.code))}`;
        const sous = texteSur(x?.sous);
        const supprimer = perso ? bouton('sty-supprimer', `sr:s-${x.id}`, t('supprimer'), 'poubelle', 'discret danger', ` data-id="${x.id}"`) : '';
        return `<div class="jm-regle${x?.active === true ? '' : ' coupee'}"><div class="jm-regle-t"><b>${esc(texteSur(x?.motif))}</b><span>${esc(sous)}</span></div>${supprimer}<button type="button" role="switch" aria-checked="${x?.active === true}" class="jm-inter f${x?.active === true ? ' oui' : ''}" data-perso="sty-regle" ${cle} data-k="${k}"><i></i><span>${esc(x?.active === true ? t('active') : t('coupee'))}</span></button></div>`;
      }).join('')}</div>`;
    }
    h += stZero
      ? `<div class="jm-suppr"><p>${esc(t('zero_q'))}</p><div class="jm-btns">${bouton('sty-zero-oui', 'sr:oui', t('oui'), 'recharger', 'danger')}${bouton('sty-zero-non', 'sr:non', t('non'), '', 'second')}</div></div>`
      : `<div class="jm-d-bas">${bouton('sty-zero', 'sr:zero', t('zero'), 'recharger', 'discret')}</div>`;
    return `${h}</div>`;
  }

  function styleHTML(d) {
    const { esc } = O, t = tx(d), v = objet(d.vide);
    const l = liste(d.liste).filter((x) => numSur(x?.id)), ouvert = objet(d.ouvert), plein = d.plein === true;
    if (stPil === 'regles' && !styleForm && Object.keys(objet(d.regles)).length) return reglesHTML(d);
    if (styleForm) {
      return `<form class="jm-p${anime()}" novalidate><h1>${esc(t('nouveau'))}</h1><p class="jm-intro">${esc(texteSur(d.intro))}</p>
        <div class="jm-grille un">
          <label class="jm-champ"><span>${esc(t('titre'))} <small>${esc(t('facultatif'))}</small></span><input class="f" ${pChamp('st', 'titre', 'st:titre', styleForm.titre, ` maxlength="${P_LONGUEURS.titre}" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_titre'))}"`)}></label>
          <label class="jm-champ"><span>${esc(t('texte'))}</span><textarea class="f jm-cv-texte moyen" data-perso="p-champ" data-zone="st" data-champ="texte" data-k="st:texte" maxlength="${P_LONGUEURS.stexte}" spellcheck="true" placeholder="${esc(t('i_texte'))}">${esc(styleForm.texte || '')}</textarea></label></div>
        <p class="jm-err" role="alert"></p>
        <div class="jm-btns">${bouton('st-enregistrer', 'st:enreg', t('enregistrer'), 'coche', 'principal')}${bouton('st-annuler', 'st:annuler', t('annuler'), '', 'second')}</div></form>`;
    }
    if (!l.length) {
      return `<div class="jm-vide${anime()}">${pilulesStyleHTML(d)}<div class="jm-v-ic plasma-bord">${icone('plume')}</div><h1>${esc(texteSur(v.titre))}</h1>
        <p>${esc(texteSur(d.intro))}</p>${bouton('st-ajouter', 'st:ajouter', t('ajouter'), 'plus', 'principal grand')}<p class="jm-note">${esc(texteSur(d.conseil))} ${esc(texteSur(d.note))}</p></div>`;
    }
    let h = `<div class="jm-p${anime()}">${pilulesStyleHTML(d)}<h1>${esc(texteSur(d.titre))}</h1><p class="jm-intro">${esc(texteSur(d.intro))}</p>
      <div class="jm-outils">${bouton('st-ajouter', 'st:ajouter', t('ajouter'), 'plus', '', plein ? ' disabled' : '')}<span>${esc(plein ? t('plein') : texteSur(d.conseil))}</span></div><div class="jm-textes">`;
    for (const x of l) {
      const oui = x.id === ouvert.id;
      h += `<div class="jm-texte${oui ? ' ouvert' : ''}"><button type="button" class="jm-texte-t f" data-perso="st-ouvrir" data-id="${x.id}" data-k="st:${x.id}" aria-expanded="${oui}"><span class="jm-texte-n"><b>${esc(texteSur(x.titre))}</b><span>${esc(texteSur(x.sous))}</span></span>${icone(oui ? 'bas' : 'droite')}</button>`;
      if (oui) {
        h += `<div class="jm-lettre petit" data-brut>${esc(texteSur(ouvert.texte))}</div>`;
        h += styleConfirme === x.id
          ? `<div class="jm-suppr"><p>${esc(texteSur(ouvert.question))}</p><div class="jm-btns">${bouton('st-suppr-oui', 'st:oui', t('oui'), 'poubelle', 'danger')}${bouton('st-suppr-non', 'st:non', t('non'), '', 'second')}</div></div>`
          : `<div class="jm-d-bas">${bouton('st-supprimer', `sts:${x.id}`, t('supprimer'), 'poubelle', 'discret danger', ` data-id="${x.id}"`)}</div>`;
      } else {
        h += `<p class="jm-extrait">${esc(texteSur(x.extrait))}</p>`;
      }
      h += '</div>';
    }
    return `${h}</div><p class="jm-note gauche">${esc(texteSur(d.note))}</p></div>`;
  }

  function styleEnregistrer() {
    if (!styleForm) return;
    const err = hote.querySelector('.jm-err');
    if (!String(styleForm.texte || '').trim()) {
      hote.querySelector('[data-k="st:texte"]')?.closest('.jm-champ')?.classList.add('faux');
      if (err) err.textContent = 'Colle un texte que tu as écrit toi-même.';
      hote.querySelector('[data-k="st:texte"]')?.focus({ preventScroll: false });
      O.son?.retour?.();
      return;
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    O.son?.ok?.();
    O.action('style-ajouter', { titre: String(styleForm.titre || '').slice(0, P_LONGUEURS.titre), texte: String(styleForm.texte).slice(0, P_LONGUEURS.stexte) });
  }

  // -------------------------------------------- Mes clés
  function clesHTML(d) {
    const { esc } = O, t = tx(d);
    let h = `<div class="jm-p${anime()}"><h1>${esc(texteSur(d.titre))}</h1><p class="jm-intro">${esc(texteSur(d.intro))}</p>`;
    for (const s of liste(d.services).filter((x) => idSur(x?.id))) {
      const parties = liste(s.parties).filter((p) => codeSurP(p?.id));
      const etat = ['gardee', 'absente', 'illisible', 'partielle'].includes(s.etat) ? s.etat : 'absente';
      const form = cleForm === s.id;
      h += `<section class="jm-cle ${etat}"><div class="jm-r-tete"><span class="jm-a-ic">${icone('cle')}</span><div class="jm-cle-t"><div class="jm-cle-l1"><b>${esc(texteSur(s.nom))}</b><span class="jm-etat">${esc(texteSur(s.etat_nom))}</span></div><p>${esc(texteSur(s.role))}</p></div></div>`;
      if (etat === 'gardee' && !form) h += `<div class="jm-parts">${parties.map((p) => `<span>${icone('cadenas')}${esc(texteSur(p.nom))} : <b>${esc(texteSur(p.apercu))}</b></span>`).join('')}</div>`;
      if (form) {
        h += `<div class="jm-grille cle">${parties.map((p) => `<label class="jm-champ${parties.length === 1 ? ' large' : ''}"><span>${esc(texteSur(p.nom))}</span><input class="f" type="password" data-perso="p-champ" data-zone="cle" data-champ="${p.id}" data-k="kf:${p.id}" maxlength="${P_LONGUEURS.cle}" autocomplete="off" spellcheck="false" placeholder="${esc(texteSur(p.indice))}"></label>`).join('')}</div>
          <p class="jm-err" role="alert"></p>
          <div class="jm-btns">${bouton('k-enregistrer', 'k:enreg', t('enregistrer'), 'coche', 'principal')}${bouton('k-annuler', 'k:annuler', t('annuler'), '', 'second')}</div>`;
      }
      if (form || etat !== 'gardee') {
        h += `<div class="jm-ouvoir"><b>${esc(t('ou'))}</b><p>${esc(texteSur(s.texte))}</p><div class="jm-lien-l"><span class="jm-lien" data-brut>${esc(texteSur(s.adresse))}</span>${bouton('k-copier', `kc:${s.id}`, t('copier'), 'lien', 'discret', ` data-id="${s.id}"`)}</div><p class="jm-petit">${esc(t('adresse_aide'))}</p></div>`;
      }
      if (!form) {
        if (cleConfirme === s.id) {
          h += `<div class="jm-suppr"><p>${esc(texteSur(s.question))}</p><div class="jm-btns">${bouton('k-effacer-oui', 'k:oui', t('oui'), 'poubelle', 'danger', ` data-id="${s.id}"`)}${bouton('k-effacer-non', 'k:non', t('non'), '', 'second')}</div></div>`;
        } else {
          const bts = etat === 'absente'
            ? bouton('k-donner', `k:${s.id}`, t('donner'), 'cle', 'principal', ` data-id="${s.id}"`)
            : bouton('k-donner', `k:${s.id}`, t('remplacer'), 'crayon', '', ` data-id="${s.id}"`)
              + (s.essai === true && etat === 'gardee' ? bouton('k-essayer', `kt:${s.id}`, t('essayer'), 'coche', '', ` data-id="${s.id}"`) : '')
              + bouton('k-effacer', `ke:${s.id}`, t('effacer'), 'poubelle', 'discret danger', ` data-id="${s.id}"`);
          h += `<div class="jm-btns">${bts}</div>`;
        }
      }
      h += '</section>';
    }
    return `${h}<p class="jm-note gauche">${esc(texteSur(d.note))}</p></div>`;
  }

  function cleEnregistrer() {
    if (!idSur(cleForm)) return;
    const champs = [...hote.querySelectorAll('[data-zone="cle"]')];
    const vide = champs.find((el) => !String(cleSaisie[el.dataset.champ] || '').trim());
    if (vide) {
      vide.closest('.jm-champ')?.classList.add('faux');
      const err = hote.querySelector('.jm-cle .jm-err'); if (err) err.textContent = 'Colle la clé dans la case.';
      vide.focus({ preventScroll: false });
      O.son?.retour?.();
      return;
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    const valeurs = Object.fromEntries(champs.map((el) => [el.dataset.champ, String(cleSaisie[el.dataset.champ] || '').slice(0, P_LONGUEURS.cle)]));
    O.son?.ok?.();
    O.action('cle-enregistrer', { service: cleForm, valeurs });
    // (la clé tapée n'est pas gardée ici : si elle est refusée, Jordan la colle de nouveau)
    cleSaisie = {};
    champs.forEach((el) => { el.value = ''; });
  }

  // Ferme le formulaire d'une clé et oublie ce qui y était tapé
  function cleFermer(focusK) {
    const id = cleForm;
    cleForm = null; cleSaisie = {};
    dessinerTout(focusK || (id ? `k:${id}` : undefined));
  }

  // -------------------------------------------- Ma boîte mail (étape 7)
  // Le guide pas à pas (la validation en deux étapes, le mot de passe d'application de Google), l'adresse et les 16
  // lettres, puis « Brancher ma boîte » ('mail-brancher') ; branchée : « Essayer », « Regarder maintenant »,
  // « Débrancher ma boîte » (jamais au premier clic). Le mot de passe tapé ne vit que dans boiteSaisie (jamais dans
  // l'état, jamais écrit dans le HTML : la case est remplie par sa propriété), envoyé une fois puis oublié.
  let boiteSaisie = { adresse: '', mdp: '' };
  let boiteConfirme = false;
  function boiteHTML(d) {
    const { esc } = O, t = tx(d);
    let h = `<div class="jm-p${anime()}"><h1>${esc(texteSur(d.titre))}</h1><p class="jm-intro">${esc(texteSur(d.intro))}</p>`;
    if (texteSur(d.en_cours)) h += `<div class="jm-annonce attention" role="status">${icone('horloge')}<div><span>${esc(texteSur(d.en_cours))}</span></div></div>`;
    else if (d.annonce && typeof d.annonce === 'object' && texteSur(d.annonce.texte)) {
      const g = d.annonce.genre === 'ok' ? 'ok' : 'erreur';
      h += `<div class="jm-annonce ${g}" role="status">${icone(g === 'ok' ? 'coche' : 'attention')}<div><span>${esc(texteSur(d.annonce.texte))}</span></div></div>`;
    }
    if (d.branchee === true) {
      h += `<section class="jm-cle gardee"><div class="jm-r-tete"><span class="jm-a-ic">${icone('mail')}</span><div class="jm-cle-t"><div class="jm-cle-l1"><b>${esc(texteSur(d.adresse))}</b><span class="jm-etat">${esc(t('branchee'))}</span></div>${d.derniere ? `<p>${esc(texteSur(d.derniere))}</p>` : ''}</div></div>
        <div class="jm-parts"><span>${icone('cadenas')}${esc(t('mot_de_passe'))} : <b>${esc(texteSur(d.apercu))}</b></span></div>`;
      if (boiteConfirme) {
        h += `<div class="jm-suppr"><p>${esc(texteSur(d.question))}</p><div class="jm-btns">${bouton('gm-decon-oui', 'gm:oui', t('oui'), 'poubelle', 'danger')}${bouton('gm-decon-non', 'gm:non', t('non'), '', 'second')}</div></div>`;
      } else {
        const occupe = texteSur(d.en_cours) ? ' disabled' : '';
        h += `<div class="jm-btns">${bouton('gm-relever', 'gm:relever', t('relever'), 'recharger', 'principal', occupe)}${bouton('gm-essayer', 'gm:essayer', t('essayer'), 'coche', '', occupe)}${bouton('gm-decon', 'gm:decon', t('debrancher'), 'poubelle', 'discret danger')}</div>`;
      }
      h += '</section>';
    } else {
      const etapes = liste(d.etapes).map((e, i) => {
        const lien = texteSur(e?.adresse) && idSur(e?.quoi)
          ? `<div class="jm-lien-l"><span class="jm-lien" data-brut>${esc(texteSur(e.adresse))}</span>${bouton('gm-copier', `gc:${e.quoi}`, t('copier'), 'lien', 'discret', ` data-id="${e.quoi}"`)}</div><p class="jm-petit">${esc(t('adresse_aide'))}</p>`
          : '';
        return `<li class="jm-etape"><span class="jm-etape-n">${i + 1}</span><div><b>${esc(texteSur(e?.titre))}</b><p>${esc(texteSur(e?.texte))}</p>${lien}</div></li>`;
      }).join('');
      h += `<ol class="jm-etapes">${etapes}</ol>
        <section class="jm-cle"><div class="jm-grille cle">
          <label class="jm-champ"><span>${esc(t('adresse'))}</span><input class="f" type="email" data-perso="p-champ" data-zone="gm" data-champ="adresse" data-k="gm:adresse" maxlength="254" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_adresse'))}" value="${esc(boiteSaisie.adresse)}"></label>
          <label class="jm-champ"><span>${esc(t('mdp'))}</span><input class="f" type="password" data-perso="p-champ" data-zone="gm" data-champ="mdp" data-k="gm:mdp" maxlength="40" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_mdp'))}"></label></div>
          <p class="jm-err" role="alert"></p>
          <div class="jm-btns">${bouton('gm-brancher', 'gm:brancher', t('brancher'), 'mail', 'principal', texteSur(d.en_cours) ? ' disabled' : '')}</div></section>`;
    }
    return `${h}<p class="jm-note gauche">${esc(texteSur(d.note))}</p></div>`;
  }

  // (la case du mot de passe, remplie par sa propriété après chaque dessin : jamais dans le HTML)
  function remplirBoite() {
    const el = hote.querySelector('[data-k="gm:mdp"]');
    if (el && boiteSaisie.mdp) el.value = boiteSaisie.mdp;
  }

  function boiteBrancher() {
    const err = hote.querySelector('.jm-cle .jm-err');
    for (const [k, m] of [['adresse', 'Écris ton adresse Gmail.'], ['mdp', "Colle le mot de passe d'application."]]) {
      if (!String(boiteSaisie[k] || '').trim()) {
        const el = hote.querySelector(`[data-k="gm:${k}"]`);
        el?.closest('.jm-champ')?.classList.add('faux');
        if (err) err.textContent = m;
        el?.focus({ preventScroll: false });
        O.son?.retour?.();
        return;
      }
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    O.son?.ok?.();
    O.action('mail-brancher', { adresse: String(boiteSaisie.adresse).slice(0, 254), mdp: String(boiteSaisie.mdp).slice(0, 40) });
    // (le mot de passe n'est pas gardé ici : refusé, Jordan le colle de nouveau)
    boiteSaisie.mdp = '';
    const el = hote.querySelector('[data-k="gm:mdp"]'); if (el) el.value = '';
  }

  // Le dossier d'une rubrique de Mon profil, selon sa vue
  function droiteProfil(d) {
    if (d.vue === 'cv') return cvHTML(d);
    if (d.vue === 'recherche') return rechercheHTML(d);
    if (d.vue === 'style') return styleHTML(d);
    if (d.vue === 'cles') return clesHTML(d);
    if (d.vue === 'mail') return boiteHTML(d);
    return null;
  }

  // Un élément de Mon profil (clic, A ou Entrée) ; vrai s'il en était un
  function validerProfil(el, d) {
    switch (d.perso) {
      case 'p-champ': {   // (Entrée dans un champ d'une ligne : le champ suivant)
        if (el.tagName !== 'INPUT') return true;
        const els = [...hote.querySelectorAll('#jm .jm-dossier .f')], i = els.indexOf(el);
        els[i + 1]?.focus({ preventScroll: false });
        return true;
      }
      // Mon CV
      case 'cv-importer': if (!el.disabled) { O.son?.ok?.(); O.action('cv-importer'); } return true;
      case 'cv-coller': if (!el.disabled) { O.son?.ok?.(); cvMode = 'coller'; cvColle = cvColle || { nom: '', texte: '' }; O.anime = !reduit(); dessinerTout('cv:nom'); } return true;
      case 'cv-annuler': O.son?.retour?.(); cvMode = 'voir'; dessinerTout('cv:coller'); return true;
      case 'cv-enregistrer': cvEnregistrer(); return true;
      case 'cv-ouvrir': { const id = Number(d.id); if (numSur(id)) { O.son?.tic?.(); cvConfirme = false; O.action('cv-ouvrir', { id }); } return true; }
      case 'cv-principal': { const c = objet(cvDossier().ouvert); if (numSur(c.id)) { O.son?.ok?.(); O.action('cv-principal', { id: c.id }); } return true; }
      case 'cv-supprimer': O.son?.ok?.(); cvConfirme = true; dessinerTout('cv:non'); hote.querySelector('.jm-suppr')?.scrollIntoView({ block: 'nearest' }); return true;
      case 'cv-suppr-non': O.son?.retour?.(); cvConfirme = false; dessinerTout('cv:suppr'); return true;
      case 'cv-suppr-oui': { const c = objet(cvDossier().ouvert); if (numSur(c.id) && cvConfirme) { cvConfirme = false; el.disabled = true; O.action('cv-supprimer', { id: c.id }); } return true; }
      // Ce que je cherche
      case 'rc-lieu': if (rch && codeSurP(d.id)) { rch.lieu = d.id; O.son?.bascule?.(); dessinerTout(el.dataset.k); } return true;
      case 'rc-rayon': { const n = Number(d.v); if (rch && Number.isInteger(n)) { rch.rayon = n; O.son?.bascule?.(); dessinerTout(el.dataset.k); } return true; }
      case 'rc-contrat': if (rch && codeSurP(d.id)) { rch.contrats = rch.contrats.includes(d.id) ? rch.contrats.filter((x) => x !== d.id) : [...rch.contrats, d.id]; O.son?.bascule?.(); dessinerTout(el.dataset.k); } return true;
      case 'rc-tele': if (rch && codeSurP(d.id)) { rch.teletravail = d.id; O.son?.bascule?.(); dessinerTout(el.dataset.k); } return true;
      case 'rc-case': if (rch && (d.id === 'sans_diplome' || d.id === 'debutant')) { rch[d.id] = !rch[d.id]; O.son?.bascule?.(); dessinerTout(el.dataset.k); } return true;
      case 'rc-enregistrer': rechercheEnregistrer(); return true;
      // Mon style
      case 'st-ajouter': if (!el.disabled) { O.son?.ok?.(); styleForm = styleForm || { titre: '', texte: '' }; O.anime = !reduit(); dessinerTout('st:titre'); } return true;
      case 'st-annuler': O.son?.retour?.(); styleForm = null; dessinerTout('st:ajouter'); return true;
      case 'st-enregistrer': styleEnregistrer(); return true;
      case 'st-ouvrir': { const id = Number(d.id); if (numSur(id)) { O.son?.tic?.(); styleConfirme = null; O.action('style-ouvrir', { id }); } return true; }
      case 'st-supprimer': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); styleConfirme = id; dessinerTout('st:non'); } return true; }
      case 'st-suppr-non': { const id = styleConfirme; O.son?.retour?.(); styleConfirme = null; dessinerTout(id ? `sts:${id}` : undefined); return true; }
      case 'st-suppr-oui': if (numSur(styleConfirme)) { const id = styleConfirme; styleConfirme = null; el.disabled = true; O.action('style-supprimer', { id }); } return true;
      // Mon style, Mes règles (étape 9)
      case 'sty-pil': if (['textes', 'regles'].includes(d.id) && stPil !== d.id) { stPil = d.id; styleForm = null; stZero = false; O.son?.tic?.(); O.anime = !reduit(); dessinerTout(`sp:${d.id}`); } return true;
      case 'sty-regle': {
        const actif = el.getAttribute('aria-checked') === 'true';
        O.son?.bascule?.();
        if (numSur(Number(d.id))) O.action('sty-regle', { id: Number(d.id), active: !actif });
        else if (typeof d.code === 'string' && /^[a-z0-9-]{1,40}$/.test(d.code)) O.action('sty-regle', { code: d.code, active: !actif });
        return true;
      }
      case 'sty-moins': case 'sty-plus': {
        const r = objet(styleDossier().regles), n = Number(r.phrase_max);
        if (Number.isInteger(n) && !el.disabled) { O.son?.tic?.(); O.action('sty-phrase', { max: n + (d.perso === 'sty-plus' ? 1 : -1) }); }
        return true;
      }
      case 'sty-int': if (codeSurP(d.id)) { O.son?.bascule?.(); O.action('sty-intensite', { code: d.id }); } return true;
      case 'sty-ajouter': {
        const err = hote.querySelector('.jm-st-ajout + .jm-err');
        if (!String(stAjout.motif || '').trim()) {
          if (err) err.textContent = texteSur(objet(objet(styleDossier().regles).textes).i_motif);
          hote.querySelector('[data-k="sr:motif"]')?.focus({ preventScroll: false });
          O.son?.retour?.();
          return true;
        }
        if (Date.now() < envoiJusqua) return true;
        envoiJusqua = Date.now() + 800; O.son?.ok?.();
        O.action('sty-ajouter', { motif: String(stAjout.motif).slice(0, 60), remplacement: String(stAjout.remplacement || '').slice(0, 60) });
        return true;
      }
      case 'sty-supprimer': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); O.action('sty-supprimer', { id }); } return true; }
      case 'sty-zero': O.son?.ok?.(); stZero = true; dessinerTout('sr:non'); hote.querySelector('.jm-suppr')?.scrollIntoView({ block: 'nearest' }); return true;
      case 'sty-zero-non': O.son?.retour?.(); stZero = false; dessinerTout('sr:zero'); return true;
      case 'sty-zero-oui': if (stZero) { stZero = false; el.disabled = true; O.action('sty-zero'); } return true;
      // Mes clés
      case 'k-donner': if (idSur(d.id)) { O.son?.ok?.(); cleForm = d.id; cleSaisie = {}; cleConfirme = null; dessinerTout(); hote.querySelector('#jm [data-zone="cle"]')?.focus({ preventScroll: false }); } return true;
      case 'k-annuler': O.son?.retour?.(); cleFermer(); return true;
      case 'k-enregistrer': cleEnregistrer(); return true;
      case 'k-copier': {
        const s = liste(objet(rubriquesDe(ongletDe('profil')).find((r) => r.id === 'cles')?.dossier).services).find((x) => x?.id === d.id);
        if (s && texteSur(s.adresse)) copierBrut(texteSur(s.adresse), el, () => O.action('p-copie', { quoi: 'adresse', service: s.id }));
        return true;
      }
      case 'k-effacer': if (idSur(d.id)) { O.son?.ok?.(); cleConfirme = d.id; dessinerTout('k:non'); } return true;
      case 'k-effacer-non': { const id = cleConfirme; O.son?.retour?.(); cleConfirme = null; dessinerTout(id ? `ke:${id}` : undefined); return true; }
      case 'k-effacer-oui': if (idSur(cleConfirme)) { const service = cleConfirme; cleConfirme = null; el.disabled = true; O.action('cle-effacer', { service }); } return true;
      case 'k-essayer': if (idSur(d.id) && !el.disabled) { O.son?.ok?.(); O.action('cle-essayer', { service: d.id }); } return true;
      // Ma boîte mail (étape 7)
      case 'gm-brancher': if (!el.disabled) boiteBrancher(); return true;
      case 'gm-essayer': if (!el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('mail-essayer'); } return true;
      case 'gm-relever': if (!el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('mail-relever'); } return true;
      case 'gm-decon': O.son?.ok?.(); boiteConfirme = true; dessinerTout('gm:non'); return true;
      case 'gm-decon-non': O.son?.retour?.(); boiteConfirme = false; dessinerTout('gm:decon'); return true;
      case 'gm-decon-oui': if (boiteConfirme) { boiteConfirme = false; el.disabled = true; O.son?.ok?.(); O.action('mail-debrancher'); } return true;
      case 'gm-copier': {
        const e = liste(objet(rubriquesDe(ongletDe('profil')).find((r) => r.id === 'mail')?.dossier).etapes).find((x) => x?.quoi === d.id);
        if (e && texteSur(e.adresse)) copierBrut(texteSur(e.adresse), el, () => O.action('mail-copie', { quoi: d.id }));
        return true;
      }
      default: return validerOffres(el, d);
    }
  }

  // Ce qui se tape dans Mon profil : gardé ici
  function saisieProfil(el, d) {
    const v = el.value;
    el.closest('.jm-champ')?.classList.remove('faux');
    const err = el.closest('form, .jm-p, .jm-cle')?.querySelector('.jm-err'); if (err) err.textContent = '';
    if (d.zone === 'colle' && (d.champ === 'nom' || d.champ === 'texte')) { cvColle = { ...(cvColle || {}), [d.champ]: v }; return; }
    if (d.zone === 'cv' && (d.champ === 'nom' || d.champ === 'texte')) {
      const c = objet(cvDossier().ouvert); if (!numSur(c.id)) return;
      const e = Object.hasOwn(cvEdits, c.id) ? cvEdits[c.id] : { nom: texteSur(c.nom), texte: texteSur(c.texte) };
      cvEdits[c.id] = { ...e, [d.champ]: v };
      return;
    }
    if (d.zone === 'rc' && rch && ['postes', 'ville', 'salaire', 'departement', 'region'].includes(d.champ)) { rch[d.champ] = v; return; }
    if (d.zone === 'st' && styleForm && (d.champ === 'titre' || d.champ === 'texte')) { styleForm[d.champ] = v; return; }
    if (d.zone === 'sr' && (d.champ === 'motif' || d.champ === 'remplacement')) { stAjout[d.champ] = v; return; }
    if (d.zone === 'cle' && cleForm && codeSurP(d.champ)) cleSaisie[d.champ] = v;
    if (d.zone === 'gm' && (d.champ === 'adresse' || d.champ === 'mdp')) boiteSaisie[d.champ] = v;
  }

  // Retour dans Mon profil : ferme d'abord ce qui est ouvert ; vrai si quelque chose l'était
  function retourProfil() {
    if (onglet !== 'profil') return false;
    if (cvConfirme) { cvConfirme = false; dessinerTout('cv:suppr'); return true; }
    if (cvMode === 'coller') { cvMode = 'voir'; dessinerTout('cv:coller'); return true; }
    if (styleConfirme) { const id = styleConfirme; styleConfirme = null; dessinerTout(`sts:${id}`); return true; }
    if (styleForm) { styleForm = null; dessinerTout('st:ajouter'); return true; }
    if (stZero) { stZero = false; dessinerTout('sr:zero'); return true; }
    if (cleConfirme) { const id = cleConfirme; cleConfirme = null; dessinerTout(`ke:${id}`); return true; }
    if (cleForm) { cleFermer(); return true; }
    if (boiteConfirme) { boiteConfirme = false; dessinerTout('gm:decon'); return true; }
    return false;
  }

  // Les ordres de l'appli pour Mon profil ; vrai si c'en était un
  function ordreProfil(o) {
    if (numSur(o.cv)) {
      cvMode = 'voir'; cvColle = null; cvConfirme = false; delete cvEdits[o.cv];
      if (rubriqueOuverte('cv')) dessinerTout('cv:enreg');
      return true;
    }
    if (numSur(o.cv_supprime)) { delete cvEdits[o.cv_supprime]; cvConfirme = false; if (rubriqueOuverte('cv')) dessinerTout(); return true; }
    if (o.recherche === 'enregistree') { rch = null; if (rubriqueOuverte('recherche')) dessinerTout('rc:enreg'); return true; }
    if (numSur(o.style)) { styleForm = null; if (rubriqueOuverte('style')) dessinerTout('st:ajouter'); return true; }
    if (numSur(o.style_regle)) { stAjout = { motif: '', remplacement: '' }; if (rubriqueOuverte('style')) dessinerTout('sr:motif'); return true; }
    if (o.style_zero === true) { stZero = false; stAjout = { motif: '', remplacement: '' }; if (rubriqueOuverte('style')) dessinerTout('sr:zero'); return true; }
    if (o.style_pil === 'regles' || o.style_pil === 'textes') { stPil = o.style_pil; if (rubriqueOuverte('style')) dessinerTout(`sp:${o.style_pil}`); return true; }
    if (idSur(o.cle)) { cleForm = null; cleSaisie = {}; cleConfirme = null; if (rubriqueOuverte('cles')) dessinerTout(`k:${o.cle}`); return true; }
    // (étape 7 : la boîte branchée ou débranchée ; construction d'essai : la boîte remplie comme Jordan)
    if (o.boite === 'branchee' || o.boite === 'debranchee') {
      boiteSaisie = { adresse: '', mdp: '' }; boiteConfirme = false;
      if (rubriqueOuverte('mail')) dessinerTout(o.boite === 'branchee' ? 'gm:relever' : 'gm:adresse');
      return true;
    }
    if (o.boite_form && typeof o.boite_form === 'object') {
      const b = o.boite_form;
      boiteSaisie = { adresse: texteSur(b.adresse).slice(0, 254), mdp: texteSur(b.mdp).slice(0, 40) };
      if (onglet !== 'profil') montrer('profil');
      ouvrirRubrique('mail', 'gm:mdp');
      if (b.envoyer === true) boiteBrancher();
      return true;
    }
    return false;
  }

  // ---------------------------------------------------------------- Offres (étape 4)
  // La barre des filtres (texte cherché dans toutes les offres gardées, lieu, contrat, télétravail, toutes / favoris /
  // plus tard / cachées), à gauche la recherche choisie, « Chercher maintenant » et la liste, à droite l'offre ouverte
  // ou « Mes recherches ». Les filtres tapés restent ici et partent à Rust ('o-filtres', un peu après la frappe) ; Rust
  // vérifie, filtre et renvoie la liste. Aucune adresse d'offre n'est ici : « Voir sur le site » la demande à Rust.
  const offresDe = () => { const o = onglets().find((x) => x.id === 'offres'); return o?.offres && typeof o.offres === 'object' ? o.offres : null; };
  const tO = (x) => { const t = objet(x?.textes); return (k) => texteSur(t[k]); };
  const rchDossier = () => objet(rubriquesDe(ongletDe('profil')).find((r) => r.id === 'recherche')?.dossier);
  const OF_CHAMPS = ['texte', 'genre', 'ville', 'rayon', 'departement', 'region', 'contrat', 'teletravail', 'liste', 'sans_diplome', 'debutant', 'tri'];
  // (étape 6 : « Aucun diplôme requis » et « Débutant accepté » sont des oui ou non)
  const OF_BASCULES = ['sans_diplome', 'debutant'];
  const valChamp = (k, v) => (k === 'rayon' ? (Number.isInteger(Number(v)) ? Number(v) : 30) : OF_BASCULES.includes(k) ? v === true || v === 'oui' || v === 'true' : texteSur(String(v ?? '')));
  let ofFiltres = null;    // les filtres en cours de saisie (une copie de ceux reçus)
  let ofMinuteur = 0;      // l'envoi des filtres, un peu après la frappe
  let ofVue = 'offre';     // à droite : 'offre' ou 'recherches' (Mes recherches)
  let ofForm = null;       // la recherche en cours d'écriture : { id (rien : nouvelle), v: { nom, mots, lieu, ville, rayon, departement, region } }
  let ofConfirme = null;   // la recherche dont la suppression est demandée
  let ofChoisie = null;    // l'offre choisie à l'instant (avant que Rust ne renvoie son dossier)
  let ofVue1 = null;       // l'offre dessinée en dernier

  function suivreOffres(x) {
    if (!ofFiltres) {
      const f = objet(x.filtres);
      ofFiltres = Object.fromEntries(OF_CHAMPS.map((k) => [k, valChamp(k, f[k])]));
      if (!ofFiltres.tri) ofFiltres.tri = 'pertinence';
      if (!ofFiltres.genre) ofFiltres.genre = 'partout';
      if (!ofFiltres.liste) ofFiltres.liste = 'toutes';
    }
    const id = numSur(x.fiche?.id) ? x.fiche.id : null;
    if (ofChoisie !== null && (id === ofChoisie || !liste(x.lignes).some((l) => l?.id === ofChoisie))) ofChoisie = null;
    // (étape 8 : une autre offre ouverte, ses pilules reviennent à « L'offre », sauf si on vient de son dossier)
    if (id !== ofVue1) {
      ofVue1 = id;
      if (prVoulu === null || prVoulu !== id) ofPil = 'offre';
      if (prVoulu === id) { ofPil = 'cand'; prVoulu = null; }
      prEdition = null; prBrute = false; prVoir = null; prInt = null; prConsigne = '';
    }
  }

  // ---------------------------------------------------------------- La candidature préparée (étape 8)
  // Dans l'offre ouverte, les pilules « L'offre » et « Ma candidature » (maquette C). « Préparer ma candidature »
  // ('pr-preparer') : Rust demande à Claude sur un fil à part, puis renvoie la version gardée (P.preparation) : la lettre
  // en morceaux (les phrases à vérifier surlignées), les réponses aux questions de l'offre, les conseils pour le CV, les
  // versions. Jordan modifie sa lettre ici ('pr-lettre' : Rust vérifie et garde ; la version de Claude reste), copie,
  // dit « C'est juste » ou « Retirer » d'une phrase à vérifier, refait une version avec une consigne courte facultative.
  let ofPil = 'offre';    // la pilule de l'offre ouverte : 'offre' ou 'cand'
  let prVoulu = null;     // l'offre à ouvrir sur « Ma candidature » (depuis le dossier d'une candidature)
  let prEdition = null;   // la lettre en train d'être modifiée : { id, texte }
  let prBrute = false;    // la version de Claude montrée (au lieu de la lettre de Jordan)
  let prConsigne = '';    // la consigne tapée pour une autre version
  const prDe = (f) => { const p = objet(P?.preparation); return f && numSur(p.offre) && p.offre === f.id ? p : null; };
  const prVersion = (p) => { const v = objet(p?.version); return numSur(v.id) ? v : null; };
  // (« Ma candidature » ouverte : le dossier descend jusqu'aux pilules, la lettre se voit)
  const voirPil = () => { const d = hote.querySelector('.jm-dossier'), p = hote.querySelector('.jm-o-pil'); if (d && p) d.scrollTop += p.getBoundingClientRect().top - d.getBoundingClientRect().top - 12; };
  // (un morceau : à vérifier, `n` ; qui sonne comme une IA, `g` son genre et `r` sa raison, montrée au survol)
  const GENRES_IA = ['plancher', 'ia', 'perso', 'pompeux', 'trois', 'longue'];
  const morceauxHTML = (l) => liste(l).map((m) => {
    let x = O.esc(texteSur(m?.t));
    if (GENRES_IA.includes(m?.g)) x = `<span class="jm-sonne g-${m.g}" title="${O.esc(texteSur(m.r))}">${x}</span>`;
    return Number.isInteger(m?.n) ? `<mark class="jm-verif">${x}</mark>` : x;
  }).join('');
  // Étape 9 : la lettre montrée (« brute » : celle de Claude ; « style » : dans mon style ; « moi » : celle de Jordan),
  // l'intensité choisie pour la réécriture
  let prVoir = null;
  let prInt = null;
  const prChoix = (v) => (v.gardee === 'moi' && v.modifiee !== true ? 'brute' : (['brute', 'style', 'moi'].includes(v.gardee) ? v.gardee : 'brute'));
  const prQuelle = (v) => {
    const q = prVoir || prChoix(v);
    return q === 'moi' && v.modifiee !== true ? 'brute' : q;
  };
  const prLettre = (v, q) => objet(objet(v.lettres)[q]);
  function scoreHTML(s, t) {
    const x = objet(s), n = Number(x.note);
    if (!Number.isInteger(n)) return '';
    const g = n >= 80 ? 'bon' : n >= 60 ? 'moyen' : 'faible';
    const rs = liste(x.raisons).filter((r) => texteSur(r));
    return `<div class="jm-score ${g}" role="status"><div class="jm-score-n"><b>${n}</b><small>${O.esc(t('sur_100'))}</small></div><div class="jm-score-t"><small>${O.esc(t('score_t'))}</small><b>${O.esc(texteSur(x.mot))}</b>${rs.map((r) => `<span>${O.esc(r)}</span>`).join('')}</div></div>`;
  }

  function pilulesOffreHTML(f, p) {
    const { esc } = O, t = tO(p);
    const pret = liste(p.versions).length > 0;
    return `<div class="jm-pilules jm-o-pil" role="tablist">${[['offre', esc(t('p_offre'))], ['cand', `${esc(t('p_cand'))}${pret ? ` <small>${esc(t('prete'))}</small>` : ''}`]].map(([id, nom]) => `<button type="button" role="tab" aria-selected="${ofPil === id}" class="jm-pil f${ofPil === id ? ' actif' : ''}" data-perso="pr-pil" data-id="${id}" data-k="op:${id}">${nom}</button>`).join('')}</div>`;
  }

  // Dans « L'offre » : préparer, ou la candidature déjà prête
  function actionPrepHTML(p) {
    const { esc } = O, t = tO(p);
    if (typeof p.en_cours === 'string' && p.en_cours) return `<div class="jm-attente" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(p.en_cours)}</span></div>`;
    if (liste(p.versions).length) return `<div class="jm-action jm-pr-a"><span class="jm-a-ic">${icone('plume')}</span><div class="jm-a-txt"><b>${esc(t('prete_t'))}</b><span>${esc(t('prete_s'))}</span></div><div class="jm-a-bts">${bouton('pr-voir', 'pr:voir', t('voir'), 'droite', 'principal')}</div></div>`;
    return `<div class="jm-action jm-pr-a"><span class="jm-a-ic">${icone('plume')}</span><div class="jm-a-txt"><b>${esc(t('preparer_t'))}</b><span>${esc(t('preparer_s'))}</span></div><div class="jm-a-bts">${bouton('pr-preparer', 'pr:preparer', t('preparer'), 'plume', 'principal', p.occupe === true ? ' disabled' : '')}</div></div>`;
  }

  function annoncePrepHTML(p) {
    const { esc } = O, t = tO(p), a = objet(p.annonce);
    if (!texteSur(a.texte)) return '';
    const g = ['ok', 'attention', 'erreur'].includes(a.genre) ? a.genre : 'attention';
    const b = a.cles === true ? bouton('of-cles', 'pr:cles', t('ouvrir_cles'), 'cle', 'discret') : a.cv === true ? bouton('pr-cv', 'pr:cv', t('ouvrir_cv'), 'cv', 'discret') : '';
    return `<div class="jm-annonce ${g} jm-pr-ann" role="status">${icone(g === 'ok' ? 'coche' : 'attention')}<div><span>${esc(a.texte)}</span>${b}</div></div>`;
  }

  // « Ma candidature »
  function candidatureHTML(p) {
    const { esc } = O, t = tO(p), v = prVersion(p);
    let h = annoncePrepHTML(p);
    if (typeof p.en_cours === 'string' && p.en_cours) h += `<div class="jm-attente" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(p.en_cours)}</span></div>`;
    if (!v) {
      if (p.en_cours) return h;
      const sans = p.claude !== true;
      return `${h}<div class="jm-action jm-pr-a"><span class="jm-a-ic">${icone(sans ? 'cle' : 'plume')}</span><div class="jm-a-txt"><b>${esc(t('pas_t'))}</b><span>${esc(sans ? t('sans_cle') : t('preparer_s'))}</span></div><div class="jm-a-bts">${sans ? bouton('of-cles', 'pr:cle', t('ouvrir_cles'), 'cle', 'principal') : bouton('pr-preparer', 'pr:preparer', t('preparer'), 'plume', 'principal', p.occupe === true ? ' disabled' : '')}</div></div>`;
    }
    const occupe = p.occupe === true ? ' disabled' : '';
    const voir = prQuelle(v), L = prLettre(v, voir), choix = prChoix(v);
    h += `<div class="jm-pr"><div class="jm-pr-g"><div class="jm-meta"><span>${esc(texteSur(voir === 'style' ? L.meta : v.meta))}</span></div>`;
    // (étape 9 : les trois lettres ; celle que Jordan garde porte « Gardée »)
    const tabs = [['brute', t('v_brute')], ['style', t('v_style')]];
    if (v.modifiee === true) tabs.push(['moi', t('v_moi')]);
    h += `<div class="jm-pilules jm-pr-pil" role="tablist">${tabs.map(([id, nom]) => `<button type="button" role="tab" aria-selected="${voir === id}" class="jm-pil f${voir === id ? ' actif' : ''}" data-perso="pr-voir-l" data-id="${id}" data-k="pr:l-${id}">${esc(nom)}${choix === id ? ` <small>${esc(t('gardee'))}</small>` : ''}</button>`).join('')}</div>`;
    if (prEdition && prEdition.id === v.id) {
      h += `<textarea class="jm-pr-zone f" data-perso="pr-zone" data-k="pr:zone" maxlength="8000" spellcheck="true" aria-label="${esc(t('lettre_t'))}">${esc(prEdition.texte)}</textarea>
        <div class="jm-btns serre">${bouton('pr-garder', 'pr:garder', t('garder'), 'coche', 'principal')}${bouton('pr-annuler', 'pr:annuler', t('annuler'), '', 'second')}</div>`;
    } else if (voir === 'style' && !Object.keys(L).length) {
      h += `<div class="jm-annonce attention jm-pr-ann" role="status">${icone('plume')}<div><span>${esc(t('pas_style'))}</span></div></div>`;
    } else {
      h += scoreHTML(L.score, t);
      h += `<div class="jm-lettre jm-pr-lettre" data-brut>${morceauxHTML(L.morceaux)}</div>`;
      h += `<div class="jm-btns serre jm-pr-bts">${choix === voir ? '' : bouton('pr-choisir', 'pr:garder-l', t('garder_celle'), 'coche', 'principal', ` data-id="${voir}"`)}${bouton('pr-copier', 'pr:copier', t('copier'), 'copie', choix === voir ? 'principal' : '')}${bouton('pr-modifier', 'pr:modifier', t('modifier'), 'crayon', '')}</div>`;
      const notes = liste(L.notes).filter((x) => texteSur(x));
      if (notes.length) h += `<div class="jm-pr-notes"><b>${esc(t('notes_t'))}</b>${notes.map((x) => `<span>${esc(x)}</span>`).join('')}</div>`;
      const pl = liste(L.plancher).filter((x) => texteSur(x));
      if (pl.length) h += `<h2 class="jm-h2 jm-pr-avt">${icone('attention')}${esc(t('plancher_t'))}</h2>${pl.map((x) => `<p class="jm-pr-plancher">${esc(x)}</p>`).join('')}`;
      const ms = liste(L.marques);
      h += `<h2 class="jm-h2">${esc(t('ia_t'))}</h2>`;
      if (ms.length) {
        h += `<p class="jm-petit jm-pr-avs">${esc(t('ia_s'))}</p><div class="jm-ia-l">${ms.map((m) => `<div><q>${esc(texteSur(m?.texte))}</q><small>${esc(texteSur(m?.raison))}</small></div>`).join('')}</div>`;
        if (Number(L.marques_n) > ms.length) h += `<p class="jm-petit">+ ${Number(L.marques_n) - ms.length} ${esc(t('autres'))}</p>`;
      } else h += `<p class="jm-petit">${esc(t('ia_aucun'))}</p>`;
    }
    // Les phrases à vérifier : celles de la version dans mon style, ou celles de la lettre de Jordan
    const style = voir === 'style';
    const av = liste(style ? L.a_verifier : v.a_verifier).filter((a) => Number.isInteger(a?.n));
    if (av.length && !(prEdition && prEdition.id === v.id)) {
      const [j, r] = style ? ['pr-s-juste', 'pr-s-retirer'] : ['pr-juste', 'pr-retirer'];
      h += `<h2 class="jm-h2 jm-pr-avt">${icone('attention')}${esc(t('a_verifier_t'))}</h2><p class="jm-petit jm-pr-avs">${esc(t('a_verifier_s'))}</p>`;
      h += av.map((a) => `<div class="jm-pr-av"><q>${esc(texteSur(a.texte))}</q><small>${esc(texteSur(a.raison))}</small><div class="jm-btns serre">${bouton(j, `pr:j-${a.n}`, t('juste'), 'coche', '', ` data-n="${a.n}"`)}${bouton(r, `pr:r-${a.n}`, t('retirer'), 'poubelle', 'discret danger', ` data-n="${a.n}"`)}</div></div>`).join('');
    }
    // Dans mon style : l'intensité, puis réécrire (ou mes règles seules, sans clé)
    const ints = liste(p.intensites).filter((x) => codeSur(x?.code));
    const ic = prInt && ints.some((x) => x.code === prInt) ? prInt : (codeSur(p.intensite) ? p.intensite : 'moyenne');
    const ix = ints.find((x) => x.code === ic) || {};
    const bloque = ix.claude === true && p.claude !== true;
    h += `<h2 class="jm-h2">${esc(t('style_t'))}</h2><p class="jm-petit">${esc(t('style_s'))}</p>`;
    h += `<div class="jm-segments" role="radiogroup" aria-label="${esc(t('intensite_t'))}">${ints.map((x) => `<button type="button" role="radio" aria-checked="${x.code === ic}" class="jm-seg f${x.code === ic ? ' actif' : ''}" data-perso="pr-int" data-id="${x.code}" data-k="pr:i-${x.code}">${esc(texteSur(x.nom))}</button>`).join('')}</div>`;
    h += `<p class="jm-petit">${esc(bloque ? t('sans_cle_style') : texteSur(ix.texte))}</p>`;
    h += `<div class="jm-btns serre">${bouton('pr-styler', 'pr:styler', ix.claude === true ? t('styler') : t('appliquer'), 'plume', 'principal', p.occupe === true || bloque ? ' disabled' : '')}${bouton('pr-regles', 'pr:regles', t('mes_regles'), 'reglages', 'discret')}</div>`;
    h += `<h2 class="jm-h2">${esc(t('refaire_t'))}</h2><div class="jm-pr-refaire"><input class="f" data-perso="pr-consigne" data-k="pr:consigne" maxlength="120" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_consigne'))}" aria-label="${esc(t('refaire_t'))}" value="${esc(prConsigne)}">${bouton('pr-refaire', 'pr:refaire', t('refaire'), 'recharger', '', occupe)}</div>`;
    h += `<p class="jm-petit">${esc(texteSur(v.cout))}</p></div><div class="jm-pr-cote">${envoiBarHTML()}${poBarHTML()}`;
    const qs = liste(v.questions);
    h += `<h3>${esc(t('questions_t'))}</h3>`;
    h += qs.length ? qs.map((q, i) => `<div class="jm-q"><div class="jm-qt"><span>${esc(texteSur(q?.question))}</span>${bouton('pr-copier-q', `pr:q-${i}`, t('copier_rep'), 'copie', 'discret', ` data-i="${i}"`)}</div><p>${morceauxHTML(q?.morceaux)}</p></div>`).join('') : `<p class="jm-petit jm-pr-vide">${esc(t('aucune_q'))}</p>`;
    const cs = liste(v.conseils);
    if (cs.length) h += `<h3>${esc(t('conseils_t'))}</h3>${cs.map((c) => `<div class="jm-conseil ${codeSur(c?.genre) ? c.genre : ''}"><b>${esc(texteSur(c?.titre))}</b><span>${esc(texteSur(c?.texte))}</span>${texteSur(c?.cv) ? `<q>${esc(c.cv)}</q>` : ''}</div>`).join('')}`;
    const vs = liste(p.versions).filter((x) => numSur(x?.id));
    h += `<h3>${esc(t('versions_t'))}</h3><div class="jm-versions">${vs.map((x) => `<button type="button" class="jm-version f${x.montree === true ? ' actif' : ''}" data-perso="pr-version" data-id="${x.id}" data-k="pr:v-${x.id}" aria-current="${x.montree === true}"><b>${esc(texteSur(x.nom))}</b><span>${esc(texteSur(x.date))}${texteSur(x.consigne) ? `, « ${esc(x.consigne)} »` : ''}${x.modifiee === true ? `, ${esc(t('modifiee'))}` : ''}</span></button>`).join('')}</div>`;
    return `${h}</div></div>`;
  }

  // Un geste de la candidature préparée ; vrai s'il en était un
  function validerPrep(el, d, f) {
    const p = prDe(f), v = prVersion(p);
    switch (d.perso) {
      case 'pr-pil': if (['offre', 'cand'].includes(d.id) && ofPil !== d.id) { ofPil = d.id; O.son?.tic?.(); O.anime = !reduit(); dessinerTout(`op:${d.id}`); if (d.id === 'cand') voirPil(); } return true;
      case 'pr-voir': O.son?.ok?.(); ofPil = 'cand'; O.anime = !reduit(); dessinerTout('op:cand'); voirPil(); return true;
      case 'pr-preparer': if (f && numSur(f.id) && !el.disabled) { el.disabled = true; O.son?.ok?.(); ofPil = 'cand'; O.action('pr-preparer', { id: f.id }); dessinerTout('op:cand'); voirPil(); } return true;
      case 'pr-consigne': // (Entrée : refaire)
      case 'pr-refaire': if (f && v && !(p?.occupe === true)) { O.son?.ok?.(); O.action('pr-preparer', { id: f.id, consigne: String(prConsigne || '').slice(0, 120), depuis: v.id }); } return true;
      case 'pr-copier': if (v) copierBrut(texteSur(prLettre(v, prQuelle(v)).texte) || texteSur(v.lettre), el, () => O.action('pr-copie', { quoi: 'lettre' })); return true;
      // Étape 9 : la lettre montrée, celle gardée, l'intensité, réécrire, les phrases à vérifier dans mon style
      case 'pr-voir-l': if (v && ['brute', 'style', 'moi'].includes(d.id)) { O.son?.tic?.(); prVoir = d.id; prEdition = null; dessinerTout(`pr:l-${d.id}`); } return true;
      case 'pr-choisir': if (v && ['brute', 'style', 'moi'].includes(d.id)) { O.son?.ok?.(); O.action('pr-choisir', { id: v.id, quelle: d.id }); } return true;
      case 'pr-int': if (codeSur(d.id)) { O.son?.bascule?.(); prInt = d.id; dessinerTout(`pr:i-${d.id}`); } return true;
      case 'pr-styler': if (v && !el.disabled && !(p?.occupe === true)) { el.disabled = true; O.son?.ok?.(); O.action('pr-styler', { id: v.id, intensite: prInt || (codeSur(p?.intensite) ? p.intensite : 'moyenne') }); } return true;
      case 'pr-s-juste': case 'pr-s-retirer': { const n = Number(d.n); if (v && Number.isInteger(n) && !el.disabled) { el.disabled = true; O.son?.ok?.(); O.action(d.perso, { id: v.id, n }); } return true; }
      case 'pr-regles': O.son?.ok?.(); stPil = 'regles'; montrer('profil', 'r:style'); ouvrirRubrique('style'); return true;
      case 'pr-copier-q': { const q = liste(v?.questions)[Number(d.i)]; if (q) copierBrut(texteSur(q.reponse), el, () => O.action('pr-copie', { quoi: 'question' })); return true; }
      case 'pr-modifier': if (v) { O.son?.ok?.(); prEdition = { id: v.id, texte: texteSur(prLettre(v, prQuelle(v)).texte) || texteSur(v.lettre) }; prBrute = false; dessinerTout('pr:zone'); } return true;
      case 'pr-zone': return true;
      case 'pr-garder': if (prEdition && Date.now() >= envoiJusqua) { envoiJusqua = Date.now() + 800; O.son?.ok?.(); O.action('pr-lettre', { id: prEdition.id, lettre: String(prEdition.texte).slice(0, 8000) }); } return true;
      case 'pr-annuler': O.son?.retour?.(); prEdition = null; dessinerTout('pr:modifier'); return true;
      case 'pr-brute': O.son?.tic?.(); prBrute = !prBrute; dessinerTout('pr:brute'); return true;
      case 'pr-juste': case 'pr-retirer': { const n = Number(d.n); if (v && Number.isInteger(n) && !el.disabled) { el.disabled = true; O.son?.ok?.(); O.action(d.perso, { id: v.id, n }); } return true; }
      case 'pr-version': { const id = Number(d.id); if (numSur(id)) { O.son?.tic?.(); prEdition = null; prBrute = false; prVoir = null; O.action('pr-version', { id }); } return true; }
      case 'pr-cv': O.son?.ok?.(); montrer('profil', 'r:cv'); ouvrirRubrique('cv'); return true;
      // (depuis le dossier d'une candidature liée à une offre)
      case 'pr-dossier': {
        const id = Number(d.id);
        if (!numSur(id)) return true;
        O.son?.ok?.(); prVoulu = offresDe()?.fiche?.id === id ? null : id; ofVue = 'offre'; ofForm = null; ofConfirme = null; ofChoisie = id; ofPil = 'cand';
        montrer('offres', 'op:cand'); O.action('o-ouvrir', { id });
        return true;
      }
      default: return false;
    }
  }

  // Envoie les filtres à Rust (tout de suite, ou un peu après la frappe)
  function envoyerFiltres(attendre) {
    clearTimeout(ofMinuteur);
    const partir = () => { if (ofFiltres) O.action('o-filtres', { ...ofFiltres, ville: String(ofFiltres.ville || '').slice(0, 60), texte: String(ofFiltres.texte || '').slice(0, 100) }); };
    if (attendre) ofMinuteur = setTimeout(partir, 450); else partir();
  }

  const optionsHTML = (l, val, sur) => liste(l).filter((x) => sur(x?.id)).map((x) => `<option value="${x.id}"${String(x.id) === String(val) ? ' selected' : ''}>${O.esc(texteSur(x.nom))}</option>`).join('');
  const choixSur = (t) => typeof t === 'string' && /^[a-z0-9_]{0,40}$/.test(t);

  function barreOffresHTML(x) {
    const { esc } = O, t = tO(x), f = ofFiltres, d = rchDossier();
    let ou = '';
    if (f.genre === 'ville') {
      ou = `<input class="f jm-b-ville" data-perso="of-champ" data-champ="ville" data-k="of:ville" maxlength="60" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_ville'))}" aria-label="${esc(t('ville'))}" value="${esc(f.ville)}">
        <select class="f" data-perso="of-champ" data-champ="rayon" data-k="of:rayon" aria-label="${esc(t('rayon'))}">${liste(x.rayons).filter(Number.isInteger).map((n) => `<option value="${n}"${n === Number(f.rayon) ? ' selected' : ''}>${n} ${esc(t('km'))}</option>`).join('')}</select>`;
    } else if (f.genre === 'departement') {
      ou = `<select class="f" data-perso="of-champ" data-champ="departement" data-k="of:dep" aria-label="${esc(t('departement'))}"><option value="">${esc(t('choisir'))}</option>${optionsHTML(d.departements, f.departement, deptSur)}</select>`;
    } else if (f.genre === 'region') {
      ou = `<select class="f" data-perso="of-champ" data-champ="region" data-k="of:reg" aria-label="${esc(t('region'))}"><option value="">${esc(t('choisir'))}</option>${optionsHTML(d.regions, f.region, codeSurP)}</select>`;
    }
    const listes = liste(x.listes).filter((l) => codeSurP(l?.id)).map((l) => {
      const oui = f.liste === l.id, n = Number.isInteger(l.nombre) && l.nombre > 0 ? ` <i>${l.nombre}</i>` : '';
      return `<button type="button" role="radio" aria-checked="${oui}" class="jm-puce petite f${oui ? ' oui' : ''}" data-perso="of-liste" data-id="${l.id}" data-k="of:l-${l.id.replace('_', '-')}">${esc(texteSur(l.nom))}${n}</button>`;
    }).join('');
    return `<div class="jm-barre">
      <div class="jm-b-rang">
        <label class="jm-cherche">${icone('loupe')}<input class="f" data-perso="of-champ" data-champ="texte" data-k="of:texte" maxlength="100" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_texte'))}" value="${esc(f.texte)}"></label>
        <div class="jm-b-lieu${f.genre !== 'partout' ? ' on' : ''}">${icone('lieu')}<select class="f" data-perso="of-champ" data-champ="genre" data-k="of:genre" aria-label="${esc(t('lieu'))}">${optionsHTML(x.lieux, f.genre, codeSurP)}</select>${ou}</div>
        ${basculesHTML(t)}
      </div>
      <div class="jm-b-rang">
        <span class="jm-b-sel${f.contrat ? ' on' : ''}"><select class="f" data-perso="of-champ" data-champ="contrat" data-k="of:contrat" aria-label="${esc(t('contrat'))}">${optionsHTML(x.contrats, f.contrat, choixSur)}</select>${icone('bas')}</span>
        <span class="jm-b-sel${f.teletravail && f.teletravail !== 'peu_importe' ? ' on' : ''}"><select class="f" data-perso="of-champ" data-champ="teletravail" data-k="of:tele" aria-label="${esc(t('teletravail'))}">${optionsHTML(x.teles, f.teletravail || 'peu_importe', codeSurP)}</select>${icone('bas')}</span>
        <span class="jm-b-sep" aria-hidden="true"></span>
        <div class="jm-puces" role="radiogroup">${listes}</div>
        <span class="jm-compte">${esc(texteSur(x.compte))}</span>
      </div></div>`;
  }

  // Étape 6 : « Aucun diplôme requis » et « Débutant accepté », des interrupteurs (aussi pour les résultats des sites)
  function basculesHTML(t) {
    const { esc } = O, f = ofFiltres || {};
    return `<div class="jm-bascules">${OF_BASCULES.map((k) => {
      const oui = f[k] === true;
      return `<button type="button" role="switch" aria-checked="${oui}" class="jm-puce petite jm-bascule f${oui ? ' oui' : ''}" data-perso="of-bascule" data-champ="${k}" data-k="of:${k === 'sans_diplome' ? 'sd' : 'deb'}">${icone(oui ? 'coche' : 'plus')}${esc(t(k))}</button>`;
    }).join('')}</div>`;
  }

  // La pastille d'une note (« Très proche », « Proche », « Un peu », « Hors sujet »)
  const pastilleNote = (n) => (codeSurP(n?.code) ? `<i class="jm-pert ${n.code}">${O.esc(texteSur(n.nom))}</i>` : '');

  // Ce que demande l'offre : la valeur, la preuve (le bout de phrase de l'offre) et qui l'a lu
  function luHTML(l, t) {
    const { esc } = O, items = liste(l).filter((i) => texteSur(i?.nom));
    if (!items.length) return '';
    return `<h2 class="jm-h2">${esc(t('lu_t'))}</h2><div class="jm-lu">${items.map((i) => {
      const preuve = texteSur(i.preuve);
      return `<div class="jm-lu-l${i.dit === true ? '' : ' pas'}"><span class="jm-lu-n">${esc(texteSur(i.nom))}</span><b>${esc(texteSur(i.valeur))}</b>${preuve ? `<q title="${esc(preuve)}">${esc(preuve)}</q>` : ''}${texteSur(i.par) ? `<small>${esc(texteSur(i.par))}</small>` : ''}</div>`;
    }).join('')}</div>`;
  }

  function listeOffresHTML(o, x) {
    const { esc } = O, t = tO(x);
    const recherches = liste(x.recherches).filter((r) => numSur(r?.id));
    const enCours = typeof x.en_cours === 'string' && x.en_cours;
    // (étape 5 bis : la source, mes offres ou un des quatre sites, en haut de la liste)
    let h = `<div class="jm-rch">${sourcesHTML()}`;
    if (recherches.length) {
      h += `<div class="jm-rch-l"><label class="jm-rch-choix"><span>${esc(t('mes_recherches'))}</span><select class="f" data-perso="of-recherche" data-k="or:choix">${recherches.map((r) => `<option value="${r.id}"${r.id === x.recherche ? ' selected' : ''}>${esc(texteSur(r.nom))}</option>`).join('')}</select></label>
          <button type="button" class="jm-rond f${ofVue === 'recherches' ? ' on' : ''}" data-perso="of-gerer" data-k="or:gerer" title="${esc(t('gerer'))}" aria-label="${esc(t('gerer'))}">${icone('reglages')}</button></div>`;
      if (x.recherche_sous) h += `<p class="jm-rch-sous">${esc(texteSur(x.recherche_sous))}</p>`;
      h += `<button type="button" class="jm-bt principal jm-rch-go f" data-perso="of-chercher" data-k="or:chercher"${enCours ? ' disabled' : ''}>${icone('loupe')}<span>${esc(t('chercher_aj'))}</span></button>`;
      if (enCours) h += `<div class="jm-attente petite" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(enCours)}</span></div>`;
      // (ce qu'a donné la dernière fois : l'annonce le dit déjà quand elle est là)
      else if (x.recherche_fois && !texteSur(x.annonce?.texte)) h += `<p class="jm-rch-fois">${esc(texteSur(x.recherche_fois))}</p>`;
    } else {
      h += `<p class="jm-rch-sous">${esc(t('sans_recherche'))}</p><div class="jm-btns serre">${bouton('of-nouvelle', 'or:nouvelle', t('nouvelle'), 'plus', '')}</div>`;
    }
    const a = x.annonce && typeof x.annonce === 'object' ? x.annonce : null;
    if (a && !enCours && texteSur(a.texte)) {
      const g = ['ok', 'attention', 'erreur'].includes(a.genre) ? a.genre : 'ok';
      h += `<div class="jm-annonce ${g}" role="status">${icone(g === 'ok' ? 'coche' : 'attention')}<div><span>${esc(texteSur(a.texte))}</span>${a.cles === true ? bouton('of-cles', 'or:cles', t('ouvrir_cles'), 'cle', 'discret') : ''}</div></div>`;
    }
    // (étape 6 : Claude lit les offres floues, ou la clé qui lui manque)
    const ia = objet(x.ia), iaA = objet(ia.annonce);
    if (typeof ia.en_cours === 'string' && ia.en_cours) {
      h += `<div class="jm-attente petite" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(ia.en_cours)}</span></div>`;
    } else {
      if (texteSur(iaA.texte)) {
        const g = ['ok', 'attention', 'erreur'].includes(iaA.genre) ? iaA.genre : 'ok';
        h += `<div class="jm-annonce ${g}" role="status">${icone(g === 'ok' ? 'coche' : 'attention')}<div><span>${esc(iaA.texte)}</span>${iaA.cles === true ? bouton('of-cles', 'or:ia-cles', t('ouvrir_cles'), 'cle', 'discret') : ''}</div></div>`;
      }
      if (numSur(ia.floues) && texteSur(ia.texte) && !(iaA.cles === true)) {
        h += `<div class="jm-ia">${icone(ia.claude === true ? 'loupe' : 'cle')}<div><span>${esc(ia.texte)}</span>${ia.claude === true ? bouton('of-lire-ia', 'or:ia', t('lire_floues'), 'droite', 'discret') : bouton('of-cles', 'or:ia-cle', t('ouvrir_cles'), 'cle', 'discret')}</div></div>`;
      }
    }
    if (texteSur(x.sans_note)) h += `<p class="jm-rch-sous jm-sans-note">${esc(x.sans_note)}</p>`;
    h += '</div>';
    const lignes = liste(x.lignes).filter((l) => numSur(l?.id));
    const ouverte = ofChoisie ?? x.fiche?.id;
    if (lignes.length) h += `<div class="jm-liste-tete"><span class="jm-b-sel jm-tri"><select class="f" data-perso="of-champ" data-champ="tri" data-k="of:tri" aria-label="${esc(t('tri'))}">${optionsHTML(x.tris, ofFiltres?.tri || 'pertinence', codeSurP)}</select>${icone('bas')}</span></div>`;
    for (const l of lignes) {
      const sel = l.id === ouverte && ofVue === 'offre';
      const marques = (l.postule === true ? `<span class="jm-o-m ok" title="${esc(t('postule'))}">${icone('coche')}</span>` : '')
        + (l.favori === true ? `<span class="jm-o-m" title="${esc(t('favori'))}">${icone('etoile')}</span>` : '')
        + (l.plus_tard === true ? `<span class="jm-o-m" title="${esc(t('plus_tard'))}">${icone('horloge')}</span>` : '')
        + (l.nouvelle === true ? `<span class="jm-neuf" title="${esc(t('nouvelle_l'))}"></span>` : '');
      h += `<button type="button" class="jm-cand jm-off f${sel ? ' sel' : ''}${l.cachee === true ? ' fini' : ''}" data-perso="of-ouvrir" data-id="${l.id}" data-k="ol:${l.id}" aria-current="${sel}">
        <span class="jm-c1"><b>${esc(texteSur(l.titre))}</b>${marques}</span>
        <span class="jm-c2">${esc(texteSur(l.sous))}</span><span class="jm-c3 pale">${esc(texteSur(l.l3))}</span>${pastilleNote(l.note) ? `<span class="jm-c4">${pastilleNote(l.note)}<span>${esc(texteSur(l.raison))}</span></span>` : ''}</button>`;
    }
    if (!lignes.length && texteSur(x.vide_liste)) h += `<p class="jm-vide-l">${esc(texteSur(x.vide_liste))}</p>`;
    if (x.plus) h += `<p class="jm-vide-l petit">${esc(texteSur(x.plus))}</p>`;
    return h;
  }

  // ---------------------------------------------------------------- L'envoi par mail (étape 10)
  // « Envoyer par mail » ('en-ouvrir') dans « Ma candidature » : Rust ouvre le récapitulatif (P.envoi) : de (la boîte
  // Gmail), à (l'adresse lue dans l'offre, ou tapée), objet, la lettre gardée, le CV joint. Rien ne part sans le clic
  // sur « Envoyer » ('en-envoyer') : Rust vérifie tout, note l'envoi, l'envoie sur un fil à part, puis renvoie ce qu'il a
  // donné (parti : le suivi ; pas parti ; incertain : Jordan dit ce qu'il voit dans Gmail, 'en-parti' ou 'en-pas-parti').
  let enSaisie = null;   // ce que Jordan tape : { offre, a, objet } (jamais renvoyé par Rust tant qu'il tape)
  let enVu = null;       // l'offre du récapitulatif dessiné en dernier
  let enFini = null;     // ce qu'il montrait (rien, « parti »...) : le choix suit quand il change
  let enJusqua = 0;      // (un double clic sur « Envoyer » : un seul envoi)
  let enAttCle = null;   // l'attente reçue de Rust (une minute entre deux envois), et quand elle finit ici
  let enAttFin = 0;
  // (l'attente se compte ici : à sa fin, le récapitulatif est redessiné et « Envoyer » revient)
  const enAttend = (e) => {
    const a = objet(e.attente), n = Number(a.secondes);
    if (!texteSur(a.texte) || !Number.isFinite(n)) { enAttCle = null; return false; }
    if (n <= 0) return true;
    const cle = `${e.offre}:${a.texte}`;
    if (cle !== enAttCle) { enAttCle = cle; enAttFin = Date.now() + n * 1000; setTimeout(() => { if (enOuvert()) dessinerTout(); }, n * 1000 + 250); }
    return Date.now() < enAttFin;
  };
  const enDe = () => { const e = objet(P?.envoi); return numSur(e.offre) ? e : null; };
  const enOuvert = () => { const e = enDe(); return !!e && e.ouvert === true && onglet === 'offres' && !sourceSite(); };
  const enIssue = (e) => (codeSur(e?.issue?.genre) ? e.issue.genre : (e?.en_cours === true ? 'cours' : (e?.incertain ? 'incertain' : null)));
  const enSaisieDe = (e) => {
    if (!enSaisie || enSaisie.offre !== e.offre) enSaisie = { offre: e.offre, a: texteSur(e.a), objet: texteSur(e.objet) };
    return enSaisie;
  };

  // Dans « Ma candidature » : « Envoyer par mail », ou « Envoyée par mail le ... »
  function envoiBarHTML() {
    const e = enDe();
    if (!e || !texteSur(e.lettre)) return '';
    const { esc } = O, t = tO(e);
    let h = '<div class="jm-envoi-bar">';
    if (e.envoyee) h += `<div class="jm-envoi-fait">${icone('coche')}<span>${esc(texteSur(e.envoyee.texte))}</span></div>`;
    h += bouton('en-ouvrir', 'en:ouvrir', e.envoyee ? t('par_mail') : t('envoyer_mail'), 'envoi', e.envoyee ? '' : 'principal');
    h += `<span class="jm-petit">${esc(texteSur(e.a) ? t('note_mail') : t('note_sans'))}</span>`;
    if (e.envoyee && numSur(e.envoyee.candidature)) h += bouton('en-dossier', 'en:voir', t('voir_dossier'), 'dossier', 'discret', ` data-id="${e.envoyee.candidature}"`);
    return `${h}</div>`;
  }

  function annonceEnHTML(genre, texte, boutons = '') {
    return `<div class="jm-annonce ${genre} jm-e-ann" role="status">${icone(genre === 'ok' ? 'coche' : 'attention')}<div><span>${O.esc(texte)}</span>${boutons ? `<div class="jm-btns serre">${boutons}</div>` : ''}</div></div>`;
  }

  // Le récapitulatif avant l'envoi (maquette C : « Envoyer ma candidature », par mail)
  function recapHTML(e) {
    const { esc } = O, t = tO(e), s = enSaisieDe(e);
    let h = `<div class="jm-envoi${enVu === e.offre ? '' : ' entree'}" role="dialog" aria-labelledby="jm-e-t"><div class="jm-e-haut"><div class="jm-e-t"><b id="jm-e-t">${esc(t('envoyer_t'))}</b><span>${esc([texteSur(e.titre), texteSur(e.entreprise)].filter(Boolean).join(', '))}</span></div>
      <span class="jm-e-facon">${icone('mail')}${esc(t('par_mail'))}</span>${bouton('en-fermer', 'en:fermer', t('fermer'), 'croix', 'second')}</div><div class="jm-e-corps">`;
    const issue = objet(e.issue);
    if (issue.genre === 'parti') {
      h += `<div class="jm-e-fini"><div class="jm-e-rond">${icone('coche')}</div><b>${esc(t('parti_t'))}</b><p>${esc(texteSur(issue.texte))}</p>
        <div class="jm-btns">${bouton('en-dossier', 'en:dossier', t('voir_dossier'), 'dossier', 'principal', ` data-id="${Number(issue.candidature) || 0}"`)}${bouton('en-fermer', 'en:revenir', t('revenir'), 'gauche', 'second')}</div></div></div></div>`;
      return h;
    }
    h += '<div class="jm-mail">';
    if (e.boite !== true) h += annonceEnHTML('attention', `${t('boite_t')}. ${t('boite_s')}`, bouton('en-boite', 'en:boite', t('ouvrir_boite'), 'mail', 'discret'));
    if (e.incertain) h += annonceEnHTML('attention', texteSur(e.incertain.texte), `${bouton('en-parti', 'en:oui', t('oui_parti'), 'coche', 'principal', ` data-tente="${Number(e.incertain.tente) || 0}"`)}${bouton('en-pas-parti', 'en:non', t('non_parti'), 'croix', 'second', ` data-tente="${Number(e.incertain.tente) || 0}"`)}`);
    if (issue.genre === 'echec') h += annonceEnHTML('erreur', texteSur(issue.texte));
    const avs = liste(e.a_verifier).filter((x) => texteSur(x));
    if (avs.length) h += annonceEnHTML('attention', texteSur(e.a_verifier_t), bouton('en-fermer', 'en:lettre', t('revoir'), 'gauche', 'discret')).replace('</span>', `</span>${avs.map((x) => `<q class="jm-e-q">${esc(x)}</q>`).join('')}`);
    const cv = objet(e.cv), cvOk = texteSur(cv.nom) && cv.present === true;
    if (!cvOk) h += annonceEnHTML('attention', texteSur(cv.nom) ? t('cv_absent') : t('sans_cv'), bouton('en-cv', 'en:cv', t('choisir_cv'), 'importer', 'principal'));
    const attend = enAttend(e);
    if (attend && !e.incertain) h += annonceEnHTML('attention', texteSur(e.attente.texte));
    if (e.envoyee && !e.incertain) h += annonceEnHTML('ok', `${t('deja')} : ${texteSur(e.envoyee.texte).replace(/^Envoyée /, '')}.`);
    const de = texteSur(e.de);
    h += `<div class="jm-m-ch"><span>${esc(t('de'))}</span><b>${esc(de || '...')}</b></div>`;
    h += `<label class="jm-m-ch"><span>${esc(t('a'))}</span><input class="f" type="email" data-perso="en-champ" data-champ="a" data-k="en:a" maxlength="200" autocomplete="off" spellcheck="false" placeholder="${esc(t('i_a'))}" value="${esc(s.a)}"></label>`;
    const lue = objet(e.a_lue);
    if (texteSur(lue.extrait) && s.a === texteSur(lue.adresse)) h += `<p class="jm-m-lue">${icone('loupe')}<span><b>${esc(t('lue'))}</b> « ${esc(lue.extrait)} »</span></p>`;
    else if (!texteSur(e.a) && !s.a) h += `<p class="jm-m-lue">${icone('attention')}<span>${esc(t('note_sans'))}</span></p>`;
    h += `<label class="jm-m-ch"><span>${esc(t('objet'))}</span><input class="f" data-perso="en-champ" data-champ="objet" data-k="en:objet" maxlength="200" autocomplete="off" spellcheck="true" value="${esc(s.objet)}"></label>`;
    if (cvOk) h += `<div class="jm-m-ch"><span>${esc(t('cv'))}</span><span class="jm-pj">${icone('cv')}<span>${esc(cv.nom)}</span>${texteSur(cv.taille) ? `<small>${esc(cv.taille)}</small>` : ''}</span></div>`;
    h += `<div class="jm-m-txt" data-brut>${esc(texteSur(e.lettre) || t('pas_lettre'))}</div>`;
    h += `<p class="jm-petit jm-m-garde">${texteSur(e.gardee) ? `<b>${esc(e.gardee)}.</b> ` : ''}${esc(t('modifier_lettre'))}</p>`;
    if (e.en_cours === true) h += `<div class="jm-attente" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(t('en_cours'))}</span></div>`;
    h += '</div></div>';
    const bloque = liste(e.a_verifier).length > 0 || e.occupe === true || e.boite !== true || !!e.incertain || !(cv.present === true) || !texteSur(e.lettre) || attend;
    const nom = e.envoyee ? t('encore') : issue.genre === 'echec' ? t('reessayer') : t('envoyer');
    h += `<div class="jm-e-bas">${icone('cadenas')}<span>${esc(t('cadenas'))}</span>${bouton('en-copier', 'en:copier', t('copier'), 'copie', 'discret')}${bouton('en-envoyer', 'en:envoyer', nom, 'envoi', 'principal', bloque ? ' disabled' : '')}</div></div>`;
    return h;
  }

  // Un geste de l'envoi ; vrai s'il en était un
  function validerEnvoi(el, d) {
    const e = enDe();
    if (!e) return true;
    switch (d.perso) {
      case 'en-ouvrir': O.son?.ok?.(); enSaisie = null; O.action('en-ouvrir', { id: e.offre }); return true;
      case 'en-fermer': O.son?.retour?.(); O.action('en-fermer'); return true;
      case 'en-champ': if (d.champ === 'a') hote.querySelector('[data-k="en:objet"]')?.focus(); else hote.querySelector('[data-k="en:envoyer"]')?.focus(); return true;
      case 'en-envoyer': {
        if (el.disabled || Date.now() < enJusqua || e.occupe === true) return true;
        enJusqua = Date.now() + 1200; el.disabled = true; O.son?.ok?.();
        const s = enSaisieDe(e);
        O.action('en-envoyer', { id: e.offre, version: Number(e.version) || 0, a: String(s.a || '').slice(0, 200), objet: String(s.objet || '').slice(0, 200), encore: !!e.envoyee });
        return true;
      }
      case 'en-parti': case 'en-pas-parti': { const n = Number(d.tente); if (numSur(n) && !el.disabled) { el.disabled = true; O.son?.ok?.(); O.action(d.perso, { tente: n }); } return true; }
      case 'en-copier': copierBrut(texteSur(e.lettre), el, () => {}); return true;
      case 'en-cv': O.son?.ok?.(); O.action('cv-importer'); return true;
      case 'en-boite': O.son?.ok?.(); O.action('en-fermer'); montrer('profil', 'r:mail'); ouvrirRubrique('mail'); return true;
      case 'en-dossier': { const k = Number(d.id); if (numSur(k)) { O.son?.ok?.(); O.action('en-fermer'); montrer('candidatures'); ouvrirCandidature(k); } return true; }
      default: return true;
    }
  }

  // ---------------------------------------------------------------- L'envoi sur le site (étape 11)
  // « Postuler sur le site » ('po-ouvrir') dans « Ma candidature » : Rust charge le formulaire du site en coulisse, le
  // remplit et le relit (P.postuler.ouvert) : chaque champ du site, ce que Jobmind y a mis, d'où ça vient, et ce que
  // Jordan doit compléter. Ce qu'il tape reste ici (poSaisie) jusqu'à « Mettre mes réponses sur le site »
  // ('po-remplir'). Rien n'est validé sur le site sans son clic sur « Envoyer » ('po-envoyer'). LinkedIn : la petite
  // fenêtre du site, avec ses textes à côté, et « C'est envoyé » ('po-fait').
  let poSaisie = null;   // { offre, v: { rang: valeur } }
  let poVu = null;       // l'offre et la phase dessinées en dernier (le choix suit quand elles changent)
  const poDe = () => { const p = objet(P?.postuler); return numSur(p.offre) ? p : null; };
  const poOuvert = () => { const p = poDe(); return p && p.ouvert && typeof p.ouvert === 'object' && onglet === 'offres' && !sourceSite() ? p : null; };
  const poSaisieDe = (p) => { if (!poSaisie || poSaisie.offre !== p.offre) poSaisie = { offre: p.offre, v: {} }; return poSaisie; };
  const poEnAttente = (p) => Object.keys(poSaisieDe(p).v).length > 0;
  const tPo = (o) => { const t = objet(o?.textes); return (k) => texteSur(t[k]); };

  // Dans « Ma candidature » : « Postuler sur Indeed », ou « Envoyée sur le site le ... », ou un envoi incertain
  function poBarHTML() {
    const p = poDe(), e = enDe();
    if (!p || !e || !texteSur(e.lettre)) return '';
    const { esc } = O;
    let h = '<div class="jm-envoi-bar jm-po-bar">';
    if (p.envoyee) h += `<div class="jm-envoi-fait">${icone('coche')}<span>${esc(texteSur(p.envoyee.texte))}</span></div>`;
    if (p.incertain) {
      const n = Number(p.incertain.tente) || 0;
      h += annonceEnHTML('attention', texteSur(p.incertain.texte), `${bouton('po-fait', 'po:fait-i', "C'est envoyé", 'coche', 'principal', ` data-tente="${n}"`)}${bouton('po-pas-parti', 'po:non-i', "Ce n'est pas parti", 'croix', 'second', ` data-tente="${n}"`)}`);
    }
    const principal = !texteSur(e.a) && !p.envoyee;
    h += bouton('po-ouvrir', 'po:ouvrir', texteSur(p.bouton), p.linkedin === true ? 'ext' : 'globe', principal ? 'principal' : '', p.occupe === true ? ' disabled' : '');
    h += `<span class="jm-petit">${esc(texteSur(p.note))}</span>`;
    return `${h}</div>`;
  }

  // Une valeur que Jordan peut taper ou choisir dans le récapitulatif
  function poChampHTML(l, s) {
    const { esc } = O, i = Number(l.i), k = `po:c-${i}`;
    const val = Object.hasOwn(s.v, i) ? String(s.v[i]) : texteSur(l.voulu ?? '');
    const choix = liste(l.choix).filter((c) => typeof c?.v === 'string');
    const lab = ` aria-label="${esc(texteSur(l.libelle))}"`;
    if (l.sorte === 'liste' || l.sorte === 'ronds') {
      return `<select class="f jm-po-in" data-perso="po-champ" data-i="${i}" data-k="${k}"${lab}><option value="">Choisir</option>${choix.map((c) => `<option value="${esc(c.v)}"${c.v === val ? ' selected' : ''}>${esc(texteSur(c.t))}</option>`).join('')}</select>`;
    }
    if (l.sorte === 'case' || l.sorte === 'cases') {
      const pris = val.split('\n');
      const opts = l.sorte === 'case' ? [{ v: '1', t: 'Oui' }] : choix;
      return `<div class="jm-po-puces">${opts.map((c, n) => { const oui = l.sorte === 'case' ? val === '1' : pris.includes(c.v); return `<button type="button" role="switch" aria-checked="${oui}" class="jm-puce petite f${oui ? ' oui' : ''}" data-perso="po-puce" data-i="${i}" data-v="${esc(c.v)}" data-k="po:p-${i}-${n}">${icone(oui ? 'coche' : 'plus')}${esc(texteSur(c.t))}</button>`; }).join('')}</div>`;
    }
    if (l.sorte === 'zone') return `<textarea class="f jm-po-in jm-po-zone" data-perso="po-champ" data-i="${i}" data-k="${k}" maxlength="8000"${lab}>${esc(val)}</textarea>`;
    return `<input class="f jm-po-in" data-perso="po-champ" data-i="${i}" data-k="${k}" maxlength="300" autocomplete="off" spellcheck="false" value="${esc(val)}"${lab}>`;
  }

  // Une ligne du récapitulatif : le champ du site, ce qui y est, d'où ça vient
  function poLigneHTML(l, s, t) {
    const { esc } = O, i = Number(l.i);
    const etat = codeSur(l.etat) ? l.etat : 'ok';
    const ed = ['a_completer', 'facultatif', 'different'].includes(etat) && l.sorte !== 'fichier' && l.genre !== 'consentement' && l.genre !== 'autre';
    const modif = l.source === 'Tapé par toi' && l.sorte !== 'fichier';
    let v;
    if (etat === 'accord') {
      const oui = s.v[i] === '1';
      v = `<button type="button" role="switch" aria-checked="${oui}" class="jm-puce f${oui ? ' oui' : ''}" data-perso="po-puce" data-i="${i}" data-v="1" data-k="po:a-${i}">${icone(oui ? 'coche' : 'plus')}${esc(t('accord'))}</button><small>${esc(t('accord_s'))}</small>`;
    } else if (etat === 'a_joindre') {
      v = `<span class="jm-po-note">${esc(t('a_joindre'))}</span>`;
    } else if (ed || modif) {
      v = poChampHTML(l, s);
      if (etat === 'different') v += `<small class="jm-po-diff">${esc(t('different'))}</small>`;
    } else if (l.sorte === 'fichier') {
      v = `<span class="jm-pj">${icone('cv')}<span>${esc(texteSur(l.valeur))}</span></span>`;
    } else {
      v = `<span class="jm-po-v${l.long === true ? ' long' : ''}" data-brut>${esc(texteSur(l.valeur))}</span>`;
    }
    const ic = etat === 'ok' ? icone('coche') : (etat === 'facultatif' ? '' : icone('attention'));
    const src = etat === 'ok' && texteSur(l.source) ? `<small class="jm-po-src">${esc(l.source)}</small>` : (etat === 'a_completer' ? `<small class="jm-po-src a">${esc(t('a_completer'))}</small>` : '');
    return `<div class="jm-po-l ${etat}"><div class="jm-po-lib"><b>${esc(texteSur(l.libelle))}</b>${l.obligatoire === true ? `<small>${esc(t('obligatoire'))}</small>` : ''}</div><div class="jm-po-val">${v}</div><div class="jm-po-et">${ic}${src}</div></div>`;
  }

  // Le récapitulatif avant l'envoi sur le site (maquette C, comme l'envoi par mail)
  function poRecapHTML(p) {
    const { esc } = O, o = objet(p.ouvert), t = tPo(o), s = poSaisieDe(p);
    let h = `<div class="jm-envoi${poVu && poVu.startsWith(`${p.offre}:`) ? '' : ' entree'}" role="dialog" aria-labelledby="jm-po-t"><div class="jm-e-haut"><div class="jm-e-t"><b id="jm-po-t">${esc(t('titre'))}</b><span>${esc([texteSur(o.titre), texteSur(o.entreprise)].filter(Boolean).join(', '))}</span></div>
      <span class="jm-e-facon">${icone('globe')}${esc(t('facon'))}</span>${bouton('po-fermer', 'po:fermer', t('fermer'), 'croix', 'second')}</div><div class="jm-e-corps">`;
    const parti = objet(o.parti);
    if (o.phase === 'parti' && numSur(parti.candidature)) {
      h += `<div class="jm-e-fini"><div class="jm-e-rond">${icone('coche')}</div><b>${esc(t('parti_t'))}</b><p>${esc(texteSur(parti.texte))}</p>${texteSur(parti.preuve) ? `<p class="jm-po-preuve">${esc(parti.preuve)}</p>` : ''}
        <div class="jm-btns">${bouton('po-dossier', 'po:dossier', t('voir_dossier'), 'dossier', 'principal', ` data-id="${parti.candidature}"`)}${bouton('po-fermer', 'po:revenir', t('revenir'), 'gauche', 'second')}</div></div></div></div>`;
      return h;
    }
    h += '<div class="jm-mail jm-po">';
    if (texteSur(o.etape)) h += `<p class="jm-po-etape">${esc(o.etape)}</p>`;
    const a = objet(o.annonce);
    if (texteSur(a.texte)) {
      const g = a.genre === 'connexion' ? 'attention' : (['ok', 'attention', 'erreur'].includes(a.genre) ? a.genre : 'attention');
      let b = '';
      if (a.genre === 'connexion' && codeSurP(a.site)) b = `${bouton('cs-connexion', 'po:cx', t('connecter'), 'cadenas', 'principal', ` data-id="${a.site}"`)}${bouton('cs-page-site', 'po:cx-site', t('page_site'), 'globe', 'second', ` data-id="${a.site}"`)}${bouton('po-ouvrir', 'po:reessayer', t('reessayer'), 'recharger', 'second')}`;
      else if (o.phase === 'bloque') b = `${bouton('po-moi', 'po:voir', t('voir'), 'ext', 'second')}${bouton('po-ouvrir', 'po:reessayer', t('reessayer'), 'recharger', 'second')}`;
      h += annonceEnHTML(g, texteSur(a.texte), b);
    }
    if (o.phase === 'incertain') {
      h += `<div class="jm-btns jm-po-inc">${bouton('po-moi', 'po:voir', t('voir'), 'ext', 'second')}${bouton('po-fait', 'po:fait', t('fait'), 'coche', 'principal')}${bouton('po-pas-parti', 'po:non', t('pas_parti'), 'croix', 'second')}</div>`;
    }
    if (typeof o.attente === 'string' && o.attente) h += `<div class="jm-attente" role="status"><span class="jm-roue" aria-hidden="true"></span><span>${esc(o.attente)}</span></div>`;
    const lignes = liste(o.lignes).filter((l) => Number.isInteger(l?.i));
    const manque = lignes.filter((l) => ['a_completer', 'accord', 'a_joindre'].includes(l.etat));
    if (manque.length && o.phase === 'pret') h += annonceEnHTML('attention', `${t('manque')} ${manque.map((l) => `« ${texteSur(l.libelle)} »`).join(', ')}`);
    if (lignes.length && o.phase !== 'attente') {
      if (o.captcha === true) h += annonceEnHTML('attention', t('captcha'));
      if (!o.bouton && o.phase === 'pret') h += annonceEnHTML('attention', t('sans_bouton'));
      h += `<h2 class="jm-h2">${esc(t('champs_t'))}</h2><p class="jm-petit">${esc(t('champs_s'))}</p><div class="jm-po-ls">${lignes.map((l) => poLigneHTML(l, s, t)).join('')}</div>`;
    }
    const faites = liste(o.faites).filter((x) => texteSur(x?.libelle));
    if (faites.length) h += `<h2 class="jm-h2">${esc(t('faites_t'))}</h2><div class="jm-po-ls passees">${faites.map((x) => `<div class="jm-po-l ok"><div class="jm-po-lib"><b>${esc(x.libelle)}</b></div><div class="jm-po-val"><span class="jm-po-v long" data-brut>${esc(texteSur(x.valeur))}</span></div><div class="jm-po-et">${icone('coche')}<small class="jm-po-src">${esc(texteSur(x.source))}</small></div></div>`).join('')}</div>`;
    h += '</div></div>';
    const attend = poEnAttente(p);
    const nom = o.bouton?.suivant === true ? t('continuer') : t('envoyer');
    const peut = o.peut === true && !attend && o.phase === 'pret';
    // (jamais deux fois le même bouton : quand la phrase du haut propose déjà la page du site, le bas ne la redit pas)
    const voirEnHaut = o.phase === 'bloque' && texteSur(a.texte) && a.genre !== 'connexion';
    const moi = ['pret', 'bloque'].includes(o.phase) && !voirEnHaut ? bouton('po-moi', 'po:moi', t('moi'), 'ext', 'discret') : '';
    h += `<div class="jm-e-bas">${icone('cadenas')}<span>${esc(t('cadenas'))}</span>${moi}${bouton('po-remplir', 'po:remplir', t('remplir'), 'envoi', 'second', attend && o.phase === 'pret' ? '' : ' hidden')}${o.phase === 'pret' ? bouton('po-envoyer', 'po:envoyer', nom, o.bouton?.suivant === true ? 'droite' : 'envoi', 'principal', peut ? '' : ' disabled') : ''}</div></div>`;
    return h;
  }

  // Après une frappe : « Mettre mes réponses sur le site » paraît, « Envoyer » attend
  function poBas() {
    const p = poDe(); if (!p) return;
    const attend = poEnAttente(p);
    const r = hote.querySelector('[data-k="po:remplir"]'); if (r) r.hidden = !attend;
    const e = hote.querySelector('[data-k="po:envoyer"]'); if (e && attend) e.disabled = true;
  }

  // Un geste de l'envoi sur le site ; vrai s'il en était un
  function validerPo(el, d) {
    const p = poDe();
    if (!p) return true;
    // (la main va au bouton : le kit ne redessine pas tant qu'une case de saisie a la main)
    if (d.perso !== 'po-champ' && /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement?.tagName || '')) el.focus?.({ preventScroll: true });
    const s = poSaisieDe(p), i = Number(d.i);
    switch (d.perso) {
      case 'po-ouvrir': if (!el.disabled) { O.son?.ok?.(); poSaisie = null; O.action('po-ouvrir', { id: p.offre }); } return true;
      case 'po-fermer': O.son?.retour?.(); poSaisie = null; O.action('po-fermer'); return true;
      case 'po-champ': if (el.tagName === 'INPUT') hote.querySelector(poEnAttente(p) ? '[data-k="po:remplir"]' : '[data-k="po:envoyer"]')?.focus(); return true;
      case 'po-puce': {
        if (!Number.isInteger(i)) return true;
        const l = liste(p.ouvert?.lignes).find((x) => x?.i === i);
        const v = String(d.v || '');
        if (l?.sorte === 'cases') {
          const pris = String(s.v[i] ?? (l.voulu || '')).split('\n').filter(Boolean);
          s.v[i] = (pris.includes(v) ? pris.filter((x) => x !== v) : [...pris, v]).join('\n');
        } else s.v[i] = s.v[i] === '1' ? '0' : '1';
        O.son?.bascule?.(); dessinerTout(el.dataset.k); return true;
      }
      case 'po-remplir': if (poEnAttente(p) && Date.now() >= enJusqua) { enJusqua = Date.now() + 800; O.son?.ok?.(); const v = { ...s.v }; s.v = {}; O.action('po-remplir', { saisies: v }); } return true;
      case 'po-envoyer': if (!el.disabled && Date.now() >= enJusqua) { enJusqua = Date.now() + 1500; el.disabled = true; O.son?.ok?.(); O.action('po-envoyer'); } return true;
      case 'po-moi': O.son?.ok?.(); O.action('po-voir'); return true;
      case 'po-fait': case 'po-pas-parti': if (!el.disabled) { el.disabled = true; O.son?.ok?.(); O.action(d.perso, { tente: Number(d.tente) || 0 }); } return true;
      case 'po-dossier': { const k = Number(d.id); if (numSur(k)) { O.son?.ok?.(); O.action('po-fermer'); montrer('candidatures'); ouvrirCandidature(k); } return true; }
      case 'po-copier': {
        const c = objet(cs().petite?.cote);
        const texte = d.quoi === 'lettre' ? texteSur(c.lettre) : texteSur(liste(c.questions)[Number(d.n)]?.reponse);
        if (texte) copierBrut(texte, el, () => {});
        return true;
      }
      default: return true;
    }
  }

  function ficheOffreHTML(x, f) {
    const { esc } = O, t = tO(x);
    const rond = (perso, k, ic, aide, on) => `<button type="button" class="jm-rond f${on ? ' on' : ''}" data-perso="${perso}" data-k="${k}" title="${esc(aide)}" aria-label="${esc(aide)}" aria-pressed="${!!on}">${icone(ic)}</button>`;
    let h = `<div class="jm-d jm-of${O.anime ? ' entree' : ''}"><div class="jm-d-tete"><div class="jm-d-titres"><h1>${esc(texteSur(f.titre))}</h1><div class="jm-d-sous">${esc(texteSur(f.sous))}</div></div>
      <div class="jm-o-actions">${rond('of-favori', 'ov:favori', 'etoile', t('favori'), f.favori === true)}${rond('of-tard', 'ov:tard', 'horloge', t('plus_tard'), f.plus_tard === true)}</div></div>`;
    h += `<div class="jm-o-src"><span>${esc(texteSur(f.sources))}</span>`;
    if (f.adzuna === true) h += `<button type="button" class="jm-adzuna f" data-perso="of-adzuna" data-k="ov:adzuna" title="${esc(t('adzuna_aide'))}"><small>${esc(t('par'))}</small><b>${esc(t('adzuna'))}</b></button>`;
    h += '</div>';
    if (f.double === true) h += `<p class="jm-o-double">${icone('copie')}<span>${esc(t('doublon'))}</span></p>`;
    // (étape 5 : « 34 candidats sur LinkedIn, vu le 18 nov. »)
    if (texteSur(f.candidats)) h += `<p class="jm-o-double jm-o-cand">${icone('profil')}<span>${esc(texteSur(f.candidats))}</span></p>`;
    // (étape 6 : la note et sa raison, hors sujet)
    const n = objet(f.note);
    if (pastilleNote(n)) h += `<div class="jm-pourquoi ${n.code}" title="${esc(t('pourquoi'))}">${pastilleNote(n)}<span>${esc(texteSur(n.raison))}</span></div>`;
    if (f.hors_sujet === true) {
      const pas = f.pas_hors_sujet === true;
      h += `<div class="jm-hors">${icone(pas ? 'coche' : 'attention')}<span>${esc(pas ? t('pas_hors_sujet_t') : t('hors_sujet_t'))}</span>${bouton('of-pas-hors', 'ov:hors', pas ? t('remettre_hors_sujet') : t('pas_hors_sujet'), pas ? 'cacher' : 'coche', 'discret')}</div>`;
    }
    // (étape 8 : « L'offre » et « Ma candidature »)
    const p = prDe(f);
    if (p) {
      h += pilulesOffreHTML(f, p);
      if (ofPil === 'cand') return `${h}${candidatureHTML(p)}</div>`;
    }
    h += `<div class="jm-infos">${liste(f.infos).map((i) => `<div><span>${esc(texteSur(i?.nom))}</span><b>${esc(texteSur(i?.valeur))}</b></div>`).join('')}</div>`;
    h += luHTML(f.lecture, t);
    if (f.ia === 'lire') h += `<div class="jm-ia">${icone('loupe')}<div><span>${esc(t('lire_ia_t'))}</span>${bouton('of-lire-ia', 'ov:ia', t('lire_ia'), 'droite', 'discret', ` data-id="${f.id}"`)}</div></div>`;
    else if (f.ia === 'cle') h += `<div class="jm-ia">${icone('cle')}<div><span>${esc(t('cle_ia_t'))}</span>${bouton('of-cles', 'ov:ia-cle', t('ouvrir_cles'), 'cle', 'discret')}</div></div>`;
    if (p) h += annoncePrepHTML(p) + actionPrepHTML(p);
    const liens = liste(f.liens).filter((l) => codeSurP(l?.source));
    const voir = liens.map((l, i) => bouton('of-voir', `ov:voir-${l.source}`, texteSur(l.nom), 'ext', i ? '' : 'principal', ` data-source="${l.source}"`)).join('');
    if (numSur(f.candidature)) {
      h += `<div class="jm-action calme-o"><span class="jm-a-ic">${icone('coche')}</span><div class="jm-a-txt"><b>${esc(t('postule_deja'))}</b></div><div class="jm-a-bts">${bouton('of-candidature', 'ov:cand', t('voir_candidature'), 'dossier', '')}</div></div>`;
      if (voir) h += `<div class="jm-btns">${voir}</div>`;
    } else {
      h += `<div class="jm-btns jm-o-bts">${voir}${bouton('of-postule', 'ov:postule', t('postule'), 'envoi', '')}</div>`;
    }
    const texte = texteSur(f.texte);
    h += `<h2 class="jm-h2">${esc(t('texte_t'))}</h2>`;
    h += texte ? `<div class="jm-o-texte" data-brut>${esc(texte)}</div><p class="jm-petit">${esc(texteSur(f.note))}</p>` : `<p class="jm-panneau-vide">${esc(texteSur(f.texte_vide))}</p>`;
    h += `<p class="jm-petit">${esc(texteSur(f.gardee))}</p>`;
    h += `<div class="jm-d-bas">${f.cachee === true ? bouton('of-cacher', 'ov:cacher', t('decacher'), 'oeil', 'discret') : bouton('of-cacher', 'ov:cacher', t('cacher'), 'cacher', 'discret')}</div>`;
    return `${h}</div>`;
  }

  function videOffresHTML(x) {
    const { esc } = O, t = tO(x), v = objet(x.vide);
    const b = { cles: ['of-cles', 'ov:cles', t('ouvrir_cles'), 'cle'], profil: ['of-profil', 'ov:profil', t('depuis_profil'), 'cible'],
      nouvelle: ['of-nouvelle', 'ov:nouvelle', t('nouvelle'), 'plus'], chercher: ['of-chercher', 'ov:chercher', t('chercher_aj'), 'loupe'],
      filtres: ['of-retirer', 'ov:retirer', t('retirer'), 'reglages'] }[v.bouton];
    const enCours = typeof x.en_cours === 'string' && x.en_cours;
    return `<div class="jm-vide${anime()}"><div class="jm-v-ic plasma-bord">${icone(v.icone)}</div><h1>${esc(texteSur(v.titre))}</h1>
      <p>${esc(texteSur(v.texte))}</p>
      ${b ? bouton(b[0], b[1], b[2], b[3], 'principal grand', enCours && b[0] === 'of-chercher' ? ' disabled' : '') : ''}
      ${v.bouton === 'profil' ? `<div class="jm-btns centre">${bouton('of-nouvelle', 'ov:nouvelle2', t('nouvelle'), 'plus', 'second')}</div>` : ''}
      ${v.note ? `<p class="jm-note">${esc(texteSur(v.note))}</p>` : ''}</div>`;
  }

  // Mes recherches : chacune avec « Chercher maintenant », « Modifier », « Supprimer » ; le formulaire en haut
  function rechercheFormHTML(x) {
    const { esc } = O, t = tO(x), v = ofForm.v, d = rchDossier();
    const champ = (k, label, indice, val, max, aide) => `<label class="jm-champ"><span>${label}</span><input class="f" data-perso="of-form" data-champ="${k}" data-k="rf:${k}" maxlength="${max}" autocomplete="off" spellcheck="false" placeholder="${esc(indice)}" value="${esc(val || '')}">${aide ? `<small class="jm-aide">${esc(aide)}</small>` : ''}</label>`;
    const lieux = liste(d.lieux).filter((l) => codeSurP(l?.id));
    let ou = '';
    if (v.lieu === 'ville') {
      ou = `${champ('ville', esc(t('ville')), t('i_ville'), v.ville, 60)}
        <div class="jm-champ"><span>${esc(t('rayon'))}</span><div class="jm-puces">${liste(d.rayons).filter(Number.isInteger).map((n) => `<button type="button" role="radio" aria-checked="${n === v.rayon}" class="jm-puce f${n === v.rayon ? ' oui' : ''}" data-perso="of-form-rayon" data-v="${n}" data-k="rf:r-${n}">${n} ${esc(t('km'))}</button>`).join('')}</div></div>`;
    } else if (v.lieu === 'departement') {
      ou = `<label class="jm-champ large"><span>${esc(t('departement'))}</span><select class="f" data-perso="of-form" data-champ="departement" data-k="rf:dep"><option value="">${esc(t('choisir'))}</option>${optionsHTML(d.departements, v.departement, deptSur)}</select></label>`;
    } else if (v.lieu === 'region') {
      ou = `<label class="jm-champ large"><span>${esc(t('region'))}</span><select class="f" data-perso="of-form" data-champ="region" data-k="rf:reg"><option value="">${esc(t('choisir'))}</option>${optionsHTML(d.regions, v.region, codeSurP)}</select></label>`;
    }
    return `<form class="jm-cle jm-rf" novalidate><b class="jm-rf-t">${esc(ofForm.id ? t('form_modifier') : t('form_nouvelle'))}</b>
      <div class="jm-grille">${champ('mots', esc(t('mots')), t('i_mots'), v.mots, 120, t('mots_aide'))}${champ('nom', `${esc(t('nom'))} <small>${esc(t('facultatif'))}</small>`, t('i_nom'), v.nom, 60)}
        <div class="jm-champ large"><span>${esc(t('lieu'))}</span><div class="jm-puces" role="radiogroup">${lieux.map((l, i) => `<button type="button" role="radio" aria-checked="${l.id === v.lieu}" class="jm-puce f${l.id === v.lieu ? ' oui' : ''}" data-perso="of-form-lieu" data-id="${l.id}" data-k="rf:l${i}">${esc(texteSur(l.nom))}</button>`).join('')}</div></div>
        ${ou}</div>
      <p class="jm-err" role="alert"></p>
      <div class="jm-btns">${bouton('of-form-ok', 'rf:ok', t('enregistrer'), 'coche', 'principal')}${bouton('of-form-non', 'rf:non', t('annuler'), '', 'second')}</div></form>`;
  }

  function gestionHTML(x) {
    const { esc } = O, t = tO(x), g = objet(x.gestion);
    const l = liste(g.liste).filter((r) => numSur(r?.id));
    const enCours = typeof x.en_cours === 'string' && x.en_cours;
    let h = `<div class="jm-p${anime()}"><h1>${esc(texteSur(g.titre))}</h1><p class="jm-intro">${esc(texteSur(g.intro))}</p>
      <div class="jm-outils">${bouton('of-nouvelle', 'og:nouvelle', t('nouvelle'), 'plus', '', g.plein === true || ofForm ? ' disabled' : '')}${g.profil === true ? bouton('of-profil', 'og:profil', t('depuis_profil'), 'cible', '', g.plein === true ? ' disabled' : '') : ''}${bouton('of-fermer', 'og:fermer', t('fermer'), 'gauche', 'discret')}${g.plein === true ? `<span>${esc(t('plein'))}</span>` : ''}</div>`;
    if (ofForm && !ofForm.id) h += rechercheFormHTML(x);
    for (const r of l) {
      if (ofForm && ofForm.id === r.id) { h += rechercheFormHTML(x); continue; }
      h += `<section class="jm-cle jm-rc${r.id === x.recherche ? ' choisie' : ''}"><div class="jm-r-tete"><span class="jm-a-ic">${icone('cible')}</span><div class="jm-cle-t"><div class="jm-cle-l1"><b>${esc(texteSur(r.nom))}</b></div><p>${esc(texteSur(r.sous))}</p><p class="jm-petit">${esc(texteSur(r.derniere))}</p></div></div>`;
      if (ofConfirme === r.id) {
        h += `<div class="jm-suppr"><p>${esc(texteSur(r.question))}</p><div class="jm-btns">${bouton('of-suppr-oui', 'og:oui', t('oui'), 'poubelle', 'danger', ` data-id="${r.id}"`)}${bouton('of-suppr-non', 'og:non', t('non'), '', 'second')}</div></div>`;
      } else {
        h += `<div class="jm-btns">${bouton('of-chercher', `og:c-${r.id}`, t('chercher'), 'loupe', 'principal', ` data-id="${r.id}"${enCours ? ' disabled' : ''}`)}${bouton('of-modifier', `og:m-${r.id}`, t('modifier'), 'crayon', '', ` data-id="${r.id}"`)}${bouton('of-supprimer', `og:s-${r.id}`, t('supprimer'), 'poubelle', 'discret danger', ` data-id="${r.id}"`)}</div>`;
      }
      h += '</section>';
    }
    return `${h}</div>`;
  }

  function droiteOffres(x) {
    if (ofVue === 'recherches') return gestionHTML(x);
    const f = x.fiche && typeof x.fiche === 'object' && numSur(x.fiche.id) ? x.fiche : null;
    if (f) return ficheOffreHTML(x, f);
    return videOffresHTML(x);
  }

  function ouvrirFormRecherche(id) {
    const x = offresDe(); if (!x) return;
    const g = objet(x.gestion);
    const r = numSur(id) ? liste(g.liste).find((y) => y?.id === id) : null;
    const v = objet(r?.valeurs);
    ofForm = r
      ? { id: r.id, v: { nom: texteSur(v.nom), mots: texteSur(v.mots), lieu: codeSurP(v.lieu) ? v.lieu : 'ville', ville: texteSur(v.ville), rayon: Number.isInteger(v.rayon) ? v.rayon : 30, departement: texteSur(v.departement), region: texteSur(v.region) } }
      : { id: null, v: { nom: '', mots: '', lieu: 'ville', ville: '', rayon: 30, departement: '', region: '' } };
    ofVue = 'recherches'; ofConfirme = null;
    O.anime = !reduit();
    dessinerTout('rf:mots');
  }

  function enregistrerRecherche() {
    if (!ofForm) return;
    const err = hote.querySelector('.jm-rf .jm-err');
    // (sans mots : tous les métiers du lieu ; ni mots ni lieu : refusé, comme dans Rust)
    if (!String(ofForm.v.mots || '').trim() && ofForm.v.lieu === 'partout') {
      hote.querySelector('[data-k="rf:mots"]')?.closest('.jm-champ')?.classList.add('faux');
      if (err) err.textContent = 'Écris un poste, ou choisis un lieu.';
      hote.querySelector('[data-k="rf:mots"]')?.focus({ preventScroll: false });
      O.son?.retour?.();
      return;
    }
    if (Date.now() < envoiJusqua) return;
    envoiJusqua = Date.now() + 800;
    const v = ofForm.v;
    const m = { nom: String(v.nom || '').slice(0, 60), mots: String(v.mots).slice(0, 120), lieu: v.lieu, ville: String(v.ville || '').slice(0, 60), rayon: v.rayon, departement: v.departement, region: v.region };
    if (numSur(ofForm.id)) m.id = ofForm.id;
    O.son?.ok?.();
    O.action('o-recherche', m);
  }

  // Un élément de l'onglet Offres (clic, A ou Entrée) ; vrai s'il en était un
  function validerOffres(el, d) {
    const x = offresDe(), f = x?.fiche;
    if (typeof d.perso === 'string' && d.perso.startsWith('en-')) return validerEnvoi(el, d);
    if (typeof d.perso === 'string' && d.perso.startsWith('po-')) return validerPo(el, d);
    if (typeof d.perso === 'string' && d.perso.startsWith('pr-')) return validerPrep(el, d, f);
    switch (d.perso) {
      case 'of-champ':   // (Entrée dans le texte cherché ou la ville : les filtres partent tout de suite)
        if (el.tagName === 'INPUT') envoyerFiltres(false);
        return true;
      case 'of-liste': if (ofFiltres && codeSurP(d.id) && ofFiltres.liste !== d.id) { ofFiltres.liste = d.id; O.son?.bascule?.(); envoyerFiltres(false); } return true;
      case 'of-bascule': if (ofFiltres && OF_BASCULES.includes(d.champ)) { ofFiltres[d.champ] = !ofFiltres[d.champ]; O.son?.bascule?.(); envoyerFiltres(false); dessinerTout(el.dataset.k); } return true;
      case 'of-pas-hors': if (f && numSur(f.id)) { O.son?.ok?.(); O.action('o-pas-hors-sujet', { id: f.id }); } return true;
      case 'of-lire-ia': if (!el.disabled) { el.disabled = true; O.son?.ok?.(); const id = Number(d.id); O.action('o-lire-ia', numSur(id) ? { id } : {}); } return true;
      case 'of-retirer':
        ofFiltres = { texte: '', genre: 'partout', ville: '', rayon: 30, departement: '', region: '', contrat: '', teletravail: 'peu_importe', liste: 'toutes', sans_diplome: false, debutant: false, tri: ofFiltres?.tri || 'pertinence' };
        O.son?.ok?.(); envoyerFiltres(false); return true;
      case 'of-ouvrir': {
        const id = Number(d.id);
        if (!numSur(id)) return true;
        O.son?.ok?.(); ofVue = 'offre'; ofForm = null; ofConfirme = null;
        if (f?.id !== id) { ofChoisie = id; O.anime = !reduit(); }
        O.action('o-ouvrir', { id });
        dessinerTout(`ol:${id}`);
        return true;
      }
      case 'of-recherche': return true;
      case 'of-chercher': {
        if (el.disabled) return true;
        const id = Number(d.id);
        O.son?.ok?.();
        O.action('o-chercher', numSur(id) ? { id } : {});
        return true;
      }
      case 'of-gerer': O.son?.tic?.(); ofVue = ofVue === 'recherches' ? 'offre' : 'recherches'; ofForm = null; ofConfirme = null; O.anime = !reduit(); dessinerTout('or:gerer'); return true;
      case 'of-fermer': O.son?.retour?.(); ofVue = 'offre'; ofForm = null; ofConfirme = null; O.anime = !reduit(); dessinerTout('or:gerer'); return true;
      case 'of-nouvelle': if (!el.disabled) { O.son?.ok?.(); ouvrirFormRecherche(null); } return true;
      case 'of-modifier': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); ouvrirFormRecherche(id); } return true; }
      case 'of-profil': if (!el.disabled) { O.son?.ok?.(); O.action('o-depuis-profil'); } return true;
      case 'of-supprimer': { const id = Number(d.id); if (numSur(id)) { O.son?.ok?.(); ofConfirme = id; dessinerTout('og:non'); } return true; }
      case 'of-suppr-non': { const id = ofConfirme; O.son?.retour?.(); ofConfirme = null; dessinerTout(id ? `og:s-${id}` : undefined); return true; }
      case 'of-suppr-oui': if (numSur(ofConfirme)) { const id = ofConfirme; ofConfirme = null; el.disabled = true; O.action('o-recherche-supprimer', { id }); } return true;
      case 'of-form': {   // (Entrée : le champ suivant)
        if (el.tagName !== 'INPUT') return true;
        const els = [...hote.querySelectorAll('.jm-rf .f')], i = els.indexOf(el);
        els[i + 1]?.focus({ preventScroll: false });
        return true;
      }
      case 'of-form-lieu': if (ofForm && codeSurP(d.id)) { ofForm.v.lieu = d.id; O.son?.bascule?.(); dessinerTout(el.dataset.k); } return true;
      case 'of-form-rayon': { const n = Number(d.v); if (ofForm && Number.isInteger(n)) { ofForm.v.rayon = n; O.son?.bascule?.(); dessinerTout(el.dataset.k); } return true; }
      case 'of-form-ok': enregistrerRecherche(); return true;
      case 'of-form-non': O.son?.retour?.(); ofForm = null; dessinerTout('og:nouvelle'); return true;
      case 'of-cles': O.son?.ok?.(); montrer('profil', 'r:cles'); ouvrirRubrique('cles'); return true;
      case 'of-favori': if (f && numSur(f.id)) { O.son?.bascule?.(); O.action('o-favori', { id: f.id }); } return true;
      case 'of-tard': if (f && numSur(f.id)) { O.son?.bascule?.(); O.action('o-plus-tard', { id: f.id }); } return true;
      case 'of-cacher': if (f && numSur(f.id)) { O.son?.ok?.(); O.action('o-cacher', { id: f.id }); } return true;
      case 'of-voir': if (f && numSur(f.id) && codeSurP(d.source)) { O.son?.ok?.(); O.action('o-voir', { id: f.id, source: d.source }); } return true;
      case 'of-adzuna': O.son?.ok?.(); O.action('o-adzuna'); return true;
      case 'of-postule': if (f && numSur(f.id) && !el.disabled) { el.disabled = true; O.son?.ok?.(); O.action('o-postule', { id: f.id }); } return true;
      case 'of-candidature': if (f && numSur(f.candidature)) { O.son?.ok?.(); montrer('candidatures'); ouvrirCandidature(f.candidature); } return true;
      default: return false;
    }
  }

  // Ce qui se tape ou se choisit dans l'onglet Offres : gardé ici, puis envoyé
  function saisieOffres(el, d) {
    if (d.perso === 'of-champ' && ofFiltres && OF_CHAMPS.includes(d.champ)) {
      const v = d.champ === 'rayon' ? Number(el.value) : String(el.value);
      ofFiltres[d.champ] = v;
      // (un autre genre de lieu : les autres valeurs du lieu partent)
      if (d.champ === 'genre') { ofFiltres.departement = ''; ofFiltres.region = ''; }
      const frappe = d.champ === 'texte' || d.champ === 'ville';
      if (!frappe) O.son?.bascule?.();
      envoyerFiltres(frappe);
      if (d.champ === 'genre') dessinerTout(d.k);
      return true;
    }
    if (d.perso === 'of-recherche') {
      const id = Number(el.value);
      if (numSur(id)) { O.son?.bascule?.(); O.action('o-choisir', { id }); }
      return true;
    }
    if (d.perso === 'of-form' && ofForm && ['nom', 'mots', 'ville', 'departement', 'region'].includes(d.champ)) {
      ofForm.v[d.champ] = el.value;
      el.closest('.jm-champ')?.classList.remove('faux');
      const err = hote.querySelector('.jm-rf .jm-err'); if (err) err.textContent = '';
      return true;
    }
    return false;
  }

  // Retour dans l'onglet Offres : ferme d'abord ce qui est ouvert ; vrai si quelque chose l'était
  function retourOffres() {
    if (onglet !== 'offres') return false;
    if (ofConfirme) { const id = ofConfirme; ofConfirme = null; dessinerTout(`og:s-${id}`); return true; }
    if (ofForm) { ofForm = null; dessinerTout('og:nouvelle'); return true; }
    if (ofVue === 'recherches') { ofVue = 'offre'; dessinerTout('or:gerer'); return true; }
    // (étape 8 : la lettre en cours de modification, la version de Claude, puis « Ma candidature »)
    if (prEdition) { prEdition = null; dessinerTout('pr:modifier'); return true; }
    if (prBrute) { prBrute = false; dessinerTout('pr:brute'); return true; }
    if (ofPil === 'cand') { ofPil = 'offre'; dessinerTout('op:offre'); return true; }
    return false;
  }

  // Les ordres de l'appli pour l'onglet Offres ; vrai si c'en était un
  function ordreOffres(o) {
    if (o.recherche_offres !== undefined) {
      ofForm = null;
      if (onglet === 'offres') dessinerTout(ofVue === 'recherches' ? 'og:nouvelle' : 'or:chercher');
      return true;
    }
    // (étape 8 : la lettre gardée, une nouvelle version prête ; pour les captures, « Ma candidature » montrée)
    if (o.preparation === 'gardee') { prEdition = null; prVoir = 'moi'; if (onglet === 'offres') dessinerTout('pr:modifier'); return true; }
    if (o.preparation === 'stylee') { prEdition = null; prVoir = 'style'; if (onglet === 'offres' && ofPil === 'cand') dessinerTout('pr:l-style'); return true; }
    if (o.preparation === 'prete') { prEdition = null; prBrute = false; prVoir = null; prInt = null; prConsigne = ''; if (onglet === 'offres' && ofPil === 'cand') { dessinerTout('pr:copier'); voirPil(); } return true; }
    if (typeof o.prep_consigne === 'string') { prConsigne = o.prep_consigne.slice(0, 120); if (onglet === 'offres') dessinerTout('pr:consigne'); return true; }
    if (o.preparation === 'voir') { if (onglet !== 'offres') montrer('offres'); ofVue = 'offre'; ofPil = 'cand'; dessinerTout('op:cand'); voirPil(); return true; }
    // (étape 10, pour les captures) le récapitulatif ouvert, une adresse tapée comme Jordan
    if (o.envoi === 'ouvrir') { const e = enDe(); if (e) { if (onglet !== 'offres') montrer('offres'); O.action('en-ouvrir', { id: e.offre }); } return true; }
    if (typeof o.envoi_a === 'string') { const e = enDe(); if (e) { enSaisieDe(e).a = o.envoi_a.slice(0, 200); if (onglet === 'offres') dessinerTout('en:a'); } return true; }
    // (essai : une valeur tapée dans le formulaire d'une recherche gardée, comme Jordan : ses mots, son nom, sa ville)
    if (o.rf_saisie && typeof o.rf_saisie === 'object') { const ch = o.rf_saisie.champ; if (ofForm && ['mots', 'nom', 'ville'].includes(ch) && typeof o.rf_saisie.v === 'string') { ofForm.v[ch] = o.rf_saisie.v.slice(0, 120); dessinerTout(`rf:${ch}`); } return true; }
    // (essai, étape 11 : une valeur tapée dans le récapitulatif de l'envoi sur le site, comme Jordan)
    if (o.po_saisie && typeof o.po_saisie === 'object') { const p = poDe(), i = Number(o.po_saisie.i); if (p && Number.isInteger(i) && typeof o.po_saisie.v === 'string') { poSaisieDe(p).v[i] = o.po_saisie.v.slice(0, 300); dessinerTout(`po:c-${i}`); hote.querySelector(`[data-k="po:c-${i}"]`)?.scrollIntoView({ block: 'center' }); } return true; }
    if (numSur(o.recherche_supprimee)) { ofConfirme = null; if (onglet === 'offres') dessinerTout('og:nouvelle'); return true; }
    // (pour les captures) une offre ouverte, Mes recherches
    if (numSur(o.offre)) {
      if (onglet !== 'offres') montrer('offres');
      ofVue = 'offre'; ofChoisie = o.offre; O.action('o-ouvrir', { id: o.offre }); dessinerTout(`ol:${o.offre}`);
      return true;
    }
    if (o.offres === 'recherches' || o.offres === 'nouvelle') {
      if (onglet !== 'offres') montrer('offres');
      if (o.offres === 'nouvelle') ouvrirFormRecherche(null); else { ofVue = 'recherches'; dessinerTout('or:gerer'); }
      return true;
    }
    if (o.offres === 'filtres') {
      // (pour les captures) des filtres posés comme si Jordan les avait choisis
      const f = objet(o.filtres);
      if (ofFiltres) for (const k of OF_CHAMPS) if (Object.hasOwn(f, k)) ofFiltres[k] = valChamp(k, f[k]);
      envoyerFiltres(false);
      return true;
    }
    return false;
  }

  // ---------------------------------------------------------------- avec le kit
  window.kitAccueil = Object.freeze({
    // L'accueil dessiné (à chaque nouvel état et à chaque retour sur l'accueil), sur l'onglet montré
    dessiner(h, perso, outils) {
      const garde = hote === h ? document.activeElement?.closest?.('#jm [data-k]')?.dataset.k : null;
      hote = h; P = perso || {}; O = { ...outils };
      ecouter(h);
      // (des notes que Rust a gardées : plus besoin de les retenir ici)
      const f = fiche();
      if (f && Object.hasOwn(notes, f.id) && notes[f.id] === texteSur(f.notes) && document.activeElement?.dataset?.perso !== 'notes') delete notes[f.id];
      dessinerTout(outils.focus || garde);
    },
    // Un onglet, une ligne, un bouton (clic, A ou Entrée)
    valider(el) {
      const d = el.dataset;
      if (validerSites(el, d)) return;
      if (typeof d.perso === 'string' && d.perso.startsWith('po-')) { validerPo(el, d); return; }
      if (d.perso === 'onglet' && idSur(d.id)) { if (d.id !== onglet) O.son?.tic?.(); montrer(d.id); return; }
      if (d.perso === 'rubrique' && idSur(d.id)) { O.son?.ok?.(); ouvrirRubrique(d.id); return; }
      validerSuivi(el, d);
    },
    // Retour (Échap, B, le bouton du bas) : d'abord ce qui est ouvert, puis le premier onglet ; sinon le kit revient
    // au menu de Mindset
    retour() {
      // (étape 5 bis : la petite fenêtre du site, puis la carte de connexion, se ferment d'abord)
      if (petiteOuverte()) { O.action('cs-fermer'); return true; }
      if (cnx) { fermerConnexion(); return true; }
      if (poOuvert()) { O.son?.retour?.(); poSaisie = null; O.action('po-fermer'); return true; }
      if (enOuvert()) { O.son?.retour?.(); O.action('en-fermer'); return true; }
      if (sitesConfirme) { sitesConfirme = false; dessinerTout('pn:decon'); return true; }
      if (srcMenu) { srcMenu = false; dessinerTout('cs:ou'); return true; }
      if (cal && calOuvert(cal.ou)) { fermerCal(); return true; }
      if (menu) { menu = false; dessinerTout('d:statut'); return true; }
      if (rdv) { rdv = null; dessinerTout('d:statut'); return true; }
      if (confirme) { confirme = false; dessinerTout('d:supprimer'); return true; }
      if (vueC === 'formulaire') { fermerFormulaire(true); return true; }
      if (vueC === 'mail') { vueC = 'dossier'; mailVu = null; dessinerTout(); return true; }
      if (vueC === 'bilan' && onglet === 'candidatures') { vueC = 'dossier'; dessinerTout('c:bilan'); return true; }
      if (rubriqueOuverte('reglages') && objet(objet(objet(rubriqueDe(ongletDe('profil'))?.dossier).bord).sauvegarde).attente) { O.son?.retour?.(); O.action('s-non'); return true; }
      if (retourProfil()) return true;
      if (retourOffres()) return true;
      const premier = onglets()[0];
      if (premier && onglet !== premier.id) { montrer(premier.id); return true; }
      return false;
    },
    // Les ordres de l'appli
    ordre(o) {
      if (!hote || !P) return;
      if (ordreProfil(o)) return;
      if (ordreOffres(o)) return;
      // (construction d'essai seulement : la carte de connexion remplie comme Jordan, avec des identifiants inventés)
      if (o.connexion && typeof o.connexion === 'object') {
        const c = o.connexion;
        if (!codeSurP(c.site) || !siteDe(c.site)) return;
        if (onglet !== 'offres') montrer('offres');
        cnx = { site: c.site, mail: texteSur(c.mail).slice(0, 200), mdp: texteSur(c.mdp).slice(0, 300) };
        dessinerTout('cx:mdp');
        if (c.envoyer === true) envoyerConnexion();
        return;
      }
      if (idSur(o.rubrique)) {
        const avec = onglets().find((x) => rubriquesDe(x).some((r) => r.id === o.rubrique));
        if (avec) { montrer(avec.id, `r:${o.rubrique}`); ouvrirRubrique(o.rubrique); }
        return;
      }
      if (numSur(o.enregistre)) {
        brouillon = null; delete modifs[o.enregistre]; form = null; vueC = 'dossier'; choisie = null;
        if (onglet === 'candidatures') dessinerTout(`c:${o.enregistre}`);
        return;
      }
      if (numSur(o.fait)) {
        rdv = null; menu = false; confirme = false;
        if (onglet === 'candidatures') dessinerTout('d:statut');
        return;
      }
      if (numSur(o.candidature)) {
        if (onglet !== 'candidatures') montrer('candidatures');
        ouvrirCandidature(o.candidature);
        return;
      }
      if (o.ajouter === true) {
        if (onglet !== 'candidatures') montrer('candidatures');
        ouvrirFormulaire(null);
        return;
      }
      if (o.statut === 'menu' || o.statut === 'entretien') {
        if (onglet !== 'candidatures') montrer('candidatures');
        const f = fiche(); if (!f || vueC !== 'dossier') return;
        if (o.statut === 'menu') { menu = true; rdv = null; dessinerTout(`m:${f.statut}`); } else { rdv = { ...(f.entretien || {}) }; menu = false; dessinerTout('e:date'); }
        return;
      }
      if (o.modifier === true) {
        if (onglet !== 'candidatures') montrer('candidatures');
        const f = fiche(); if (f) ouvrirFormulaire(f);
        return;
      }
      // (pour les captures) le calendrier de la date ouverte : celle du formulaire, sinon celle de l'entretien
      if (o.calendrier === true) {
        if (onglet !== 'candidatures') return;
        if (vueC === 'formulaire' && form) ouvrirCal('f'); else if (rdv) ouvrirCal('e');
        return;
      }
      // (pour les captures) le dossier ouvert descendu tout en bas
      if (o.defiler === 'bas') { const d = hote.querySelector('.jm-dossier'); if (d) d.scrollTop = d.scrollHeight; return; }
      // (pour les captures, étape 6) la liste descendue jusqu'à ses offres
      if (o.defiler === 'liste') { const l = hote.querySelector('.jm-liste'), e = hote.querySelector('.jm-liste .jm-liste-tete'); if (l && e) l.scrollTop = e.offsetTop - 12; return; }
      // (pour les captures) un clic sur un élément de l'écran, par son data-k (« cal:2026-11-10 », « f:enregistrer »)
      if (typeof o.cliquer === 'string' && /^[a-z]{1,4}:[a-z0-9-]{1,12}$/.test(o.cliquer)) {
        hote.querySelector(`#jm [data-k="${o.cliquer}"]`)?.click();
        return;
      }
      // (pour les captures) une zone de texte prise puis quittée par Échap : la « zone libre »
      if (typeof o.libre === 'string' && /^[a-z]{1,4}:[a-z0-9-]{1,12}$/.test(o.libre)) {
        const z = hote.querySelector(`#jm textarea[data-k="${o.libre}"]`);
        if (z) { z.focus({ preventScroll: true }); libre(z, true); }
        return;
      }
      // (étape 7) un mail à ranger ouvert (pour les captures, et après « Rien à voir » : le suivant)
      if (numSur(o.mail)) { if (onglet !== 'candidatures') montrer('candidatures'); ouvrirMail(o.mail); return; }
      if (o.mails === 'ranger') { const m = liste(mailsDe().a_ranger).find((x) => numSur(x?.id)); if (onglet !== 'candidatures') montrer('candidatures'); if (m) ouvrirMail(m.id); return; }
      if (numSur(o.mail_range)) { if (onglet === 'candidatures') dessinerTout(); return; }
      // (étape 12) Mon bilan ouvert (pour les captures) ; la question de la sauvegarde à remettre
      if (o.bilan === true) { if (onglet !== 'candidatures') montrer('candidatures'); ouvrirBilan(); return; }
      if (o.sauvegarde === 'question') {
        if (!rubriqueOuverte('reglages')) { montrer('profil'); ouvrirRubrique('reglages'); }
        dessinerTout('rs:oui');
        hote.querySelector('[data-k="rs:oui"]')?.scrollIntoView({ block: 'center' });
        return;
      }
      if (idSur(o.onglet)) montrer(o.onglet);
    },
  });
})();
