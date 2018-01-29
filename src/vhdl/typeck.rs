// Copyright (c) 2017 Fabian Schuiki

//! This module implements the type calculation of the scoreboard.

use std::fmt::Debug;
use std::cell::Cell;

use moore_common::NodeId;
use moore_common::errors::*;
use moore_common::source::{Span, Spanned};
use moore_common::score::{NodeMaker, Result};

use score::*;
use ty::*;
use konst::*;
use hir;

/// A context to typecheck things in.
///
/// This context helps checking the types of things and keeping track of errors.
pub struct TypeckContext<'sbc, 'sb: 'sbc, 'ast: 'sb, 'ctx: 'sb> {
	/// The parent context.
	ctx: &'sbc ScoreContext<'sb, 'ast, 'ctx>,
	/// Whether any of the type checking failed.
	failed: Cell<bool>,
}

impl<'sbc, 'sb, 'ast, 'ctx> TypeckContext<'sbc, 'sb, 'ast, 'ctx> {
	/// Create a new type checking context.
	pub fn new(ctx: &'sbc ScoreContext<'sb, 'ast, 'ctx>) -> TypeckContext<'sbc, 'sb, 'ast, 'ctx> {
		TypeckContext {
			ctx: ctx,
			failed: Cell::new(false),
		}
	}

	/// Consume the context and return the result of the typeck.
	pub fn finish(self) -> bool {
		!self.failed.get()
	}

	/// Emit a diagnostic message.
	pub fn emit(&self, diag: DiagBuilder2) {
		if diag.severity >= Severity::Error {
			self.failed.set(true);
		}
		self.ctx.sess.emit(diag)
	}

	/// Type check the time expression in a delay mechanism.
	pub fn typeck_delay_mechanism(&self, node: &'ctx hir::DelayMechanism) {
		// TODO: implement this
	}

	/// Type check a waveform.
	pub fn typeck_waveform(&self, node: &'ctx hir::Waveform, exp: &'ctx Ty) {
		for elem in node {
			self.typeck_wave_elem(elem, exp);
		}
	}

	/// Type check a waveform element.
	pub fn typeck_wave_elem(&self, node: &'ctx hir::WaveElem, exp: &'ctx Ty) {
		if let Some(value) = node.value {
			self.typeck_node(value, exp);
		}
		if let Some(after) = node.after {
			// TODO: type check time expression
			// self.typeck_node(after, /* time type */);
		}
	}

	/// Type check any node that can have its type calculated.
	pub fn typeck_node<I>(&self, id: I, exp: &'ctx Ty)
		where
			I: 'ctx + Copy + Debug + Into<NodeId>,
			ScoreContext<'sb, 'ast, 'ctx>: NodeMaker<I, &'ctx Ty>
	{
		if let Ok(act) = self.ctx.ty(id) {
			if act != exp {
				// TODO: We need some span information here!
				self.emit(
					DiagBuilder2::error(format!("typecheck failed, expected {:?}, got {:?}", exp, act))
				);
			}
		} else {
			self.failed.set(true);
		}
	}

	/// Type check a slice of nodes.
	pub fn typeck_slice<T,I>(&self, ids: T)
		where
			T: AsRef<[I]>,
			I: Copy,
			TypeckContext<'sbc, 'sb, 'ast, 'ctx>: Typeck<I>,
	{
		for &id in ids.as_ref() {
			self.typeck(id);
		}
	}
}

/// Performs a type check.
pub trait Typeck<I> {
	fn typeck(&self, id: I);
}

/// A macro to implement the `Typeck` trait.
macro_rules! impl_typeck {
	($slf:tt, $id:ident: $id_ty:ty => $blk:block) => {
		impl<'sbc, 'sb, 'ast, 'ctx> Typeck<$id_ty> for TypeckContext<'sbc, 'sb, 'ast, 'ctx> {
			fn typeck(&$slf, $id: $id_ty) $blk
		}
	}
}

/// A macro to implement the `Typeck` trait.
macro_rules! impl_typeck_err {
	($slf:tt, $id:ident: $id_ty:ty => $blk:block) => {
		impl<'sbc, 'sb, 'ast, 'ctx> Typeck<$id_ty> for TypeckContext<'sbc, 'sb, 'ast, 'ctx> {
			fn typeck(&$slf, $id: $id_ty) {
				use std;
				let res = (move || -> Result<()> { $blk })();
				std::mem::forget(res);
			}
		}
	}
}

// Implement the `Typeck` trait for everything that supports type calculation.
impl<'sbc, 'sb: 'sbc, 'ast: 'sb, 'ctx: 'sb, I> Typeck<I> for TypeckContext<'sbc, 'sb, 'ast, 'ctx> where ScoreContext<'sb, 'ast, 'ctx>: NodeMaker<I, &'ctx Ty> {
	fn typeck(&self, id: I) {
		match ScoreContext::make(self.ctx, id) {
			Ok(_) => (),
			Err(()) => self.failed.set(true),
		}
	}
}

