use crate::lang::job::Job;
use crate::lang::errors::{CrushResult, error, argument_error, parse_error};
use crate::lang::call_definition::CallDefinition;
use crate::lang::argument::ArgumentDefinition;
use crate::lang::value::{ValueDefinition, Value};
use std::ops::Deref;
use crate::lang::command::SimpleCommand;

static ADD: SimpleCommand = SimpleCommand { call:crate::lib::math::add, can_block:true};
static SUB: SimpleCommand = SimpleCommand { call:crate::lib::math::sub, can_block:true};
static MUL: SimpleCommand = SimpleCommand { call:crate::lib::math::mul, can_block:true};
static DIV: SimpleCommand = SimpleCommand { call:crate::lib::math::div, can_block:true};

static LT: SimpleCommand = SimpleCommand { call:crate::lib::comp::lt, can_block:true};
static LTE: SimpleCommand = SimpleCommand { call:crate::lib::comp::lte, can_block:true};
static GT: SimpleCommand = SimpleCommand { call:crate::lib::comp::gt, can_block:true};
static GTE: SimpleCommand = SimpleCommand { call:crate::lib::comp::gte, can_block:true};
static EQ: SimpleCommand = SimpleCommand { call:crate::lib::comp::eq, can_block:true};
static NEQ: SimpleCommand = SimpleCommand { call:crate::lib::comp::neq, can_block:true};
static NOT: SimpleCommand = SimpleCommand { call:crate::lib::comp::not, can_block:true};

static AND: SimpleCommand = SimpleCommand { call:crate::lib::cond::and, can_block:true};
static OR: SimpleCommand = SimpleCommand { call:crate::lib::cond::or, can_block:true};

static LET: SimpleCommand = SimpleCommand { call:crate::lib::var::r#let, can_block:true};
static SET: SimpleCommand = SimpleCommand { call:crate::lib::var::set, can_block:true};

#[derive(Debug)]
pub struct JobListNode {
    pub jobs: Vec<JobNode>,
}

impl JobListNode {
    pub fn generate(&self) -> CrushResult<Vec<Job>> {
        self.jobs.iter().map(|j| j.generate()).collect()
    }
}

#[derive(Debug)]
pub struct JobNode {
    pub commands: Vec<CommandNode>,
}

impl JobNode {
    pub fn generate(&self) -> CrushResult<Job> {
        Ok(Job::new(self.commands.iter().map(|c| c.generate()).collect::<CrushResult<Vec<CallDefinition>>>()?))
    }
}

#[derive(Debug)]
pub struct CommandNode {
    pub expressions: Vec<ExpressionNode>,
}

impl CommandNode {
    pub fn generate(&self) -> CrushResult<CallDefinition> {
        let s = self.expressions[0].generate_standalone()?;
        if let Some(c) = s {
            if self.expressions.len() == 1 {
                Ok(c)
            } else {
                error("Stray arguments")
            }
        } else {
            let cmd = self.expressions[0].generate_argument()?;

            let arguments = self.expressions[1..].iter()
                .map(|e| e.generate_argument())
                .collect::<CrushResult<Vec<ArgumentDefinition>>>()?;
            Ok(CallDefinition::new(cmd.value, arguments))
        }
    }
}

#[derive(Debug)]
pub enum ExpressionNode {
    Assignment(AssignmentNode),
    //    ListLiteral(JobListNode),
    Substitution(JobNode),
    Closure(JobListNode),
}

impl ExpressionNode {
    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        match self {
            ExpressionNode::Assignment(a) => a.generate_standalone(),
            ExpressionNode::Substitution(_) => Ok(None),
            ExpressionNode::Closure(_) => Ok(None),
        }
    }

    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        match self {
            ExpressionNode::Assignment(a) => {
                a.generate_argument()
            }
            ExpressionNode::Substitution(s) =>
                Ok(ArgumentDefinition::unnamed(
                    ValueDefinition::JobDefinition(
                        s.generate()?
                    )
                )),
            ExpressionNode::Closure(c) =>
                Ok(ArgumentDefinition::unnamed(
                    ValueDefinition::ClosureDefinition(
                        c.generate()?
                    )
                )),
        }
    }
}

#[derive(Debug)]
pub enum AssignmentNode {
    Assignment(ItemNode, Box<ExpressionNode>),
    Declaration(ItemNode, Box<ExpressionNode>),
    Logical(LogicalNode),
}

