use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneRulesData {
    pub transitions: Vec<i64>,
    pub offsets_after: Vec<i32>,
    pub first_offset: i32,
}

impl ZoneRulesData {
    pub fn fixed(offset_seconds: i32) -> Self {
        ZoneRulesData {
            transitions: Vec::new(),
            offsets_after: Vec::new(),
            first_offset: offset_seconds,
        }
    }

    // cite: libcore ZoneRules.java#isFixedOffset
    pub fn is_fixed(&self) -> bool {
        self.transitions.is_empty()
    }

    // cite: libcore ZoneRules.java#getOffset
    pub fn offset_seconds_at(&self, epoch_second: i64) -> i32 {
        match self.transitions.partition_point(|&t| t <= epoch_second) {
            0 => self.first_offset,
            n => self.offsets_after[n - 1],
        }
    }

    // cite: libcore ZoneRules.java#getOffsetInfo
    pub fn offset_seconds_for_local(&self, local_second: i64) -> i32 {
        let mut result = self.first_offset;
        for (index, &transition) in self.transitions.iter().enumerate() {
            let offset_before = if index == 0 {
                self.first_offset
            } else {
                self.offsets_after[index - 1]
            };
            let offset_after = self.offsets_after[index];
            let window_start = transition + i64::from(offset_before.min(offset_after));
            let window_end = transition + i64::from(offset_before.max(offset_after));
            if local_second < window_start {
                break;
            }
            if local_second >= window_end {
                result = offset_after;
                continue;
            }
            result = offset_before;
            break;
        }

        result
    }
}

// cite: libcore ZoneId.java#of
pub fn zone_rules_for_id(zone_id: &str) -> Option<ZoneRulesData> {
    if matches!(zone_id, "Z" | "UTC" | "GMT" | "UT") {
        return Some(ZoneRulesData::fixed(0));
    }
    if let Some(offset) = zone_id
        .strip_prefix("UTC")
        .or_else(|| zone_id.strip_prefix("GMT"))
        .or_else(|| zone_id.strip_prefix("UT"))
    {
        return parse_offset_id(offset).map(ZoneRulesData::fixed);
    }
    if zone_id.starts_with('+') || zone_id.starts_with('-') {
        return parse_offset_id(zone_id).map(ZoneRulesData::fixed);
    }

    load_region_zone(zone_id)
}

// cite: libcore ZoneOffset.java#of
fn parse_offset_id(offset_id: &str) -> Option<i32> {
    let (sign, digits) = match offset_id.as_bytes().first()? {
        b'+' => (1, &offset_id[1..]),
        b'-' => (-1, &offset_id[1..]),
        _ => return None,
    };
    let mut parts = digits.split(':');
    let hours: i32 = parts.next()?.parse().ok()?;
    let minutes: i32 = parts.next().map_or(Some(0), |part| part.parse().ok())?;
    let seconds: i32 = parts.next().map_or(Some(0), |part| part.parse().ok())?;
    if parts.next().is_some() || hours > 18 || minutes > 59 || seconds > 59 {
        return None;
    }

    Some(sign * (hours * 3600 + minutes * 60 + seconds))
}

fn load_region_zone(zone_id: &str) -> Option<ZoneRulesData> {
    if zone_id
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == ".." || part.contains('\\'))
    {
        return None;
    }

    for base in ["/usr/share/zoneinfo", "/var/db/timezone/zoneinfo"] {
        let path = Path::new(base).join(zone_id);
        if let Ok(bytes) = std::fs::read(&path) {
            return parse_tzif(&bytes);
        }
    }

    None
}

struct TzifCounts {
    isutcnt: usize,
    isstdcnt: usize,
    leapcnt: usize,
    timecnt: usize,
    typecnt: usize,
    charcnt: usize,
}

