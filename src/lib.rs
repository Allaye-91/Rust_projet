#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]
extern crate alloc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::format;
pub mod structs;
use structs::BiosParameterBlock;

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
