//! La boîte « Ouvrir » de Windows pour choisir un CV (`IFileOpenDialog`) : une fenêtre voulue, pour Jordan, ouverte
//! seulement quand il clique « Importer un fichier ». Elle tourne sur un fil à elle (COM en appartement simple), posée
//! sur la fenêtre de Jobmind ; le fil de la fenêtre continue de tourner pendant ce temps. Filtre « PDF ou Word »,
//! fichiers du disque seulement, rien ajouté aux documents récents de Windows.

use std::path::PathBuf;

use windows::Win32::Foundation::{E_FAIL, ERROR_CANCELLED};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoCreateInstance, CoInitializeEx,
    CoTaskMemFree, CoUninitialize,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{FOLDERID_Documents, IShellItem, KF_FLAG_DEFAULT, SHGetKnownFolderItem};
use windows::Win32::UI::Shell::{
    FOS_DONTADDTORECENT, FOS_FILEMUSTEXIST, FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_OVERWRITEPROMPT,
    FOS_PATHMUSTEXIST, FileOpenDialog, FileSaveDialog, IFileOpenDialog, IFileSaveDialog, SIGDN_FILESYSPATH,
};
use windows::core::{GUID, HRESULT, w};

use super::hwnd;

/// L'état que Windows garde pour cette boîte (le dernier dossier ouvert) : propre à l'import des CV de Jobmind, jamais
/// mêlé à celui des autres applis de la suite (toutes s'appellent Mindset.exe). La construction d'essai a le sien, effacé
/// après chaque essai : un essai ne change rien de ce que Windows garde pour la vraie Jobmind.
#[cfg(not(feature = "essai"))]
const CLIENT: GUID = GUID::from_u128(0x6a0b_5c1e_9d3f_4e27_b8a1_2f4c_7e9d_0b13);
#[cfg(feature = "essai")]
const CLIENT: GUID = GUID::from_u128(0x6a0b_5c1e_9d3f_4e27_b8a1_2f4c_7e9d_e55a);

/// Ce que la boîte a donné.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choix {
    Fichier(PathBuf),
    Annule,
}

/// Ouvre la boîte « Ouvrir » posée sur cette fenêtre (0 : aucune) et attend le choix de Jordan. À appeler sur un fil
/// qui n'a pas encore COM (un fil à part).
pub fn choisir_cv(proprietaire: usize) -> Result<Choix, String> {
    // SAFETY : COM pour ce fil seulement, rendu à la fin s'il a été pris ici
    let co = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    let r = boite(proprietaire);
    if co.is_ok() {
        // SAFETY : un appel pour l'appel réussi de CoInitializeEx, sur le même fil
        unsafe { CoUninitialize() };
    }
    r.map_err(|e| format!("boîte d'ouverture ({e})"))
}

fn boite(proprietaire: usize) -> windows::core::Result<Choix> {
    // SAFETY : objet COM créé et utilisé sur ce fil seulement ; les textes sont des constantes
    unsafe {
        let d: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        let filtres = [
            COMDLG_FILTERSPEC { pszName: w!("CV en PDF ou Word"), pszSpec: w!("*.pdf;*.docx") },
            COMDLG_FILTERSPEC { pszName: w!("Tous les fichiers"), pszSpec: w!("*.*") },
        ];
        d.SetFileTypes(&filtres)?;
        d.SetFileTypeIndex(1)?;
        d.SetTitle(w!("Choisis ton CV (PDF ou Word)"))?;
        d.SetOkButtonLabel(w!("Importer"))?;
        d.SetClientGuid(&CLIENT)?;
        // (la première fois : le dossier Documents)
        if let Ok(documents) = SHGetKnownFolderItem::<IShellItem>(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) {
            let _ = d.SetDefaultFolder(&documents);
        }
        let options = d.GetOptions()?;
        d.SetOptions(
            options
                | FOS_FILEMUSTEXIST
                | FOS_PATHMUSTEXIST
                | FOS_FORCEFILESYSTEM
                | FOS_NOCHANGEDIR
                | FOS_DONTADDTORECENT,
        )?;
        let parent = (proprietaire != 0).then(|| hwnd(proprietaire));
        let montre = d.Show(parent);
        #[cfg(feature = "essai")]
        let _ = d.ClearClientData();
        match montre {
            Ok(()) => {}
            Err(e) if e.code() == HRESULT::from_win32(ERROR_CANCELLED.0) => return Ok(Choix::Annule),
            Err(e) => return Err(e),
        }
        let item = d.GetResult()?;
        chemin_de(&item)
    }
}

