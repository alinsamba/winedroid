use std::collections::HashMap;

// cite: art profiling_info.h#kIndividualCacheSize
pub const INLINE_CACHE_SIZE: usize = 5;

// cite: art profiling_info.h#InlineCache
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineCache {
    classes: HashMap<String, u32>,
}

impl InlineCache {
    // cite: art profiling_info.cc#AddInvokeInfo
    pub fn add(&mut self, class_descriptor: &str) {
        *self.classes.entry(class_descriptor.to_owned()).or_insert(0) += 1;
    }

    // cite: art inliner.cc#TryInlineMonomorphicCall
    pub fn monomorphic(&self) -> Option<&str> {
        (self.classes.len() == 1)
            .then(|| self.classes.keys().next().map(String::as_str))
            .flatten()
    }

    // cite: art inliner.cc#kInlineCacheMegamorphic
    pub fn is_megamorphic(&self) -> bool {
        self.classes.len() > INLINE_CACHE_SIZE
    }
}

pub type CallSite = (usize, u32, u32);

// cite: art profiling_info.h#ProfilingInfo
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Profile {
    sites: HashMap<CallSite, InlineCache>,
}

impl Profile {
    // cite: art profiling_info.cc#AddInvokeInfo
    pub fn add_invoke_info(&mut self, site: CallSite, class_descriptor: &str) {
        self.sites.entry(site).or_default().add(class_descriptor);
    }

    // cite: art profiling_info.cc#GetInlineCache
    pub fn get_inline_cache(&self, site: CallSite) -> Option<&InlineCache> {
        self.sites.get(&site)
    }

    // cite: art inliner.cc#TryInlineMonomorphicCall
    pub fn monomorphic(&self, site: CallSite) -> Option<&str> {
        self.get_inline_cache(site)?.monomorphic()
    }

    // cite: art profile_compilation_info.cc#Save
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(self.sites.len() as u32).to_le_bytes());
        let mut sites: Vec<(&CallSite, &InlineCache)> = self.sites.iter().collect();
        sites.sort_by_key(|(site, _)| **site);
        for ((dex, offset, pc), cache) in sites {
            out.extend_from_slice(&(*dex as u32).to_le_bytes());
            out.extend_from_slice(&offset.to_le_bytes());
            out.extend_from_slice(&pc.to_le_bytes());
            out.extend_from_slice(&(cache.classes.len() as u32).to_le_bytes());
            let mut classes: Vec<(&String, &u32)> = cache.classes.iter().collect();
            classes.sort_by(|a, b| a.0.cmp(b.0));
            for (descriptor, count) in classes {
                out.extend_from_slice(&(descriptor.len() as u32).to_le_bytes());
                out.extend_from_slice(descriptor.as_bytes());
                out.extend_from_slice(&count.to_le_bytes());
            }
        }
        out
    }

    // cite: art profile_compilation_info.cc#Load
    pub fn deserialize(bytes: &[u8]) -> Option<Profile> {
        let mut cursor = 0usize;
        let mut read_u32 = |cursor: &mut usize| -> Option<u32> {
            let slice = bytes.get(*cursor..*cursor + 4)?;
            *cursor += 4;
            Some(u32::from_le_bytes(slice.try_into().ok()?))
        };
        let site_count = read_u32(&mut cursor)?;
        let mut profile = Profile::default();
        for _ in 0..site_count {
            let dex = read_u32(&mut cursor)? as usize;
            let offset = read_u32(&mut cursor)?;
            let pc = read_u32(&mut cursor)?;
            let class_count = read_u32(&mut cursor)?;
            let mut cache = InlineCache::default();
            for _ in 0..class_count {
                let len = read_u32(&mut cursor)? as usize;
                let descriptor = std::str::from_utf8(bytes.get(cursor..cursor + len)?).ok()?;
                cursor += len;
                let count = read_u32(&mut cursor)?;
                cache.classes.insert(descriptor.to_owned(), count);
            }
            profile.sites.insert((dex, offset, pc), cache);
        }
        Some(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_observed_class_is_monomorphic() {
        let mut profile = Profile::default();
        let site = (0, 100, 4);
        profile.add_invoke_info(site, "Lcom/example/Dog;");
        profile.add_invoke_info(site, "Lcom/example/Dog;");
        assert_eq!(profile.monomorphic(site), Some("Lcom/example/Dog;"));
    }

    #[test]
    fn two_classes_are_not_monomorphic() {
        let mut profile = Profile::default();
        let site = (0, 100, 4);
        profile.add_invoke_info(site, "Lcom/example/Dog;");
        profile.add_invoke_info(site, "Lcom/example/Cat;");
        assert_eq!(profile.monomorphic(site), None);
        assert!(!profile.get_inline_cache(site).unwrap().is_megamorphic());
    }

    #[test]
    fn many_classes_are_megamorphic() {
        let mut cache = InlineCache::default();
        for i in 0..INLINE_CACHE_SIZE + 1 {
            cache.add(&format!("Lc{i};"));
        }
        assert!(cache.is_megamorphic());
        assert_eq!(cache.monomorphic(), None);
    }

    #[test]
    fn serialize_roundtrips() {
        let mut profile = Profile::default();
        profile.add_invoke_info((0, 100, 4), "Lcom/example/Dog;");
        profile.add_invoke_info((0, 100, 4), "Lcom/example/Dog;");
        profile.add_invoke_info((0, 100, 4), "Lcom/example/Cat;");
        profile.add_invoke_info((1, 200, 8), "Ljava/lang/String;");
        let bytes = profile.serialize();
        let restored = Profile::deserialize(&bytes).expect("roundtrips");
        assert_eq!(restored, profile);
    }

    #[test]
    fn deserialize_rejects_truncated_input() {
        assert!(Profile::deserialize(&[9, 9, 9]).is_none());
    }
}
