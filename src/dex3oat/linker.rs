use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkableField {
    pub name: String,
    pub type_descriptor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkableMethod {
    pub name: String,
    pub proto: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkableClass {
    pub descriptor: String,
    pub superclass: Option<String>,
    pub instance_fields: Vec<LinkableField>,
    pub static_fields: Vec<LinkableField>,
    pub virtual_methods: Vec<LinkableMethod>,
}

pub trait ClassSource {
    fn linkable_class(&self, descriptor: &str) -> Option<LinkableClass>;
}

impl ClassSource for HashMap<String, LinkableClass> {
    fn linkable_class(&self, descriptor: &str) -> Option<LinkableClass> {
        self.get(descriptor).cloned()
    }
}

// cite: art class_linker.cc#FieldTypeOrderFromFirstDescriptorCharacter
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FieldTypeOrder {
    Reference,
    Long,
    Double,
    Int,
    Float,
    Char,
    Short,
    Boolean,
    Byte,
}

impl FieldTypeOrder {
    // cite: art class_linker.cc#FieldTypeOrderFromFirstDescriptorCharacter
    fn from_type_descriptor(type_descriptor: &str) -> Self {
        match type_descriptor.as_bytes().first() {
            Some(b'J') => FieldTypeOrder::Long,
            Some(b'D') => FieldTypeOrder::Double,
            Some(b'I') => FieldTypeOrder::Int,
            Some(b'F') => FieldTypeOrder::Float,
            Some(b'C') => FieldTypeOrder::Char,
            Some(b'S') => FieldTypeOrder::Short,
            Some(b'Z') => FieldTypeOrder::Boolean,
            Some(b'B') => FieldTypeOrder::Byte,
            _ => FieldTypeOrder::Reference,
        }
    }

    fn is_reference(self) -> bool {
        self == FieldTypeOrder::Reference
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSlot {
    pub name: String,
    pub type_descriptor: String,
    pub declaring_class: String,
    pub slot: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VtableEntry {
    pub name: String,
    pub proto: String,
    pub declaring_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassLayout {
    pub descriptor: String,
    pub superclass: Option<String>,
    pub fields: Vec<FieldSlot>,
    pub reference_field_count: u16,
    pub static_fields: Vec<FieldSlot>,
    pub static_reference_field_count: u16,
    pub vtable: Vec<VtableEntry>,
}

impl ClassLayout {
    pub fn slot_of(&self, name: &str, type_descriptor: &str) -> Option<u16> {
        self.fields
            .iter()
            .find(|field| field.name == name && field.type_descriptor == type_descriptor)
            .map(|field| field.slot)
    }

    // cite: art class_linker.cc#LinkStaticFields
    pub fn static_slot_of(&self, name: &str, type_descriptor: &str) -> Option<u16> {
        self.static_fields
            .iter()
            .find(|field| field.name == name && field.type_descriptor == type_descriptor)
            .map(|field| field.slot)
    }

    pub fn static_slot_count(&self) -> usize {
        self.static_fields.len()
    }

    pub fn vtable_index_of(&self, name: &str, proto: &str) -> Option<usize> {
        self.vtable
            .iter()
            .position(|entry| entry.name == name && entry.proto == proto)
    }
}

const MAX_HIERARCHY_DEPTH: usize = 256;

#[derive(Debug, Default)]
pub struct Dex3oatLinker {
    layouts: HashMap<String, ClassLayout>,
}

impl Dex3oatLinker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn link_class(
        &mut self,
        source: &dyn ClassSource,
        descriptor: &str,
    ) -> Option<&ClassLayout> {
        if !self.layouts.contains_key(descriptor) {
            let layout = self.build_layout(source, descriptor, 0)?;
            self.layouts.insert(descriptor.to_owned(), layout);
        }
        self.layouts.get(descriptor)
    }

    pub fn layout(&self, descriptor: &str) -> Option<&ClassLayout> {
        self.layouts.get(descriptor)
    }

    fn build_layout(
        &mut self,
        source: &dyn ClassSource,
        descriptor: &str,
        depth: usize,
    ) -> Option<ClassLayout> {
        if depth > MAX_HIERARCHY_DEPTH {
            return None;
        }
        let class = source.linkable_class(descriptor)?;

        let super_layout = match &class.superclass {
            Some(super_descriptor) => {
                if !self.layouts.contains_key(super_descriptor) {
                    let built = self.build_layout(source, super_descriptor, depth + 1)?;
                    self.layouts.insert(super_descriptor.clone(), built);
                }
                self.layouts.get(super_descriptor)
            }
            None => None,
        };

        // cite: art class_linker.cc#LinkFields
        let mut fields: Vec<FieldSlot> = super_layout
            .map(|layout| layout.fields.clone())
            .unwrap_or_default();
        let mut reference_field_count = super_layout
            .map(|layout| layout.reference_field_count)
            .unwrap_or(0);
        let base_slot = fields.len();

        // cite: art class_linker.cc#LinkFields, FieldTypeOrderFromFirstDescriptorCharacter
        let mut ordered: Vec<(usize, &LinkableField)> =
            class.instance_fields.iter().enumerate().collect();
        ordered.sort_by(|(index_a, a), (index_b, b)| {
            FieldTypeOrder::from_type_descriptor(&a.type_descriptor)
                .cmp(&FieldTypeOrder::from_type_descriptor(&b.type_descriptor))
                .then(index_a.cmp(index_b))
        });

        for (position, (_, field)) in ordered.into_iter().enumerate() {
            if FieldTypeOrder::from_type_descriptor(&field.type_descriptor).is_reference() {
                reference_field_count += 1;
            }
            fields.push(FieldSlot {
                name: field.name.clone(),
                type_descriptor: field.type_descriptor.clone(),
                declaring_class: descriptor.to_owned(),
                slot: u16::try_from(base_slot + position).ok()?,
            });
        }

        // cite: art class_linker.cc#LinkStaticFields, LinkFieldsHelper
        let mut static_fields: Vec<FieldSlot> = Vec::new();
        let mut static_reference_field_count = 0u16;
        let mut ordered_statics: Vec<(usize, &LinkableField)> =
            class.static_fields.iter().enumerate().collect();
        ordered_statics.sort_by(|(index_a, a), (index_b, b)| {
            FieldTypeOrder::from_type_descriptor(&a.type_descriptor)
                .cmp(&FieldTypeOrder::from_type_descriptor(&b.type_descriptor))
                .then(index_a.cmp(index_b))
        });
        for (position, (_, field)) in ordered_statics.into_iter().enumerate() {
            if FieldTypeOrder::from_type_descriptor(&field.type_descriptor).is_reference() {
                static_reference_field_count += 1;
            }
            static_fields.push(FieldSlot {
                name: field.name.clone(),
                type_descriptor: field.type_descriptor.clone(),
                declaring_class: descriptor.to_owned(),
                slot: u16::try_from(position).ok()?,
            });
        }

        // cite: art class_linker.cc#AssignVTableIndexes
        let mut vtable: Vec<VtableEntry> = super_layout
            .map(|layout| layout.vtable.clone())
            .unwrap_or_default();
        for method in &class.virtual_methods {
            let entry = VtableEntry {
                name: method.name.clone(),
                proto: method.proto.clone(),
                declaring_class: descriptor.to_owned(),
            };
            match vtable
                .iter()
                .position(|existing| existing.name == entry.name && existing.proto == entry.proto)
            {
                Some(index) => vtable[index] = entry,
                None => vtable.push(entry),
            }
        }

        Some(ClassLayout {
            descriptor: descriptor.to_owned(),
            superclass: class.superclass.clone(),
            fields,
            reference_field_count,
            static_fields,
            static_reference_field_count,
            vtable,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(name: &str, type_descriptor: &str) -> LinkableField {
        LinkableField {
            name: name.to_owned(),
            type_descriptor: type_descriptor.to_owned(),
        }
    }

    fn method(name: &str, proto: &str) -> LinkableMethod {
        LinkableMethod {
            name: name.to_owned(),
            proto: proto.to_owned(),
        }
    }

    fn class(
        descriptor: &str,
        superclass: Option<&str>,
        instance_fields: Vec<LinkableField>,
        virtual_methods: Vec<LinkableMethod>,
    ) -> LinkableClass {
        LinkableClass {
            descriptor: descriptor.to_owned(),
            superclass: superclass.map(str::to_owned),
            instance_fields,
            static_fields: Vec::new(),
            virtual_methods,
        }
    }

    // cite: art class_linker.cc#LinkStaticFields
    #[test]
    fn static_fields_get_their_own_reference_first_slot_block_per_class() {
        let mut base = class("Lp;", None, vec![field("i", "I")], Vec::new());
        base.static_fields = vec![field("count", "I"), field("shared", "Ljava/lang/Object;")];
        let mut sub = class("Lq;", Some("Lp;"), Vec::new(), Vec::new());
        sub.static_fields = vec![field("total", "J")];
        let mut source = HashMap::new();
        source.insert("Lp;".to_owned(), base);
        source.insert("Lq;".to_owned(), sub);

        let mut linker = Dex3oatLinker::new();
        let base_layout = linker.link_class(&source, "Lp;").unwrap().clone();
        assert_eq!(
            base_layout.static_slot_of("shared", "Ljava/lang/Object;"),
            Some(0)
        );
        assert_eq!(base_layout.static_slot_of("count", "I"), Some(1));
        assert_eq!(base_layout.static_reference_field_count, 1);

        let sub_layout = linker.link_class(&source, "Lq;").unwrap();
        assert_eq!(sub_layout.static_slot_of("total", "J"), Some(0));
        assert_eq!(sub_layout.static_slot_of("count", "I"), None);
        assert_eq!(sub_layout.static_slot_count(), 1);
    }

    // cite: art class_linker.cc#LinkFields, FieldTypeOrderFromFirstDescriptorCharacter
    #[test]
    fn orders_fields_references_first_then_by_descending_width() {
        let mut source = HashMap::new();
        source.insert(
            "LFoo;".to_owned(),
            class(
                "LFoo;",
                None,
                vec![
                    field("a", "I"),
                    field("b", "Ljava/lang/Object;"),
                    field("c", "J"),
                    field("d", "Z"),
                ],
                Vec::new(),
            ),
        );
        let mut linker = Dex3oatLinker::new();
        let layout = linker.link_class(&source, "LFoo;").expect("link");
        assert_eq!(layout.slot_of("b", "Ljava/lang/Object;"), Some(0));
        assert_eq!(layout.slot_of("c", "J"), Some(1));
        assert_eq!(layout.slot_of("a", "I"), Some(2));
        assert_eq!(layout.slot_of("d", "Z"), Some(3));
        assert_eq!(layout.reference_field_count, 1);
    }

    // cite: art class_linker.cc#LinkFields
    #[test]
    fn subclass_fields_append_after_inherited_superclass_slots() {
        let mut source = HashMap::new();
        source.insert(
            "LBase;".to_owned(),
            class(
                "LBase;",
                None,
                vec![field("x", "Ljava/lang/Object;"), field("y", "I")],
                Vec::new(),
            ),
        );
        source.insert(
            "LDerived;".to_owned(),
            class(
                "LDerived;",
                Some("LBase;"),
                vec![field("z", "I")],
                Vec::new(),
            ),
        );
        let mut linker = Dex3oatLinker::new();
        let layout = linker.link_class(&source, "LDerived;").expect("link");
        assert_eq!(layout.slot_of("x", "Ljava/lang/Object;"), Some(0));
        assert_eq!(layout.slot_of("y", "I"), Some(1));
        assert_eq!(layout.slot_of("z", "I"), Some(2));
        assert_eq!(layout.fields.len(), 3);
    }

    // cite: art class_linker.cc#AssignVTableIndexes
    #[test]
    fn vtable_inherits_overrides_by_signature_and_appends_new() {
        let mut source = HashMap::new();
        source.insert(
            "LBase;".to_owned(),
            class(
                "LBase;",
                None,
                Vec::new(),
                vec![method("foo", "()V"), method("bar", "(I)V")],
            ),
        );
        source.insert(
            "LDerived;".to_owned(),
            class(
                "LDerived;",
                Some("LBase;"),
                Vec::new(),
                vec![method("foo", "()V"), method("baz", "()V")],
            ),
        );
        let mut linker = Dex3oatLinker::new();
        let layout = linker
            .link_class(&source, "LDerived;")
            .expect("link")
            .clone();
        assert_eq!(layout.vtable_index_of("foo", "()V"), Some(0));
        assert_eq!(layout.vtable_index_of("bar", "(I)V"), Some(1));
        assert_eq!(layout.vtable_index_of("baz", "()V"), Some(2));
        assert_eq!(layout.vtable[0].declaring_class, "LDerived;");
        assert_eq!(layout.vtable[1].declaring_class, "LBase;");
        assert_eq!(layout.vtable.len(), 3);
    }

    // cite: art class_linker.cc#AssignVTableIndexes
    #[test]
    fn vtable_overload_with_different_proto_does_not_override() {
        let mut source = HashMap::new();
        source.insert(
            "LBase;".to_owned(),
            class("LBase;", None, Vec::new(), vec![method("foo", "()V")]),
        );
        source.insert(
            "LDerived;".to_owned(),
            class(
                "LDerived;",
                Some("LBase;"),
                Vec::new(),
                vec![method("foo", "(I)V")],
            ),
        );
        let mut linker = Dex3oatLinker::new();
        let layout = linker.link_class(&source, "LDerived;").expect("link");
        assert_eq!(layout.vtable_index_of("foo", "()V"), Some(0));
        assert_eq!(layout.vtable_index_of("foo", "(I)V"), Some(1));
        assert_eq!(layout.vtable.len(), 2);
    }

    #[test]
    fn missing_superclass_fails_the_link_without_panicking() {
        let mut source = HashMap::new();
        source.insert(
            "LOrphan;".to_owned(),
            class(
                "LOrphan;",
                Some("LGone;"),
                vec![field("a", "I")],
                Vec::new(),
            ),
        );
        let mut linker = Dex3oatLinker::new();
        assert!(linker.link_class(&source, "LOrphan;").is_none());
    }
}
