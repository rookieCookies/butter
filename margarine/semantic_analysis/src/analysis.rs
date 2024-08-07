use std::fmt::Write;

use common::{buffer::Buffer, copy_slice_in, string_map::{OptStringIndex, StringIndex, StringMap}};
use parser::nodes::{decl::{Decl, DeclId, FunctionSignature, UseItem, UseItemKind}, expr::{BinaryOperator, Expr, ExprId, UnaryOperator}, stmt::{Stmt, StmtId}, NodeId};
use sti::{alloc::GlobalAlloc, arena::Arena, vec::Vec, write};

use crate::{errors::Error, namespace::{Namespace, NamespaceId}, scope::{FunctionScope, GenericsScope, Scope, ScopeId, ScopeKind, VariableScope}, syms::{containers::{Container, ContainerKind}, func::{FunctionArgument, FunctionKind, FunctionTy}, sym_map::{Generic, GenericKind, SymbolId, VarSub}, ty::Sym, Symbol, SymbolKind}, AnalysisResult, TyChecker};

impl<'me, 'out, 'temp, 'ast, 'str> TyChecker<'me, 'out, 'temp, 'ast, 'str> {
    pub fn block(&mut self, path: StringIndex, scope: ScopeId, body: &[NodeId]) -> AnalysisResult {
        let scope = scope;
        let namespace = Namespace::new(path);
        let namespace = self.namespaces.push(namespace);

        // Collect type names
        self.collect_names(path, namespace, body, 0);

        // Update the current scope so the following functions
        // are able to see the namespace
        let scope = Scope::new(scope.some(), ScopeKind::ImplicitNamespace(namespace));
        let mut scope = self.scopes.push(scope);

        // Collect impls
        self.collect_impls(path, scope, namespace, body);

        // Collect imports
        self.collect_uses(scope, namespace, body);

        // Compute types & functions
        self.compute_types(path, scope, namespace, body, None);

        // Analyze all nodes
        let mut last_node = None;
        for node in body.iter() {
            let eval = self.node(path, &mut scope, namespace, *node);
            last_node = Some(eval);
        }

        // Finalise
        let result = match last_node {
            Some(v) => v,
            None    => AnalysisResult::new(Sym::UNIT, true),
        };

        result
    }


    pub fn collect_names(&mut self, path: StringIndex, ns_id: NamespaceId, nodes: &[NodeId], gen_count: usize) {
        for n in nodes {
            let NodeId::Decl(decl) = n
            else { continue };

            let mut ns = self.namespaces.get_ns_mut(ns_id);
            let decl = self.ast.decl(*decl);
            match decl {
                | Decl::Enum { name, header, generics, .. } 
                | Decl::Struct { name, header, generics, .. }
                | Decl::Function { sig: FunctionSignature { name, source: header, generics, .. }, .. }=> {
                    if let Some(sym) = ns.get_sym(name) {
                        if sym.is_ok() { ns.set_err_sym(name) }

                        self.error(*n, Error::NameIsAlreadyDefined {
                            source: header, name });
                        continue
                    }

                    if matches!(name, StringMap::ITER_NEXT_FUNC | StringMap::ITER_MUTATE)
                        && !matches!(decl, Decl::Function { .. }) {
                        self.error(*n, Error::NameIsReservedForFunctions { source: header });
                    }

                    let path = self.string_map.concat(path, name);
                    let pend = self.syms.pending(&mut self.namespaces, path, generics.len() + gen_count);
                    ns = self.namespaces.get_ns_mut(ns_id);

                    ns.add_sym(name, pend);
                },


                Decl::Extern { functions }=> {
                    for f in functions {
                        if let Some(sym) = ns.get_sym(f.name()) {
                            if sym.is_ok() { ns.set_err_sym(f.name()) }
                            self.error(*n, Error::NameIsAlreadyDefined {
                                source: f.range(), name: f.name() });
                            ns = self.namespaces.get_ns_mut(ns_id);
                            continue
                        }

                        let path = self.string_map.concat(path, f.name());
                        let pend = self.syms.pending(&mut self.namespaces, path, 0);
                        ns = self.namespaces.get_ns_mut(ns_id);

                        ns.add_sym(f.name(), pend);
                    }
                },


                Decl::Module { name, header, body } => {
                    if ns.get_ns(name).is_some() {
                        self.error(*n, Error::NameIsAlreadyDefined {
                            source: header, name });
                        continue
                    }

                    let path = self.string_map.concat(path, name);

                    let module_ns = Namespace::new(path);
                    let module_ns = self.namespaces.push(module_ns);

                    self.namespaces.get_ns_mut(ns_id).add_ns(name, module_ns);
                    self.collect_names(path, module_ns, &*body, gen_count);
                },


                Decl::Attribute { decl, .. } => self.collect_names(path, ns_id, &[decl.into()], gen_count),

                _ => (),
            }
        }
    }


    pub fn collect_impls(&mut self, path: StringIndex, scope: ScopeId, ns_id: NamespaceId, nodes: &[NodeId]) {
        for n in nodes {
            let NodeId::Decl(decl) = n
            else { continue };

            let decl = self.ast.decl(*decl);
            match decl {
                Decl::Module { name, body, .. } => {
                    let module_ns = self.namespaces.get_ns(ns_id).get_ns(name).unwrap();
                    let scope = Scope::new(scope.some(), ScopeKind::ImplicitNamespace(module_ns));
                    let scope = self.scopes.push(scope);
                    self.collect_impls(path, scope, module_ns, &*body);
                }


                Decl::Impl { data_type, gens, body } => {
                    let s = self.scopes.get(scope);
                    let ty = match self.dt_to_gen(s, data_type, gens) {
                        Ok(v) => v,
                        Err(v) => {
                            self.error(*n, v);
                            continue;
                        },
                    };

                    let Some(sym) = ty.sym()
                    else {
                        let source = self.ast.range(*n);
                        self.error(*n, Error::ImplOnGeneric(source));
                        continue;
                    };


                    let ns = self.syms.sym_ns(sym);

                    self.collect_names(path, ns, &body, gens.len());
                    self.collect_impls(path, scope, ns, &body);
                }


                Decl::Attribute { decl, .. } => self.collect_impls(path, scope, ns_id, &[decl.into()]),

                _ => (),
            }
        }
    }


