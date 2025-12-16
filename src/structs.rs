#![allow(dead_code)]

// --- STRUCTURE BPB (Bios Parameter Block) ---
// Cette structure correspond exactement aux 512 premiers octets d'une partition FAT32.
// J'ai trouvé sur le wiki OSDev qu'il est CRUCIAL d'utiliser #[repr(C, packed)].
// - repr(C) : Garde l'ordre des champs comme en langage C (standard pour les OS).
// - packed : Interdit à Rust d'ajouter du "padding" (alignement mémoire) entre les champs.
// Sans ça, les offsets seraient faux et la lecture du disque échouerait.
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct BiosParameterBlock {
    pub jmp_boot: [u8; 3],          // Instruction de saut (assembleur)
    pub oem_name: [u8; 8],          // Nom du constructeur
    pub bytes_per_sector: u16,      // Généralement 512 octets
    pub sectors_per_cluster: u8,    // Clusters = blocs de données
    pub reserved_sectors: u16,      // Secteurs réservés avant la FAT
    pub num_fats: u8,               // Nombre de tables d'allocation (souvent 2)
    pub root_entry_count: u16,      // 0 pour FAT32
    pub total_sectors_16: u16,      // Obilère en FAT32
    pub media: u8,                  // Type de média (F8 pour disque dur)
    pub fat_size_16: u16,           // 0 pour FAT32
    pub sectors_per_track: u16,     // Géométrie disque
    pub num_heads: u16,             // Géométrie disque
    pub hidden_sectors: u32,        // Secteurs cachés (LBA)
    pub total_sectors_32: u32,      // Nombre total de secteurs
    pub fat_size_32: u32,           // Taille d'une FAT en secteurs
    pub ext_flags: u16,             // Drapeaux étendus
    pub fs_version: u16,            // Version du système de fichier
    pub root_cluster: u32,          // Cluster de départ de la racine (Root)
    pub fs_info: u16,               // Secteur d'information FS
    pub backup_boot_sector: u16,    // Copie de sauvegarde du boot
    pub reserved: [u8; 12],         // Réservé par Microsoft
    pub drive_number: u8,           // Numéro de lecteur (0x80)
    pub reserved1: u8,
    pub boot_signature: u8,         // Signature (0x29)
    pub vol_id: u32,                // ID de volume (Série)
    pub vol_label: [u8; 11],        // Nom du volume
    pub fs_type: [u8; 8],           // Chaîne "FAT32   "
}

impl BiosParameterBlock {
    // Méthode utilitaire pour transformer un tableau d'octets brut (buffer) en structure.
    // Utilise `unsafe` et `read_unaligned` car le buffer réseau/disque n'est pas 
    // forcément aligné en mémoire comme Rust le voudrait par défaut.
    pub unsafe fn depuis_octets(data: &[u8]) -> Self {
        let ptr = data.as_ptr() as *const BiosParameterBlock;
        core::ptr::read_unaligned(ptr)
    }
}

// --- STRUCTURE ENTRÉE DE RÉPERTOIRE (Directory Entry) ---
// Représente un fichier ou un dossier dans la table (32 octets).
// C'est ici qu'était l'erreur : il faut ABSOLUMENT #[repr(C, packed)] ici aussi !
// Sinon Rust décale la lecture du cluster et le test échoue.
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct DirEntry {
    pub name: [u8; 8],              // Nom du fichier (8 caractères)
    pub ext: [u8; 3],               // Extension (3 caractères)
    pub attributes: u8,             // Attributs (Lecture seule, Caché, Dossier...)
    pub reserved: u8,               // Réservé pour Windows NT
    pub create_time_tenth: u8,      // Dixièmes de seconde création
    pub create_time: u16,           // Heure création
    pub create_date: u16,           // Date création
    pub last_access_date: u16,      // Date dernier accès
    pub cluster_high: u16,          // Partie haute du numéro de cluster (FAT32)
    pub write_time: u16,            // Heure modification
    pub write_date: u16,            // Date modification
    pub cluster_low: u16,           // Partie basse du numéro de cluster
    pub file_size: u32,             // Taille du fichier en octets
}
