use std::collections::HashMap;

use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use crate::lexer::TokenType;
use crate::parser::{Expr, LiteralValue};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    Number,
    Bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JitValue {
    Number(f64),
    Bool(bool),
}

// Variable storage: 8 bytes per slot (f64 size)
const SLOT_SIZE: i32 = 8;

#[derive(Debug, Clone)]
struct VarInfo {
    slot: usize,
    var_type: ValueType,
}

pub struct JIT {
    module: JITModule,
    ctx: Context,
    func_counter: u32,
    variables: HashMap<String, VarInfo>,
    var_memory: Vec<u8>,
    next_slot: usize,
}

impl JIT {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();

        let builder = JITBuilder::new(cranelift_module::default_libcall_names()).unwrap();
        let module = JITModule::new(builder);
        let ctx = module.make_context();

        let var_memory = vec![0u8; 256 * SLOT_SIZE as usize];

        Self {
            module,
            ctx,
            func_counter: 0,
            variables: HashMap::new(),
            var_memory,
            next_slot: 0,
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<JitValue, String> {
        let result_type = self.infer_type(expr)?;
        let cranelift_type = match result_type {
            ValueType::Number => types::F64,
            ValueType::Bool => types::I8,
        };

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(cranelift_type));

        let func_name = format!("__repl_eval_{}", self.func_counter);
        self.func_counter += 1;

        let func_id = self
            .module
            .declare_function(&func_name, Linkage::Local, &sig)
            .map_err(|e| e.to_string())?;

        self.ctx.func.signature = sig;
        let mut builder_ctx = FunctionBuilderContext::new();

        let variables = self.variables.clone();
        let next_slot = self.next_slot;
        let mut new_vars: Vec<(String, VarInfo)> = Vec::new();
        {
            let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);
            let entry_block = builder.create_block();
            builder.append_block_params_for_function_params(entry_block);
            builder.switch_to_block(entry_block);
            builder.seal_block(entry_block);

            let mut compile_ctx = CompileContext {
                env_ptr: builder.block_params(entry_block)[0],
                variables: &variables,
                new_vars: &mut new_vars,
                next_slot,
            };

            let (result, _) = compile_expr(expr, &mut builder, &mut compile_ctx)?;

            builder.ins().return_(&[result]);
            builder.finalize();
        }

        for (name, info) in new_vars {
            if info.slot >= self.next_slot {
                self.next_slot = info.slot + 1;
            }
            self.variables.insert(name, info);
        }

        self.module
            .define_function(func_id, &mut self.ctx)
            .map_err(|e| e.to_string())?;

        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().unwrap();

        let code_ptr = self.module.get_finalized_function(func_id);
        let env_ptr = self.var_memory.as_mut_ptr();

        match result_type {
            ValueType::Number => {
                let func: fn(*mut u8) -> f64 = unsafe { std::mem::transmute(code_ptr) };
                Ok(JitValue::Number(func(env_ptr)))
            }
            ValueType::Bool => {
                let func: fn(*mut u8) -> u8 = unsafe { std::mem::transmute(code_ptr) };
                Ok(JitValue::Bool(func(env_ptr) != 0))
            }
        }
    }

    fn infer_type(&self, expr: &Expr) -> Result<ValueType, String> {
        match expr {
            Expr::Literal(lit) => match lit {
                LiteralValue::Number(_) => Ok(ValueType::Number),
                LiteralValue::Bool(_) => Ok(ValueType::Bool),
                LiteralValue::String(_) => Err("Strings not supported yet".to_string()),
            },

            Expr::Binary { operator, left, .. } => match operator.token_type {
                TokenType::Plus | TokenType::Minus | TokenType::Star | TokenType::Slash => {
                    Ok(ValueType::Number)
                }
                TokenType::Greater
                | TokenType::GreaterEqual
                | TokenType::Less
                | TokenType::LessEqual
                | TokenType::EqualEqual
                | TokenType::BangEqual => Ok(ValueType::Bool),
                TokenType::LogicalAnd | TokenType::LogicalOr => Ok(ValueType::Bool),
                _ => self.infer_type(left),
            },

            Expr::Unary { operator, right } => match operator.token_type {
                TokenType::Minus | TokenType::Plus => Ok(ValueType::Number),
                TokenType::Bang => Ok(ValueType::Bool),
                _ => self.infer_type(right),
            },

            Expr::Variable { name } => {
                if let Some(info) = self.variables.get(&name.lexeme) {
                    Ok(info.var_type)
                } else {
                    // Default to Number for undefined variables
                    Ok(ValueType::Number)
                }
            }

            Expr::Assign { value, .. } => self.infer_type(value),

            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_type = self.infer_type(then_branch)?;
                if let Some(else_expr) = else_branch {
                    let else_type = self.infer_type(else_expr)?;
                    if then_type != else_type {
                        return Err(format!(
                            "if branches must have same type: then is {:?}, else is {:?}",
                            then_type, else_type
                        ));
                    }
                }
                Ok(then_type)
            }
        }
    }
}

