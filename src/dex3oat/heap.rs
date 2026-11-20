use crate::dex3oat::reg_type::RegType;

pub const NULL_REFERENCE: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HeapValue {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Null,
    Reference(u32),
}

#[derive(Debug)]
enum Cell {
    Object {
        layout_index: u32,
        slots: Vec<u64>,
    },
    // cite: art array.h#GetLength, first_element_
    Array {
        component: RegType,
        elements: Vec<u64>,
    },
}

// cite: art class_linker.cc#LinkFields
// cite: art art_field.h#GetOffset
#[derive(Debug, Default)]
pub struct Dex3oatHeap {
    cells: Vec<Cell>,
}

impl Dex3oatHeap {
    pub fn new() -> Self {
        Self {
            cells: vec![Cell::Object {
                layout_index: 0,
                slots: Vec::new(),
            }],
        }
    }

    pub fn allocate(&mut self, layout_index: u32, slot_count: usize) -> u32 {
        let id = self.cells.len() as u32;
        self.cells.push(Cell::Object {
            layout_index,
            slots: vec![0u64; slot_count],
        });
        id
    }

    // cite: art class_linker.cc#CreateArrayClass
    // cite: art array.cc#Alloc
    pub fn allocate_array(&mut self, component: RegType, length: i32) -> Option<u32> {
        let length = usize::try_from(length).ok()?;
        let id = self.cells.len() as u32;
        self.cells.push(Cell::Array {
            component,
            elements: vec![0u64; length],
        });
        Some(id)
    }

    pub fn layout_index(&self, id: u32) -> Option<u32> {
        match self.cells.get(id as usize)? {
            Cell::Object { layout_index, .. } => Some(*layout_index),
            Cell::Array { .. } => None,
        }
    }

    fn slot_mut(&mut self, id: u32, slot: u16) -> Option<&mut u64> {
        match self.cells.get_mut(id as usize)? {
            Cell::Object { slots, .. } => slots.get_mut(slot as usize),
            Cell::Array { .. } => None,
        }
    }

    fn slot(&self, id: u32, slot: u16) -> Option<u64> {
        match self.cells.get(id as usize)? {
            Cell::Object { slots, .. } => slots.get(slot as usize).copied(),
            Cell::Array { .. } => None,
        }
    }

    // cite: art array.h#GetLength
    pub fn array_length(&self, id: u32) -> Option<i32> {
        match self.cells.get(id as usize)? {
            Cell::Array { elements, .. } => i32::try_from(elements.len()).ok(),
            Cell::Object { .. } => None,
        }
    }

    pub fn array_component(&self, id: u32) -> Option<RegType> {
        match self.cells.get(id as usize)? {
            Cell::Array { component, .. } => Some(*component),
            Cell::Object { .. } => None,
        }
    }

    // cite: art array-inl.h#GetWithoutChecks
    fn array_slot_mut(&mut self, id: u32, index: i32) -> Option<&mut u64> {
        let index = usize::try_from(index).ok()?;
        match self.cells.get_mut(id as usize)? {
            Cell::Array { elements, .. } => elements.get_mut(index),
            Cell::Object { .. } => None,
        }
    }

    fn array_word(&self, id: u32, index: i32) -> Option<u64> {
        let index = usize::try_from(index).ok()?;
        match self.cells.get(id as usize)? {
            Cell::Array { elements, .. } => elements.get(index).copied(),
            Cell::Object { .. } => None,
        }
    }

    pub fn array_set_word(&mut self, id: u32, index: i32, word: u64) -> Option<()> {
        *self.array_slot_mut(id, index)? = word;
        Some(())
    }

    pub fn array_get_word(&self, id: u32, index: i32) -> Option<u64> {
        self.array_word(id, index)
    }

    pub fn set_int(&mut self, id: u32, slot: u16, value: i32) -> Option<()> {
        *self.slot_mut(id, slot)? = u64::from(value as u32);
        Some(())
    }

    pub fn set_long(&mut self, id: u32, slot: u16, value: i64) -> Option<()> {
        *self.slot_mut(id, slot)? = value as u64;
        Some(())
    }

    pub fn set_float(&mut self, id: u32, slot: u16, value: f32) -> Option<()> {
        *self.slot_mut(id, slot)? = u64::from(value.to_bits());
        Some(())
    }

    pub fn set_double(&mut self, id: u32, slot: u16, value: f64) -> Option<()> {
        *self.slot_mut(id, slot)? = value.to_bits();
        Some(())
    }

    pub fn set_reference(&mut self, id: u32, slot: u16, referent: u32) -> Option<()> {
        *self.slot_mut(id, slot)? = u64::from(referent);
        Some(())
    }

    pub fn get_int(&self, id: u32, slot: u16) -> Option<i32> {
        Some(self.slot(id, slot)? as u32 as i32)
    }

    pub fn get_long(&self, id: u32, slot: u16) -> Option<i64> {
        Some(self.slot(id, slot)? as i64)
    }

    pub fn get_reference(&self, id: u32, slot: u16) -> Option<u32> {
        Some(self.slot(id, slot)? as u32)
    }

    pub fn materialize(&self, id: u32, slot: u16, ty: RegType) -> Option<HeapValue> {
        Self::word_as(self.slot(id, slot)?, ty)
    }

