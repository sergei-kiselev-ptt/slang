use anyhow::Result;
use std::collections::HashMap;
use std::fmt::Debug;
use thiserror::Error;

use crate::{
    lexer::TokenType,
    parser::{Expr, LiteralValue},
};

pub struct Compiler {
    counter: usize,
    vars: HashMap<String, (String, ResType)>, // name -> (stack_slot_tmp, type)
}

#[derive(Debug, PartialEq, Clone)]
enum ResType {
    Number, // QBE type 'd' (double)
    Bool,   // QBE type 'w' (word)
}

impl ResType {
    fn alloc_instr(&self, slot: &str) -> String {
        match self {
            ResType::Number => format!("{} =l alloc8 8", slot),
            ResType::Bool => format!("{} =l alloc4 4", slot),
        }
    }

    fn store_instr(&self, val: &str, slot: &str) -> String {
        match self {
            ResType::Number => format!("stored {}, {}", val, slot),
            ResType::Bool => format!("storew {}, {}", val, slot),
        }
    }

    fn load_instr(&self, tmp: &str, slot: &str) -> String {
        match self {
            ResType::Number => format!("{} =d loadd {}", tmp, slot),
            ResType::Bool => format!("{} =w loadsw {}", tmp, slot),
        }
    }

    fn init_default_instr(&self, tmp: &str) -> String {
        match self {
            ResType::Number => format!("{} =d copy d_0", tmp),
            ResType::Bool => format!("{} =w copy 0", tmp),
        }
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            counter: 0,
            vars: HashMap::new(),
        }
    }

    fn next_tmp(&mut self) -> String {
        let tmp = format!("%t{}", self.counter);
        self.counter += 1;
        tmp
    }

    pub fn compile(&mut self, exprs: Vec<Expr>) -> Result<Vec<String>> {
        let mut out = vec![];
        out.push("export function w $main() {".to_string());
        out.push("@start".to_string());

        for expr in &exprs {
            let (_, _, instructions) = self.compile_expr(expr)?;
            for line in instructions {
                if line.starts_with('@') {
                    out.push(line);
                } else {
                    out.push(format!("  {}", line));
                }
            }
        }

        out.push("  ret 0".to_string());
        out.push("}".to_string());
        out.push("\n".to_string());
        out.push("data $fmt_print_d = { b \"%g\\n\", b 0 }".to_string());
        out.push("data $str_true_nl = { b \"true\\n\", b 0 }".to_string());
        out.push("data $str_false_nl = { b \"false\\n\", b 0 }".to_string());
        Ok(out)
    }

    fn emit_arithmetic(
        &mut self,
        op: &TokenType,
        l: &str,
        r: &str,
    ) -> Result<(String, ResType, String)> {
        let opcode = match op {
            TokenType::Plus => "add",
            TokenType::Minus => "sub",
            TokenType::Star => "mul",
            TokenType::Slash => "div",
            _ => {
                return Err(QbeError::CompilationError(format!(
                    "{:?} is not an arithmetic operator",
                    op
                ))
                .into());
            }
        };
        let tmp = self.next_tmp();
        Ok((
            tmp.clone(),
            ResType::Number,
            format!("{} =d {} {}, {}", tmp, opcode, l, r),
        ))
    }

    fn emit_comparison(
        &mut self,
        op: &TokenType,
        l: &str,
        r: &str,
        l_type: &ResType,
    ) -> Result<(String, ResType, String)> {
        let opcode = match (op, l_type) {
            (TokenType::EqualEqual, ResType::Number) => "ceqd",
            (TokenType::EqualEqual, ResType::Bool) => "ceqw",
            (TokenType::BangEqual, ResType::Number) => "cned",
            (TokenType::BangEqual, ResType::Bool) => "cnew",
            (TokenType::Greater, _) => "cgtd",
            (TokenType::GreaterEqual, _) => "cged",
            (TokenType::Less, _) => "cltd",
            (TokenType::LessEqual, _) => "cled",
            _ => {
                return Err(QbeError::CompilationError(format!(
                    "{:?} is not a comparison operator",
                    op
                ))
                .into());
            }
        };
        let tmp = self.next_tmp();
        Ok((
            tmp.clone(),
            ResType::Bool,
            format!("{} =w {} {}, {}", tmp, opcode, l, r),
        ))
    }

    fn emit_logical(
        &mut self,
        op: &TokenType,
        l: &str,
        r: &str,
    ) -> Result<(String, ResType, String)> {
        let opcode = match op {
            TokenType::LogicalOr => "or",
            TokenType::LogicalAnd => "and",
            _ => {
                return Err(QbeError::CompilationError(format!(
                    "{:?} is not a logical operator",
                    op
                ))
                .into());
            }
        };
        let tmp = self.next_tmp();
        Ok((
            tmp.clone(),
            ResType::Bool,
            format!("{} =w {} {}, {}", tmp, opcode, l, r),
        ))
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(String, ResType, Vec<String>)> {
        match expr {
            Expr::Literal(literal_value) => {
                let tmp = self.next_tmp();
                match literal_value {
                    LiteralValue::Number(n) => Ok((
                        tmp.clone(),
                        ResType::Number,
                        vec![format!("{} =d copy d_{}", tmp, n)],
                    )),
                    LiteralValue::Bool(b) => {
                        let val = if *b { 1 } else { 0 };
                        Ok((
                            tmp.clone(),
                            ResType::Bool,
                            vec![format!("{} =w copy {}", tmp, val)],
                        ))
                    }
                    LiteralValue::String(_) => todo!("strings not yet supported"),
                }
            }

            Expr::Unary { operator, right } => {
                let (right_tmp, _right_ty, mut instructions) = self.compile_expr(right)?;
                let tmp = self.next_tmp();
                match operator.token_type {
                    TokenType::Minus => {
                        instructions.push(format!("{} =d neg d_{}", tmp, right_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }
                    TokenType::Plus => {
                        instructions.push(format!("{} =d copy d_{}", tmp, right_tmp));
                        Ok((tmp, ResType::Number, instructions))
                    }
                    TokenType::Bang => {
                        instructions.push(format!("{} =w ceqw {}, 0", tmp, right_tmp));
                        Ok((tmp, ResType::Bool, instructions))
                    }
                    _ => panic!("Unknown unary operator: {:?}", operator.token_type),
                }
            }

            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let (l_tmp, l_type, l_instructions) = self.compile_expr(left)?;
                let (r_tmp, r_type, r_instructions) = self.compile_expr(right)?;
                let mut instructions = l_instructions;
                instructions.extend(r_instructions);
                if l_type != r_type {
                    println!(
                        "operands in binary expression should be of the same type, got {:?}, {:?}",
                        l_type, r_type
                    )
                }

                use TokenType::*;
                let (tmp, res_type, instr) = match operator.token_type {
                    Plus | Minus | Star | Slash => {
                        self.emit_arithmetic(&operator.token_type, &l_tmp, &r_tmp)?
                    }
                    EqualEqual | BangEqual | Greater | GreaterEqual | Less | LessEqual => {
                        self.emit_comparison(&operator.token_type, &l_tmp, &r_tmp, &l_type)?
                    }
                    LogicalOr | LogicalAnd => {
                        self.emit_logical(&operator.token_type, &l_tmp, &r_tmp)?
                    }
                    _ => {
                        return Err(QbeError::CompilationError(format!(
                            "{} cannot be a binary operator",
                            operator.lexeme
                        ))
                        .into());
                    }
                };
                instructions.push(instr);
                Ok((tmp, res_type, instructions))
            }
            Expr::Variable { name } => {
                let var_name = &name.lexeme;
                match self.vars.get(var_name) {
                    None => Err(QbeError::CompilationError(format!(
                        "undefined variable '{}'",
                        var_name
                    ))
                    .into()),
                    Some((slot, ty)) => {
                        let (slot, ty) = (slot.clone(), ty.clone());
                        let tmp = self.next_tmp();
                        Ok((tmp.clone(), ty.clone(), vec![ty.load_instr(&tmp, &slot)]))
                    }
                }
            }
            Expr::Assign { name, value } => {
                let var_name = name.lexeme.clone();
                let (val_tmp, ty, mut instructions) = self.compile_expr(value)?;
                let slot = match self.vars.get(&var_name) {
                    Some((slot, _)) => slot.clone(),
                    None => {
                        let slot = self.next_tmp();
                        instructions.push(ty.alloc_instr(&slot));
                        self.vars.insert(var_name, (slot.clone(), ty.clone()));
                        slot
                    }
                };
                instructions.push(ty.store_instr(&val_tmp, &slot));
                Ok((val_tmp, ty, instructions))
            }
            Expr::While { condition, body } => {
                let id = self.counter;
                let cond_label = format!("@cond_{}", id);
                let body_label = format!("@body_{}", id);
                let end_label = format!("@end_{}", id);

                let mut out = vec![];
                out.push(format!("jmp {}", cond_label));

                out.push(cond_label.clone());
                let (cond_tmp, _, cond_instrs) = self.compile_expr(condition)?;
                out.extend(cond_instrs);
                out.push(format!("jnz {}, {}, {}", cond_tmp, body_label, end_label));

                out.push(body_label);
                for expr in body {
                    let (_, _, instrs) = self.compile_expr(expr)?;
                    out.extend(instrs);
                }
                out.push(format!("jmp {}", cond_label));

                out.push(end_label);
                let result_tmp = self.next_tmp();
                out.push(ResType::Number.init_default_instr(&result_tmp));

                Ok((result_tmp, ResType::Number, out))
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let (cond_tmp, _, cond_instrs) = self.compile_expr(condition)?;
                let (then_tmp, then_ty, then_instrs) = self.compile_expr(then_branch)?;
                let else_compiled = else_branch
                    .as_ref()
                    .map(|eb| self.compile_expr(eb))
                    .transpose()?;

                let id = self.counter;
                let slot = self.next_tmp();
                let then_label = format!("@then_{}", id);
                let end_label = format!("@end_{}", id);

                let mut out = cond_instrs;

                out.push(then_ty.alloc_instr(&slot));

                if let Some((else_tmp, _, else_instrs)) = else_compiled {
                    let else_label = format!("@else_{}", id);
                    out.push(format!("jnz {}, {}, {}", cond_tmp, then_label, else_label));

                    out.push(then_label);
                    out.extend(then_instrs);
                    out.push(then_ty.store_instr(&then_tmp, &slot));
                    out.push(format!("jmp {}", end_label));

                    out.push(else_label);
                    out.extend(else_instrs);
                    out.push(then_ty.store_instr(&else_tmp, &slot));
                    out.push(format!("jmp {}", end_label));
                } else {
                    // no else: initialize slot to type default, skip then if false
                    let default_tmp = self.next_tmp();
                    out.push(then_ty.init_default_instr(&default_tmp));
                    out.push(then_ty.store_instr(&default_tmp, &slot));
                    out.push(format!("jnz {}, {}, {}", cond_tmp, then_label, end_label));

                    out.push(then_label);
                    out.extend(then_instrs);
                    out.push(then_ty.store_instr(&then_tmp, &slot));
                    out.push(format!("jmp {}", end_label));
                }

                out.push(end_label);
                let result_tmp = self.next_tmp();
                out.push(then_ty.load_instr(&result_tmp, &slot));

                Ok((result_tmp, then_ty, out))
            }
            Expr::Print { value } => {
                let (val_tmp, ty, mut instructions) = self.compile_expr(value)?;
                match ty {
                    ResType::Number => {
                        instructions
                            .push(format!("call $printf(l $fmt_print_d, ..., d {})", val_tmp));
                    }
                    ResType::Bool => {
                        let id = self.counter;
                        let _ = self.next_tmp(); // reserve id
                        let true_label = format!("@print_true_{}", id);
                        let false_label = format!("@print_false_{}", id);
                        let end_label = format!("@print_end_{}", id);
                        instructions
                            .push(format!("jnz {}, {}, {}", val_tmp, true_label, false_label));
                        instructions.push(true_label);
                        instructions.push("call $printf(l $str_true_nl)".to_string());
                        instructions.push(format!("jmp {}", end_label));
                        instructions.push(false_label);
                        instructions.push("call $printf(l $str_false_nl)".to_string());
                        instructions.push(end_label);
                    }
                }
                Ok((val_tmp, ty, instructions))
            }
        }
    }
}

#[derive(Error, Debug)]
enum QbeError {
    #[error("{0}")]
    CompilationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::parse_into_tokens;
    use crate::parser::Parser;

    fn compile_expr(source: &str) -> (String, ResType, Vec<String>) {
        let tokens = parse_into_tokens(source).unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse();
        let mut compiler = Compiler::new();
        compiler.compile_expr(&expr).unwrap()
    }

    #[test]
    fn comparison_equal() {
        let (tmp, ty, instrs) = compile_expr("1 == 2");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w ceqd %t0, %t1", tmp));
    }

    #[test]
    fn comparison_not_equal() {
        let (tmp, ty, instrs) = compile_expr("1 != 2");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w cned %t0, %t1", tmp));
    }

    #[test]
    fn comparison_greater() {
        let (tmp, ty, instrs) = compile_expr("3 > 2");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w cgtd %t0, %t1", tmp));
    }

    #[test]
    fn comparison_greater_equal() {
        let (tmp, ty, instrs) = compile_expr("3 >= 3");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w cged %t0, %t1", tmp));
    }

    #[test]
    fn comparison_less() {
        let (tmp, ty, instrs) = compile_expr("1 < 2");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w cltd %t0, %t1", tmp));
    }

    #[test]
    fn comparison_less_equal() {
        let (tmp, ty, instrs) = compile_expr("2 <= 2");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w cled %t0, %t1", tmp));
    }

    #[test]
    fn assign_number() {
        let (tmp, ty, instrs) = compile_expr("x = 42");
        assert_eq!(ty, ResType::Number);
        // alloc, then stored
        assert!(instrs.iter().any(|i| i.contains("alloc8")));
        assert!(
            instrs
                .iter()
                .any(|i| i.contains("stored") && i.contains(&tmp))
        );
    }

    #[test]
    fn assign_bool() {
        let (tmp, ty, instrs) = compile_expr("flag = true");
        assert_eq!(ty, ResType::Bool);
        assert!(instrs.iter().any(|i| i.contains("alloc4")));
        assert!(
            instrs
                .iter()
                .any(|i| i.contains("storew") && i.contains(&tmp))
        );
    }

    #[test]
    fn variable_read_number() {
        let tokens = crate::lexer::parse_into_tokens("x = 5").unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let assign = parser.parse();
        let mut compiler = Compiler::new();
        compiler.compile_expr(&assign).unwrap();

        // now read x
        let tokens = crate::lexer::parse_into_tokens("x").unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let var = parser.parse();
        let (tmp, ty, instrs) = compiler.compile_expr(&var).unwrap();
        assert_eq!(ty, ResType::Number);
        assert_eq!(instrs.len(), 1);
        assert!(instrs[0].contains("loadd"));
        assert!(instrs[0].starts_with(&tmp));
    }

    #[test]
    fn logical_and() {
        let (tmp, ty, instrs) = compile_expr("true && false");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w and %t0, %t1", tmp));
    }

    #[test]
    fn logical_or() {
        let (tmp, ty, instrs) = compile_expr("false || true");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w or %t0, %t1", tmp));
    }

    #[test]
    fn bool_equal() {
        let (tmp, ty, instrs) = compile_expr("true == false");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w ceqw %t0, %t1", tmp));
    }

    #[test]
    fn bool_not_equal() {
        let (tmp, ty, instrs) = compile_expr("true != false");
        assert_eq!(ty, ResType::Bool);
        assert_eq!(instrs.last().unwrap(), &format!("{} =w cnew %t0, %t1", tmp));
    }

    #[test]
    fn if_with_else_number() {
        let (tmp, ty, instrs) = compile_expr("if true { 1 } else { 2 }");
        assert_eq!(ty, ResType::Number);
        assert!(instrs.iter().any(|i| i.contains("jnz")));
        assert!(instrs.iter().any(|i| i.starts_with("@then_")));
        assert!(instrs.iter().any(|i| i.starts_with("@else_")));
        assert!(instrs.iter().any(|i| i.starts_with("@end_")));
        assert!(
            instrs.last().unwrap().contains("loadd") && instrs.last().unwrap().starts_with(&tmp)
        );
    }

    #[test]
    fn if_without_else_number() {
        let (tmp, ty, instrs) = compile_expr("if true { 42 }");
        assert_eq!(ty, ResType::Number);
        assert!(instrs.iter().any(|i| i.contains("jnz")));
        assert!(instrs.iter().any(|i| i.starts_with("@then_")));
        assert!(!instrs.iter().any(|i| i.starts_with("@else_")));
        assert!(instrs.iter().any(|i| i.contains("d_0"))); // default init
        assert!(
            instrs.last().unwrap().contains("loadd") && instrs.last().unwrap().starts_with(&tmp)
        );
    }

    #[test]
    fn if_with_else_bool() {
        let (tmp, ty, instrs) = compile_expr("if false { true } else { false }");
        assert_eq!(ty, ResType::Bool);
        assert!(
            instrs.last().unwrap().contains("loadsw") && instrs.last().unwrap().starts_with(&tmp)
        );
    }

    #[test]
    fn print_number() {
        let (tmp, ty, instrs) = compile_expr("print 42");
        assert_eq!(ty, ResType::Number);
        assert_eq!(
            instrs.last().unwrap(),
            &format!("call $printf(l $fmt_print_d, ..., d {})", tmp)
        );
    }

    #[test]
    fn print_bool() {
        let (tmp, ty, instrs) = compile_expr("print true");
        assert_eq!(ty, ResType::Bool);
        assert!(instrs.iter().any(|i| i.contains("jnz") && i.contains(&tmp)));
        assert!(instrs.iter().any(|i| i.contains("$str_true_nl")));
        assert!(instrs.iter().any(|i| i.contains("$str_false_nl")));
        assert!(instrs.last().unwrap().starts_with("@print_end_"));
    }
}
