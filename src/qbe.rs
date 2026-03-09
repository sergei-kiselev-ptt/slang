use anyhow::Result;
use std::collections::HashMap;
use std::fmt::Debug;
use thiserror::Error;

use crate::{
    lexer::{Span, TokenType},
    parser::{Expr, LiteralValue, TypeAnnotation},
};

impl Compiler {
    pub fn compile(&mut self, exprs: Vec<Expr>) -> Result<Vec<String>> {
        let mut func_defs = vec![];
        let mut main_exprs = vec![];
        for expr in exprs {
            match expr {
                Expr::FuncDef { .. } => func_defs.push(expr),
                other => main_exprs.push(other),
            }
        }

        for expr in &func_defs {
            if let Expr::FuncDef {
                name,
                params,
                return_type,
                ..
            } = expr
            {
                let sig = FuncSig {
                    params: params
                        .iter()
                        .map(|(tok, ann)| (tok.lexeme.clone(), res_type_from_annotation(ann)))
                        .collect(),
                    return_type: res_type_from_annotation(return_type),
                };
                self.functions.insert(name.lexeme.clone(), sig);
            }
        }

        let mut out = vec![];

        for expr in &func_defs {
            if let Expr::FuncDef { name, body, .. } = expr {
                let lines = self.compile_func_def(name, body)?;
                out.extend(lines);
                out.push(String::new());
            }
        }

        out.push("export function w $main() {".to_string());
        out.push("@start".to_string());
        for expr in &main_exprs {
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
        out.push("data $fmt_print_d = { b \"%g\\n\", b 0 }".to_string());
        out.push("data $str_true_nl = { b \"true\\n\", b 0 }".to_string());
        out.push("data $str_false_nl = { b \"false\\n\", b 0 }".to_string());
        Ok(out)
    }

    pub fn new() -> Self {
        Self {
            counter: 0,
            vars: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    fn next_tmp(&mut self) -> String {
        let tmp = format!("%t{}", self.counter);
        self.counter += 1;
        tmp
    }

    fn compile_func_def(
        &mut self,
        name: &crate::lexer::Token,
        body: &[Expr],
    ) -> Result<Vec<String>> {
        let sig = self.functions[&name.lexeme].clone();
        let ret_ty = sig.return_type;
        let param_types = sig.params;

        // Save outer scope
        let saved_vars = std::mem::take(&mut self.vars);

        // Build QBE parameter list using %p_<name> as the QBE temps for params
        let qbe_params = param_types
            .iter()
            .map(|(n, ty)| format!("{} %p_{}", ty.qbe_abi_type(), n))
            .collect::<Vec<_>>()
            .join(", ");

        let mut out = vec![];
        out.push(format!(
            "export function {} ${}({}) {{",
            ret_ty.qbe_abi_type(),
            name.lexeme,
            qbe_params
        ));
        out.push("@start".to_string());

        // Allocate slots for each parameter and store into them
        for (param_name, param_ty) in &param_types {
            let slot = self.next_tmp();
            out.push(format!("  {}", param_ty.alloc_instr(&slot)));
            out.push(format!(
                "  {}",
                param_ty.store_instr(&format!("%p_{}", param_name), &slot)
            ));
            self.vars
                .insert(param_name.clone(), (slot, param_ty.clone()));
        }

        let (_, _, body_instrs) = self.compile_block(body)?;
        for line in body_instrs {
            if line.starts_with('@') {
                out.push(line);
            } else {
                out.push(format!("  {}", line));
            }
        }
        out.push("}".to_string());

        // Restore outer scope
        self.vars = saved_vars;

        Ok(out)
    }

    /// Compile a sequence of expressions (block body), returning the last value.
    fn compile_block(&mut self, exprs: &[Expr]) -> Result<(String, ResType, Vec<String>)> {
        let mut all_instrs = vec![];
        let mut last_tmp = String::new();
        let mut last_ty = ResType::Number;
        for expr in exprs {
            let (tmp, ty, instrs) = self.compile_expr(expr)?;
            let terminates = last_is_terminator(&instrs);
            all_instrs.extend(instrs);
            last_tmp = tmp;
            last_ty = ty;
            if terminates {
                break;
            }
        }
        Ok((last_tmp, last_ty, all_instrs))
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
                return Err(
                    QbeError::no_span(format!("{:?} is not an arithmetic operator", op)).into(),
                );
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
                return Err(
                    QbeError::no_span(format!("{:?} is not a comparison operator", op)).into(),
                );
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
                return Err(
                    QbeError::no_span(format!("{:?} is not a logical operator", op)).into(),
                );
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
            Expr::Literal(literal_value, _) => self.compile_literal(literal_value),
            Expr::Unary { operator, right } => self.compile_unary(operator, right),
            Expr::Binary {
                left,
                operator,
                right,
            } => self.compile_binary_expr(left, operator, right),
            Expr::Variable { name } => self.compile_var(name),
            Expr::Assign { name, value } => self.compile_assign(name, value),
            Expr::While { condition, body } => self.compile_while(condition, body),
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if(condition, then_branch, else_branch),
            Expr::Return { value } => {
                let (val_tmp, ty, mut instructions) = self.compile_expr(value)?;
                instructions.push(format!("ret {}", val_tmp));
                Ok((val_tmp, ty, instructions))
            }
            Expr::FuncDef { .. } => {
                Err(QbeError::no_span("function definitions must be at the top level").into())
            }
            Expr::Call { name, args } => self.compile_func_call(name, args),
            Expr::Print { value } => self.compile_print(value),
            Expr::Let {
                name,
                type_ann,
                value,
            } => self.compile_let(name, type_ann, value),
        }
    }

    fn compile_literal(
        &mut self,
        literal_value: &LiteralValue,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
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

    fn compile_unary(
        &mut self,
        operator: &crate::lexer::Token,
        right: &Box<Expr>,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
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
            _ => {
                return Err(QbeError::new(
                    format!("unknown unary operator '{}'", operator.lexeme),
                    operator.span,
                )
                .into());
            }
        }
    }

    fn compile_var(
        &mut self,
        name: &crate::lexer::Token,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
        let var_name = &name.lexeme;
        match self.vars.get(var_name) {
            None => {
                Err(QbeError::new(format!("undefined variable '{}'", var_name), name.span).into())
            }
            Some((slot, ty)) => {
                let (slot, ty) = (slot.clone(), ty.clone());
                let tmp = self.next_tmp();
                Ok((tmp.clone(), ty.clone(), vec![ty.load_instr(&tmp, &slot)]))
            }
        }
    }

    fn compile_assign(
        &mut self,
        name: &crate::lexer::Token,
        value: &Box<Expr>,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
        let var_name = name.lexeme.clone();
        let (val_tmp, ty, mut instructions) = self.compile_expr(value)?;
        let slot = match self.vars.get(&var_name) {
            Some((slot, _)) => slot.clone(),
            None => {
                return Err(QbeError::new(
                    format!(
                        "undefined variable '{}': use 'let' to declare it first",
                        var_name
                    ),
                    name.span,
                )
                .into());
            }
        };
        instructions.push(ty.store_instr(&val_tmp, &slot));
        Ok((String::new(), ResType::Void, instructions))
    }

    fn compile_let(
        &mut self,
        name: &crate::lexer::Token,
        type_ann: &crate::parser::TypeAnnotation,
        value: &Box<Expr>,
    ) -> Result<(String, ResType, Vec<String>)> {
        let declared_ty = res_type_from_annotation(type_ann);
        let (val_tmp, _, mut instructions) = self.compile_expr(value)?;
        let slot = self.next_tmp();
        instructions.push(declared_ty.alloc_instr(&slot));
        instructions.push(declared_ty.store_instr(&val_tmp, &slot));
        self.vars
            .insert(name.lexeme.clone(), (slot, declared_ty.clone()));
        Ok((val_tmp, declared_ty, instructions))
    }

    fn compile_while(
        &mut self,
        condition: &Box<Expr>,
        body: &Vec<Expr>,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
        let id = self.counter;
        let cond_label = format!("@cond_{}", id);
        let body_label = format!("@body_{}", id);
        let end_label = format!("@end_{}", id);
        let mut out = vec![];
        out.push(format!("jmp {}", cond_label));
        out.push(cond_label.clone());
        let (cond_tmp, cond_ty, cond_instrs) = self.compile_expr(condition)?;
        if !matches!(cond_ty, ResType::Bool) {
            let span = condition.span().unwrap_or_default();
            return Err(QbeError::new(
                format!("while condition must be a bool, got {:?}", cond_ty),
                span,
            )
            .into());
        }
        out.extend(cond_instrs);
        out.push(format!("jnz {}, {}, {}", cond_tmp, body_label, end_label));
        out.push(body_label);
        let (_, _, body_instrs) = self.compile_block(body)?;
        let body_terminates = last_is_terminator(&body_instrs);
        out.extend(body_instrs);
        if !body_terminates {
            out.push(format!("jmp {}", cond_label));
        }
        out.push(end_label);
        let result_tmp = self.next_tmp();
        out.push(ResType::Number.init_default_instr(&result_tmp));
        Ok((result_tmp, ResType::Number, out))
    }

    fn compile_if(
        &mut self,
        condition: &Box<Expr>,
        then_branch: &Vec<Expr>,
        else_branch: &Option<Vec<Expr>>,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
        let (cond_tmp, cond_ty, cond_instrs) = self.compile_expr(condition)?;
        if !matches!(cond_ty, ResType::Bool) {
            let span = condition.span().unwrap_or_default();
            return Err(QbeError::new(
                format!("if condition must be a boolean, got {:?}", cond_ty),
                span,
            )
            .into());
        }
        let (then_tmp, then_ty, then_instrs) = self.compile_block(then_branch)?;
        let else_compiled = else_branch
            .as_ref()
            .map(|eb| self.compile_block(eb))
            .transpose()?;
        let id = self.counter;
        let slot = self.next_tmp();
        let then_label = format!("@then_{}", id);
        let end_label = format!("@end_{}", id);
        let mut out = cond_instrs;
        out.push(then_ty.alloc_instr(&slot));
        let then_terminates = last_is_terminator(&then_instrs);
        if let Some((else_tmp, _, else_instrs)) = else_compiled {
            let else_label = format!("@else_{}", id);
            let else_terminates = last_is_terminator(&else_instrs);

            out.push(format!("jnz {}, {}, {}", cond_tmp, then_label, else_label));

            out.push(then_label);
            out.extend(then_instrs);
            if !then_terminates {
                out.push(then_ty.store_instr(&then_tmp, &slot));
                out.push(format!("jmp {}", end_label));
            }

            out.push(else_label);
            out.extend(else_instrs);
            if !else_terminates {
                out.push(then_ty.store_instr(&else_tmp, &slot));
                out.push(format!("jmp {}", end_label));
            }
        } else {
            // no else: initialize slot to type default, skip then if false
            let default_tmp = self.next_tmp();
            out.push(then_ty.init_default_instr(&default_tmp));
            out.push(then_ty.store_instr(&default_tmp, &slot));
            out.push(format!("jnz {}, {}, {}", cond_tmp, then_label, end_label));

            out.push(then_label);
            out.extend(then_instrs);
            if !then_terminates {
                out.push(then_ty.store_instr(&then_tmp, &slot));
                out.push(format!("jmp {}", end_label));
            }
        }
        out.push(end_label);
        let result_tmp = self.next_tmp();
        out.push(then_ty.load_instr(&result_tmp, &slot));
        Ok((result_tmp, then_ty, out))
    }

    fn compile_func_call(
        &mut self,
        name: &crate::lexer::Token,
        args: &Vec<Expr>,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
        let func_name = &name.lexeme;
        let sig = self
            .functions
            .get(func_name)
            .ok_or_else(|| QbeError::new(format!("undefined function '{}'", func_name), name.span))?
            .clone();
        if args.len() != sig.params.len() {
            return Err(QbeError::new(
                format!(
                    "function '{}' expects {} arguments, got {}",
                    func_name,
                    sig.params.len(),
                    args.len()
                ),
                name.span,
            )
            .into());
        }
        let mut instructions = vec![];
        let mut arg_tmps: Vec<(String, ResType)> = vec![];
        for (arg_expr, (_, param_ty)) in args.iter().zip(sig.params.iter()) {
            let (arg_tmp, _, arg_instrs) = self.compile_expr(arg_expr)?;
            instructions.extend(arg_instrs);
            arg_tmps.push((arg_tmp, param_ty.clone()));
        }
        let result_tmp = self.next_tmp();
        let ret_abi = sig.return_type.qbe_abi_type();
        let args_str = arg_tmps
            .iter()
            .map(|(tmp, ty)| format!("{} {}", ty.qbe_abi_type(), tmp))
            .collect::<Vec<_>>()
            .join(", ");
        instructions.push(format!(
            "{} ={} call ${}({})",
            result_tmp, ret_abi, func_name, args_str
        ));
        Ok((result_tmp, sig.return_type.clone(), instructions))
    }

    fn compile_print(
        &mut self,
        value: &Box<Expr>,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
        let (val_tmp, ty, mut instructions) = self.compile_expr(value)?;
        match ty {
            ResType::Void => {
                let span = value.span().unwrap_or_default();
                return Err(QbeError::new("cannot print a void value (assignment has no value)", span).into());
            }
            ResType::Number => {
                instructions.push(format!("call $printf(l $fmt_print_d, ..., d {})", val_tmp));
            }
            ResType::Bool => {
                let id = self.counter;
                let _ = self.next_tmp(); // reserve id
                let true_label = format!("@print_true_{}", id);
                let false_label = format!("@print_false_{}", id);
                let end_label = format!("@print_end_{}", id);
                instructions.push(format!("jnz {}, {}, {}", val_tmp, true_label, false_label));
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

    fn compile_binary_expr(
        &mut self,
        left: &Box<Expr>,
        operator: &crate::lexer::Token,
        right: &Box<Expr>,
    ) -> std::result::Result<(String, ResType, Vec<String>), anyhow::Error> {
        let (l_tmp, l_type, l_instructions) = self.compile_expr(left)?;
        let (r_tmp, r_type, r_instructions) = self.compile_expr(right)?;
        let mut instructions = l_instructions;
        instructions.extend(r_instructions);
        if l_type != r_type {
            return Err(QbeError::new(
                format!(
                    "type mismatch in '{}': left is {:?}, right is {:?}",
                    operator.lexeme, l_type, r_type
                ),
                operator.span,
            )
            .into());
        }
        use TokenType::*;
        let (tmp, res_type, instr) = match operator.token_type {
            Plus | Minus | Star | Slash => {
                self.emit_arithmetic(&operator.token_type, &l_tmp, &r_tmp)?
            }
            EqualEqual | BangEqual | Greater | GreaterEqual | Less | LessEqual => {
                self.emit_comparison(&operator.token_type, &l_tmp, &r_tmp, &l_type)?
            }
            LogicalOr | LogicalAnd => self.emit_logical(&operator.token_type, &l_tmp, &r_tmp)?,
            _ => {
                return Err(QbeError::new(
                    format!("'{}' cannot be a binary operator", operator.lexeme),
                    operator.span,
                )
                .into());
            }
        };
        instructions.push(instr);
        Ok((tmp, res_type, instructions))
    }
}

fn last_is_terminator(instrs: &[String]) -> bool {
    instrs
        .iter()
        .rev()
        .find(|i| !i.trim().is_empty() && !i.trim_start().starts_with('@'))
        .map(|i| {
            let s = i.trim();
            s.starts_with("ret") || s.starts_with("jmp ") || s.starts_with("jnz ")
        })
        .unwrap_or(false)
}

fn res_type_from_annotation(ann: &TypeAnnotation) -> ResType {
    match ann {
        TypeAnnotation::Num => ResType::Number,
        TypeAnnotation::Bool => ResType::Bool,
    }
}

#[derive(Error, Debug)]
pub enum QbeError {
    #[error("{}: {message}", span.map(|s| s.to_string()).unwrap_or_default())]
    CompilationError { message: String, span: Option<Span> },
}

impl QbeError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        QbeError::CompilationError {
            message: message.into(),
            span: Some(span),
        }
    }

    fn no_span(message: impl Into<String>) -> Self {
        QbeError::CompilationError {
            message: message.into(),
            span: None,
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            QbeError::CompilationError { span, .. } => *span,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            QbeError::CompilationError { message, .. } => message,
        }
    }
}

#[derive(Clone)]
struct FuncSig {
    params: Vec<(String, ResType)>,
    return_type: ResType,
}

pub struct Compiler {
    counter: usize,
    vars: HashMap<String, (String, ResType)>, // name -> (stack_slot_tmp, type)
    functions: HashMap<String, FuncSig>,
}

#[derive(Debug, PartialEq, Clone)]
enum ResType {
    Number, // QBE type 'd' (double)
    Bool,   // QBE type 'w' (word)
    Void,   // assignment / statement — has no runtime value
}

impl ResType {
    fn alloc_instr(&self, slot: &str) -> String {
        match self {
            ResType::Number => format!("{} =l alloc8 8", slot),
            ResType::Bool => format!("{} =l alloc4 4", slot),
            ResType::Void => panic!("cannot alloc Void"),
        }
    }

    fn store_instr(&self, val: &str, slot: &str) -> String {
        match self {
            ResType::Number => format!("stored {}, {}", val, slot),
            ResType::Bool => format!("storew {}, {}", val, slot),
            ResType::Void => panic!("cannot store Void"),
        }
    }

    fn load_instr(&self, tmp: &str, slot: &str) -> String {
        match self {
            ResType::Number => format!("{} =d loadd {}", tmp, slot),
            ResType::Bool => format!("{} =w loadsw {}", tmp, slot),
            ResType::Void => panic!("cannot load Void"),
        }
    }

    fn init_default_instr(&self, tmp: &str) -> String {
        match self {
            ResType::Number => format!("{} =d copy d_0", tmp),
            ResType::Bool => format!("{} =w copy 0", tmp),
            ResType::Void => panic!("cannot init Void"),
        }
    }

    fn qbe_abi_type(&self) -> &str {
        match self {
            ResType::Number => "d",
            ResType::Bool => "w",
            ResType::Void => panic!("Void has no QBE ABI type"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::parse_into_tokens;
    use crate::parser::Parser;

    fn compile_expr(source: &str) -> (String, ResType, Vec<String>) {
        let tokens = parse_into_tokens(source).unwrap();
        let mut parser = Parser::new(tokens);
        let expr = parser.parse().unwrap();
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
        let tokens = parse_into_tokens("let x: num = 0").unwrap();
        let mut compiler = Compiler::new();
        compiler
            .compile_expr(&Parser::new(tokens).parse().unwrap())
            .unwrap();

        let tokens = parse_into_tokens("x = 42").unwrap();
        let (_, ty, instrs) = compiler
            .compile_expr(&Parser::new(tokens).parse().unwrap())
            .unwrap();
        assert_eq!(ty, ResType::Void);
        assert!(instrs.iter().any(|i| i.contains("stored")));
    }

    #[test]
    fn assign_bool() {
        let tokens = parse_into_tokens("let flag: bool = false").unwrap();
        let mut compiler = Compiler::new();
        compiler
            .compile_expr(&Parser::new(tokens).parse().unwrap())
            .unwrap();

        let tokens = parse_into_tokens("flag = true").unwrap();
        let (_, ty, instrs) = compiler
            .compile_expr(&Parser::new(tokens).parse().unwrap())
            .unwrap();
        assert_eq!(ty, ResType::Void);
        assert!(
            instrs
                .iter()
                .any(|i| i.contains("storew"))
        );
    }

    #[test]
    fn assign_undeclared_errors() {
        let tokens = parse_into_tokens("x = 42").unwrap();
        let mut compiler = Compiler::new();
        let result = compiler.compile_expr(&Parser::new(tokens).parse().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn variable_read_number() {
        let tokens = crate::lexer::parse_into_tokens("let x: num = 5").unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let assign = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile_expr(&assign).unwrap();

        // now read x
        let tokens = crate::lexer::parse_into_tokens("x").unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let var = parser.parse().unwrap();
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
    fn let_number() {
        let (tmp, ty, instrs) = compile_expr("let x: num = 42");
        assert_eq!(ty, ResType::Number);
        assert!(instrs.iter().any(|i| i.contains("alloc8")));
        assert!(
            instrs
                .iter()
                .any(|i| i.contains("stored") && i.contains(&tmp))
        );
    }

    #[test]
    fn let_bool() {
        let (tmp, ty, instrs) = compile_expr("let flag: bool = true");
        assert_eq!(ty, ResType::Bool);
        assert!(instrs.iter().any(|i| i.contains("alloc4")));
        assert!(
            instrs
                .iter()
                .any(|i| i.contains("storew") && i.contains(&tmp))
        );
    }

    #[test]
    fn let_variable_readable() {
        let tokens = parse_into_tokens("let x: num = 10").unwrap();
        let mut parser = Parser::new(tokens);
        let let_expr = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile_expr(&let_expr).unwrap();

        let tokens = parse_into_tokens("x").unwrap();
        let mut parser = Parser::new(tokens);
        let var = parser.parse().unwrap();
        let (tmp, ty, instrs) = compiler.compile_expr(&var).unwrap();
        assert_eq!(ty, ResType::Number);
        assert!(instrs[0].contains("loadd") && instrs[0].starts_with(&tmp));
    }

    fn compile_program(source: &str) -> Vec<String> {
        let tokens = parse_into_tokens(source).unwrap();
        let mut parser = Parser::new(tokens);
        let exprs = parser.parse_program().unwrap();
        let mut compiler = Compiler::new();
        compiler.compile(exprs).unwrap()
    }

    #[test]
    fn func_def_emitted_before_main() {
        let out =
            compile_program("func double(x: num) -> num {\n return x + x\n}\nprint double(3)");
        // function block must appear before $main
        let func_pos = out.iter().position(|l| l.contains("$double")).unwrap();
        let main_pos = out.iter().position(|l| l.contains("$main")).unwrap();
        assert!(func_pos < main_pos);
    }

    #[test]
    fn func_def_return_type_in_signature() {
        let out = compile_program("func square(x: num) -> num {\n return x * x\n}");
        assert!(out.iter().any(|l| l.contains("export function d $square")));
    }

    #[test]
    fn func_def_bool_return() {
        let out = compile_program("func always_true() -> bool {\n return true\n}");
        assert!(
            out.iter()
                .any(|l| l.contains("export function w $always_true"))
        );
    }

    #[test]
    fn call_emits_call_instruction() {
        let out = compile_program("func id(x: num) -> num {\n x\n}\nid(42)");
        assert!(out.iter().any(|l| l.contains("call $id")));
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