/// Checks whether a node is of a given type.
pub trait TypeckNode<'ctx, I> {
	fn typeck_node(&self, id: I, expected: &'ctx Ty) -> Result<()>;
}

// Implement the `TypeckNode` trait for everything that supports type
// calculation.
impl<'sb, 'ast, 'ctx, I> TypeckNode<'ctx, I> for ScoreContext<'sb, 'ast, 'ctx> where ScoreContext<'sb, 'ast, 'ctx>: NodeMaker<I, &'ctx Ty> {
	fn typeck_node(&self, id: I, expected: &'ctx Ty) -> Result<()> {
		let actual = self.make(id)?;
		if actual != expected {
			self.emit(
				DiagBuilder2::error(format!("typecheck failed, expected {:?}, got {:?}", expected, actual))
			);
			Err(())
		} else {
			Ok(())
		}
	}
}

macro_rules! unimp {
	($slf:tt, $id:expr) => {{
		$slf.emit(DiagBuilder2::bug(format!("typeck of {:?} not implemented", $id)));
		return;
	}}
}

macro_rules! unimp_err {
	($slf:tt, $id:expr) => {{
		$slf.emit(DiagBuilder2::bug(format!("typeck of {:?} not implemented", $id)));
		return Err(());
	}}
}

macro_rules! unimpmsg {
	($slf:tt, $span:expr, $msg:expr) => {{
		$slf.emit(DiagBuilder2::bug(format!("{} not implemented", $msg)).span($span));
		return Err(());
	}}
}

impl_typeck_err!(self, id: LibRef => {
	let hir = self.ctx.hir(id)?;
	self.typeck_slice(&hir.pkg_decls);
	self.typeck_slice(&hir.pkg_insts);
	self.typeck_slice(&hir.pkg_bodies);
	self.typeck_slice(&hir.ctxs);
	self.typeck_slice(&hir.entities);
	self.typeck_slice(&hir.archs);
	self.typeck_slice(&hir.cfgs);
	Ok(())
});

impl_typeck_err!(self, id: PkgDeclRef => {
	let hir = self.ctx.hir(id)?;
	self.typeck_slice(&hir.generics);
	self.typeck_slice(&hir.decls);
	Ok(())
});

impl_typeck!(self, id: CtxRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: CfgRef => {
	unimp!(self, id)
});

impl_typeck_err!(self, id: EntityRef => {
	let hir = self.ctx.hir(id)?;
	for &generic in &hir.generics {
		self.typeck(generic);
	}
	for &port in &hir.ports {
		self.typeck(port);
	}
	Ok(())
});

impl_typeck_err!(self, id: ArchRef => {
	let hir = self.ctx.hir(id)?;
	self.typeck(hir.entity);
	for &decl in &hir.decls {
		self.typeck(decl);
	}
	for &stmt in &hir.stmts {
		self.typeck(stmt);
	}
	Ok(())
});

impl_typeck!(self, id: GenericRef => {
	match id {
		GenericRef::Type(id)    => self.typeck(id),
		GenericRef::Subprog(id) => self.typeck(id),
		GenericRef::Pkg(id)     => self.typeck(id),
		GenericRef::Const(id)   => self.typeck(id),
	}
});

// impl_typeck!(self, id: IntfSignalRef => {
// 	self.typeck(self.hir(id)?.ty)
// });

impl_typeck!(self, id: IntfTypeRef => {
	unimp!(self, id)
	// self.typeck(self.hir(id)?.ty)
});

