
#![feature(geobacter)]
#![feature(specialization)]
#![feature(intrinsics)]
#![feature(link_llvm_intrinsics)]

#[macro_use]
extern crate rustc_middle;

use std::geobacter::kernel::KernelInstanceRef;

use rustc_middle::ty::{self, Instance, ParamEnv, Ty};
use rustc_middle::ty::layout::HasTyCtxt;
use rustc_serialize::Decodable;

use crate::codec::GeobacterDecoder;

pub mod codec;
pub mod intrinsics;
pub mod const_builder;
pub mod mir_builder;

pub trait TyCtxtKernelInstance<'tcx>: HasTyCtxt<'tcx> {
    fn convert_kernel_instance(&self, k: KernelInstanceRef<'_>) -> Option<Instance<'tcx>> {
        log::trace!("converting kernel instance for `{}`", k.name);

        let mut alloc_state = None;
        let mut decoder = GeobacterDecoder::new(self.tcx(), k.instance,
                                                &mut alloc_state);

        Instance::decode(&mut decoder).ok()
    }
    fn expect_instance(&self, k: KernelInstanceRef<'_>) -> Instance<'tcx> {
        self.convert_kernel_instance(k)
            .unwrap_or_else(|| panic!("no ty::Instance for `{}`", k.name) )
    }

    fn extract_opt_fn_instance(&self, intrinsic: Instance<'tcx>,
                               f_ty: Ty<'tcx>)
        -> Option<ty::Instance<'tcx>>
    {
        log::trace!("extract_opt_fn_instance(`{}`, `{}`", intrinsic, f_ty);

        let tcx = self.tcx();
        let reveal_all = ParamEnv::reveal_all();

        let mut ty = if let Some(substs) = intrinsic.substs_for_mir_body() {
            tcx.subst_and_normalize_erasing_regions(substs, reveal_all, &f_ty)
        } else {
            tcx.normalize_erasing_regions(reveal_all, f_ty)
        };

        loop {
            if ty == tcx.types.unit { return None; }

            let instance = match ty.kind {
                ty::Ref(_, &ty::TyS {
                    kind: ty::FnDef(def_id, subs),
                    ..
                }, ..) |
                ty::FnDef(def_id, subs) => {
                    Instance::resolve(tcx, reveal_all, def_id, subs)
                        .expect("must be resolvable")
                },
                ty::Ref(_, inner @ &ty::TyS {
                    kind: ty::Ref(..),
                    ..
                }, ..) => {
                    ty = inner;
                    continue;
                },
                _ => {
                    let msg = format!("unexpected param type {:?} in intrinsic", ty);
                    tcx.sess.fatal(&msg);
                },
            };

            return Some(instance);
        }
    }
    fn extract_fn_instance(&self, intrinsic: Instance<'tcx>,
                           f_ty: Ty<'tcx>)
        -> ty::Instance<'tcx>
    {
        self.extract_opt_fn_instance(intrinsic, f_ty)
            .expect("non-optional function parameter")
    }
}
impl<'tcx, T> TyCtxtKernelInstance<'tcx> for T
    where T: HasTyCtxt<'tcx>,
{ }