    pub fn collect_uses(&mut self, scope_id: ScopeId, ns_id: NamespaceId, nodes: &[NodeId]) {
        let scope = self.scopes.get(scope_id);
        for n in nodes {
            let NodeId::Decl(id) = *n
            else { continue; };

            match self.ast.decl(id) {
                Decl::Module { name, body, .. } => {
                    let module_ns = self.namespaces.get_ns(ns_id).get_ns(name).unwrap();
                    let scope = Scope::new(scope_id.some(), ScopeKind::ImplicitNamespace(module_ns));
                    let scope = self.scopes.push(scope);
                    self.collect_uses(scope, module_ns, &body);
                }


                Decl::Impl { data_type, gens, body } => {
                    let Ok(ty) = self.dt_to_gen(scope, data_type, gens)
                    else { continue; };

                    let Some(sym) = ty.sym()
                    else { continue; };


                    let ns = self.syms.sym_ns(sym);
                    self.collect_uses(scope_id, ns, &body)
                }


                Decl::Using { item } => {
                    self.collect_use_item(*n, scope, ns_id, item)
                }


                Decl::Attribute { decl, .. } => self.collect_uses(scope_id, ns_id, &[decl.into()]),

                _ => continue,
            }
        }
    }


    fn collect_use_item(&mut self, node: NodeId, scope: Scope, ns_id: NamespaceId, item: UseItem) {
        match item.kind() {
            UseItemKind::List { list } => {
                let Some((import_ns, _)) = scope.find_ns(item.name(), &self.scopes, &self.namespaces, &self.syms)
                else {
                    self.error(node, Error::NamespaceNotFound { source: item.range(), namespace: item.name() });
                    return;
                };

                let scope = Scope::new(None, ScopeKind::ImplicitNamespace(import_ns));
                for ui in list {
                    self.collect_use_item(node, scope, ns_id, *ui);
                }
            },


            UseItemKind::BringName => {
                if let Some(import_sym) = scope.find_sym(item.name(), &self.scopes, &mut self.syms, &self.namespaces) {
                    let ns = self.namespaces.get_ns_mut(ns_id);
                    match import_sym {
                        Ok(v) => ns.add_import_sym(item.name(), v),
                        Err(_) => ns.set_err_sym(item.name()),
                    };
                    return;
                };


                if let Some((import_ns, _)) = scope.find_ns(item.name(), &self.scopes, &self.namespaces, &self.syms) {
                    self.namespaces.get_ns_mut(ns_id).add_ns(item.name(), import_ns);
                    return;
                };


                self.error(node, Error::NamespaceNotFound { source: item.range(), namespace: item.name() });
            },


            UseItemKind::All => {
                let Some((import_ns, _)) = scope.find_ns(item.name(), &self.scopes, &self.namespaces, &self.syms)
                else {
                    self.error(node, Error::NamespaceNotFound { source: item.range(), namespace: item.name() });
                    return;
                };

                let (ns, import_ns) = self.namespaces.get_double(ns_id, import_ns);

                for s in import_ns.syms() {
                    if ns.get_sym(*s.0).is_some() {
                        Self::error_ex(&mut self.errors, &mut self.type_info,
                                       node, Error::NameIsAlreadyDefined { source: item.range(), name: *s.0 });
                        continue;
                    }

                    let Some(sym) = s.1
                    else { continue };

                    if ns.get_sym(*s.0).is_none() {
                        ns.add_import_sym(*s.0, *sym)
                    }
                }

                for n in import_ns.nss() {
                    ns.add_import_ns(*n.0, *n.1)
                }
            },
        };

    }


