use schemars::JsonSchema;
use serde::Deserialize;
use std::sync::{Arc, RwLock};

use crate::{
    models::{
        app_state::{AppState, RemoteInfo},
        args::AppArgs,
        config::Config,
        vars::Vars,
    },
    utils::errors::{EmptyResult, ResultTrait, ResultWithError},
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
    Finished,
}

impl HookType {
    pub fn pre_hooks() -> Vec<HookType> {
        vec![Self::Connect, Self::VerifySystem, Self::PrepareSystem]
    }

    pub fn post_hooks() -> Vec<HookType> {
        vec![Self::Finished]
    }
}

/// Context passed to all hooks during execution providing access to CLI args,
/// configuration, and shared mutable application state.
#[allow(dead_code)]
pub struct HookContext<'a, State> {
    pub args: &'a AppArgs,
    pub config: &'a Config,
    pub vars: &'a Vars,
    pub state: Arc<RwLock<State>>,
}

impl<'a, State> HookContext<'a, State> {
    pub fn read_state(&self) -> ResultWithError<std::sync::RwLockReadGuard<'_, State>> {
        self.state
            .read()
            .auto_err("Failed to acquire read lock for state")
    }
}

impl<'a> HookContext<'a, AppState> {
    pub fn initiate_remote(&self, remote: RemoteInfo) -> EmptyResult {
        let mut state = self
            .state
            .write()
            .auto_err("Failed to acquire write lock")?;
        state.remote = Some(remote);
        Ok(())
    }
}

/// Trait that all hook implementations must adhere to.
pub trait Hook {
    fn get_type(&self) -> HookType;
    fn continue_on_error(&self) -> bool {
        false
    }
    fn run(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult;
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
