use std::{ops::Deref, ptr::NonNull};

use llvm_sys::{core::{LLVMAddAttributeAtIndex, LLVMCreateBuilderInContext, LLVMCreateEnumAttribute, LLVMGetEnumAttributeKindForName, LLVMSetLinkage}, LLVMAttributeFunctionIndex, LLVMLinkage};

use crate::{builder::Builder, cstr, ctx::ContextRef, tys::{func::FunctionType, ptr::PtrTy, TypeKind}};

use super::Value;

#[derive(Clone, Copy, Debug)]
pub struct FunctionPtr<'ctx>(Value<'ctx>);


impl<'ctx> FunctionPtr<'ctx> {
    /// # Safety
    /// Undefined behaviour if the value isn't a function
    pub unsafe fn new(val: Value<'ctx>) -> Self {
        debug_assert_eq!(val.ty().kind(), TypeKind::Ptr);

        Self(val)
    }


    pub fn builder(self, ctx: ContextRef<'ctx>, ty: FunctionType<'ctx>) -> Builder<'ctx> {
        let ptr = unsafe { LLVMCreateBuilderInContext(ctx.ptr.as_ptr()) };
        Builder::new(NonNull::new(ptr).unwrap(), ctx, self, ty)
    }


    pub fn ty(self) -> PtrTy<'ctx> { unsafe { PtrTy::new(self.deref().ty()) } }


    pub fn set_linkage(self, linkage: Linkage) {
        unsafe { LLVMSetLinkage(self.llvm_val().as_ptr(), linkage.llvm_linkage()); }
    }


    pub fn set_noreturn(self, ctx: ContextRef<'ctx>) {
        let attr_kind = unsafe { LLVMGetEnumAttributeKindForName(cstr!("noreturn"), 8) };
        let attr = unsafe { LLVMCreateEnumAttribute(ctx.ptr.as_ptr(), attr_kind, 0) };
        unsafe { LLVMAddAttributeAtIndex(self.llvm_val().as_ptr(), LLVMAttributeFunctionIndex, attr) };
    }
}


impl<'ctx> Deref for FunctionPtr<'ctx> {
    type Target = Value<'ctx>;

    fn deref(&self) -> &Self::Target { &self.0 }
}


pub enum FunctionAttribute {
    NoReturn,
}


#[derive(Clone, Copy, Debug)]
pub enum Linkage {
    External,
    AvailableExternally,
    LinkOnceAny,
    LinkOnceODR,
    LinkONceODRAutoHide,
    WeakAny,
    WeakODR,
    Appending,
    Internal,
    Private,
    LLImport,
    LLExport,
    ExternalWeak,
    Ghost,
    Common,
    LinkerPrivate,
    LinkerPrivateWeak,
}
impl Linkage {
    pub fn llvm_linkage(self) -> LLVMLinkage {
        match self {
            Linkage::External => LLVMLinkage::LLVMExternalLinkage,
            Linkage::AvailableExternally => LLVMLinkage::LLVMAvailableExternallyLinkage,
            Linkage::LinkOnceAny => LLVMLinkage::LLVMLinkOnceAnyLinkage,
            Linkage::LinkOnceODR => LLVMLinkage::LLVMLinkOnceODRLinkage,
            Linkage::LinkONceODRAutoHide => LLVMLinkage::LLVMLinkOnceODRAutoHideLinkage,
            Linkage::WeakAny => LLVMLinkage::LLVMWeakAnyLinkage,
            Linkage::WeakODR => LLVMLinkage::LLVMWeakODRLinkage,
            Linkage::Appending => LLVMLinkage::LLVMAppendingLinkage,
            Linkage::Internal => LLVMLinkage::LLVMInternalLinkage,
            Linkage::Private => LLVMLinkage::LLVMLinkerPrivateLinkage,
            Linkage::LLImport => LLVMLinkage::LLVMDLLImportLinkage,
            Linkage::LLExport => LLVMLinkage::LLVMDLLExportLinkage,
            Linkage::ExternalWeak => LLVMLinkage::LLVMExternalWeakLinkage,
            Linkage::Ghost => LLVMLinkage::LLVMGhostLinkage,
            Linkage::Common => LLVMLinkage::LLVMCommonLinkage,
            Linkage::LinkerPrivate => LLVMLinkage::LLVMLinkerPrivateLinkage,
            Linkage::LinkerPrivateWeak => LLVMLinkage::LLVMLinkerPrivateWeakLinkage,
        }
    }
}

