#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
#![allow(unused_imports)]

extern crate alloc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;

pub mod structs;
use structs::{BiosParameterBlock, DirEntry};

pub trait Disque {
    fn lire_secteur(&self, idx_secteur: u64, tampon: &mut [u8]) -> Result<(), &'static str>;
}

pub struct SystemeFichier<D: Disque> {
    disque: D,
    bpb: BiosParameterBlock,
    debut_fat: u64,
    debut_donnees: u64,
    cluster_courant: u32 
}

impl<D: Disque> SystemeFichier<D> {
    
    pub fn initialiser(disque: D) -> Result<Self, &'static str> {
        let mut tampon = [0u8; 512];
        disque.lire_secteur(0, &mut tampon)?;
        let bpb = unsafe { BiosParameterBlock::depuis_octets(&tampon) };
        let debut_fat = bpb.reserved_sectors as u64;
        let debut_donnees = debut_fat + (bpb.num_fats as u64 * bpb.fat_size_32 as u64);
        Ok(Self { disque, bpb, debut_fat, debut_donnees, cluster_courant: bpb.root_cluster })
    }

    fn cluster_vers_secteur(&self, cluster: u32) -> u64 {
        let cluster_effectif = cluster.saturating_sub(2);
        self.debut_donnees + (cluster_effectif as u64 * self.bpb.sectors_per_cluster as u64)
    }

    pub fn lister_repertoire(&self) -> Result<Vec<String>, &'static str> {
        let sect = self.cluster_vers_secteur(self.cluster_courant);
        let mut buf = [0u8; 512];
        self.disque.lire_secteur(sect, &mut buf)?;

        let mut res = Vec::new();
        for chunk in buf.chunks_exact(32) {
            let e = unsafe { core::ptr::read_unaligned(chunk.as_ptr() as *const DirEntry) };
            if e.name[0] == 0 { break; }
            if e.name[0] == 0xE5 || e.attributes == 0x0F { continue; }

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

    fn cluster_suivant(&self, cluster: u32) -> Result<u32, &'static str> {
        let off = cluster * 4;
        let sec = self.debut_fat + (off as u64 / 512);
        let mut buf = [0u8; 512];
        self.disque.lire_secteur(sec, &mut buf)?;
        let val = unsafe {
            let ptr = buf.as_ptr().add((off % 512) as usize) as *const u32;
            core::ptr::read_unaligned(ptr)
        } & 0x0FFFFFFF;
        Ok(val)
    }

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

    pub fn changer_repertoire(&mut self, nom: &str) -> Result<(), &'static str> {
        let e = self.trouver_entree(nom)?;
        if (e.attributes & 0x10) == 0 { return Err("Pas un dossier"); }
        self.cluster_courant = (e.cluster_high as u32) << 16 | (e.cluster_low as u32);
        Ok(())
}
    pub fn lire_fichier(&self, nom: &str) -> Result<String, &'static str> {
        let e = self.trouver_entree(nom)?;
        if (e.attributes & 0x10) != 0 { return Err("Est un dossier"); }
        let mut res = Vec::new();
        let mut clus = (e.cluster_high as u32) << 16 | (e.cluster_low as u32);
        loop {
            let sec = self.cluster_vers_secteur(clus);
            let mut buf = [0u8; 512];
            self.disque.lire_secteur(sec, &mut buf)?;
            res.extend_from_slice(&buf);
            let next = self.cluster_suivant(clus)?;
            if next >= 0x0FFFFFF8 { break; }
            clus = next;
        }
        res.truncate(e.file_size as usize);
        Ok(String::from_utf8_lossy(&res).to_string())
    }

}
