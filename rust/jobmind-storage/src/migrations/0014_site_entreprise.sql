-- Migration 14 (20/11/2026) : le site internet de l'entreprise qui recrute, quand l'annonce le donne (l'onglet
-- « L'entreprise » d'une offre). Lu avec la page sur « Ajouter à Jobmind », gardé sur la première offre du groupe ;
-- vide si l'annonce n'en donne pas (jamais deviné). Seulement le schéma, l'hôte et le chemin, sans paramètres.

ALTER TABLE offres ADD COLUMN site_entreprise TEXT NOT NULL DEFAULT '';
