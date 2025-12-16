#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![allow(unused_imports)]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;

pub mod structs;
use structs::{BiosParameterBlock, DirEntry};

// --- ABSTRACTION MATÉRIELLE ---
// J'ai lu dans le "Rust Book" (chapitre 10) que les Traits sont parfaits pour définir 
// des comportements partagés. Ici, cela me permet de simuler un disque sans avoir le vrai matériel.
pub trait Disque {
    fn lire_secteur(&self, idx_secteur: u64, tampon: &mut [u8]) -> Result<(), &'static str>;
}

// --- STRUCTURE PRINCIPALE ---
// J'utilise des Generics <D: Disque> pour lier mon système de fichier à n'importe quel 
// type de disque implémentant mon Trait.
pub struct SystemeFichier<D: Disque> {
    disque: D,
    bpb: BiosParameterBlock,
    debut_fat: u64,
    debut_donnees: u64,
    cluster_courant: u32 
}

impl<D: Disque> SystemeFichier<D> {
    
    // Initialisation : Lecture du Secteur 0 (Boot Sector)
    // J'ai trouvé dans la doc de `core::ptr` comment lire une structure C (packed) depuis des octets bruts.
    // L'utilisation de `unsafe` est obligatoire ici car on "force" le typage de la mémoire brute.
    pub fn initialiser(disque: D) -> Result<Self, &'static str> {
        let mut tampon = [0u8; 512];
        disque.lire_secteur(0, &mut tampon)?;
        
        // Conversion du buffer brut en structure BPB via pointeur
        let bpb = unsafe { BiosParameterBlock::depuis_octets(&tampon) };
        
        // Calcul des offsets (Formules trouvées sur le Wiki OSDev pour FAT32)
        let debut_fat = bpb.reserved_sectors as u64;
        let debut_donnees = debut_fat + (bpb.num_fats as u64 * bpb.fat_size_32 as u64);
        
        Ok(Self { disque, bpb, debut_fat, debut_donnees, cluster_courant: bpb.root_cluster })
    }

    // Helper : Conversion Cluster -> Secteur LBA
    // J'ai dû soustraire 2 au cluster car la doc FAT précise que les clusters de données commencent à l'index 2.
    fn cluster_vers_secteur(&self, cluster: u32) -> u64 {
        let cluster_effectif = cluster.saturating_sub(2);
        self.debut_donnees + (cluster_effectif as u64 * self.bpb.sectors_per_cluster as u64)
    }

    // Commande LS : Lister le répertoire
    // J'utilise `Vec` pour créer une liste dynamique de fichiers.
    // J'utilise `chunks_exact(32)` car la doc FAT32 indique qu'une entrée fait exactement 32 octets.
    pub fn lister_repertoire(&self) -> Result<Vec<String>, &'static str> {
        let sect = self.cluster_vers_secteur(self.cluster_courant);
        let mut buf = [0u8; 512];
        self.disque.lire_secteur(sect, &mut buf)?;

        let mut res = Vec::new();
        for chunk in buf.chunks_exact(32) {
            // Lecture unsafe de l'entrée répertoire
            let e = unsafe { core::ptr::read_unaligned(chunk.as_ptr() as *const DirEntry) };
            
            // Conditions d'arrêt standard FAT32 (0x00 = fin, 0xE5 = supprimé)
            if e.name[0] == 0 { break; }
            if e.name[0] == 0xE5 || e.attributes == 0x0F { continue; }

            // J'utilise `String::from_utf8_lossy` pour gérer les caractères non-ASCII sans crasher.
            let nom = String::from_utf8_lossy(&e.name).trim().to_string();
            let ext = String::from_utf8_lossy(&e.ext).trim().to_string();
            
            if !nom.is_empty() {
                let full = if ext.is_empty() { nom } else { format!("{}.{}", nom, ext) };
                let typ = if (e.attributes & 0x10) != 0 { "<DOS>" } else { "     " };
                res.push(format!("{} {}", typ, full));
            }
        }
        Ok(res)
    }

    // Navigation dans la FAT
    // Cette fonction lit la table FAT pour trouver le cluster suivant d'un fichier chaîné.
    // Masque 0x0FFFFFFF appliqué car seuls les 28 premiers bits sont significatifs en FAT32.
    fn cluster_suivant(&self, cluster: u32) -> Result<u32, &'static str> {
        let off = cluster * 4; // 4 octets par entrée en FAT32
        let sec = self.debut_fat + (off as u64 / 512);
        let mut buf = [0u8; 512];
        self.disque.lire_secteur(sec, &mut buf)?;
        
        // Pointeur arithmétique pour lire l'entier u32 précis dans le secteur
        let val = unsafe {
            let ptr = buf.as_ptr().add((off % 512) as usize) as *const u32;
            core::ptr::read_unaligned(ptr)
        } & 0x0FFFFFFF;
        Ok(val)
    }

    // Recherche d'un fichier spécifique
    // Réutilisation de la logique de parcours par "chunks" de 32 octets.
    fn trouver_entree(&self, cible: &str) -> Result<DirEntry, &'static str> {
        let sect = self.cluster_vers_secteur(self.cluster_courant);
        let mut buf = [0u8; 512];
        self.disque.lire_secteur(sect, &mut buf)?;
        for chunk in buf.chunks_exact(32) {
            let e = unsafe { core::ptr::read_unaligned(chunk.as_ptr() as *const DirEntry) };
            if e.name[0] == 0 { break; }
            if e.name[0] == 0xE5 || e.attributes == 0x0F { continue; }
            
            let n = String::from_utf8_lossy(&e.name).trim().to_string();
            let x = String::from_utf8_lossy(&e.ext).trim().to_string();
            let full = if x.is_empty() { n.clone() } else { format!("{}.{}", n, x) };
            
            if full == cible { return Ok(e); }
        }
        Err("Introuvable")
    }

    // Commande CD : Changer de répertoire
    // Met à jour le `cluster_courant` qui sert de point de référence pour les lectures.
    pub fn changer_repertoire(&mut self, nom: &str) -> Result<(), &'static str> {
        let e = self.trouver_entree(nom)?;
        // Vérification du bit 0x10 (Attribut Répertoire)
        if (e.attributes & 0x10) == 0 { return Err("Pas un dossier"); }
        self.cluster_courant = (e.cluster_high as u32) << 16 | (e.cluster_low as u32);
        Ok(())
    }

    // Commande CAT : Lire le contenu d'un fichier
    // J'implémente ici une boucle qui suit la chaîne de clusters via la FAT
    // jusqu'à trouver le marqueur de fin (>= 0x0FFFFFF8).
    pub fn lire_fichier(&self, nom: &str) -> Result<String, &'static str> {
        let e = self.trouver_entree(nom)?;
        if (e.attributes & 0x10) != 0 { return Err("Est un dossier"); }
        
        let mut res = Vec::new();
        let mut clus = (e.cluster_high as u32) << 16 | (e.cluster_low as u32);
        
        loop {
            let sec = self.cluster_vers_secteur(clus);
            let mut buf = [0u8; 512];
            self.disque.lire_secteur(sec, &mut buf)?;
            res.extend_from_slice(&buf); // Ajout des données au vecteur
            
            let next = self.cluster_suivant(clus)?;
            // 0x0FFFFFF8 est le marqueur de fin de chaîne en FAT32
            if next >= 0x0FFFFFF8 { break; }
            clus = next;
        }
        // On coupe le vecteur à la taille exacte du fichier (indiquée dans DirEntry)
        res.truncate(e.file_size as usize);
        Ok(String::from_utf8_lossy(&res).to_string())
    }

} 

