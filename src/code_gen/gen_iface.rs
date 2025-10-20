use crate::{
    hooks::iface::HookContext,
    models::{config::ProjectType, gen_state::GenState},
    utils::errors::EmptyResult,
};

/// Trait that all code generation implementations must adhere to.
pub trait CodeGenTrait {
    fn get_type(&self) -> ProjectType;
    fn run(&self, ctx: &HookContext<'_, GenState>) -> EmptyResult;
}