impl AssignmentNode {
    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        match self {
            AssignmentNode::Assignment(target, value) => {
                match target {
                    ItemNode::Label(t) => Ok(ArgumentDefinition::named(t.deref(), value.generate_argument()?.value)),
                    ItemNode::Text(_) => error("Invalid left side in assignment"),
                    ItemNode::Integer(_) => error("Invalid left side in assignment"),
                    ItemNode::Get(_, _) => error("Invalid left side in assignment"),
                    ItemNode::Path(_, _) => error("Invalid left side in assignment"),
                }
            }
            AssignmentNode::Declaration(target, value) => {
                error("Variable declarations not supported as arguments")
            }
            AssignmentNode::Logical(l) => {
                l.generate_argument()
            }
        }
    }

    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        match self {
            AssignmentNode::Logical(e) => e.generate_standalone(),
            AssignmentNode::Assignment(target, value) => {
                match target {
                    ItemNode::Label(t) => Ok(Some(
                        CallDefinition::new(
                            ValueDefinition::Value(Value::Command(SET.clone())),
                            vec![ArgumentDefinition::named(t, value.generate_argument()?.value)])
                    )),
                    ItemNode::Text(_) => error("Invalid left side in assignment"),
                    ItemNode::Integer(_) => error("Invalid left side in assignment"),
                    ItemNode::Get(_, _) => error("Invalid left side in assignment"),
                    ItemNode::Path(_, _) => error("Invalid left side in assignment"),
                }
            }
            AssignmentNode::Declaration(target, value) => {
                match target {
                    ItemNode::Label(t) => Ok(Some(
                        CallDefinition::new(
                            ValueDefinition::Value(Value::Command(LET.clone())),
                            vec![ArgumentDefinition::named(t, value.generate_argument()?.value)])
                    )),
                    ItemNode::Text(_) => error("Invalid left side in assignment"),
                    ItemNode::Integer(_) => error("Invalid left side in assignment"),
                    ItemNode::Get(_, _) => error("Invalid left side in assignment"),
                    ItemNode::Path(_, _) => error("Invalid left side in assignment"),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum LogicalNode {
    LogicalOperation(Box<LogicalNode>, Box<str>, ComparisonNode),
    Comparison(ComparisonNode),
}

impl LogicalNode {
    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        match self {
            LogicalNode::LogicalOperation(l, op, r) => {
                match op.as_ref() {
                    "&&" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(AND.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    "||" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(OR.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    _ => error("Unknown operator")
                }
            }
            LogicalNode::Comparison(c) => {
                c.generate_standalone()
            }
        }
    }

    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        match self {
            LogicalNode::LogicalOperation(l, op, r) => {
                Ok(ArgumentDefinition::unnamed(ValueDefinition::JobDefinition(
                    Job::new(vec![self.generate_standalone()?.unwrap()])
                )))
            }
            LogicalNode::Comparison(c) => {
                c.generate_argument()
            }
        }
    }
}

#[derive(Debug)]
pub enum ComparisonNode {
    Comparison(Box<ComparisonNode>, Box<str>, TermNode),
    Term(TermNode),
}

impl ComparisonNode {
    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        match self {
            ComparisonNode::Comparison(l, op, r) => {
                match op.as_ref() {
                    "<" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(LT.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    "<=" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(LTE.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    ">" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(GT.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    ">=" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(GTE.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    "==" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(EQ.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    "!=" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(NEQ.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    _ => error("Unknown operator")
                }
            }
            ComparisonNode::Term(t) => {
                t.generate_standalone()
            }
        }
    }

    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        match self {
            ComparisonNode::Comparison(l, op, r) => {
                Ok(ArgumentDefinition::unnamed(ValueDefinition::JobDefinition(
                    Job::new(vec![self.generate_standalone()?.unwrap()])
                )))
            }
            ComparisonNode::Term(t) => {
                t.generate_argument()
            }
        }
    }
}


#[derive(Debug)]
pub enum TermNode {
    Term(Box<TermNode>, Box<str>, FactorNode),
    Factor(FactorNode),
}

impl TermNode {
    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        match self {
            TermNode::Term(l, op, r) => {
                match op.as_ref() {
                    "+" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(ADD.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    "-" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(SUB.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    _ => error("Unknown operator")
                }
            }
            TermNode::Factor(f) =>
                f.generate_standalone(),
        }
    }

    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        match self {
            TermNode::Term(l, op, r) => {
                Ok(ArgumentDefinition::unnamed(ValueDefinition::JobDefinition(
                    Job::new(vec![self.generate_standalone()?.unwrap()])
                )))
            }
            TermNode::Factor(f) => {
                f.generate_argument()
            }
        }
    }
}

#[derive(Debug)]
pub enum FactorNode {
    Factor(Box<FactorNode>, Box<str>, UnaryNode),
    Unary(UnaryNode),
}

impl FactorNode {
    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        match self {
            FactorNode::Factor(l, op, r) => {
                match op.as_ref() {
                    "*" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(MUL.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    "//" => {
                        Ok(Some(CallDefinition::new(
                            ValueDefinition::Value(Value::Command(DIV.clone())),
                            vec![l.generate_argument()?, r.generate_argument()?])
                        ))
                    }
                    _ => error(format!("Unknown operator {}", op).as_str())
                }
            }
            FactorNode::Unary(u) => {
                u.generate_standalone()
            }
        }
    }

    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        match self {
            FactorNode::Factor(l, op, r) => {
                Ok(ArgumentDefinition::unnamed(ValueDefinition::JobDefinition(
                    Job::new(vec![self.generate_standalone()?.unwrap()])
                )))
            }
            FactorNode::Unary(u) => {
                u.generate_argument()
            }
        }
    }
}

#[derive(Debug)]
pub enum UnaryNode {
    Unary(Box<str>, Box<UnaryNode>),
    Item(ItemNode),
}

impl UnaryNode {
    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        Ok(None)
    }

    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        match self {
            UnaryNode::Unary(op, r) => {
                match op.deref() {
                    "!" => {
                        Ok(ArgumentDefinition::unnamed(ValueDefinition::JobDefinition(
                            Job::new(vec![CallDefinition::new(
                                ValueDefinition::Value(Value::Command(NOT.clone())),
                                vec![r.generate_argument()?])
                            ]))))
                    }
                    _ => error("Unknown operator")
                }
            }
            UnaryNode::Item(i) => {
                i.generate_argument()
            }
        }
    }
}

#[derive(Debug)]
pub enum ItemNode {
    Label(Box<str>),
    Text(Box<str>),
    Integer(i128),
    Get(Box<ItemNode>, Box<JobNode>),
    Path(Box<ItemNode>, Box<str>),
}

fn unescape(s: &str) -> String {
    let mut res = "".to_string();
    let mut was_backslash = false;
    for c in s[1..s.len() - 1].chars() {
        if was_backslash {
            match c {
                'n' => res += "\n",
                'r' => res += "\r",
                't' => res += "\t",
                _ => res += &c.to_string(),
            }
        } else {
            if c == '\\' {
                was_backslash = true;
            } else {
                res += &c.to_string();
            }
        }
    }
    res
}

impl ItemNode {
    pub fn generate_standalone(&self) -> CrushResult<Option<CallDefinition>> {
        Ok(None)
    }

    pub fn generate_argument(&self) -> CrushResult<ArgumentDefinition> {
        Ok(ArgumentDefinition::unnamed(match self {
            ItemNode::Label(l) => ValueDefinition::Lookup(l.clone()),
            ItemNode::Text(t) => ValueDefinition::Value(Value::Text(unescape(t).into_boxed_str())),
            ItemNode::Integer(i) => ValueDefinition::Value(Value::Integer(i.clone())),
            ItemNode::Get(node, field) =>
                ValueDefinition::Get(
                    Box::new(node.generate_argument()?.value),
                    Box::new(ValueDefinition::JobDefinition(field.generate()?))),
            ItemNode::Path(node, label) => ValueDefinition::Path(Box::new(node.generate_argument()?.value), label.clone()),
        }))
    }
    /*
        pub fn path(&self) -> Option<Vec<Box<str>>> {
            match self {
                ItemNode::Label(l) => Some(vec![l.clone()]),
                ItemNode::Text(t) => None,
                ItemNode::Integer(i) => None,
                ItemNode::Get(node, field) => None,
                ItemNode::Path(node, label) => {
                    v = node.path()?;
                    v.push(label);
                    Some(v)
                },
            }
        }
        */
}
