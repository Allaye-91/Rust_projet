#![allow(dead_code)]
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct BiosParameterBlock {
    pub jmp_boot: [u8; 3], pub oem_name: [u8; 8], pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8, pub reserved_sectors: u16, pub num_fats: u8,
    pub root_entry_count: u16, pub total_sectors_16: u16, pub media: u8,
    pub fat_size_16: u16, pub sectors_per_track: u16, pub num_heads: u16,
    pub hidden_sectors: u32, pub total_sectors_32: u32, pub fat_size_32: u32,
    pub ext_flags: u16, pub fs_version: u16, pub root_cluster: u32,
    pub fs_info: u16, pub backup_boot_sector: u16, pub reserved: [u8; 12],
    pub drive_number: u8, pub reserved1: u8, pub boot_signature: u8,
    pub vol_id: u32, pub vol_label: [u8; 11], pub fs_type: [u8; 8],
}
impl BiosParameterBlock {
    pub unsafe fn depuis_octets(data: &[u8]) -> Self {
        let ptr = data.as_ptr() as *const BiosParameterBlock;
        core::ptr::read_unaligned(ptr)
    }
}
pub struct DirEntry {
    pub name: [u8; 8], pub ext: [u8; 3], pub attributes: u8, pub reserved: u8,
    pub create_time_tenth: u8, pub create_time: u16, pub create_date: u16,
    pub last_access_date: u16, pub cluster_high: u16, pub write_time: u16,
    pub write_date: u16, pub cluster_low: u16, pub file_size: u32,
}