impl_typeck!(self, id: IntfSubprogRef => {
	unimp!(self, id)
	// self.typeck(self.hir(id)?.ty)
});

impl_typeck!(self, id: IntfPkgRef => {
	unimp!(self, id)
	// self.typeck(self.hir(id)?.ty)
});

impl_typeck!(self, id: IntfConstRef => {
	unimp!(self, id)
	// self.typeck(self.hir(id)?.ty)
});

impl_typeck!(self, id: DeclInPkgRef => {
	match id {
		DeclInPkgRef::Pkg(id)     => self.typeck(id),
		DeclInPkgRef::PkgInst(id) => self.typeck(id),
		DeclInPkgRef::Type(id)    => self.typeck(id),
		DeclInPkgRef::Subtype(id) => self.typeck(id),
	}
});

impl_typeck!(self, id: DeclInBlockRef => {
	match id {
		DeclInBlockRef::Pkg(id)       => self.typeck(id),
		DeclInBlockRef::PkgInst(id)   => self.typeck(id),
		DeclInBlockRef::Type(id)      => self.typeck(id),
		DeclInBlockRef::Subtype(id)   => self.typeck(id),
		DeclInBlockRef::Const(id)     => self.typeck(id),
		DeclInBlockRef::Signal(id)    => self.typeck(id),
		DeclInBlockRef::SharedVar(id) => self.typeck(id),
		DeclInBlockRef::File(id)      => self.typeck(id),
	}
});

impl_typeck!(self, id: DeclInProcRef => {
	match id {
		DeclInProcRef::Pkg(id)     => self.typeck(id),
		DeclInProcRef::PkgBody(id) => self.typeck(id),
		DeclInProcRef::PkgInst(id) => self.typeck(id),
		DeclInProcRef::Type(id)    => self.typeck(id),
		DeclInProcRef::Subtype(id) => self.typeck(id),
		DeclInProcRef::Const(id)   => self.typeck(id),
		DeclInProcRef::Var(id)     => self.typeck(id),
		DeclInProcRef::File(id)    => self.typeck(id),
	}
});

impl_typeck!(self, id: ConcStmtRef => {
	match id {
		ConcStmtRef::Block(id)         => self.typeck(id),
		ConcStmtRef::Process(id)       => self.typeck(id),
		ConcStmtRef::ConcProcCall(id)  => self.typeck(id),
		ConcStmtRef::ConcAssert(id)    => self.typeck(id),
		ConcStmtRef::ConcSigAssign(id) => self.typeck(id),
		ConcStmtRef::CompInst(id)      => self.typeck(id),
		ConcStmtRef::ForGen(id)        => self.typeck(id),
		ConcStmtRef::IfGen(id)         => self.typeck(id),
		ConcStmtRef::CaseGen(id)       => self.typeck(id),
	}
});

impl_typeck!(self, id: SeqStmtRef => {
	match id {
		SeqStmtRef::Wait(id)      => self.typeck(id),
		SeqStmtRef::Assert(id)    => self.typeck(id),
		SeqStmtRef::Report(id)    => self.typeck(id),
		SeqStmtRef::SigAssign(id) => self.typeck(id),
		SeqStmtRef::VarAssign(id) => self.typeck(id),
		SeqStmtRef::ProcCall(id)  => self.typeck(id),
		SeqStmtRef::If(id)        => self.typeck(id),
		SeqStmtRef::Case(id)      => self.typeck(id),
		SeqStmtRef::Loop(id)      => self.typeck(id),
		SeqStmtRef::Next(id)      => self.typeck(id),
		SeqStmtRef::Exit(id)      => self.typeck(id),
		SeqStmtRef::Return(id)    => self.typeck(id),
		SeqStmtRef::Null(id)      => self.typeck(id),
	}
});

impl_typeck!(self, id: PkgBodyRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: PkgInstRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ConstDeclRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: SharedVarDeclRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: VarDeclRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: FileDeclRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: BlockStmtRef => {
	unimp!(self, id)
});

