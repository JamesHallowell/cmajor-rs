#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct BindingPower(u8);

#[derive(Debug, Copy, Clone)]
pub struct InfixBindingPower {
    pub binds_at: BindingPower,
    pub min_for_rhs: BindingPower,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PrecedenceLevel {
    Assign,
    Ternary,
    Or,
    And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equality,
    Relational,
    Shift,
    Additive,
    Multiplicative,
    Power,
    Unary,
}

impl PrecedenceLevel {
    pub const fn lowest() -> BindingPower {
        BindingPower(0)
    }

    pub const fn base(self) -> BindingPower {
        BindingPower(2 * (self as u8 + 1))
    }

    pub const fn left_associative(self) -> InfixBindingPower {
        let base = self.base();
        InfixBindingPower {
            binds_at: base,
            min_for_rhs: BindingPower(base.0 + 1),
        }
    }

    pub const fn right_associative(self) -> InfixBindingPower {
        let base = self.base();
        InfixBindingPower {
            binds_at: BindingPower(base.0 + 1),
            min_for_rhs: base,
        }
    }
}
