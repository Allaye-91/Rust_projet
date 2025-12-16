#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

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
        let cluster_racine = bpb.root_cluster;

        Ok(Self {
            disque,
            bpb,
            debut_fat,
            debut_donnees,
            cluster_courant: cluster_racine,
        })
}
// Fonction B : Helper (Celle que vous venez d'ajouter)
    fn cluster_vers_secteur(&self, cluster: u32) -> u64 {
        let cluster_effectif = cluster.saturating_sub(2);
        self.debut_donnees + (cluster_effectif as u64 * self.bpb.sectors_per_cluster as u64)
    }

} //
