use crate::dex3oat::linker::{ClassLayout, FieldSlot, VtableEntry};

// cite: art oat.h#kOatMagic
const WART_MAGIC: [u8; 4] = *b"wart";

// cite: art oat.h#kOatVersion
pub const WART_FORMAT_VERSION: u32 = 1;

pub const DEX3OAT_VERSION: u32 = 1;

const HEADER_LEN: usize = 4 + 4 + 4 + 8;

// cite: art oat.h#OatHeader
// cite: art dex_file.h#GetLocationChecksum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dex3oatArtifactHeader {
    pub format_version: u32,
    pub dex3oat_version: u32,
    pub source_dex_checksum: u64,
}

// cite: art dex_file.h#GetLocationChecksum, checksum_
pub fn source_dex_checksum(dex_bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in dex_bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

impl Dex3oatArtifactHeader {
    pub fn current(source_dex_checksum: u64) -> Self {
        Self {
            format_version: WART_FORMAT_VERSION,
            dex3oat_version: DEX3OAT_VERSION,
            source_dex_checksum,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(HEADER_LEN);
        bytes.extend_from_slice(&WART_MAGIC);
        bytes.extend_from_slice(&self.format_version.to_le_bytes());
        bytes.extend_from_slice(&self.dex3oat_version.to_le_bytes());
        bytes.extend_from_slice(&self.source_dex_checksum.to_le_bytes());
        bytes
    }

    // cite: art oat.h#IsValid, CheckOatVersion
    pub fn deserialize(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < HEADER_LEN || bytes[0..4] != WART_MAGIC {
            return None;
        }
        Some(Self {
            format_version: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            dex3oat_version: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            source_dex_checksum: u64::from_le_bytes(bytes[12..20].try_into().ok()?),
        })
    }

    // cite: art oat.h#CheckOatVersion
    // cite: art dex_file.h#GetLocationChecksum
    pub fn is_fresh_for(&self, current_dex_checksum: u64, current_dex3oat_version: u32) -> bool {
        self.format_version == WART_FORMAT_VERSION
            && self.dex3oat_version == current_dex3oat_version
            && self.source_dex_checksum == current_dex_checksum
    }
}

fn put_str(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn get_u32(bytes: &[u8], pos: &mut usize) -> Option<u32> {
    let slice = bytes.get(*pos..*pos + 4)?;
    *pos += 4;
    Some(u32::from_le_bytes(slice.try_into().ok()?))
}

fn get_u16(bytes: &[u8], pos: &mut usize) -> Option<u16> {
    let slice = bytes.get(*pos..*pos + 2)?;
    *pos += 2;
    Some(u16::from_le_bytes(slice.try_into().ok()?))
}

fn get_str(bytes: &[u8], pos: &mut usize) -> Option<String> {
    let len = get_u32(bytes, pos)? as usize;
    let slice = bytes.get(*pos..*pos + len)?;
    *pos += len;
    String::from_utf8(slice.to_vec()).ok()
}

fn put_field(out: &mut Vec<u8>, field: &FieldSlot) {
    put_str(out, &field.name);
    put_str(out, &field.type_descriptor);
    put_str(out, &field.declaring_class);
    out.extend_from_slice(&field.slot.to_le_bytes());
}

fn get_field(bytes: &[u8], pos: &mut usize) -> Option<FieldSlot> {
    Some(FieldSlot {
        name: get_str(bytes, pos)?,
        type_descriptor: get_str(bytes, pos)?,
        declaring_class: get_str(bytes, pos)?,
        slot: get_u16(bytes, pos)?,
    })
}

fn put_field_list(out: &mut Vec<u8>, fields: &[FieldSlot]) {
    out.extend_from_slice(&(fields.len() as u32).to_le_bytes());
    for field in fields {
        put_field(out, field);
    }
}

fn get_field_list(bytes: &[u8], pos: &mut usize) -> Option<Vec<FieldSlot>> {
    let count = get_u32(bytes, pos)? as usize;
    (0..count).map(|_| get_field(bytes, pos)).collect()
}

// cite: art oat_file.h#OatClass
pub fn serialize_layouts(header: &Dex3oatArtifactHeader, layouts: &[ClassLayout]) -> Vec<u8> {
    let mut out = header.serialize();
    out.extend_from_slice(&(layouts.len() as u32).to_le_bytes());
    for layout in layouts {
        put_str(&mut out, &layout.descriptor);
        match &layout.superclass {
            Some(descriptor) => {
                out.push(1);
                put_str(&mut out, descriptor);
            }
            None => out.push(0),
        }
        out.extend_from_slice(&layout.reference_field_count.to_le_bytes());
        out.extend_from_slice(&layout.static_reference_field_count.to_le_bytes());
        put_field_list(&mut out, &layout.fields);
        put_field_list(&mut out, &layout.static_fields);
        out.extend_from_slice(&(layout.vtable.len() as u32).to_le_bytes());
        for entry in &layout.vtable {
            put_str(&mut out, &entry.name);
            put_str(&mut out, &entry.proto);
            put_str(&mut out, &entry.declaring_class);
        }
    }
    out
}

// cite: art oat_file.h#OatClass
// cite: art oat.h#CheckOatVersion
pub fn deserialize_layouts(
    bytes: &[u8],
    current_dex_checksum: u64,
    current_dex3oat_version: u32,
) -> Option<Vec<ClassLayout>> {
    let header = Dex3oatArtifactHeader::deserialize(bytes)?;
    if !header.is_fresh_for(current_dex_checksum, current_dex3oat_version) {
        return None;
    }
    let mut pos = HEADER_LEN;
    let count = get_u32(bytes, &mut pos)? as usize;
    let mut layouts = Vec::with_capacity(count);
    for _ in 0..count {
        let descriptor = get_str(bytes, &mut pos)?;
        let superclass = match bytes.get(pos)? {
            0 => {
                pos += 1;
                None
            }
            1 => {
                pos += 1;
                Some(get_str(bytes, &mut pos)?)
            }
            _ => return None,
        };
        let reference_field_count = get_u16(bytes, &mut pos)?;
        let static_reference_field_count = get_u16(bytes, &mut pos)?;
        let fields = get_field_list(bytes, &mut pos)?;
        let static_fields = get_field_list(bytes, &mut pos)?;
        let vtable_count = get_u32(bytes, &mut pos)? as usize;
        let mut vtable = Vec::with_capacity(vtable_count);
        for _ in 0..vtable_count {
            vtable.push(VtableEntry {
                name: get_str(bytes, &mut pos)?,
                proto: get_str(bytes, &mut pos)?,
                declaring_class: get_str(bytes, &mut pos)?,
            });
        }
        layouts.push(ClassLayout {
            descriptor,
            superclass,
            fields,
            reference_field_count,
            static_fields,
            static_reference_field_count,
            vtable,
        });
    }
    Some(layouts)
}

// cite: art oat_file.h#Open
pub fn write_layouts(
    path: &std::path::Path,
    header: &Dex3oatArtifactHeader,
    layouts: &[ClassLayout],
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serialize_layouts(header, layouts))
}

// cite: art oat_file_assistant.h#GetBestOatFile, IsUpToDate
pub fn load_layouts(
    path: &std::path::Path,
    current_dex_checksum: u64,
    current_dex3oat_version: u32,
) -> Option<Vec<ClassLayout>> {
    let bytes = std::fs::read(path).ok()?;
    deserialize_layouts(&bytes, current_dex_checksum, current_dex3oat_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    // cite: art oat.h#IsValid
    #[test]
    fn header_round_trips_through_bytes() {
        let header = Dex3oatArtifactHeader::current(source_dex_checksum(b"class bytes"));
        let restored = Dex3oatArtifactHeader::deserialize(&header.serialize()).unwrap();
        assert_eq!(header, restored);
    }

    // cite: art dex_file.h#GetLocationChecksum
    #[test]
    fn a_fresh_artifact_matches_its_source_and_compiler_version() {
        let checksum = source_dex_checksum(b"class bytes");
        let header = Dex3oatArtifactHeader::current(checksum);
        assert!(header.is_fresh_for(checksum, DEX3OAT_VERSION));
    }

    // cite: art dex_file.h#GetLocationChecksum
    #[test]
    fn an_updated_apk_makes_the_artifact_stale() {
        let header = Dex3oatArtifactHeader::current(source_dex_checksum(b"old app bytes"));
        let new_checksum = source_dex_checksum(b"new app bytes");
        assert_ne!(header.source_dex_checksum, new_checksum);
        assert!(!header.is_fresh_for(new_checksum, DEX3OAT_VERSION));
    }

    // cite: art oat.h#CheckOatVersion
    #[test]
    fn a_newer_compiler_makes_the_artifact_stale() {
        let checksum = source_dex_checksum(b"class bytes");
        let header = Dex3oatArtifactHeader::current(checksum);
        assert!(!header.is_fresh_for(checksum, DEX3OAT_VERSION + 1));
    }

    // cite: art oat.h#IsValid
    #[test]
    fn a_bad_magic_or_truncated_header_is_rejected() {
        assert!(Dex3oatArtifactHeader::deserialize(b"not-a-wart-file-xx").is_none());
        assert!(Dex3oatArtifactHeader::deserialize(b"wart").is_none());
        assert!(Dex3oatArtifactHeader::deserialize(&[]).is_none());
    }

    fn sample_layouts() -> Vec<ClassLayout> {
        vec![
            ClassLayout {
                descriptor: "Ljava/lang/Object;".to_owned(),
                superclass: None,
                fields: Vec::new(),
                reference_field_count: 0,
                static_fields: Vec::new(),
                static_reference_field_count: 0,
                vtable: vec![VtableEntry {
                    name: "hashCode".to_owned(),
                    proto: "()I".to_owned(),
                    declaring_class: "Ljava/lang/Object;".to_owned(),
                }],
            },
            ClassLayout {
                descriptor: "Lp;".to_owned(),
                superclass: Some("Ljava/lang/Object;".to_owned()),
                fields: vec![FieldSlot {
                    name: "ref".to_owned(),
                    type_descriptor: "Ljava/lang/Object;".to_owned(),
                    declaring_class: "Lp;".to_owned(),
                    slot: 0,
                }],
                reference_field_count: 1,
                static_fields: vec![FieldSlot {
                    name: "count".to_owned(),
                    type_descriptor: "I".to_owned(),
                    declaring_class: "Lp;".to_owned(),
                    slot: 0,
                }],
                static_reference_field_count: 0,
                vtable: Vec::new(),
            },
        ]
    }

    // cite: art oat_file.h#OatClass
    #[test]
    fn the_layout_payload_round_trips_through_bytes() {
        let checksum = source_dex_checksum(b"app bytes");
        let header = Dex3oatArtifactHeader::current(checksum);
        let layouts = sample_layouts();
        let bytes = serialize_layouts(&header, &layouts);
        let restored = deserialize_layouts(&bytes, checksum, DEX3OAT_VERSION).unwrap();
        assert_eq!(restored, layouts);
    }

    // cite: art oat.h#CheckOatVersion
    #[test]
    fn a_stale_source_or_compiler_rejects_the_payload() {
        let checksum = source_dex_checksum(b"app bytes");
        let bytes = serialize_layouts(&Dex3oatArtifactHeader::current(checksum), &sample_layouts());
        let other = source_dex_checksum(b"different app bytes");
        assert!(deserialize_layouts(&bytes, other, DEX3OAT_VERSION).is_none());
        assert!(deserialize_layouts(&bytes, checksum, DEX3OAT_VERSION + 1).is_none());
    }

    // cite: art oat_file.h#Open
    // cite: art oat_file_assistant.h#GetBestOatFile
    #[test]
    fn the_artifact_persists_to_disk_and_reloads_when_fresh() {
        let checksum = source_dex_checksum(b"app bytes on disk");
        let header = Dex3oatArtifactHeader::current(checksum);
        let layouts = sample_layouts();
        let mut path = std::env::temp_dir();
        path.push(format!("wart-test-{}.wart", std::process::id()));

        write_layouts(&path, &header, &layouts).unwrap();
        let reloaded = load_layouts(&path, checksum, DEX3OAT_VERSION).unwrap();
        assert_eq!(reloaded, layouts);

        let stale = source_dex_checksum(b"app bytes changed on disk");
        assert!(load_layouts(&path, stale, DEX3OAT_VERSION).is_none());

        std::fs::remove_file(&path).ok();
    }

    // cite: art oat_file_assistant.h#GetBestOatFile
    #[test]
    fn a_missing_artifact_file_is_a_cache_miss_not_an_error() {
        let mut path = std::env::temp_dir();
        path.push("wart-test-does-not-exist.wart");
        std::fs::remove_file(&path).ok();
        assert!(load_layouts(&path, 0, DEX3OAT_VERSION).is_none());
    }
}