impl TzifCounts {
    fn data_block_len(&self, time_size: usize) -> usize {
        self.timecnt * time_size
            + self.timecnt
            + self.typecnt * 6
            + self.charcnt
            + self.leapcnt * (time_size + 4)
            + self.isstdcnt
            + self.isutcnt
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_i32(bytes: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_i64(bytes: &[u8], offset: usize) -> Option<i64> {
    Some(i64::from_be_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

fn parse_tzif_counts(bytes: &[u8], header_offset: usize) -> Option<TzifCounts> {
    if bytes.get(header_offset..header_offset + 4)? != b"TZif" {
        return None;
    }

    Some(TzifCounts {
        isutcnt: read_u32(bytes, header_offset + 20)? as usize,
        isstdcnt: read_u32(bytes, header_offset + 24)? as usize,
        leapcnt: read_u32(bytes, header_offset + 28)? as usize,
        timecnt: read_u32(bytes, header_offset + 32)? as usize,
        typecnt: read_u32(bytes, header_offset + 36)? as usize,
        charcnt: read_u32(bytes, header_offset + 40)? as usize,
    })
}

// cite: libcore ZoneInfo.java#mTransitions
fn parse_tzif(bytes: &[u8]) -> Option<ZoneRulesData> {
    let v1_counts = parse_tzif_counts(bytes, 0)?;
    let version = *bytes.get(4)?;

    let (counts, data_offset, time_size) = if version >= b'2' {
        let v2_header = 44 + v1_counts.data_block_len(4);
        let v2_counts = parse_tzif_counts(bytes, v2_header)?;
        (v2_counts, v2_header + 44, 8)
    } else {
        (v1_counts, 44, 4)
    };

    if counts.typecnt == 0 {
        return None;
    }

    let mut transitions = Vec::with_capacity(counts.timecnt);
    for index in 0..counts.timecnt {
        let value = if time_size == 8 {
            read_i64(bytes, data_offset + index * 8)?
        } else {
            i64::from(read_i32(bytes, data_offset + index * 4)?)
        };
        transitions.push(value);
    }

    let type_index_offset = data_offset + counts.timecnt * time_size;
    let ttinfo_offset = type_index_offset + counts.timecnt;
    let mut type_offsets = Vec::with_capacity(counts.typecnt);
    for index in 0..counts.typecnt {
        type_offsets.push(read_i32(bytes, ttinfo_offset + index * 6)?);
    }

    let mut offsets_after = Vec::with_capacity(counts.timecnt);
    for index in 0..counts.timecnt {
        let type_index = *bytes.get(type_index_offset + index)? as usize;
        offsets_after.push(*type_offsets.get(type_index)?);
    }

    Some(ZoneRulesData {
        transitions,
        offsets_after,
        first_offset: type_offsets[0],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_offset_ids_parse_without_tz_database() {
        assert_eq!(zone_rules_for_id("Z"), Some(ZoneRulesData::fixed(0)));
        assert_eq!(zone_rules_for_id("UTC"), Some(ZoneRulesData::fixed(0)));
        assert_eq!(
            zone_rules_for_id("+05:30"),
            Some(ZoneRulesData::fixed(5 * 3600 + 30 * 60))
        );
        assert_eq!(
            zone_rules_for_id("UTC-08:00"),
            Some(ZoneRulesData::fixed(-8 * 3600))
        );
        assert_eq!(zone_rules_for_id("+19:00"), None);
    }

    #[test]
    fn region_zone_offsets_match_recorded_utc_instants() {
        let Some(rules) = zone_rules_for_id("Asia/Kolkata") else {
            return;
        };
        assert!(!rules.is_fixed());
        assert_eq!(rules.offset_seconds_at(1_750_000_000), 5 * 3600 + 30 * 60);

        let Some(london) = zone_rules_for_id("Europe/London") else {
            return;
        };
        assert_eq!(london.offset_seconds_at(1_750_000_000), 3600);
        assert_eq!(london.offset_seconds_at(1_735_689_600), 0);
    }

    #[test]
    fn local_time_offsets_match_zone_rules_get_offset_local() {
        let Some(london) = zone_rules_for_id("Europe/London") else {
            return;
        };
        assert_eq!(london.offset_seconds_for_local(1_750_003_600 + 3600), 3600);
        assert_eq!(london.offset_seconds_for_local(1_735_689_600), 0);
        let gap_local = 1_743_296_400 + 1800;
        assert_eq!(london.offset_seconds_for_local(gap_local), 0);
    }

    #[test]
    fn traversal_ids_are_rejected() {
        assert_eq!(zone_rules_for_id("../etc/passwd"), None);
        assert_eq!(zone_rules_for_id(""), None);
    }
}