    // `Self::collect_names` must be ran before this
    pub fn compute_types(&mut self, path: StringIndex, scope: ScopeId,
                         ns: NamespaceId, nodes: &[NodeId], impl_block: Option<&[StringIndex]>) {
        for id in nodes {
            let NodeId::Decl(id) = id
            else { continue };

            let decl = self.ast.decl(*id);
            match decl {
                 Decl::Struct { name, fields, generics, .. } => {
                    let ns = self.namespaces.get_ns(ns);
                    let mut structure_fields = Buffer::new(self.output, fields.len());
                    let Ok(tsi) = ns.get_sym(name).unwrap()
                    else { continue };

                    for f in fields {
                        let sym = self.dt_to_gen(self.scopes.get(scope), f.1, generics);
                        let sym = match sym {
                            Ok(v) => v,
                            Err(v) => {
                                self.error(*id, v);
                                Generic::new(f.1.range(), GenericKind::ERROR)
                            },
                        };

                        let field = (f.0.some(), sym);
                        structure_fields.push(field);
                    }

                    // finalise
                    let generics = copy_slice_in(self.output, generics);
                    let sym_name = self.string_map.concat(path, name);
                    let cont = Container::new(structure_fields.leak(), ContainerKind::Struct);
                    let kind = SymbolKind::Container(cont);

                    let sym = Symbol::new(sym_name, generics, kind);
                    self.syms.add_sym(tsi, sym);
                },


                Decl::Enum { name, mappings, generics, .. } => {
                    let ns = self.namespaces.get_ns(ns);
                    let mut enum_mappings = Buffer::new(self.output, mappings.len());
                    let Ok(tsi) = ns.get_sym(name).unwrap()
                    else { continue };

                    for f in mappings {
                        let sym = self.dt_to_gen(self.scopes.get(scope), *f.data_type(), generics);
                        let sym = match sym {
                            Ok(v) => v,
                            Err(v) => {
                                self.error(*id, v);
                                Generic::new(f.data_type().range(), GenericKind::ERROR)
                            },
                        };

                        let mapping = (f.name().some(), sym);
                        enum_mappings.push(mapping);
                    }

                    // finalise
                    let generics = copy_slice_in(self.output, generics);
                    let name = self.string_map.concat(path, name);
                    let source = self.ast.range(*id);
                    self.syms.add_enum(tsi, &mut self.namespaces, self.string_map,
                                        source, name,
                                        enum_mappings.leak(), generics);
                },


                Decl::Function { sig, is_in_impl, .. } => {
                    let generics = {
                        let impl_gens = impl_block.unwrap_or(&[]);
                        let mut vec = Buffer::new(self.output, impl_gens.len() + sig.generics.len());
                        vec.extend_from_slice(impl_gens);
                        vec.extend_from_slice(sig.generics);
                        vec.leak()
                    };

                    let Some(Ok(fid)) = self.namespaces.get_ns(ns).get_sym(sig.name)
                    else { continue };

                    let mut args = Buffer::new(self.output, sig.arguments.len());

                    for a in sig.arguments {
                        let sym = self.dt_to_gen(self.scopes.get(scope), a.data_type(), generics);
                        let sym = match sym {
                            Ok(v) => v,
                            Err(v) => {
                                self.error(*id, v);
                                Generic::new(a.data_type().range(), GenericKind::ERROR)
                            },
                        };

                        let arg = FunctionArgument::new(a.name(), sym, a.is_inout());
                        args.push(arg);
                    }


                    let ret = self.dt_to_gen( self.scopes.get(scope), sig.return_type, generics);
                    let ret = match ret {
                        Ok(v) => v,
                        Err(v) => {
                            self.error(*id, v);
                            Generic::new(sig.return_type.range(), GenericKind::ERROR)
                        },
                    };


                    // Check for special functions
                    if sig.is_system && (!sig.generics.is_empty() || is_in_impl.is_some()) {
                        self.error(*id, Error::InvalidSystem(sig.source));
                    }

                    if is_in_impl.is_some() && sig.name == StringMap::ITER_NEXT_FUNC {
                        let validate_sig = || {
                            if sig.arguments.len() != 1 { return false }
                            let impl_ty = is_in_impl.unwrap();
                            if sig.arguments[0].data_type().kind() != impl_ty.kind() { return false; }

                            if !sig.arguments[0].is_inout() { return false; }
                            if ret.sym() != Some(SymbolId::OPTION) { return false; }

                            true
                        };


                        if !validate_sig() {
                            self.error(*id, Error::IteratorFunctionInvalidSig(sig.source));
                        }
                    }


                    // Finalise
                    let sym_name = self.string_map.concat(path, sig.name);

                    let func = FunctionTy::new(args.leak(), ret, FunctionKind::UserDefined { decl: *id });
                    let func = Symbol::new(sym_name, generics, SymbolKind::Function(func));

                    self.syms.add_sym(fid, func);
                }


                Decl::Extern { functions } => {
                    for f in functions {
                        let mut args = Buffer::new(self.output, f.args().len());

                        for a in f.args() {
                            let sym = self.dt_to_gen(self.scopes.get(scope), a.data_type(), &[]);
                            let sym = match sym {
                                Ok(v) => v,
                                Err(v) => {
                                    self.error(*id, v);
                                    Generic::new(a.data_type().range(), GenericKind::ERROR)
                                },
                            };

                            let arg = FunctionArgument::new(a.name(), sym, a.is_inout());
                            args.push(arg);
                        }


                        let ret = self.dt_to_gen(self.scopes.get(scope), f.return_type(), &[]);
                        let ret = match ret {
                            Ok(v) => v,
                            Err(v) => {
                                self.error(*id, v);
                                Generic::new(f.return_type().range(), GenericKind::ERROR)
                            },
                        };


                        // Finalise
                        let sym_name = self.string_map.concat(path, f.name());

                        let func = FunctionTy::new(args.leak(), ret, FunctionKind::Extern(f.path()));
                        let func = Symbol::new(sym_name, &[], SymbolKind::Function(func));

                        let Ok(id) = self.namespaces.get_ns(ns).get_sym(f.name()).unwrap()
                        else { continue };

                        self.syms.add_sym(id, func);
                    }
                }


                Decl::Module { name, body, .. } => {
                    let ns = self.namespaces.get_ns(ns);
                    let Some(module_ns) = ns.get_ns(name)
                    else { continue };

                    let scope = self.scopes.push(self.scopes.get(scope));
                    let scope = Scope::new(scope.some(), ScopeKind::ImplicitNamespace(module_ns));
                    let scope = self.scopes.push(scope);

                    let path = self.namespaces.get_ns(module_ns).path;
                    self.compute_types(path, scope, module_ns, &*body, None);
                }


                Decl::Impl { data_type, body, gens } => {
                    let s = self.scopes.get(scope);
                    let Ok(ty) = self.dt_to_gen(s, data_type, gens)
                    else { continue; };

                    let Some(sym) = ty.sym()
                    else { continue; };


                    let ns = self.syms.sym_ns(sym);

                    self.compute_types(path, scope, ns, &body, Some(gens));
                }

                Decl::Attribute { decl, .. } => {
                    self.compute_types(path, scope, ns, &[decl.into()], impl_block);
                },

                _ => (),
            }
        }
    }


    pub fn node(&mut self, path: StringIndex,
                scope: &mut ScopeId, ns: NamespaceId, node: NodeId) -> AnalysisResult {
        match node {
            NodeId::Decl(decl) => {
                self.decl(scope, ns, decl);
                AnalysisResult::new(Sym::UNIT, true)
            },

            NodeId::Stmt(stmt) => {
                self.stmt(path, scope, stmt);
                AnalysisResult::new(Sym::UNIT, true)
            },

            NodeId::Expr(expr) => self.expr(path, *scope, expr),

            NodeId::Err(_) => {
                AnalysisResult::new(Sym::ERROR, true)
            },
        }
    }