struct CompileContext<'a> {
    env_ptr: Value,
    variables: &'a HashMap<String, VarInfo>,
    new_vars: &'a mut Vec<(String, VarInfo)>,
    next_slot: usize,
}

impl<'a> CompileContext<'a> {
    fn get_var_info(&self, name: &str) -> Option<&VarInfo> {
        for (n, info) in self.new_vars.iter() {
            if n == name {
                return Some(info);
            }
        }
        self.variables.get(name)
    }

    fn get_or_create_var(&mut self, name: &str, var_type: ValueType) -> VarInfo {
        if let Some(info) = self.get_var_info(name) {
            return info.clone();
        }

        let slot = self.next_slot + self.new_vars.len();
        let info = VarInfo { slot, var_type };
        self.new_vars.push((name.to_string(), info.clone()));
        info
    }
}

fn compile_expr(
    expr: &Expr,
    builder: &mut FunctionBuilder,
    ctx: &mut CompileContext,
) -> Result<(Value, ValueType), String> {
    match expr {
        Expr::Literal(lit) => match lit {
            LiteralValue::Number(n) => Ok((builder.ins().f64const(*n), ValueType::Number)),
            LiteralValue::Bool(b) => {
                let val = builder.ins().iconst(types::I8, if *b { 1 } else { 0 });
                Ok((val, ValueType::Bool))
            }
            LiteralValue::String(_) => Err("Strings not supported yet".to_string()),
        },

        Expr::Variable { name } => {
            let info = ctx
                .get_var_info(&name.lexeme)
                .ok_or_else(|| format!("Undefined variable: {}", name.lexeme))?
                .clone();

            let offset = (info.slot as i32) * SLOT_SIZE;
            let addr = builder.ins().iadd_imm(ctx.env_ptr, offset as i64);

            let val = match info.var_type {
                ValueType::Number => builder.ins().load(types::F64, MemFlags::new(), addr, 0),
                ValueType::Bool => builder.ins().load(types::I8, MemFlags::new(), addr, 0),
            };

            Ok((val, info.var_type))
        }

        Expr::Assign { name, value } => {
            let (val, val_type) = compile_expr(value, builder, ctx)?;

            let info = ctx.get_or_create_var(&name.lexeme, val_type);
            let offset = (info.slot as i32) * SLOT_SIZE;
            let addr = builder.ins().iadd_imm(ctx.env_ptr, offset as i64);

            builder.ins().store(MemFlags::new(), val, addr, 0);

            Ok((val, val_type))
        }

        Expr::Binary {
            left,
            operator,
            right,
        } => {
            let (lhs, lhs_type) = compile_expr(left, builder, ctx)?;
            let (rhs, _rhs_type) = compile_expr(right, builder, ctx)?;

            match operator.token_type {
                TokenType::Plus => Ok((builder.ins().fadd(lhs, rhs), ValueType::Number)),
                TokenType::Minus => Ok((builder.ins().fsub(lhs, rhs), ValueType::Number)),
                TokenType::Star => Ok((builder.ins().fmul(lhs, rhs), ValueType::Number)),
                TokenType::Slash => Ok((builder.ins().fdiv(lhs, rhs), ValueType::Number)),

                TokenType::Greater => {
                    let cmp = builder.ins().fcmp(FloatCC::GreaterThan, lhs, rhs);
                    Ok((cmp, ValueType::Bool))
                }
                TokenType::GreaterEqual => {
                    let cmp = builder.ins().fcmp(FloatCC::GreaterThanOrEqual, lhs, rhs);
                    Ok((cmp, ValueType::Bool))
                }
                TokenType::Less => {
                    let cmp = builder.ins().fcmp(FloatCC::LessThan, lhs, rhs);
                    Ok((cmp, ValueType::Bool))
                }
                TokenType::LessEqual => {
                    let cmp = builder.ins().fcmp(FloatCC::LessThanOrEqual, lhs, rhs);
                    Ok((cmp, ValueType::Bool))
                }
                TokenType::EqualEqual => match lhs_type {
                    ValueType::Number => {
                        let cmp = builder.ins().fcmp(FloatCC::Equal, lhs, rhs);
                        Ok((cmp, ValueType::Bool))
                    }
                    ValueType::Bool => {
                        let cmp = builder.ins().icmp(IntCC::Equal, lhs, rhs);
                        Ok((cmp, ValueType::Bool))
                    }
                },
                TokenType::BangEqual => match lhs_type {
                    ValueType::Number => {
                        let cmp = builder.ins().fcmp(FloatCC::NotEqual, lhs, rhs);
                        Ok((cmp, ValueType::Bool))
                    }
                    ValueType::Bool => {
                        let cmp = builder.ins().icmp(IntCC::NotEqual, lhs, rhs);
                        Ok((cmp, ValueType::Bool))
                    }
                },

                TokenType::LogicalAnd => {
                    let result = builder.ins().band(lhs, rhs);
                    Ok((result, ValueType::Bool))
                }
                TokenType::LogicalOr => {
                    let result = builder.ins().bor(lhs, rhs);
                    Ok((result, ValueType::Bool))
                }

                _ => Err(format!("Unknown binary operator: {:?}", operator)),
            }
        }

        Expr::Unary { operator, right } => {
            let (val, val_type) = compile_expr(right, builder, ctx)?;

            match operator.token_type {
                TokenType::Minus => Ok((builder.ins().fneg(val), ValueType::Number)),
                TokenType::Plus => Ok((val, val_type)),
                TokenType::Bang => {
                    let one = builder.ins().iconst(types::I8, 1);
                    let result = builder.ins().bxor(val, one);
                    Ok((result, ValueType::Bool))
                }
                _ => Err(format!("Unknown unary operator: {:?}", operator)),
            }
        }

        Expr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let (cond_val, cond_type) = compile_expr(condition, builder, ctx)?;
            if cond_type != ValueType::Bool {
                // error!("If condition should be a boolean type");
                return Err(format!(
                    "Condition should be a boolean type, got {:?}",
                    cond_type
                ));
            }

            let then_block = builder.create_block();
            let else_block = builder.create_block();
            let merge_block = builder.create_block();

            builder
                .ins()
                .brif(cond_val, then_block, &[], else_block, &[]);

            builder.switch_to_block(then_block);
            builder.seal_block(then_block);
            let (then_val, then_type) = compile_expr(then_branch, builder, ctx)?;
            builder.ins().jump(merge_block, &[then_val]);

            builder.switch_to_block(else_block);
            builder.seal_block(else_block);
            let else_val = if let Some(else_expr) = else_branch {
                let (val, _) = compile_expr(else_expr, builder, ctx)?;
                val
            } else {
                match then_type {
                    ValueType::Number => builder.ins().f64const(0.0),
                    ValueType::Bool => builder.ins().iconst(types::I8, 0),
                }
            };
            builder.ins().jump(merge_block, &[else_val]);

            let result_type = match then_type {
                ValueType::Number => types::F64,
                ValueType::Bool => types::I8,
            };
            builder.append_block_param(merge_block, result_type);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);
            let result = builder.block_params(merge_block)[0];

            Ok((result, then_type))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::parse_into_tokens;
    use crate::parser::Parser;

    fn eval(source: &str) -> JitValue {
        let tokens = parse_into_tokens(source).unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();
        let mut jit = JIT::new();
        jit.eval(&expr).unwrap()
    }

    fn eval_with_jit(jit: &mut JIT, source: &str) -> JitValue {
        let tokens = parse_into_tokens(source).unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();
        jit.eval(&expr).unwrap()
    }

    #[test]
    fn jit_number() {
        assert_eq!(eval("42"), JitValue::Number(42.0));
    }

    #[test]
    fn jit_bool_true() {
        assert_eq!(eval("true"), JitValue::Bool(true));
    }

    #[test]
    fn jit_bool_false() {
        assert_eq!(eval("false"), JitValue::Bool(false));
    }

    #[test]
    fn jit_addition() {
        assert_eq!(eval("2 + 3"), JitValue::Number(5.0));
    }

    #[test]
    fn jit_precedence() {
        assert_eq!(eval("2 + 3 * 4"), JitValue::Number(14.0));
    }

    #[test]
    fn jit_parentheses() {
        assert_eq!(eval("(2 + 3) * 4"), JitValue::Number(20.0));
    }

    #[test]
    fn jit_comparison() {
        assert_eq!(eval("5 > 3"), JitValue::Bool(true));
        assert_eq!(eval("5 < 3"), JitValue::Bool(false));
        assert_eq!(eval("3 >= 3"), JitValue::Bool(true));
        assert_eq!(eval("3 <= 2"), JitValue::Bool(false));
    }

    #[test]
    fn jit_equality() {
        assert_eq!(eval("5 == 5"), JitValue::Bool(true));
        assert_eq!(eval("5 != 5"), JitValue::Bool(false));
        assert_eq!(eval("true == true"), JitValue::Bool(true));
        assert_eq!(eval("true == false"), JitValue::Bool(false));
        assert_eq!(eval("true != false"), JitValue::Bool(true));
    }

    #[test]
    fn jit_unary() {
        assert_eq!(eval("-5"), JitValue::Number(-5.0));
        assert_eq!(eval("--5"), JitValue::Number(5.0));
    }

    #[test]
    fn jit_logical_not() {
        assert_eq!(eval("!true"), JitValue::Bool(false));
        assert_eq!(eval("!false"), JitValue::Bool(true));
        assert_eq!(eval("!!true"), JitValue::Bool(true));
    }

    #[test]
    fn jit_logical_and() {
        assert_eq!(eval("true && true"), JitValue::Bool(true));
        assert_eq!(eval("true && false"), JitValue::Bool(false));
        assert_eq!(eval("false && true"), JitValue::Bool(false));
        assert_eq!(eval("false && false"), JitValue::Bool(false));
    }

    #[test]
    fn jit_logical_or() {
        assert_eq!(eval("true || true"), JitValue::Bool(true));
        assert_eq!(eval("true || false"), JitValue::Bool(true));
        assert_eq!(eval("false || true"), JitValue::Bool(true));
        assert_eq!(eval("false || false"), JitValue::Bool(false));
    }

    #[test]
    fn jit_complex_logical() {
        assert_eq!(eval("(true && false) || true"), JitValue::Bool(true));
        assert_eq!(eval("true && (false || true)"), JitValue::Bool(true));
        assert_eq!(eval("!(true && false)"), JitValue::Bool(true));
    }

    #[test]
    fn jit_variable_assignment() {
        let mut jit = JIT::new();
        assert_eq!(eval_with_jit(&mut jit, "x = 42"), JitValue::Number(42.0));
    }

    #[test]
    fn jit_variable_read() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "x = 42");
        assert_eq!(eval_with_jit(&mut jit, "x"), JitValue::Number(42.0));
    }

    #[test]
    fn jit_variable_in_expression() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "x = 10");
        assert_eq!(eval_with_jit(&mut jit, "x + 5"), JitValue::Number(15.0));
    }

    #[test]
    fn jit_variable_reassignment() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "x = 10");
        eval_with_jit(&mut jit, "x = 20");
        assert_eq!(eval_with_jit(&mut jit, "x"), JitValue::Number(20.0));
    }

    #[test]
    fn jit_multiple_variables() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "x = 10");
        eval_with_jit(&mut jit, "y = 20");
        assert_eq!(eval_with_jit(&mut jit, "x + y"), JitValue::Number(30.0));
    }

    #[test]
    fn jit_chained_assignment() {
        let mut jit = JIT::new();
        assert_eq!(eval_with_jit(&mut jit, "x = y = 5"), JitValue::Number(5.0));
        assert_eq!(eval_with_jit(&mut jit, "x"), JitValue::Number(5.0));
        assert_eq!(eval_with_jit(&mut jit, "y"), JitValue::Number(5.0));
    }

    #[test]
    fn jit_bool_variable() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "flag = true");
        assert_eq!(eval_with_jit(&mut jit, "flag"), JitValue::Bool(true));
        assert_eq!(eval_with_jit(&mut jit, "!flag"), JitValue::Bool(false));
    }

    #[test]
    fn jit_variable_comparison() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "a = 5");
        eval_with_jit(&mut jit, "b = 3");
        assert_eq!(eval_with_jit(&mut jit, "a > b"), JitValue::Bool(true));
        assert_eq!(eval_with_jit(&mut jit, "a == b"), JitValue::Bool(false));
    }

    #[test]
    fn jit_if_true() {
        assert_eq!(eval("if true { 1 } else { 0 }"), JitValue::Number(1.0));
    }

    #[test]
    fn jit_if_false() {
        assert_eq!(eval("if false { 1 } else { 0 }"), JitValue::Number(0.0));
    }

    #[test]
    fn jit_if_with_comparison() {
        assert_eq!(
            eval("if 5 > 3 { 100 } else { 200 }"),
            JitValue::Number(100.0)
        );
        assert_eq!(
            eval("if 5 < 3 { 100 } else { 200 }"),
            JitValue::Number(200.0)
        );
    }

    #[test]
    fn jit_if_with_variable() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "x = 10");
        assert_eq!(
            eval_with_jit(&mut jit, "if x > 5 { 1 } else { 0 }"),
            JitValue::Number(1.0)
        );
        assert_eq!(
            eval_with_jit(&mut jit, "if x < 5 { 1 } else { 0 }"),
            JitValue::Number(0.0)
        );
    }

    #[test]
    fn jit_if_bool_result() {
        assert_eq!(
            eval("if true { true } else { false }"),
            JitValue::Bool(true)
        );
        assert_eq!(
            eval("if false { true } else { false }"),
            JitValue::Bool(false)
        );
    }

    #[test]
    fn jit_if_nested() {
        assert_eq!(
            eval("if true { if false { 1 } else { 2 } } else { 3 }"),
            JitValue::Number(2.0)
        );
    }

    #[test]
    fn jit_if_complex_condition() {
        assert_eq!(
            eval("if (5 > 3) && (2 < 4) { 100 } else { 200 }"),
            JitValue::Number(100.0)
        );
        assert_eq!(
            eval("if (5 > 3) && (2 > 4) { 100 } else { 200 }"),
            JitValue::Number(200.0)
        );
    }

    #[test]
    fn jit_if_no_else_true() {
        assert_eq!(eval("if true { 42 }"), JitValue::Number(42.0));
    }

    #[test]
    fn jit_if_no_else_false() {
        assert_eq!(eval("if false { 42 }"), JitValue::Number(0.0));
    }

    #[test]
    fn jit_if_no_else_with_variable() {
        let mut jit = JIT::new();
        eval_with_jit(&mut jit, "x = 10");
        assert_eq!(
            eval_with_jit(&mut jit, "if x > 5 { 100 }"),
            JitValue::Number(100.0)
        );
        assert_eq!(
            eval_with_jit(&mut jit, "if x < 5 { 100 }"),
            JitValue::Number(0.0)
        );
    }

    #[test]
    fn jit_if_no_else_bool() {
        assert_eq!(eval("if true { true }"), JitValue::Bool(true));
        assert_eq!(eval("if false { true }"), JitValue::Bool(false)); // default false
    }
}
