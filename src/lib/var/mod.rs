use crate::lang::scope::Scope;
use crate::lang::errors::{CrushResult, argument_error};
use crate::lang::{value::Value, command::SimpleCommand};
use crate::lang::command::ExecutionContext;

mod unset;
mod env;
mod r#use;

pub fn r#let(context: ExecutionContext) -> CrushResult<()> {
    for arg in context.arguments.iter() {
        if arg.val_or_empty().is_empty() {
            return argument_error("Missing variable name");
        }
    }
    for arg in context.arguments {
        context.env.declare_str(arg.name.unwrap().as_ref(), arg.value)?;
    }
    Ok(())
}

pub fn set(context: ExecutionContext) -> CrushResult<()> {
    context.output.initialize(vec![]);

    for arg in context.arguments.iter() {
        if arg.val_or_empty().is_empty() {
            return argument_error("Missing variable name");
        }
    }
    for arg in context.arguments {
        context.env.set_str(arg.name.unwrap().as_ref(), arg.value)?;
    }
    Ok(())
}

pub fn declare(root: &Scope) -> CrushResult<()> {
    let env = root.create_namespace("var")?;
    root.r#use(&env);
    env.declare_str("let", Value::Command(SimpleCommand::new(r#let, false)))?;
    env.declare_str("set", Value::Command(SimpleCommand::new(set, false)))?;
    env.declare_str("unset", Value::Command(SimpleCommand::new(unset::perform, false)))?;
    env.declare_str("env", Value::Command(SimpleCommand::new(env::perform, false)))?;
    env.declare_str("use", Value::Command(SimpleCommand::new(r#use::perform, false)))?;
    env.readonly();
    Ok(())
}