    pub fn decl(&mut self, scope: &mut ScopeId, ns: NamespaceId, id: DeclId) {
        let decl = self.ast.decl(id);
        match decl {
            Decl::Struct { .. } => (),
            Decl::Enum { .. } => (),

            
            Decl::Function { sig, body, .. } => {
                let ns = self.namespaces.get_ns(ns);
                let Some(Ok(func)) = ns.get_sym(sig.name)
                else { return };

                // we need a scope that'd fake the generics
                let sym = self.syms.sym(func);
                let SymbolKind::Function(func) = sym.kind()
                else { unreachable!() };

                let generics = sym.generics();
                let generics = {
                    let mut vec = Buffer::new(&*self.output, generics.len());
                    for gen in generics {
                        let ty = self.syms.pending(&mut self.namespaces, *gen, 0);
                        let kind = SymbolKind::Container(Container::new(&[], ContainerKind::Struct));
                        self.syms.add_sym(ty, Symbol::new(*gen, &[], kind));
                        vec.push((*gen, self.syms.get_ty(ty, &[])));
                    }

                    vec
                };
                
                // fake args
                let generics = generics.leak();
                let gscope = GenericsScope::new(generics);
                let mut scope = Scope::new(scope.some(), ScopeKind::Generics(gscope));

                for a in func.args() {
                    let ty = a.symbol().to_ty(&generics, &mut self.syms);
                    let ty = match ty {
                        Ok(v) => v,
                        Err(v) => {
                            self.error(id, v);
                            Sym::ERROR
                        }
                    };

                    let vs = VariableScope::new(a.name(), ty, a.inout());
                    scope = Scope::new(self.scopes.push(scope).some(), ScopeKind::VariableScope(vs))
                }

                let ret = func.ret().to_ty(&generics, &mut self.syms);
                let ret = match ret {
                    Ok(v) => v,
                    Err(v) => {
                        self.error(id, v);
                        Sym::ERROR
                    }
                };

                // func scope
                let fs = FunctionScope::new(ret, sig.return_type.range());
                scope = Scope::new(self.scopes.push(scope).some(), ScopeKind::Function(fs));

                let scope = self.scopes.push(scope);

                // GO GO GO
                let anal = self.block(sym.name(), scope, &*body);

                if !anal.ty.eq(&mut self.syms, ret) {
                    self.error(id, Error::FunctionBodyAndReturnMismatch {
                        header: sig.source, item: body.range(),
                        return_type: ret, body_type: anal.ty });
                }
            },


            Decl::Impl { data_type, gens, body  } => {
                let s = self.scopes.get(*scope);
                let Ok(ty) = self.dt_to_gen(s, data_type, gens)
                else { return; };
                
                let GenericKind::Sym(sym, _) = ty.kind()
                else { return; };

                let ns = self.syms.sym_ns(sym);

                let path = self.namespaces.get_ns(ns).path;
                for n in body.iter() {
                    self.node(path, scope, ns, *n);
                }

            },


            Decl::Module { name, body, .. } => {
                let ns = self.namespaces.get_ns(ns);
                let Some(module_ns) = ns.get_ns(name)
                else { return };
                
                let scope = Scope::new(scope.some(), ScopeKind::ImplicitNamespace(module_ns));
                let mut scope = self.scopes.push(scope);

                let path = self.namespaces.get_ns(module_ns).path;
                for n in body.iter() {
                    self.node(path, &mut scope, module_ns, *n);
                }
            },


            Decl::Using { .. } => (),
            Decl::Extern { .. } => (),

            Decl::Attribute { decl: decl_id, attr, attr_range } => {
                self.decl(scope, ns, decl_id);

                match self.string_map.get(attr) {
                    "startup" => {
                        let decl = self.ast.decl(decl_id);
                        let Decl::Function { sig, .. } = decl
                        else {
                            let range = self.ast.range(decl_id);
                            self.error(id, Error::InvalidValueForAttr {
                                attr: (attr_range, attr), value: range, expected: "a system function" });
                            return;
                        };

                        let Ok(func) = self.namespaces.get_ns(ns).get_sym(sig.name).unwrap()
                        else { return };

                        self.startups.push(func);
                    }
                    _ => self.error(id, Error::UnknownAttr(attr_range, attr))
                }
            },
        }
    }


    pub fn stmt(&mut self, path: StringIndex,
                scope: &mut ScopeId, id: StmtId) {
        let source = self.ast.range(id);
        let stmt = self.ast.stmt(id);
        match stmt {
            Stmt::Variable { name, hint, is_mut, rhs } => {
                let rhs_anal = self.expr(path, *scope, rhs);
                
                let place_dummy = |slf: &mut TyChecker<'_, 'out, '_, '_, '_>, scope: &mut ScopeId| {
                    let vs = VariableScope::new(name, Sym::ERROR, is_mut);
                    *scope = slf.scopes.push(Scope::new(scope.some(), ScopeKind::VariableScope(vs)));
                };

                // Validation
                if let Ok(sym) = rhs_anal.ty.sym(&mut self.syms) {
                    if sym == SymbolId::ERR {
                        place_dummy(self, scope);
                        return;
                    }
                }

                let mut ty = rhs_anal.ty;
                if let Some(hint) = hint {
                    let hint = match self.dt_to_ty(*scope, id, hint) {
                        Ok(v)  => v,
                        Err(v) => {
                            place_dummy(self, scope);
                            self.error(id, v);
                            return
                        },
                    };

                    if !rhs_anal.ty.eq(&mut self.syms, hint) {
                        let vs = VariableScope::new(name, hint, is_mut);
                        *scope = self.scopes.push(Scope::new(scope.some(), ScopeKind::VariableScope(vs)));

                        self.error(id, Error::VariableValueAndHintDiffer {
                            value_type: rhs_anal.ty, hint_type: hint, source });
                        return
                    }

                    ty = hint;
                }

                // finalise
                let vs = VariableScope::new(name, ty, is_mut);
                *scope = self.scopes.push(Scope::new(scope.some(),
                                          ScopeKind::VariableScope(vs)));
            },


            Stmt::VariableTuple { names, hint, rhs  } => {
                let rhs_anal = self.expr(path, *scope, rhs);

                let place_dummy = |slf: &mut TyChecker<'_, 'out, '_, '_, '_>, scope: &mut ScopeId| {
                    for n in names {
                        let vs = VariableScope::new(n.0, Sym::ERROR, n.1);
                        *scope = slf.scopes.push(Scope::new(scope.some(), ScopeKind::VariableScope(vs)));
                    }
                };

                // check if rhs is a tuple
                let sym = match rhs_anal.ty.sym(&mut self.syms) {
                    Ok(v) => v,
                    Err(v) => {
                        place_dummy(self, scope);
                        self.error(id, v);
                        return;
                    },
                };

                let sym = self.syms.sym(sym);
                let SymbolKind::Container(cont) = sym.kind()
                else {
                    place_dummy(self, scope);
                    let range = self.ast.range(rhs);
                    self.error(id, Error::VariableValueNotTuple(range));
                    return;
                };


                if cont.kind() != ContainerKind::Tuple {
                    place_dummy(self, scope);
                    let range = self.ast.range(rhs);
                    self.error(id, Error::VariableValueNotTuple(range));
                    return;
                }


                if cont.fields().len() == names.len() {
                }
            },


            Stmt::UpdateValue { lhs, rhs  } => {
                let lhs_anal = self.expr(path, *scope, lhs);
                let rhs_anal = self.expr(path, *scope, rhs);

                if !lhs_anal.is_mut {
                    let range = self.ast.range(lhs);
                    self.error(id, Error::ValueUpdateNotMut { source: range });
                }


                if !lhs_anal.ty.eq(&mut self.syms, rhs_anal.ty) {
                    self.error(id, Error::InvalidType { source, found: rhs_anal.ty, expected: lhs_anal.ty });
                }
            },


