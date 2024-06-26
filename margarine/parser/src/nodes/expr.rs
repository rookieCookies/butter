use std::{fmt::Display, ops::Deref};

use common::{source::SourceRange, string_map::StringIndex, ImmutableData};
use lexer::Literal;
use sti::define_key;

use crate::DataType;

use super::NodeId;

define_key!(u32, pub ExprId);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Expr<'a> {
    Unit,
    
    Literal(Literal),

    Identifier(StringIndex),

    Deref(ExprId),

    Range {
        lhs: ExprId,
        rhs: ExprId,
    },

    BinaryOp {
        operator: BinaryOperator,
        lhs: ExprId,
        rhs: ExprId,
    },

    UnaryOp {
        operator: UnaryOperator,
        rhs: ExprId,
    },

    If {
        condition: ExprId,
        body: ExprId,
        else_block: Option<ExprId>,
    },

    Match {
        value: ExprId,
        taken_as_inout: bool,
        mappings: &'a [MatchMapping],
    },

    Block {
        block: Block<'a>,
    },

    CreateStruct {
        data_type: DataType<'a>,
        fields: &'a [(StringIndex, SourceRange, ExprId)],
    },

    AccessField {
        val: ExprId,
        field_name: StringIndex,
    },

    CallFunction {
        name: StringIndex,
        is_accessor: bool,
        args: &'a [(ExprId, bool)],
    },

    WithinNamespace {
        namespace: StringIndex,
        namespace_source: SourceRange,
        action: ExprId,
    },

    WithinTypeNamespace {
        namespace: DataType<'a>,
        action: ExprId,
    },

    Loop {
        body: Block<'a>,
    },
    
    Return(ExprId),
    Continue,
    Break,

    Tuple(&'a [ExprId]),

    AsCast {
        lhs: ExprId,
        data_type: DataType<'a>,
    },

    Unwrap(ExprId),

    OrReturn(ExprId),
}


#[derive(Debug, PartialEq, Clone, Copy, ImmutableData)]
pub struct MatchMapping {
    variant: StringIndex,
    binding: StringIndex,
    binding_range: SourceRange,
    range: SourceRange,
    expr: ExprId,
    is_inout: bool,
}


impl MatchMapping {
    pub fn new(
        variant: StringIndex, 
        binding: StringIndex, 
        binding_range: SourceRange,
        source_range: SourceRange, 
        expression: ExprId,
        is_inout: bool,
    ) -> Self { 
        Self { 
            variant, 
            binding, 
            expr: expression,
            range: source_range, 
            is_inout,
            binding_range,
        } 
    }
}


#[derive(Debug, PartialEq, Clone, Copy, ImmutableData)]
pub struct Block<'a> {
    body: &'a [NodeId],
    range: SourceRange
}


impl<'a> Block<'a> {
    pub fn new(body: &'a [NodeId], range : SourceRange) -> Self {
        Self { body, range } 
    }
}


impl<'a> Deref for Block<'a> {
    type Target = &'a [NodeId];

    fn deref(&self) -> &Self::Target {
        &self.body
    }
}



#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    /// '+'
    Add,
    /// '-'
    Sub,
    /// '*'
    Mul,
    /// '/'
    Div,
    /// '%'
    Rem,

    /// '<<'
    BitshiftLeft,
    /// '>>'
    BitshiftRight,
    /// '&'
    BitwiseAnd,
    /// '|'
    BitwiseOr,
    /// '^'
    BitwiseXor,

    /// '=='
    Eq,
    /// '!='
    Ne,
    /// '>'
    Gt,
    /// '>='
    Ge,
    /// '<'
    Lt,
    /// '<='
    Le,
}

impl BinaryOperator {
    pub fn is_arith(self) -> bool {
        match self {
            | BinaryOperator::BitshiftLeft
            | BinaryOperator::BitshiftRight
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOr
            | BinaryOperator::BitwiseXor
            | BinaryOperator::Eq
            | BinaryOperator::Ne
            | BinaryOperator::Gt
            | BinaryOperator::Ge
            | BinaryOperator::Lt
            | BinaryOperator::Le 
             => false,
            
            | BinaryOperator::Add
            | BinaryOperator::Sub
            | BinaryOperator::Mul
            | BinaryOperator::Div
            | BinaryOperator::Rem
             => true,
        }

    }

    
    pub fn is_bw(self) -> bool {
        match self {
            | BinaryOperator::Eq
            | BinaryOperator::Ne
            | BinaryOperator::Gt
            | BinaryOperator::Ge
            | BinaryOperator::Lt
            | BinaryOperator::Le
            | BinaryOperator::Add
            | BinaryOperator::Sub
            | BinaryOperator::Mul
            | BinaryOperator::Div
            | BinaryOperator::Rem
             => false,

            | BinaryOperator::BitshiftLeft
            | BinaryOperator::BitshiftRight
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOr
            | BinaryOperator::BitwiseXor
             => true,
        }

    }

    
    pub fn is_ocomp(self) -> bool {
        match self {
            | BinaryOperator::Add
            | BinaryOperator::Sub
            | BinaryOperator::Mul
            | BinaryOperator::Div
            | BinaryOperator::Rem
            | BinaryOperator::BitshiftLeft
            | BinaryOperator::BitshiftRight
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOr
            | BinaryOperator::BitwiseXor
            | BinaryOperator::Eq
            | BinaryOperator::Ne
             => false,

            | BinaryOperator::Gt
            | BinaryOperator::Ge
            | BinaryOperator::Lt
            | BinaryOperator::Le
             => true,
        }
    }

    
    pub fn is_ecomp(self) -> bool {
        match self {
            | BinaryOperator::Add
            | BinaryOperator::Sub
            | BinaryOperator::Mul
            | BinaryOperator::Div
            | BinaryOperator::Rem
            | BinaryOperator::BitshiftLeft
            | BinaryOperator::BitshiftRight
            | BinaryOperator::BitwiseAnd
            | BinaryOperator::BitwiseOr
            | BinaryOperator::BitwiseXor
            | BinaryOperator::Gt
            | BinaryOperator::Ge
            | BinaryOperator::Lt
            | BinaryOperator::Le
             => false,

            | BinaryOperator::Eq
            | BinaryOperator::Ne
             => true,
        }
    }
}


impl Display for BinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
            BinaryOperator::Rem => "%",
            BinaryOperator::BitshiftLeft => ">>",
            BinaryOperator::BitshiftRight => "<<",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::BitwiseOr => "|",
            BinaryOperator::BitwiseXor => "^",
            BinaryOperator::Eq => "==",
            BinaryOperator::Ne => "!=",
            BinaryOperator::Gt => ">",
            BinaryOperator::Ge => ">=",
            BinaryOperator::Lt => "<",
            BinaryOperator::Le => "<=",
        })
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    Neg,
}


impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            UnaryOperator::Not => "!",
            UnaryOperator::Neg => "-",
        })
    }
}