impl_typeck_err!(self, id: ProcessStmtRef => {
	let hir = self.ctx.hir(id)?;
	for &decl in &hir.decls {
		self.typeck(decl);
	}
	for &stmt in &hir.stmts {
		self.typeck(stmt);
	}
	Ok(())
});

impl_typeck!(self, id: ConcProcCallStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ConcAssertStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ConcSigAssignStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: CompInstStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ForGenStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: IfGenStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: CaseGenStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: WaitStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: AssertStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ReportStmtRef => {
	unimp!(self, id)
});

impl_typeck_err!(self, id: SigAssignStmtRef => {
	let hir = self.ctx.hir(id)?;
	let lhs_ty = match hir.target {
		hir::SigAssignTarget::Name(sig) => self.ctx.ty(sig)?,
		hir::SigAssignTarget::Aggregate => unimpmsg!(self, hir.target_span, "assignment to aggregate signal"),
	};
	// let mut ctx = TypeckContext::new(self);
	// let typeck_dm = |dm| match dm {
	// 	// TODO: typeck time expression
	// 	// &hir::DelayMechanism::RejectInertial(expr) => self.typeck_node(expr, self.intern_ty(/* time type */))?,
	// 	_ => Ok(()),
	// };
	match hir.kind {
		hir::SigAssignKind::SimpleWave(ref dm, ref wave) => {
			self.typeck_delay_mechanism(dm);
			self.typeck_waveform(wave, lhs_ty);
		}
		hir::SigAssignKind::SimpleForce(_, _expr) => {
			// self.typeck_node(expr, lhs_ty)?;
		}
		hir::SigAssignKind::SimpleRelease(_) => (),
		hir::SigAssignKind::CondWave(ref dm, ref _cond) => {
			self.typeck_delay_mechanism(dm);
			// self.typeck_node(cond, lhs_ty)?;
		}
		hir::SigAssignKind::CondForce(_, ref _cond) => {
			// self.typeck_node(cond, lhs_ty)?;
		}
		hir::SigAssignKind::SelWave(ref dm, ref _sel) => {
			self.typeck_delay_mechanism(dm);
			// self.typeck_node(sel, lhs_ty)?;
		}
		hir::SigAssignKind::SelForce(_, ref _sel) => {
			// self.typeck_node(sel, lhs_ty)?;
		}
	}
	Ok(())
});

impl_typeck!(self, id: VarAssignStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ProcCallStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: IfStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: CaseStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: LoopStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: NextStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ExitStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, id: ReturnStmtRef => {
	unimp!(self, id)
});

impl_typeck!(self, _id: NullStmtRef => {
	// The null statement always typechecks.
});

impl<'sb, 'ast, 'ctx> ScoreContext<'sb, 'ast, 'ctx> {
	/// Replace `Ty::Named` by the actual type definition recursively.
	pub fn deref_named_type<'a>(&self, ty: &'a Ty) -> Result<&'a Ty> where 'ctx: 'a {
		match ty {
			&Ty::Named(_, tmr) => {
				let inner = self.ty(tmr)?;
				self.deref_named_type(inner)
			}
			other => Ok(other)
		}
	}
}


/// Determine the type of a type mark.
impl_make!(self, id: TypeMarkRef => &Ty {
	match id {
		TypeMarkRef::Type(id) => self.make(id),
		TypeMarkRef::Subtype(id) => self.make(id),
	}
});


