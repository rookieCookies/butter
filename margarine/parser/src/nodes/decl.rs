use common::{string_map::StringIndex, source::SourceRange};
use sti::define_key;

use crate::{DataType, Block};

define_key!(u32, pub DeclId);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Decl<'a> {
    Struct {
        kind: StructKind,
        name: StringIndex,
        header: SourceRange,
        fields: &'a [(StringIndex, DataType<'a>, SourceRange)],
        generics: &'a [StringIndex],
    },

    Enum {
        name: StringIndex,
        header: SourceRange,
        mappings: &'a [EnumMapping<'a>],
        generics: &'a [StringIndex],
    },

    Function {
        sig: FunctionSignature<'a>,
        body: Block<'a>,
        is_in_impl: Option<DataType<'a>>,
    },
    
    Impl {
        data_type: DataType<'a>,
        gens: &'a [StringIndex],
        body: Block<'a>,
    },

    Using {
        item: UseItem<'a>,
    },

    Module {
        name: StringIndex,
        header: SourceRange,
        body: Block<'a>,
    },

    Extern {
        functions: &'a [ExternFunction<'a>],
    },

    Attribute {
        attr: StringIndex,
        attr_range: SourceRange,
        decl: DeclId,
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StructKind {
    Component,
    Resource,
    Normal,
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FunctionSignature<'a> {
    pub is_system  : bool,
    pub name       : StringIndex,
    pub source     : SourceRange,
    pub arguments  : &'a [FunctionArgument<'a>],
    pub generics   : &'a [StringIndex],
    pub return_type: DataType<'a>,
}

impl<'a> FunctionSignature<'a> {
    pub fn new(
        is_system: bool, name: StringIndex, 
        source: SourceRange, arguments: &'a [FunctionArgument<'a>], 
        generics: &'a [StringIndex], return_type: DataType<'a>) -> Self { 
        Self { is_system, name, source, arguments, return_type, generics }
    }
}


#[derive(Debug, PartialEq)]
pub struct ExternFunction<'arena> {
    name: StringIndex,
    path: StringIndex,
    args: &'arena [FunctionArgument<'arena>],
    return_type: DataType<'arena>,
    source_range: SourceRange,
}

impl<'arena> ExternFunction<'arena> {
    pub(crate) fn new(name: StringIndex, path: StringIndex, args: &'arena [FunctionArgument<'arena>], return_type: DataType<'arena>, source_range: SourceRange) -> Self { 
        Self { name, args, return_type, source_range, path } 
    }


    #[inline(always)]
    pub fn name(&self) -> StringIndex { self.name }
    #[inline(always)]
    pub fn path(&self) -> StringIndex { self.path }
    #[inline(always)]
    pub fn args(&self) -> &[FunctionArgument<'arena>] { &self.args }
    #[inline(always)]
    pub fn return_type(&self) -> DataType<'arena> { self.return_type }
    #[inline(always)]
    pub fn range(&self) -> SourceRange { self.source_range }

}


#[derive(Debug, PartialEq)]
pub struct FunctionArgument<'a> {
    name: StringIndex,
    data_type: DataType<'a>,
    is_inout: bool,
    source_range: SourceRange,
}


impl<'arena> FunctionArgument<'arena> {
    pub fn new(name: StringIndex, data_type: DataType<'arena>, is_inout: bool, source_range: SourceRange) -> Self { 
        Self { name, data_type, is_inout, source_range } 
    }


    #[inline(always)]
    pub fn data_type(&self) -> DataType<'arena> { self.data_type }
    #[inline(always)]
    pub fn name(&self) -> StringIndex { self.name }
    #[inline(always)]
    pub fn is_inout(&self) -> bool { self.is_inout }
    #[inline(always)]
    pub fn range(&self) -> SourceRange { self.source_range }
}


#[derive(Debug, PartialEq)]
pub struct EnumMapping<'a> {
    name: StringIndex,
    number: u16,
    data_type: DataType<'a>,
    source_range: SourceRange,
    is_implicit_unit: bool,
}

impl<'arena> EnumMapping<'arena> {
    pub fn new(name: StringIndex, number: u16, data_type: DataType<'arena>, source_range: SourceRange, is_implicit_unit: bool) -> Self { 
        if is_implicit_unit {
            assert!(data_type.kind().is(&crate::DataTypeKind::Unit));
        }

        Self { name, data_type, source_range, is_implicit_unit, number } 
    }

    
    #[inline(always)]
    pub fn name(&self) -> StringIndex { self.name }
    #[inline(always)]
    pub fn data_type(&self) -> &DataType<'arena> { &self.data_type }
    #[inline(always)]
    pub fn range(&self) -> SourceRange { self.source_range }
    #[inline(always)]
    pub fn is_implicit_unit(&self) -> bool { self.is_implicit_unit }
    #[inline(always)]
    pub fn number(&self) -> u16 { self.number }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UseItem<'a> {
    kind: UseItemKind<'a>,
    name: StringIndex,
    range: SourceRange,
}

impl<'a> UseItem<'a> {
    pub fn new(name: StringIndex, kind: UseItemKind<'a>, range: SourceRange) -> Self { Self { kind, range, name } }
    #[inline(always)]
    pub fn name(self) -> StringIndex { self.name}
    #[inline(always)]
    pub fn kind(self) -> UseItemKind<'a> { self.kind }
    #[inline(always)]
    pub fn range(self) -> SourceRange { self.range }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UseItemKind<'a> {
    List {
        list: &'a [UseItem<'a>],
    },
    BringName,
    All,
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Attribute {
    Startup,
}