            Stmt::ForLoop { binding, expr, body } => {
                let anal = self.expr(path, *scope, expr.1);

                // check inoutness
                if !anal.is_mut && expr.0 {
                    let range = self.ast.range(expr.1);
                    self.error(id, Error::InOutValueIsntMut(range));
                }

                if expr.0 && !binding.0 {
                    self.error(id, Error::InOutValueWithoutInOutBinding { value_range: binding.2 });
                }

                if !expr.0 && binding.0 {
                    let range = self.ast.range(expr.1);
                    self.error(id, Error::InOutBindingWithoutInOutValue { value_range: range });
                }


                // check if the exprs type is an iterable
                let Ok(sym) = anal.ty.sym(&mut self.syms)
                else {
                    let range = self.ast.range(expr.1);
                    self.error(id, Error::ValueIsntAnIterator { ty: anal.ty, range });
                    for n in body.iter() { self.error(*n, Error::Bypass) }

                    return;
                };

                let func = self.syms.sym_ns(sym);
                let ns = self.namespaces.get_ns(func);
                let Some(sym) = ns.get_sym(StringMap::ITER_NEXT_FUNC)
                else { 
                    let range = self.ast.range(expr.1);
                    self.error(id, Error::ValueIsntAnIterator { ty: anal.ty, range });
                    for n in body.iter() { self.error(*n, Error::Bypass) }
                    return;
                };

                let Ok(sym) = sym else { return };
                

                // check if the exprs type is a mutable iterable
                if expr.0 && ns.get_sym(StringMap::ITER_MUTATE).is_none() {
                    let range = self.ast.range(expr.1);
                    self.error(id, Error::ValueIsntAMutableIterator { ty: anal.ty, range });
                }

                let binding_ty = self.syms.sym(sym);
                let SymbolKind::Function(binding_ty) = binding_ty.kind()
                else { unreachable!() };

                let gens = anal.ty.gens(&self.syms);
                let gens = self.syms.get_gens(gens);

                let binding_ty = binding_ty.ret().to_ty(gens, &mut self.syms);
                let binding_ty = match binding_ty {
                    Ok(v) => v,
                    Err(v) => {
                        self.error(id, v);
                        Sym::ERROR
                    },
                };

                // unwrap the option
                let binding_ty = binding_ty.gens(&self.syms);
                let binding_ty = self.syms.get_gens(binding_ty);
                let binding_ty = binding_ty[0].1;

                let vs = VariableScope::new(binding.1, binding_ty, binding.0);
                let scope = self.scopes.push(Scope::new(scope.some(), ScopeKind::VariableScope(vs)));

                let _ = self.block(path, scope, &body);

            },
        }
    }


    pub fn expr(&mut self, path: StringIndex, scope: ScopeId, id: ExprId) -> AnalysisResult {
        let range = self.ast.range(id);
        let expr = self.ast.expr(id);
        let result = (|| Ok(match expr {
            Expr::Unit => AnalysisResult::new(Sym::UNIT, true),


            Expr::Literal(lit) => {
                match lit {
                    lexer::Literal::Integer(_) => AnalysisResult::new(self.syms.new_var_ex(id, range, VarSub::Integer), true),
                    lexer::Literal::Float(_)   => AnalysisResult::new(self.syms.new_var_ex(id, range, VarSub::Float), true),
                    lexer::Literal::String(_)  => AnalysisResult::new(Sym::STR, true),
                    lexer::Literal::Bool(_)    => AnalysisResult::new(Sym::BOOL, true),
                }
            },


            Expr::Identifier(ident) => {
                let Some(variable) = self.scopes.get(scope).find_var(ident, &self.scopes)
                else { return Err(Error::VariableNotFound { name: ident, source: range }) };

                AnalysisResult::new(variable.ty(), variable.is_mut())
            },


            Expr::Deref(e) => {
                let expr = self.expr(path, scope, e);
                let sym = expr.ty.sym(&mut self.syms)?;

                if sym == SymbolId::ERR { return Err(Error::Bypass) }
                
                if sym != SymbolId::PTR {
                    let range = self.ast.range(e);
                    return Err(Error::DerefOnNonPtr(range));
                }

                let gens = expr.ty.gens(&self.syms);
                let gens = self.syms.get_gens(gens);
                debug_assert_eq!(gens.len(), 1);

                AnalysisResult::new(gens[0].1, expr.is_mut)
            },


            Expr::Range { lhs, rhs  } => {
                let lhs_anal = self.expr(path, scope, lhs);
                let rhs_anal = self.expr(path, scope, rhs);

                if !lhs_anal.ty.is_int(&mut self.syms) {
                    let range = self.ast.range(lhs);
                    return Err(Error::InvalidRange { source: range, ty: lhs_anal.ty });
                }


                if !rhs_anal.ty.is_int(&mut self.syms) {
                    let range = self.ast.range(rhs);
                    return Err(Error::InvalidRange { source: range, ty: rhs_anal.ty });
                }


                AnalysisResult::new(Sym::RANGE, true)
            },


            Expr::BinaryOp { operator, lhs, rhs } => {
                let lhs_anal = self.expr(path, scope, lhs);
                let rhs_anal = self.expr(path, scope, rhs);

                lhs_anal.ty.eq(&mut self.syms, rhs_anal.ty);

                let lhs_sym = lhs_anal.ty.sym(&mut self.syms)?;

                if lhs_sym == SymbolId::ERR { return Ok(AnalysisResult::error()) }
                if lhs_sym == SymbolId::NEVER { return Ok(AnalysisResult::never()) }

                let rhs_sym = rhs_anal.ty.sym(&mut self.syms)?;

                if rhs_sym == SymbolId::ERR { return Ok(AnalysisResult::error()) }
                if rhs_sym == SymbolId::NEVER { return Ok(AnalysisResult::never()) }

                let mut validate = || {
                    if !lhs_anal.ty.eq(&mut self.syms, rhs_anal.ty) { return Ok(false) }
                    let sym = match lhs_anal.ty.sym(&mut self.syms) {
                        Ok(v) => v,
                        Err(v) => return Err(v),
                    };

                    Ok(if operator.is_arith() { sym.supports_arith() } else { true }
                    && if operator.is_bw() { sym.supports_bw() } else { true }
                    && if operator.is_ocomp() { sym.supports_ord() } else { true }
                    && if operator.is_ecomp() { sym.supports_eq() } else { true })
                };


                let validate = validate()?;

                if !validate {
                    return Err(Error::InvalidBinaryOp {
                        operator, lhs: lhs_anal.ty, rhs: rhs_anal.ty, source: range });
                }

                let result = match operator {
                      BinaryOperator::Add 
                    | BinaryOperator::Sub
                    | BinaryOperator::Mul
                    | BinaryOperator::Div
                    | BinaryOperator::Rem
                    | BinaryOperator::BitshiftLeft
                    | BinaryOperator::BitshiftRight
                    | BinaryOperator::BitwiseAnd 
                    | BinaryOperator::BitwiseOr 
                    | BinaryOperator::BitwiseXor => lhs_anal.ty,

                      BinaryOperator::Eq 
                    | BinaryOperator::Ne 
                    | BinaryOperator::Gt 
                    | BinaryOperator::Ge 
                    | BinaryOperator::Lt 
                    | BinaryOperator::Le => Sym::BOOL
                };

                AnalysisResult::new(result, true)
            },


            Expr::UnaryOp { operator, rhs } => {
                let rhs_anal = self.expr(path, scope, rhs);
                let sym = rhs_anal.ty.sym(&mut self.syms)?;

                if sym == SymbolId::ERR { return Ok(AnalysisResult::error()) }
                if sym == SymbolId::NEVER { return Ok(AnalysisResult::never()) }

                match operator {
                    UnaryOperator::Not if sym == SymbolId::BOOL => (),
                    UnaryOperator::Neg if sym.is_sint() => (),
                    
                    _ => return Err(Error::InvalidUnaryOp { operator, rhs: rhs_anal.ty, source: range })
                }

                AnalysisResult::new(rhs_anal.ty, true)
            },


            Expr::If { condition, body, else_block } => {
                let cond = self.expr(path, scope, condition);

                if let Ok(sym) = cond.ty.sym(&mut self.syms) {
                    if sym == SymbolId::ERR { return Ok(AnalysisResult::error()) }
                    if sym == SymbolId::NEVER { return Ok(AnalysisResult::never()) }
                }

                if !cond.ty.eq(&mut self.syms, Sym::BOOL) {
                    let range = self.ast.range(condition);
                    return Err(Error::InvalidType {
                        source: range, found: cond.ty, expected: Sym::BOOL })
                }

                let body_anal = self.expr(path, scope, body);
                let mut value = body_anal.ty;

                (|| {
                    let Some(el) = else_block
                    else { return };

                    let el_anal = self.expr(path, scope, el);

                    if value.is_err(&mut self.syms) || value.is_never(&mut self.syms) {
                        value = el_anal.ty
                    } else if el_anal.ty.ne(&mut self.syms, value) {
                        let body = self.ast.range(body);
                        let else_block = self.ast.range(el);
                        self.error(el, Error::IfBodyAndElseMismatch {
                            body: (body, value), else_block: (else_block, el_anal.ty) });
                        return
                    }
                })();

                if else_block.is_none() && (value.is_err(&mut self.syms) | value.is_never(&mut self.syms)) {
                    value = Sym::UNIT;
                }

                if value.ne(&mut self.syms, Sym::UNIT) && else_block.is_none() {
                    let body = self.ast.range(body);
                    return Err(Error::IfMissingElse { body: (body, value) })
                }

                AnalysisResult::new(value, true)
            },


            Expr::Match { value, taken_as_inout, mappings  } => {
                let anal = self.expr(path, scope, value);

                let sym = anal.ty.sym(&mut self.syms)?;
                let sym = self.syms.sym(sym);

                let SymbolKind::Container(cont) = sym.kind()
                else {
                    let range = self.ast.range(value);
                    return Err(Error::MatchValueIsntEnum { source: range, typ: anal.ty });
                };

                // check if the value is an enum
                if !matches!(cont.kind(), ContainerKind::Enum) {
                    let range = self.ast.range(value);
                    return Err(Error::MatchValueIsntEnum { source: range, typ: anal.ty });
                }

                // asserts assumptions on struct
                debug_assert!(cont.fields().iter().all(|x| x.0.is_some()));

                // check the mapping names
                for (i, m) in mappings.iter().enumerate() {
                    let exists = cont.fields().iter().any(|x| {
                        let Some(name) = x.0.to_option()
                        else { unreachable!() };

                        m.variant() == name
                    });

                    if !exists {
                        return Err(Error::InvalidMatch {
                            name: m.variant(), range: m.range(), value: anal.ty });
                    }

                    for o in mappings.iter().skip(i+1) {
                        if o.variant() == m.variant() {
                            return Err(Error::DuplicateMatch {
                                declared_at: m.range(), error_point: o.range() });
                        }
                    }
                }

                
                let mut missings = Vec::new_in(self.temp);
                for sm in cont.fields().iter() {
                    let Some(name) = sm.0.to_option()
                    else { unreachable!() };

                    if !mappings.iter().any(|x| x.variant() == name) {
                        missings.push(name);
                    }
                }

                if !missings.is_empty() {
                    return Err(Error::MissingMatch { name: missings.move_into(GlobalAlloc), range });
                }


                // ty chck
                let ret_ty = self.syms.new_var(id, range);
                for (m, f) in mappings.iter().zip(cont.fields().iter()) {
                    if m.is_inout() && !taken_as_inout {
                        self.error(m.expr(), Error::InOutValueWithoutInOutBinding { value_range: m.range() });
                    }

                    let gens = anal.ty.gens(&self.syms);
                    let gens = self.syms.get_gens(gens);
                    let vs = VariableScope::new(m.binding(), f.1.to_ty(gens, &mut self.syms)?, m.is_inout());

                    let scope = Scope::new(scope.some(), ScopeKind::VariableScope(vs));
                    let scope = self.scopes.push(scope);

                    let anal = self.expr(path, scope, m.expr());

                    if !anal.ty.eq(&mut self.syms, ret_ty) {
                        let range = self.ast.range(m.expr());
                        self.error(m.expr(), Error::InvalidType {
                            source: range, found: anal.ty, expected: ret_ty });
                    }
                }
                

                AnalysisResult::new(ret_ty, true)
            },


            Expr::Block { block } => self.block(path, scope, &*block),


            Expr::CreateStruct { data_type, fields  } => {
                let ty = self.dt_to_ty(scope, id, data_type)?;

                let sym = ty.sym(&mut self.syms)?;
                let sym = self.syms.sym(sym);

                let SymbolKind::Container(cont) = sym.kind()
                else { return Err(Error::StructCreationOnNonStruct { source: range, typ: ty }) };

                // check if the sym is a struct
                if !matches!(cont.kind(), ContainerKind::Struct) {
                    return Err(Error::StructCreationOnNonStruct { source: range, typ: ty });
                }

                // asserts assumptions on struct
                debug_assert!(cont.fields().iter().all(|x| x.0.is_some()));


                // check if the fields are valid
                for f in fields.iter() {
                    let exists = cont.fields().iter().any(|x| {
                        let Some(name) = x.0.to_option()
                        else { unreachable!() };

                        name == f.0
                    });

                    if !exists {
                        return Err(Error::FieldDoesntExist {
                            source: f.1, field: f.0, typ: ty });
                    }
                }


                // check missing fields
                let mut missing_fields = Vec::new_in(self.temp);
                for f in cont.fields().iter() {
                    let Some(name) = f.0.to_option()
                    else { unreachable!() };

                    if !fields.iter().any(|x| x.0 == name) {
                        missing_fields.push(name);
                    }
                }

                if !missing_fields.is_empty() {
                    return Err(Error::MissingFields { source: range, fields: missing_fields.move_into(GlobalAlloc) });
                }


                // type check the fields
                let sym_fields = {
                    let mut vec = Buffer::new(self.temp, cont.fields().len());
                    let gens = ty.gens(&mut self.syms);
                    let gens = self.syms.get_gens(gens);

                    for f in cont.fields() {
                        vec.push((f.0, f.1.to_ty(gens, &mut self.syms)?))
                    }

                    vec
                };

                for f in fields.iter() {
                    let expr = self.expr(path, scope, f.2);
                    let g = sym_fields.iter().find(|x| x.0.unwrap() == f.0).unwrap();

                    if !expr.ty.eq(&mut self.syms, g.1) {
                        self.error(id, Error::InvalidType {
                            source: f.1, found: expr.ty, expected: g.1 });
                    }
                }

                AnalysisResult::new(ty, true)
            },


            Expr::AccessField { val, field_name  } => {
                let expr = self.expr(path, scope, val);

                let sym = expr.ty.sym(&mut self.syms)?;
                let sym = self.syms.sym(sym);

                let SymbolKind::Container(cont) = sym.kind()
                else { return Err(Error::FieldAccessOnNonEnumOrStruct { source: range, typ: expr.ty }) };

                let mut str = sti::string::String::new_in(self.temp);
                let field = cont.fields().iter().enumerate().find(|(i, f)| {
                    let name = match f.0.to_option() {
                        Some(v) => v,
                        None => {
                            str.clear();
                            write!(str, "{i}");
                            let index = self.string_map.insert(&str);
                            index
                        },
                    };

                    field_name == name
                });

                let Some((_, field)) = field
                else { return Err(Error::FieldDoesntExist {
                    source: range, field: field_name, typ: expr.ty }) };

                let gens = expr.ty.gens(&self.syms);
                let gens = self.syms.get_gens(gens);

                let field_ty = field.1.to_ty(gens, &mut self.syms)?;

                let ty = match cont.kind() {
                    ContainerKind::Struct => field_ty,

                    ContainerKind::Enum => {
                        let gens = self.output.alloc_new([(StringMap::T, field_ty)]);
                        Sym::Ty(SymbolId::OPTION, self.syms.add_gens(gens))
                    },

                    ContainerKind::Tuple => field_ty,
                };

                AnalysisResult::new(ty, expr.is_mut)
            },


            Expr::CallFunction { name, is_accessor, args } => {
                let pool = Arena::tls_get_rec();
                let args_anals = {
                    let mut vec = sti::vec::Vec::with_cap_in(&*pool, args.len());

                    for a in args {
                        let range = self.ast.range(a.0);
                        vec.push((range, self.expr(path, scope, a.0), a.1, a.0));
                    }

                    vec.leak()
                };


                let func = {
                    if is_accessor {
                        let sym = args_anals[0].1.ty.sym(&mut self.syms)?;
                        let ns = self.syms.sym_ns(sym);
                        let ns = self.namespaces.get_ns(ns);
                        ns.get_sym(name)
                    } else {
                        self.scopes.get(scope).find_sym(name, &self.scopes, &mut self.syms, &self.namespaces)
                    }
                };


                let Some(sym_id) = func
                else { return Err(Error::FunctionNotFound { source: range, name }) };

                let Ok(sym_id) = sym_id
                else { return Err(Error::Bypass) };

                let sym = self.syms.sym(sym_id);
                let SymbolKind::Function(func) = sym.kind()
                else { return Err(Error::CallOnNonFunction { source: range, name }) };

                // check arg len
                if func.args().len() != args.len() {
                    return Err(Error::FunctionArgsMismatch {
                        source: range, sig_len: func.args().len(), call_len: args.len() });
                }


                // create gens
                let func_generics = {
                    let mut vec = sti::vec::Vec::with_cap_in(self.output, sym.generics().len());
                    for g in sym.generics() {
                        vec.push((*g, self.syms.new_var(id, range)));
                    }

                    vec.leak()
                };


                // find out the args
                let func_args = {
                    let mut vec = sti::vec::Vec::with_cap_in(&*pool, func.args().len());
                    for g in func.args() {
                        vec.push((g.symbol().to_ty(func_generics, &mut self.syms)?, g.inout()));
                    }

                    vec
                };

                let ret = func.ret().to_ty(func_generics, &mut self.syms)?;

                // ty & inout check args
                for (i, (a, fa)) in args_anals.iter().zip(func_args.iter()).enumerate() {
                    if !a.1.ty.eq(&mut self.syms, fa.0) {
                        self.error(a.3, Error::InvalidType {
                            source: a.0, found: a.1.ty, expected: fa.0 })
                    }
                    
                    let is_inout = if fa.1 && is_accessor && i == 0 { true }
                                    else { a.2 };
                    // check inoutness
                    if is_inout && !fa.1 {
                        self.error(id, Error::InOutValueWithoutInOutBinding { value_range: a.0 });
                    } else if is_inout && !a.1.is_mut {
                        self.error(id, Error::InOutValueIsntMut(a.0));
                    }

                    if !is_inout && fa.1 {
                        self.error(id, Error::InOutBindingWithoutInOutValue { value_range: a.0 });
                    }
                }


                let gens = self.syms.add_gens(func_generics);
                self.type_info.set_func_call(id, (sym_id, gens));
                AnalysisResult::new(ret, true)
            },


            Expr::WithinNamespace { namespace, namespace_source, action  } => {
                let ns = self.scopes.get(scope).find_ns(namespace, &self.scopes, &self.namespaces, &self.syms);
                let Some(ns) = ns
                else { return Err(Error::NamespaceNotFound { source: namespace_source, namespace }) };

                if ns.1 { return Err(Error::Bypass) }

                let scope = Scope::new(scope.some(), ScopeKind::ImplicitNamespace(ns.0));
                let scope = self.scopes.push(scope);

                self.expr(path, scope, action)
            },


            Expr::WithinTypeNamespace { namespace, action  } => {
                let ty = self.dt_to_ty(scope, id, namespace)?;
                let sym = ty.sym(&mut self.syms)?;
                let ns = self.syms.sym_ns(sym);

                let scope = Scope::new(scope.some(), ScopeKind::ImplicitNamespace(ns));
                let scope = self.scopes.push(scope);

                self.expr(path, scope, action)
            },


            Expr::Loop { body } => {
                let scope = Scope::new(scope.some(), ScopeKind::Loop);
                let scope = self.scopes.push(scope);
                self.block(path, scope, &*body);

                AnalysisResult::new(Sym::UNIT, true)
            },


            Expr::Return(ret) => {
                let Some(func) = self.scopes.get(scope).find_curr_func(&self.scopes)
                else { return Err(Error::OutsideOfAFunction { source: range }) };

                let ret_anal = self.expr(path, scope, ret);
                if ret_anal.ty.is_err(&mut self.syms) { return Ok(AnalysisResult::error()) }
                if ret_anal.ty.is_never(&mut self.syms) { return Ok(AnalysisResult::never()) }

                if ret_anal.ty.ne(&mut self.syms, func.ret) {
                    return Err(Error::ReturnAndFuncTypDiffer {
                        source: range, func_source: func.ret_source,
                        typ: ret_anal.ty, func_typ: func.ret })
                }

                AnalysisResult::new(Sym::NEVER, true)
            },


            Expr::Continue => {
                if self.scopes.get(scope).find_loop(&self.scopes).is_none() { 
                    return Err(Error::ContinueOutsideOfLoop(range)) 
                }

                AnalysisResult::new(Sym::NEVER, true)
            },


            Expr::Break => {
                if self.scopes.get(scope).find_loop(&self.scopes).is_none() { 
                    return Err(Error::ContinueOutsideOfLoop(range)) 
                }

                AnalysisResult::new(Sym::NEVER, true)
            },


            Expr::Tuple(values) => {
                let pool = Arena::tls_get_rec();

                let fields = {
                    let mut vec = sti::vec::Vec::with_cap_in(&*pool, values.len());
                    for _ in 0..values.len() {
                        vec.push(OptStringIndex::NONE);
                    }

                    vec.leak()
                };

                let sym = self.tuple_sym(range, fields);

                let gens = {
                    let mut vec = sti::vec::Vec::with_cap_in(self.output, values.len());
                    let mut str = sti::string::String::new_in(&*pool);
                    for (index, value) in values.iter().enumerate() {
                        str.clear();
                        write!(str, "{index}");
                        let ty = self.expr(path, scope, *value);
                        let str = self.string_map.insert(&str);
                        vec.push((str, ty.ty));
                    }

                    vec.leak()
                };

                let gens = self.syms.add_gens(gens);

                AnalysisResult::new(Sym::Ty(sym, gens), true)
            },


            Expr::AsCast { lhs, data_type  } => {
                let anal = self.expr(path, scope, lhs);
                let ty = self.dt_to_ty(scope, id, data_type)?;

                if anal.ty.eq(&mut self.syms, ty) {
                    return Ok(AnalysisResult::new(ty, true))
                }

                if anal.ty.is_err(&mut self.syms)
                    || anal.ty.is_never(&mut self.syms)
                    || ty.is_err(&mut self.syms)
                    || ty.is_never(&mut self.syms)
                    || anal.ty.eq(&mut self.syms, ty) {
                    return Ok(AnalysisResult::new(ty, true))
                }

                if !(anal.ty.is_num(&mut self.syms) && ty.is_num(&mut self.syms)) {
                    self.error(id, Error::InvalidCast {
                        range, from_ty: anal.ty, to_ty: ty });
                }

                AnalysisResult::new(ty, true)
            },


            Expr::Unwrap(val) => {
                let expr = self.expr(path, scope, val);
                let sym = expr.ty.sym(&mut self.syms)?;

                if sym != SymbolId::OPTION
                   && sym != SymbolId::RESULT {
                    return Err(Error::CantUnwrapOnGivenType(range, expr.ty));
                }

                let gens = expr.ty.gens(&self.syms);
                let gens = self.syms.get_gens(gens);
                
                AnalysisResult::new(gens[0].1, expr.is_mut)
            },


            Expr::OrReturn(val) => {
                let expr = self.expr(path, scope, val);
                let sym = expr.ty.sym(&mut self.syms)?;
                let Some(func) = self.scopes.get(scope).find_curr_func(&self.scopes)
                else { return Err(Error::OutsideOfAFunction { source: range }) };

                if sym == SymbolId::OPTION {
                    let func_sym = func.ret.sym(&mut self.syms)?;

                    if func_sym != SymbolId::OPTION {
                        return Err(Error::FunctionDoesntReturnAnOption { source: range, func_typ: func.ret });
                    }

                    let gens = expr.ty.gens(&self.syms);
                    let gens = self.syms.get_gens(gens);

                    return Ok(AnalysisResult::new(gens[0].1, expr.is_mut));
                }

                
                if sym == SymbolId::RESULT {
                    let func_sym = func.ret.sym(&mut self.syms)?;

                    if func_sym != SymbolId::RESULT {
                        return Err(Error::FunctionDoesntReturnAResult { source: range, func_typ: func.ret });
                    }

                    let func_gens = func.ret.gens(&self.syms);
                    let func_gens = self.syms.get_gens(func_gens);

                    let gens = expr.ty.gens(&self.syms);
                    let gens = self.syms.get_gens(gens);

                    debug_assert_eq!(func_gens.len(), 2);
                    debug_assert_eq!(gens.len(), 2);

                    if !func_gens[1].1.eq(&mut self.syms, gens[1].1) {
                        return Err(Error::FunctionReturnsAResultButTheErrIsntTheSame {
                            source: range, func_source: func.ret_source,
                            func_err_typ: func_gens[1].1, err_typ: gens[1].1 });
                    }

                    return Ok(AnalysisResult::new(gens[0].1, expr.is_mut));
                }


                return Err(Error::CantTryOnGivenType(range, expr.ty));
            },


        }))();

        match result {
            Ok(v) => {
                self.type_info.set_expr(id, v);
                v
            },

            Err(v) => {
                self.error(id, v);
                AnalysisResult::error()
            },
        }
    }
}