/// Determine the type of a subtype indication.
impl_make!(self, id: SubtypeIndRef => &Ty {
	let hir = self.hir(id)?;
	match hir.constraint {
		None => Ok(self.intern_ty(Ty::Named(hir.type_mark.span, hir.type_mark.value))),

		// For range constraints, we first have to check if the constraint is
		// applicable given the type mark. If it is, check if the provided
		// range actually is a proper subtype, and then apply the constraint.
		Some(Spanned{ value: hir::Constraint::Range(_, expr_id), span }) => {
			let inner = self.deref_named_type(self.ty(hir.type_mark.value)?)?;
			match *inner {
				Ty::Int(ref inner) => {
					// Evaluate the expression to a constant range.
					let range = match *self.const_value(expr_id)? {
						Const::IntRange(ref r) => r,
						ref wrong => {
							self.emit(
								DiagBuilder2::error(format!("{} used to constrain integer type", wrong.kind_desc()))
								.span(span)
							);
							return Err(());
						}
					};

					// Make sure that this is actually a subtype.
					if inner.dir != range.dir || inner.left_bound > range.left_bound.value || inner.right_bound < range.right_bound.value {
						self.emit(
							DiagBuilder2::error(format!("`{}` is not a subrange of `{}`", range, inner))
							.span(span)
						);
						return Err(());
					}

					// Create the new type.
					Ok(self.intern_ty(IntTy::new(inner.dir, range.left_bound.value.clone(), range.right_bound.value.clone()).maybe_null()))
				}

				// All other types we simply cannot constrain by range.
				_ => {
					self.emit(
						DiagBuilder2::error(format!("{} cannot be constrained by range", inner.kind_desc()))
						.span(span)
					);
					return Err(());
				}
			}
		}

		// This is more or less the same as above, but for the more modern Term-
		// based constraint determination.
		Some(Spanned{ value: hir::Constraint::Range2(dir, lb, rb), span }) => {
			let lb = self.const_value(lb)?;
			let rb = self.const_value(rb)?;
			let inner = self.deref_named_type(self.ty(hir.type_mark.value)?)?;
			match *inner {
				Ty::Int(ref inner) => {
					// Make sure we have an integer range.
					let (lb, rb) = match (lb, rb) {
						(&Const::Int(ref lb), &Const::Int(ref rb)) => (lb, rb),
						_ => {
							self.emit(
								DiagBuilder2::error(format!("non-integer range `{} {} {}` cannot constrain an integer type", lb, dir, rb))
								.span(span)
							);
							return Err(());
						}
					};

					// Make sure that this is actually a subtype.
					if inner.dir != dir || inner.left_bound > lb.value || inner.right_bound < rb.value {
						self.emit(
							DiagBuilder2::error(format!("`{} {} {}` is not a subrange of `{}`", lb, dir, rb, inner))
							.span(span)
						);
						return Err(());
					}

					// Create the new type.
					Ok(self.intern_ty(IntTy::new(inner.dir, lb.value.clone(), rb.value.clone()).maybe_null()))
				}

				// All other types we simply cannot constrain by range.
				_ => {
					self.emit(
						DiagBuilder2::error(format!("{} cannot be constrained by range", inner.kind_desc()))
						.span(span)
					);
					return Err(());
				}
			}

		}

		Some(Spanned{ value: hir::Constraint::Array(ref ac), span }) => {
			self.emit(
				DiagBuilder2::error("Array constraints on subtypes not yet supported")
				.span(ac.span)
			);
			Err(())
		}

		Some(Spanned{ value: hir::Constraint::Record(ref rc), span }) => {
			self.emit(
				DiagBuilder2::error("Record constraints on subtypes not yet supported")
				.span(rc.span)
			);
			Err(())
		}
	}
});


/// Determine the type of a type declaration.
impl_make!(self, id: TypeDeclRef => &Ty {
	let hir = self.hir(id)?;
	let data = match hir.data {
		Some(ref d) => d,
		None => {
			self.emit(
				DiagBuilder2::error(format!("Declaration of type `{}` is incomplete", hir.name.value))
				.span(hir.name.span)
			);
			return Err(());
		}
	};
	match data.value {
		hir::TypeData::Range(dir, lb_id, rb_id) => {
			self.make_range_ty(dir, lb_id, rb_id, data.span)
		}

		hir::TypeData::Enum(..) => {
			Ok(self.intern_ty(EnumTy::new(id)))
		}

		hir::TypeData::Access(subty_id) => {
			let ty = self.ty(subty_id)?.clone();
			Ok(self.intern_ty(Ty::Access(Box::new(ty))))
		}

		hir::TypeData::Array(ref index_ids, elem_ty) => {
			// To determine the type of an array, we first need to obtain the
			// HIR of each index. Based on that we can decide whether this is an
			// unbounded or constrained array type, and proceed accordingly.
			let mut had_fails = false;
			let mut indices = Vec::new();
			for &index_id in index_ids {
				let hir = match self.hir(index_id) {
					Ok(h) => h,
					Err(()) => { had_fails = true; continue; }
				};
				indices.push(match hir.value {
					hir::ArrayTypeIndex::Unbounded(tm) => {
						ArrayIndex::Unbounded(Box::new(self.ty(tm.value)?.clone()))
					}
					hir::ArrayTypeIndex::Subtype(subty) => {
						ArrayIndex::Constrained(Box::new(self.ty(subty)?.clone()))
					}
					hir::ArrayTypeIndex::Range(dir, lb_id, rb_id) => {
						ArrayIndex::Constrained(Box::new(
							self.make_range_ty(dir, lb_id, rb_id, hir.span)?.clone()
						))
					}
				});
			}
			if had_fails {
				return Err(());
			}
			let elem_ty = self.ty(elem_ty)?.clone();
			Ok(self.intern_ty(ArrayTy::new(indices, Box::new(elem_ty))))
		}
	}
});