/// Le chemin d'un élément choisi dans une boîte.
///
/// SAFETY : à appeler sur le fil de la boîte, avec un élément qu'elle vient de rendre.
unsafe fn chemin_de(item: &IShellItem) -> windows::core::Result<Choix> {
    // SAFETY : l'élément est valable ; le texte rendu est libéré une fois copié
    unsafe {
        let nom = item.GetDisplayName(SIGDN_FILESYSPATH)?;
        let chemin = nom.to_string();
        CoTaskMemFree(Some(nom.0 as *const core::ffi::c_void));
        Ok(Choix::Fichier(PathBuf::from(chemin.map_err(|_| windows::core::Error::from(E_FAIL))?)))
    }
}

// ---------------------------------------------------------------------------------------------------------------
// Étape 12 : « Enregistrer sous » (l'export CSV ou PDF, la sauvegarde) et « Ouvrir » pour une sauvegarde
// ---------------------------------------------------------------------------------------------------------------

/// Ce qui s'enregistre ou s'ouvre : un tableau (CSV), un PDF, une sauvegarde (.zip).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    Csv,
    Pdf,
    Zip,
}

impl Genre {
    fn filtre(self) -> COMDLG_FILTERSPEC {
        match self {
            Genre::Csv => COMDLG_FILTERSPEC { pszName: w!("Tableau pour Excel (CSV)"), pszSpec: w!("*.csv") },
            Genre::Pdf => COMDLG_FILTERSPEC { pszName: w!("Document PDF"), pszSpec: w!("*.pdf") },
            Genre::Zip => COMDLG_FILTERSPEC { pszName: w!("Sauvegarde de Jobmind (.zip)"), pszSpec: w!("*.zip") },
        }
    }

    fn extension(self) -> windows::core::PCWSTR {
        match self {
            Genre::Csv => w!("csv"),
            Genre::Pdf => w!("pdf"),
            Genre::Zip => w!("zip"),
        }
    }

    /// L'état que Windows garde pour chaque boîte (le dernier dossier) : propre à Jobmind, un pour les exports, un pour
    /// les sauvegardes ; ceux de l'essai à part, effacés après chaque essai.
    fn client(self) -> GUID {
        #[cfg(not(feature = "essai"))]
        let base: u128 = 0x6a0b_5c1e_9d3f_4e27_b8a1_2f4c_7e9d_1c20;
        #[cfg(feature = "essai")]
        let base: u128 = 0x6a0b_5c1e_9d3f_4e27_b8a1_2f4c_7e9d_e560;
        GUID::from_u128(base + if self == Genre::Zip { 1 } else { 0 })
    }
}

/// La boîte « Enregistrer sous » de Windows posée sur cette fenêtre, le dossier Documents la première fois, ce nom
/// proposé ; Windows demande lui-même avant de remplacer un fichier. À appeler sur un fil à part.
pub fn enregistrer(proprietaire: usize, genre: Genre, nom: &str) -> Result<Choix, String> {
    // SAFETY : COM pour ce fil seulement, rendu à la fin s'il a été pris ici
    let co = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    let r = boite_enregistrer(proprietaire, genre, nom);
    if co.is_ok() {
        // SAFETY : un appel pour l'appel réussi de CoInitializeEx, sur le même fil
        unsafe { CoUninitialize() };
    }
    r.map_err(|e| format!("boîte d'enregistrement ({e})"))
}

