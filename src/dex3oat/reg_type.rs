// cite: art reg_type.h#IsUndefined, IsConflict, IsInteger, IsFloat, IsLongLo, IsLongHi, IsDoubleLo, IsDoubleHi, IsReference, IsZero, IsNull, IsConstant
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegType {
    Undefined,
    Conflict,
    Zero,
    Const32,
    Const64Lo,
    Const64Hi,
    Int32,
    Float,
    LongLo,
    LongHi,
    DoubleLo,
    DoubleHi,
    Reference,
    Null,
}

impl RegType {
    pub fn is_wide_low(self) -> bool {
        matches!(
            self,
            RegType::LongLo | RegType::DoubleLo | RegType::Const64Lo
        )
    }

    pub fn is_wide_high(self) -> bool {
        matches!(
            self,
            RegType::LongHi | RegType::DoubleHi | RegType::Const64Hi
        )
    }

    pub fn is_reference(self) -> bool {
        matches!(self, RegType::Reference | RegType::Null | RegType::Zero)
    }

    // cite: art reg_type.h#IsIntegralTypes, IsConstant
    pub fn is_category1_int(self) -> bool {
        matches!(self, RegType::Int32 | RegType::Zero | RegType::Const32)
    }

    // cite: art reg_type.h#IsFloatTypes, IsConstant
    pub fn is_float_compatible(self) -> bool {
        matches!(
            self,
            RegType::Float | RegType::Int32 | RegType::Zero | RegType::Const32
        )
    }

    // cite: art reg_type.cc#Merge
    pub fn merge(self, incoming: RegType) -> RegType {
        if self == incoming {
            return self;
        }
        if self == RegType::Undefined || incoming == RegType::Undefined {
            return RegType::Undefined;
        }
        if self == RegType::Conflict || incoming == RegType::Conflict {
            return RegType::Conflict;
        }
        match (self, incoming) {
            // cite: art reg_type.cc#Merge
            (RegType::Zero | RegType::Const32, RegType::Int32)
            | (RegType::Int32, RegType::Zero | RegType::Const32) => RegType::Int32,
            (RegType::Zero | RegType::Const32, RegType::Float)
            | (RegType::Float, RegType::Zero | RegType::Const32) => RegType::Float,
            (RegType::Zero, RegType::Const32) | (RegType::Const32, RegType::Zero) => {
                RegType::Const32
            }
            (RegType::Const64Lo, RegType::LongLo) | (RegType::LongLo, RegType::Const64Lo) => {
                RegType::LongLo
            }
            (RegType::Const64Lo, RegType::DoubleLo) | (RegType::DoubleLo, RegType::Const64Lo) => {
                RegType::DoubleLo
            }
            (RegType::Const64Hi, RegType::LongHi) | (RegType::LongHi, RegType::Const64Hi) => {
                RegType::LongHi
            }
            (RegType::Const64Hi, RegType::DoubleHi) | (RegType::DoubleHi, RegType::Const64Hi) => {
                RegType::DoubleHi
            }
            (RegType::Zero, RegType::Reference) | (RegType::Reference, RegType::Zero) => {
                RegType::Reference
            }
            (RegType::Zero, RegType::Null) | (RegType::Null, RegType::Zero) => RegType::Null,
            (RegType::Null, RegType::Reference) | (RegType::Reference, RegType::Null) => {
                RegType::Reference
            }
            _ => RegType::Conflict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // cite: art reg_type.cc#Merge
    #[test]
    fn merge_is_identity_on_equal_types() {
        for t in [
            RegType::Int32,
            RegType::Reference,
            RegType::LongLo,
            RegType::Null,
        ] {
            assert_eq!(t.merge(t), t);
        }
    }

    // cite: art reg_type.cc#Merge
    #[test]
    fn undefined_and_conflict_dominate_merges() {
        assert_eq!(RegType::Undefined.merge(RegType::Int32), RegType::Undefined);
        assert_eq!(RegType::Int32.merge(RegType::Undefined), RegType::Undefined);
        assert_eq!(
            RegType::Conflict.merge(RegType::Reference),
            RegType::Conflict
        );
        assert_eq!(
            RegType::Reference.merge(RegType::Conflict),
            RegType::Conflict
        );
    }

    // cite: art reg_type.cc#Merge
    #[test]
    fn zero_unifies_with_int_or_reference() {
        assert_eq!(RegType::Zero.merge(RegType::Int32), RegType::Int32);
        assert_eq!(RegType::Zero.merge(RegType::Reference), RegType::Reference);
        assert_eq!(RegType::Zero.merge(RegType::Null), RegType::Null);
        assert_eq!(RegType::Null.merge(RegType::Reference), RegType::Reference);
    }

    // cite: art reg_type.cc#Merge
    #[test]
    fn a_nonzero_const_is_int_and_float_compatible() {
        assert_eq!(RegType::Const32.merge(RegType::Int32), RegType::Int32);
        assert_eq!(RegType::Const32.merge(RegType::Float), RegType::Float);
        assert_eq!(RegType::Float.merge(RegType::Const32), RegType::Float);
        assert_eq!(RegType::Const32.merge(RegType::Zero), RegType::Const32);
        assert_eq!(
            RegType::Const32.merge(RegType::Reference),
            RegType::Conflict
        );
        assert!(RegType::Const32.is_category1_int());
        assert!(RegType::Const32.is_float_compatible());
        assert!(!RegType::Const32.is_reference());
    }

    // cite: art reg_type.cc#Merge
    #[test]
    fn a_wide_const_is_long_and_double_compatible() {
        assert_eq!(RegType::Const64Lo.merge(RegType::LongLo), RegType::LongLo);
        assert_eq!(
            RegType::Const64Lo.merge(RegType::DoubleLo),
            RegType::DoubleLo
        );
        assert_eq!(
            RegType::DoubleLo.merge(RegType::Const64Lo),
            RegType::DoubleLo
        );
        assert_eq!(
            RegType::Const64Hi.merge(RegType::DoubleHi),
            RegType::DoubleHi
        );
        assert!(RegType::Const64Lo.is_wide_low());
        assert!(RegType::Const64Hi.is_wide_high());
    }

    // cite: art reg_type.cc#Merge
    #[test]
    fn incompatible_categories_merge_to_conflict() {
        assert_eq!(RegType::Int32.merge(RegType::Reference), RegType::Conflict);
        assert_eq!(RegType::Int32.merge(RegType::Float), RegType::Conflict);
        assert_eq!(RegType::LongLo.merge(RegType::Int32), RegType::Conflict);
        assert_eq!(RegType::Reference.merge(RegType::Float), RegType::Conflict);
    }
}
