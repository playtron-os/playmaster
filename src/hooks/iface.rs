use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::{
    models::{app_state::AppState, args::AppArgs, config::Config},
    utils::errors::EmptyResult,
};

/// Defines the types of hooks available in the system.
/// Connect: For establishing connections to a remote host if needed.
/// VerifySystem: For verifying system prerequisites and dependencies.
/// PrepareSystem: For preparing the system before running tests.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    Connect,
    VerifySystem,
    PrepareSystem,
    BeforeAll,
    BeforeTest,
    AfterTest,
    AfterAll,
}

impl HookType {
    pub fn pre_hooks() -> Vec<HookType> {
        vec![
            Self::Connect,
            Self::VerifySystem,
            Self::PrepareSystem,
            Self::BeforeAll,
        ]
    }

    pub fn post_hooks() -> Vec<HookType> {
        vec![Self::AfterAll]
    }
}

/// Context passed to all hooks during execution providing access to CLI args,
/// configuration, and shared mutable application state.
#[allow(dead_code)]
pub struct HookContext<'a> {
    pub args: &'a AppArgs,
    pub config: &'a Config,
    pub state: Arc<RwLock<AppState>>,
}

/// Trait that all hook implementations must adhere to.
pub trait Hook {
    fn get_type(&self) -> HookType;
    fn run(&self, ctx: &HookContext) -> EmptyResult;
}

pub trait HookListExt {
    fn hooks_of_type(&self, hook_type: HookType) -> Vec<&dyn Hook>;
}

impl HookListExt for Vec<Box<dyn Hook>> {
    fn hooks_of_type(&self, hook_type: HookType) -> Vec<&dyn Hook> {
        self.iter()
            .filter(|hook| hook.get_type() == hook_type)
            .map(|hook| hook.as_ref())
            .collect()
    }
}