fn boite_enregistrer(proprietaire: usize, genre: Genre, nom: &str) -> windows::core::Result<Choix> {
    let nom: Vec<u16> = nom.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY : objet COM créé et utilisé sur ce fil seulement ; le nom reste en vie pendant l'appel
    unsafe {
        let d: IFileSaveDialog = CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)?;
        d.SetFileTypes(&[genre.filtre()])?;
        d.SetFileTypeIndex(1)?;
        d.SetDefaultExtension(genre.extension())?;
        d.SetTitle(if genre == Genre::Zip { w!("Où ranger ta sauvegarde ?") } else { w!("Où ranger le fichier ?") })?;
        d.SetOkButtonLabel(w!("Enregistrer"))?;
        d.SetClientGuid(&genre.client())?;
        d.SetFileName(windows::core::PCWSTR(nom.as_ptr()))?;
        if let Ok(documents) = SHGetKnownFolderItem::<IShellItem>(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) {
            let _ = d.SetDefaultFolder(&documents);
        }
        let options = d.GetOptions()?;
        d.SetOptions(
            options
                | FOS_OVERWRITEPROMPT
                | FOS_PATHMUSTEXIST
                | FOS_FORCEFILESYSTEM
                | FOS_NOCHANGEDIR
                | FOS_DONTADDTORECENT,
        )?;
        let parent = (proprietaire != 0).then(|| hwnd(proprietaire));
        let montre = d.Show(parent);
        #[cfg(feature = "essai")]
        let _ = d.ClearClientData();
        match montre {
            Ok(()) => {}
            Err(e) if e.code() == HRESULT::from_win32(ERROR_CANCELLED.0) => return Ok(Choix::Annule),
            Err(e) => return Err(e),
        }
        let item = d.GetResult()?;
        chemin_de(&item)
    }
}

/// La boîte « Ouvrir » pour choisir une sauvegarde de Jobmind (.zip). À appeler sur un fil à part.
pub fn choisir_sauvegarde(proprietaire: usize) -> Result<Choix, String> {
    // SAFETY : COM pour ce fil seulement, rendu à la fin s'il a été pris ici
    let co = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) };
    let r = boite_sauvegarde(proprietaire);
    if co.is_ok() {
        // SAFETY : un appel pour l'appel réussi de CoInitializeEx, sur le même fil
        unsafe { CoUninitialize() };
    }
    r.map_err(|e| format!("boîte d'ouverture ({e})"))
}

fn boite_sauvegarde(proprietaire: usize) -> windows::core::Result<Choix> {
    // SAFETY : objet COM créé et utilisé sur ce fil seulement ; les textes sont des constantes
    unsafe {
        let d: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        d.SetFileTypes(&[Genre::Zip.filtre()])?;
        d.SetFileTypeIndex(1)?;
        d.SetTitle(w!("Choisis ta sauvegarde de Jobmind"))?;
        d.SetOkButtonLabel(w!("Choisir"))?;
        d.SetClientGuid(&Genre::Zip.client())?;
        if let Ok(documents) = SHGetKnownFolderItem::<IShellItem>(&FOLDERID_Documents, KF_FLAG_DEFAULT, None) {
            let _ = d.SetDefaultFolder(&documents);
        }
        let options = d.GetOptions()?;
        d.SetOptions(
            options
                | FOS_FILEMUSTEXIST
                | FOS_PATHMUSTEXIST
                | FOS_FORCEFILESYSTEM
                | FOS_NOCHANGEDIR
                | FOS_DONTADDTORECENT,
        )?;
        let parent = (proprietaire != 0).then(|| hwnd(proprietaire));
        let montre = d.Show(parent);
        #[cfg(feature = "essai")]
        let _ = d.ClearClientData();
        match montre {
            Ok(()) => {}
            Err(e) if e.code() == HRESULT::from_win32(ERROR_CANCELLED.0) => return Ok(Choix::Annule),
            Err(e) => return Err(e),
        }
        let item = d.GetResult()?;
        chemin_de(&item)
    }
}