    // cite: art array-inl.h#GetWithoutChecks
    pub fn array_element(&self, id: u32, index: i32) -> Option<HeapValue> {
        let component = self.array_component(id)?;
        Self::word_as(self.array_get_word(id, index)?, component)
    }

    // cite: art reg_type.cc#IsZero, IsNull
    fn word_as(word: u64, ty: RegType) -> Option<HeapValue> {
        Some(match ty {
            RegType::Int32 | RegType::Zero | RegType::Const32 => HeapValue::Int(word as u32 as i32),
            RegType::Float => HeapValue::Float(f32::from_bits(word as u32)),
            RegType::LongLo => HeapValue::Long(word as i64),
            RegType::DoubleLo => HeapValue::Double(f64::from_bits(word)),
            RegType::Reference | RegType::Null => {
                if word as u32 == NULL_REFERENCE {
                    HeapValue::Null
                } else {
                    HeapValue::Reference(word as u32)
                }
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // cite: art class_linker.cc#LinkFields
    #[test]
    fn allocation_sizes_slots_to_the_layout_and_zero_initializes() {
        let mut heap = Dex3oatHeap::new();
        let id = heap.allocate(3, 4);
        assert_eq!(heap.layout_index(id), Some(3));
        assert_eq!(heap.get_int(id, 0), Some(0));
        assert_eq!(heap.get_reference(id, 3), Some(NULL_REFERENCE));
        assert_eq!(heap.get_int(id, 4), None);
    }

    // cite: art art_field.h#GetOffset
    #[test]
    fn typed_slot_writes_round_trip_through_raw_words() {
        let mut heap = Dex3oatHeap::new();
        let id = heap.allocate(0, 4);
        heap.set_int(id, 0, -7).unwrap();
        heap.set_long(id, 1, 0x0123_4567_89ab_cdef).unwrap();
        heap.set_reference(id, 2, 42).unwrap();
        heap.set_float(id, 3, 2.5).unwrap();
        assert_eq!(heap.get_int(id, 0), Some(-7));
        assert_eq!(heap.get_long(id, 1), Some(0x0123_4567_89ab_cdef));
        assert_eq!(heap.get_reference(id, 2), Some(42));
        assert_eq!(
            heap.materialize(id, 3, RegType::Float),
            Some(HeapValue::Float(2.5))
        );
    }

    // cite: art reg_type.cc#IsZero, IsNull
    #[test]
    fn materialize_uses_the_verified_type_and_null_is_zero() {
        let mut heap = Dex3oatHeap::new();
        let id = heap.allocate(0, 2);
        heap.set_int(id, 0, 9).unwrap();
        assert_eq!(
            heap.materialize(id, 0, RegType::Int32),
            Some(HeapValue::Int(9))
        );
        assert_eq!(
            heap.materialize(id, 1, RegType::Reference),
            Some(HeapValue::Null)
        );
        heap.set_reference(id, 1, 5).unwrap();
        assert_eq!(
            heap.materialize(id, 1, RegType::Reference),
            Some(HeapValue::Reference(5))
        );
    }

    #[test]
    fn distinct_objects_of_one_layout_have_independent_storage() {
        let mut heap = Dex3oatHeap::new();
        let a = heap.allocate(1, 2);
        let b = heap.allocate(1, 2);
        heap.set_int(a, 0, 100).unwrap();
        heap.set_int(b, 0, 200).unwrap();
        assert_ne!(a, b);
        assert_eq!(heap.get_int(a, 0), Some(100));
        assert_eq!(heap.get_int(b, 0), Some(200));
    }

    // cite: art array.h#GetLength
    // cite: art array.cc#Alloc
    #[test]
    fn an_array_carries_length_component_and_zeroed_elements() {
        let mut heap = Dex3oatHeap::new();
        let id = heap.allocate_array(RegType::Int32, 3).unwrap();
        assert_eq!(heap.array_length(id), Some(3));
        assert_eq!(heap.array_component(id), Some(RegType::Int32));
        assert_eq!(heap.array_element(id, 0), Some(HeapValue::Int(0)));
        assert_eq!(heap.array_length(0), None);
    }

    // cite: art array.cc#Alloc
    #[test]
    fn a_negative_length_array_is_rejected() {
        let mut heap = Dex3oatHeap::new();
        assert_eq!(heap.allocate_array(RegType::Int32, -1), None);
    }

    // cite: art array-inl.h#GetWithoutChecks
    #[test]
    fn array_element_writes_round_trip_and_out_of_bounds_is_none() {
        let mut heap = Dex3oatHeap::new();
        let id = heap.allocate_array(RegType::LongLo, 2).unwrap();
        heap.array_set_word(id, 0, 0x1122_3344_5566_7788).unwrap();
        heap.array_set_word(id, 1, 42).unwrap();
        assert_eq!(
            heap.array_element(id, 0),
            Some(HeapValue::Long(0x1122_3344_5566_7788))
        );
        assert_eq!(heap.array_element(id, 1), Some(HeapValue::Long(42)));
        assert_eq!(heap.array_get_word(id, 2), None);
        assert_eq!(heap.array_get_word(id, -1), None);
        assert_eq!(heap.array_set_word(id, 2, 0), None);
    }
}
