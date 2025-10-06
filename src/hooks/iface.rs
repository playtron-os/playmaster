use serde::Deserialize;

use crate::{
    models::{args::AppArgs, config::Config},
    utils::errors::EmptyResult,
};

/// Defines the types of hooks available in the system.
/// Connect: For establishing connections to a remote host if needed.
/// VerifySystem: For verifying system prerequisites and dependencies.
/// PrepareSystem: For preparing the system before running tests.
/// BeforeAll: For actions to be performed before all tests.
/// BeforeTest: For actions to be performed before each individual test.
/// AfterTest: For actions to be performed after each individual test.
/// AfterAll: For actions to be performed after all tests have completed.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Deserialize)]
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

/// Trait that all hook implementations must adhere to.
pub trait Hook {
    fn get_type(&self) -> HookType;
    fn run(&self, args: &AppArgs, config: &Config) -> EmptyResult;
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
