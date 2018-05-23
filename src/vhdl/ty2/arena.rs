// Copyright (c) 2018 Fabian Schuiki

use arenas::{Alloc, AllocOwned};
use ty2::prelude::*;
use ty2::types::NullType;
use ty2::ints::*;
use ty2::floats::*;
use ty2::enums::*;
use ty2::physical::*;
use ty2::access::*;

make_arenas!(
    /// An arena to allocate types nodes into.
    pub struct TypeArena<'t> {
        integer_basetype: IntegerBasetype,
        integer_subtype: IntegerSubtype<'t>,
        floating_basetype: FloatingBasetype,
        floating_subtype: FloatingSubtype<'t>,
        enum_basetype: EnumBasetype,
        enum_subtype: EnumSubtype<'t>,
        physical_basetype: PhysicalBasetype,
        physical_subtype: PhysicalSubtype<'t>,
        access: AccessType<'t>,
    }
);

impl<'t> AllocOwned<'t, 't, Type> for TypeArena<'t> {
    fn alloc_owned(&'t self, value: OwnedType<'t>) -> &'t Type {
        match value {
            OwnedType::IntegerBasetype(t) => self.alloc(t),
            OwnedType::IntegerSubtype(t) => self.alloc(t),
            OwnedType::FloatingBasetype(t) => self.alloc(t),
            OwnedType::FloatingSubtype(t) => self.alloc(t),
            OwnedType::EnumBasetype(t) => self.alloc(t),
            OwnedType::EnumSubtype(t) => self.alloc(t),
            OwnedType::PhysicalBasetype(t) => self.alloc(t),
            OwnedType::PhysicalSubtype(t) => self.alloc(t),
            OwnedType::Access(t) => self.alloc(t),
            OwnedType::Null => &NullType,
            OwnedType::UniversalInteger => &UniversalIntegerType,
            OwnedType::UniversalReal => &UniversalRealType,
        }
    }
}