// --- MODULE DE TEST ---
#[cfg(test)]
mod tests {
    use super::*;

    struct MockDisque {
        secteur_boot: [u8; 512],
        rep_racine: [u8; 512],
        donnees_fichier: [u8; 512],
    }

    impl Disque for MockDisque {
        fn lire_secteur(&self, idx: u64, tampon: &mut [u8]) -> Result<(), &'static str> {
            if idx == 0 {
                tampon.copy_from_slice(&self.secteur_boot);
            } else if idx == 100 { 
                tampon.copy_from_slice(&self.rep_racine);
            } else if idx == 101 { 
                tampon.copy_from_slice(&self.donnees_fichier);
            } else {
                tampon[12] = 0xFF; tampon[13] = 0xFF; tampon[14] = 0xFF; tampon[15] = 0x0F;
            }
            Ok(())
        }
    }

    #[test]
    fn test_simulation_terminal() {
        // --- 1. PRÉPARATION DU DISQUE VIRTUEL ---
        let mut boot = [0u8; 512];
        boot[11] = 0x00; boot[12] = 0x02; boot[13] = 1; boot[14] = 32; boot[16] = 2; boot[44] = 2; 

        let mut racine = [0u8; 512];
        // CHANGEMENT ICI : On met "BONJOUR TXT" (Attention aux espaces pour faire 8+3 char)
        // "BONJOUR " (7 lettres + 1 espace) + "TXT"
        let nom = b"BONJOUR TXT"; 
        for i in 0..11 { racine[i] = nom[i]; }
        racine[11] = 0x20; racine[26] = 3; racine[27] = 0; racine[28] = 12; 

        let mut fichier_data = [0u8; 512];
        // CHANGEMENT ICI : Le contenu du fichier
        let texte = b"Ceci est un test !";
        for (i, &b) in texte.iter().enumerate() { fichier_data[i] = b; }
        // On met à jour la taille du fichier dans l'entrée répertoire (18 octets)
        racine[28] = texte.len() as u8;

        let disque = MockDisque { secteur_boot: boot, rep_racine: racine, donnees_fichier: fichier_data };
        let mut fs = SystemeFichier::initialiser(disque).unwrap();
        
        fs.debut_donnees = 100; 
        fs.debut_fat = 50; 

        // --- 2. SIMULATION DE LA CONSOLE ---
        
        println!("\n===========================================");
        println!("      DÉMARRAGE DE ALLAYE_OS (TEST)      ");
        println!("===========================================");

        // --- TEST COMMANDE LS ---
        println!("\nroot@allaye_os:/> ls");
        match fs.lister_repertoire() {
            Ok(liste) => {
                for f in liste {
                    println!("{}", f); 
                }
            },
            Err(e) => println!("Erreur LS: {}", e),
        }

        // --- TEST COMMANDE CAT ---
        println!("\nroot@allaye_os:/> cat BONJOUR.TXT");
        match fs.lire_fichier("BONJOUR.TXT") {
            Ok(contenu) => println!("{}", contenu), 
            Err(e) => println!("Erreur CAT: {}", e),
        }

        println!("\n===========================================");
        
        // Assertions pour garantir que le code marche vraiment
        let verif_ls = fs.lister_repertoire().unwrap();
        assert!(verif_ls[0].contains("BONJOUR.TXT"));
        assert_eq!(fs.lire_fichier("BONJOUR.TXT").unwrap(), "Ceci est un test !");

        println!("Tout fonctionne bien et on est dans le bon !");
        println!("===========================================\n");
    }
}