impl<'sb, 'ast, 'ctx> ScoreContext<'sb, 'ast, 'ctx> {
	pub fn make_range_ty(&self, dir: hir::Dir, lb_id: ExprRef, rb_id: ExprRef, span: Span) -> Result<&'ctx Ty> {
		let lb = self.const_value(lb_id)?;
		let rb = self.const_value(rb_id)?;
		Ok(match (lb, rb) {
			(&Const::Int(ref lb), &Const::Int(ref rb)) => {
				self.intern_ty(IntTy::new(dir, lb.value.clone(), rb.value.clone()).maybe_null())
			}

			(&Const::Float(ref _lb), &Const::Float(ref _rb)) => {
				self.emit(
					DiagBuilder2::error("Float range bounds not yet supported")
					.span(span)
				);
				return Err(());
			}

			_ => {
				self.emit(
					DiagBuilder2::error("Bounds of range are not of the same type")
					.span(span)
				);
				return Err(());
			}
		})
	}
}


/// Determine the type of a subtype declaration.
impl_make!(self, id: SubtypeDeclRef => &Ty {
	let hir = self.hir(id)?;
	self.ty(hir.subty)
});


/// Determine the type of a signal declaration.
impl_make!(self, id: SignalDeclRef => &Ty {
	let hir = self.existing_hir(id)?;
	self.ty(hir.subty)
});


/// Determine the type of an expression.
impl_make!(self, id: ExprRef => &Ty {
	let hir = self.hir(id)?;
	match hir.data {
		hir::ExprData::IntegerLiteral(ref c) => {
			// Integer literals either have a type attached, or they inherit
			// their type from the context.
			if let Some(ref ty) = c.ty {
				return Ok(self.intern_ty(ty.clone()));
			}
			if let Some(ty) = self.type_context_resolved(id)? {
				if let &Ty::Int(_) = self.deref_named_type(ty)? {
					return Ok(ty);
				}
			}
			self.emit(
				DiagBuilder2::error(format!("cannot infer type of `{}` from context", hir.span.extract()))
				.span(hir.span)
			);
			Err(())
		}

		hir::ExprData::FloatLiteral(ref _c) => {
			unimp_err!(self, id);
			// // Float literals either have a type attached, or they inherit their
			// // type from the context.
			// if let Some(ref ty) = c.ty {
			// 	return Ok(self.intern_ty(ty.clone()));
			// }
			// if let Some(ty) = self.type_context_resolved(id)? {
			// 	if let &Ty::Float(_) = self.deref_named_type(ty)? {
			// 		return Ok(ty);
			// 	}
			// }
			// self.emit(
			// 	DiagBuilder2::error("cannot infer type of float literal from context")
			// 	.span(hir.span)
			// );
			// Err(())
		}

		_ => unimp_err!(self, id),
	}
});


/// Determine the type of a typed node.
impl_make!(self, id: TypedNodeRef => &Ty {
	match id {
		TypedNodeRef::SubtypeInd(id) => self.make(id),
		TypedNodeRef::Signal(id)     => self.make(id),
	}
});

impl_make!(self, id: SignalRef => &Ty {
	match id {
		SignalRef::Intf(id) => self.make(id),
		SignalRef::Decl(id) => self.make(id),
	}
});

impl_make!(self, id: IntfSignalRef => &Ty {
	let hir = self.hir(id)?;
	self.ty(hir.ty)
});