use std::collections::HashMap;
use std::env;
use std::fs;

#[derive(Debug, Clone)]
enum Tok {
    Word(String),
    Num(i32),
    Str(String), // S" ... "
    Colon,
    Semi,
}

fn is_space(c: char) -> bool {
    c.is_whitespace()
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let mut t = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];

        if is_space(c) {
            i += 1;
            continue;
        }

        // comment: ( ... )
        if c == '(' {
            i += 1;
            while i < chars.len() && chars[i] != ')' {
                i += 1;
            }
            if i >= chars.len() {
                return Err("Unterminated comment '('".into());
            }
            i += 1; // skip ')'
            continue;
        }

        // colon / semicolon
        if c == ':' {
            t.push(Tok::Colon);
            i += 1;
            continue;
        }
        if c == ';' {
            t.push(Tok::Semi);
            i += 1;
            continue;
        }

        // S" ... "
        if c == 'S' && i + 1 < chars.len() && chars[i + 1] == '"' {
            i += 2; // skip S"
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                s.push(chars[i]);
                i += 1;
            }
            if i >= chars.len() {
                return Err("Unterminated string literal S\"".into());
            }
            i += 1; // skip closing "
            t.push(Tok::Str(s));
            continue;
        }

        // general word/number token until whitespace or delimiter
        let mut buf = String::new();
        while i < chars.len() {
            let cc = chars[i];
            if is_space(cc) || cc == '(' || cc == ':' || cc == ';' {
                break;
            }
            buf.push(cc);
            i += 1;
        }

        // number? (i32)
        if let Ok(v) = buf.parse::<i32>() {
            t.push(Tok::Num(v));
        } else {
            t.push(Tok::Word(buf));
        }
    }

    Ok(t)
}

struct LlvmBuilder {
    out: String,
    globals: String,
    tmp: u32,
    lbl: u32,
}

impl LlvmBuilder {
    fn new() -> Self {
        Self {
            out: String::new(),
            globals: String::new(),
            tmp: 0,
            lbl: 0,
        }
    }
    fn fresh_tmp(&mut self) -> String {
        self.tmp += 1;
        format!("%t{}", self.tmp)
    }
    fn fresh_lbl(&mut self, prefix: &str) -> String {
        self.lbl += 1;
        format!("{}.{}", prefix, self.lbl)
    }
    fn emit_line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn emit_global_line(&mut self, s: &str) {
        self.globals.push_str(s);
        self.globals.push('\n');
    }
}

#[derive(Debug, Clone)]
enum Control {
    If {
        else_lbl: String,
        end_lbl: String,
        has_else: bool,
    },
    Begin {
        begin_lbl: String,
        // For WHILE/REPEAT
        while_false_lbl: Option<String>,
        while_true_lbl: Option<String>,
    },
}

struct Codegen<'a> {
    b: LlvmBuilder,
    // ABI: stack_base: i32*, sp_ptr: i32*
    stack_base: &'a str,
    sp_ptr: &'a str,
    ctrl: Vec<Control>,
    externs: HashMap<String, String>, // word -> llvm callee
}

impl<'a> Codegen<'a> {
    fn new() -> Self {
        let mut externs = HashMap::new();
        // Map your high-level service words to C runtime symbols
        externs.insert("PWRITE-I32".into(), "pwrite_i32".into());
        externs.insert("PWRITE-BOOL".into(), "pwrite_bool".into());
        externs.insert("PWRITE-CHAR".into(), "pwrite_char".into());
        externs.insert("PWRITE-STR".into(), "pwrite_str".into());
        externs.insert("PWRITELN".into(), "pwriteln".into());
        externs.insert("PWRITE-HEX".into(), "pwrite_hex".into());

        externs.insert("PREAD-I32".into(), "pread_i32".into());
        externs.insert("PREAD-BOOL".into(), "pread_bool".into());
        externs.insert("PREAD-CHAR".into(), "pread_char".into());
        externs.insert("PREADLN".into(), "preadln".into());

        // Variable/field accessors as services (you can later lower them)
        externs.insert("PVAR@".into(), "pvar_get".into());
        externs.insert("PVAR!".into(), "pvar_set".into());
        externs.insert("PFIELD@".into(), "pfield_get".into());
        externs.insert("PFIELD!".into(), "pfield_set".into());

        externs.insert("PBOOL".into(), "pbool".into());

        Self {
            b: LlvmBuilder::new(),
            stack_base: "%stack_base",
            sp_ptr: "%sp_ptr",
            ctrl: Vec::new(),
            externs,
        }
    }

    fn emit_prelude(&mut self) {
        self.b.emit_line("; ModuleID = 'forthc'");
        self.b.emit_line("");

        // extern declarations (edit to match your runtime.c)
        self.b.emit_line("declare void @pwrite_i32(i32)");
        self.b.emit_line("declare void @pwrite_bool(i32)");
        self.b.emit_line("declare void @pwrite_char(i32)");
        self.b.emit_line("declare void @pwrite_hex(i32)");
        self.b.emit_line("declare void @pwriteln()");
        self.b.emit_line("declare void @pwrite_str(i8*)");

        self.b.emit_line("declare i32 @pread_i32()");
        self.b.emit_line("declare i32 @pread_bool()");
        self.b.emit_line("declare i32 @pread_char()");
        self.b.emit_line("declare void @preadln()");

        self.b.emit_line("declare i32 @pvar_get(i32)");
        self.b.emit_line("declare void @pvar_set(i32, i32)");
        self.b.emit_line("declare i32 @pfield_get(i32, i32)");
        self.b.emit_line("declare void @pfield_set(i32, i32, i32)");
        self.b.emit_line("declare i32 @pbool(i32)");
        self.b.emit_line("");
    }

    fn emit_main_wrapper(&mut self, entry: &str) {
        // A tiny C-like main in LLVM, allocating stack+sp on entry.
        // You can also do this in C instead; this is just convenience.
        self.b.emit_line("define i32 @main() {");
        self.b.emit_line("entry:");
        self.b.emit_line("  %stack = alloca [1024 x i32], align 16");
        self.b.emit_line("  %sp = alloca i32, align 4");
        self.b.emit_line("  store i32 0, i32* %sp, align 4");
        self.b.emit_line("  %base = getelementptr inbounds [1024 x i32], [1024 x i32]* %stack, i32 0, i32 0");
        self.b.emit_line(&format!("  call void @{}(i32* %base, i32* %sp)", entry));
        self.b.emit_line("  ret i32 0");
        self.b.emit_line("}");
        self.b.emit_line("");
    }

    fn begin_func(&mut self, name: &str) {
        self.b.emit_line(&format!(
            "define void @{}(i32* %stack_base, i32* %sp_ptr) {{",
            name
        ));
        self.b.emit_line("entry:");
    }

    fn end_func(&mut self) {
        self.b.emit_line("  ret void");
        self.b.emit_line("}");
        self.b.emit_line("");
    }

    // stack ops: push/pop using memory stack + sp_ptr
    fn load_sp(&mut self) -> String {
        let t = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = load i32, i32* {}, align 4", t, self.sp_ptr));
        t
    }
    fn store_sp(&mut self, sp: &str) {
        self.b
            .emit_line(&format!("  store i32 {}, i32* {}, align 4", sp, self.sp_ptr));
    }

    fn push_i32(&mut self, v: &str) {
        let sp = self.load_sp();
        let ptr = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = getelementptr inbounds i32, i32* {}, i32 {}",
            ptr, self.stack_base, sp
        ));
        self.b.emit_line(&format!("  store i32 {}, i32* {}, align 4", v, ptr));
        let sp2 = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = add i32 {}, 1", sp2, sp)); // wrap
        self.store_sp(&sp2);
    }

    fn pop_i32(&mut self) -> String {
        let sp = self.load_sp();
        let sp2 = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = sub i32 {}, 1", sp2, sp)); // wrap
        self.store_sp(&sp2);
        let ptr = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = getelementptr inbounds i32, i32* {}, i32 {}",
            ptr, self.stack_base, sp2
        ));
        let v = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = load i32, i32* {}, align 4", v, ptr));
        v
    }

    fn dup(&mut self) {
        let v = self.pop_i32();
        self.push_i32(&v);
        self.push_i32(&v);
    }
    fn drop(&mut self) {
        let _ = self.pop_i32();
    }

    fn binop(&mut self, op: &str) {
        let b = self.pop_i32();
        let a = self.pop_i32();
        let r = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = {} i32 {}, {}", r, op, a, b));
        self.push_i32(&r);
    }

    fn cmp_to_bool_minus1(&mut self, pred: &str) {
        let b = self.pop_i32();
        let a = self.pop_i32();
        let c = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = icmp {} i32 {}, {}", c, pred, a, b));
        // zext i1->i32 gives 0/1; we need 0/-1
        let z = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = zext i1 {} to i32", z, c));
        let neg = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = sub i32 0, {}", neg, z)); // 0 - z => 0 or -1
        self.push_i32(&neg);
    }

    fn unary_negate(&mut self) {
        let a = self.pop_i32();
        let r = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = sub i32 0, {}", r, a));
        self.push_i32(&r);
    }

    fn and(&mut self) {
        self.binop("and");
    }

    fn div_mod(&mut self, is_mod: bool) {
        let b = self.pop_i32();
        let a = self.pop_i32();
        let r = self.b.fresh_tmp();
        if is_mod {
            self.b.emit_line(&format!("  {} = srem i32 {}, {}", r, a, b)); // 0方向
        } else {
            self.b.emit_line(&format!("  {} = sdiv i32 {}, {}", r, a, b)); // 0方向
        }
        self.push_i32(&r);
    }

    fn zero_eq(&mut self) {
        let a = self.pop_i32();
        let c = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = icmp eq i32 {}, 0", c, a));
        let z = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = zext i1 {} to i32", z, c));
        let neg = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = sub i32 0, {}", neg, z));
        self.push_i32(&neg);
    }

    fn zero_lt(&mut self) {
        let a = self.pop_i32();
        let c = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = icmp slt i32 {}, 0", c, a));
        let z = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = zext i1 {} to i32", z, c));
        let neg = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = sub i32 0, {}", neg, z));
        self.push_i32(&neg);
    }

    // Control flow sugar (IF/ELSE/THEN, BEGIN/UNTIL, BEGIN/WHILE/REPEAT)
    fn emit_br_cond_zero_to(&mut self, cond_i32: &str, if_zero_lbl: &str, if_nz_lbl: &str) {
        let c = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = icmp eq i32 {}, 0",
            c, cond_i32
        ));
        self.b.emit_line(&format!(
            "  br i1 {}, label %{}, label %{}",
            c, if_zero_lbl, if_nz_lbl
        ));
    }

    fn begin_if(&mut self) -> Result<(), String> {
        let cond = self.pop_i32();
        let then_lbl = self.b.fresh_lbl("then");
        let else_lbl = self.b.fresh_lbl("else");
        let end_lbl = self.b.fresh_lbl("endif");

        self.emit_br_cond_zero_to(&cond, &else_lbl, &then_lbl);
        self.b.emit_line(&format!("{}:", then_lbl));

        self.ctrl.push(Control::If {
            else_lbl,
            end_lbl,
            has_else: false,
        });
        Ok(())
    }

    fn do_else(&mut self) -> Result<(), String> {
        match self.ctrl.last_mut() {
            Some(Control::If {
                else_lbl,
                end_lbl,
                has_else,
                ..
            }) => {
                // jump to end from then-branch
                self.b.emit_line(&format!("  br label %{}", end_lbl));
                // start else label
                self.b.emit_line(&format!("{}:", else_lbl));
                *has_else = true;
                Ok(())
            }
            _ => Err("ELSE without IF".into()),
        }
    }

    fn end_then(&mut self) -> Result<(), String> {
        match self.ctrl.pop() {
            Some(Control::If {
                else_lbl,
                end_lbl,
                has_else,
                ..
            }) => {
                // if there was no ELSE, else_lbl is the end target
                if !has_else {
                    self.b.emit_line(&format!("  br label %{}", end_lbl));
                    self.b.emit_line(&format!("{}:", else_lbl));
                    self.b.emit_line(&format!("  br label %{}", end_lbl));
                } else {
                    self.b.emit_line(&format!("  br label %{}", end_lbl));
                }
                self.b.emit_line(&format!("{}:", end_lbl));
                Ok(())
            }
            _ => Err("THEN without IF".into()),
        }
    }

    fn begin_begin(&mut self) {
        let begin_lbl = self.b.fresh_lbl("begin");
        self.b.emit_line(&format!("  br label %{}", begin_lbl));
        self.b.emit_line(&format!("{}:", begin_lbl));
        self.ctrl.push(Control::Begin {
            begin_lbl,
            while_false_lbl: None,
            while_true_lbl: None,
        });
    }

    fn begin_while(&mut self) -> Result<(), String> {
        // WHILE must be inside BEGIN ... REPEAT
        let cond = self.pop_i32();
        if !matches!(self.ctrl.last(), Some(Control::Begin { .. })) {
            return Err("WHILE without matching BEGIN".into());
        }

        let true_lbl = self.b.fresh_lbl("while_true");
        let false_lbl = self.b.fresh_lbl("while_false");
        self.emit_br_cond_zero_to(&cond, &false_lbl, &true_lbl);
        self.b.emit_line(&format!("{}:", true_lbl));

        match self.ctrl.last_mut() {
            Some(Control::Begin {
                begin_lbl: _,
                while_false_lbl,
                while_true_lbl,
            }) => {
                *while_true_lbl = Some(true_lbl);
                *while_false_lbl = Some(false_lbl);
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn end_repeat(&mut self) -> Result<(), String> {
        match self.ctrl.pop() {
            Some(Control::Begin {
                begin_lbl,
                while_false_lbl,
                ..
            }) => {
                // if WHILE was used, we must close the true-branch back to begin,
                // and continue at while_false label.
                if let Some(false_lbl) = while_false_lbl {
                    self.b.emit_line(&format!("  br label %{}", begin_lbl));
                    self.b.emit_line(&format!("{}:", false_lbl));
                } else {
                    // plain BEGIN ... REPEAT is infinite loop
                    self.b.emit_line(&format!("  br label %{}", begin_lbl));
                }
                Ok(())
            }
            _ => Err("REPEAT without BEGIN".into()),
        }
    }

    fn end_until(&mut self) -> Result<(), String> {
        let cond = self.pop_i32();
        match self.ctrl.pop() {
            Some(Control::Begin { begin_lbl, .. }) => {
                // UNTIL: loop until cond is true (-1). We'll treat nonzero as true.
                // If cond == 0 => continue loop.
                let done_lbl = self.b.fresh_lbl("until_done");
                let cont_lbl = begin_lbl;
                // cond==0 -> cont, else -> done
                let is_zero = self.b.fresh_tmp();
                self.b
                    .emit_line(&format!("  {} = icmp eq i32 {}, 0", is_zero, cond));
                self.b.emit_line(&format!(
                    "  br i1 {}, label %{}, label %{}",
                    is_zero, cont_lbl, done_lbl
                ));
                self.b.emit_line(&format!("{}:", done_lbl));
                Ok(())
            }
            _ => Err("UNTIL without BEGIN".into()),
        }
    }

    fn emit_string_global(&mut self, s: &str) -> String {
        // naive global string emission; creates a new global each time.
        // Escaping is minimal.
        let mut bytes: Vec<u8> = s.as_bytes().to_vec();
        bytes.push(0);

        let name = self.b.fresh_lbl("str");
        let n = bytes.len();
        let body: String = bytes
            .into_iter()
            .map(|b| format!("\\{:02X}", b))
            .collect();

        self.b.emit_global_line(&format!(
            "@{} = private constant [{} x i8] c\"{}\"",
            name, n, body
        ));

        // Return pointer to first element
        let ptr = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = getelementptr inbounds [{} x i8], [{} x i8]* @{}, i32 0, i32 0",
            ptr, n, n, name
        ));
        ptr
    }

    fn call_extern(&mut self, word: &str, arg_mode: ExternArgMode, str_arg: Option<String>) -> Result<(), String> {
        let callee = self
            .externs
            .get(word)
            .cloned()
            .ok_or_else(|| format!("Unknown extern service word: {}", word))?;

        match arg_mode {
            ExternArgMode::PopI32Void => {
                let v = self.pop_i32();
                self.b.emit_line(&format!("  call void @{}(i32 {})", callee, v));
            }
            ExternArgMode::Void => {
                self.b.emit_line(&format!("  call void @{}()", callee));
            }
            ExternArgMode::RetI32Push => {
                let r = self.b.fresh_tmp();
                self.b.emit_line(&format!("  {} = call i32 @{}()", r, callee));
                self.push_i32(&r);
            }
            ExternArgMode::PopI32RetI32Push => {
                let v = self.pop_i32();
                let r = self.b.fresh_tmp();
                self.b.emit_line(&format!("  {} = call i32 @{}(i32 {})", r, callee, v));
                self.push_i32(&r);
            }
            ExternArgMode::StrVoid => {
                let s = str_arg.ok_or("Missing string argument for PWRITE-STR")?;
                let p = self.emit_string_global(&s);
                self.b.emit_line(&format!("  call void @{}(i8* {})", callee, p));
            }
        }
        Ok(())
    }

    fn compile_body(&mut self, toks: &[Tok]) -> Result<(), String> {
        // Minimal: supports numbers, words, S" + PWRITE-STR, and a set of core words.
        let mut i = 0usize;
        while i < toks.len() {
            match &toks[i] {
                Tok::Num(v) => self.push_i32(&format!("{}", v)),
                Tok::Str(s) => {
                    // For now, require next token to be PWRITE-STR or you can define semantics later.
                    // We'll push nothing; we keep it as immediate string for service call.
                    // A more FORTH-like approach: push address. But we keep it simple here.
                    // Store in a side channel: next token must be Word("PWRITE-STR")
                    if i + 1 >= toks.len() {
                        return Err("S\" must be followed by a word (e.g., PWRITE-STR)".into());
                    }
                    match &toks[i + 1] {
                        Tok::Word(w) if w == "PWRITE-STR" => {
                            self.call_extern("PWRITE-STR", ExternArgMode::StrVoid, Some(s.clone()))?;
                            i += 1; // consume following word
                        }
                        _ => return Err("S\" currently only supported as: S\" ...\" PWRITE-STR".into()),
                    }
                }
                Tok::Word(w) => {
                    match w.as_str() {
                        // stack ops
                        "DUP" => self.dup(),
                        "DROP" => self.drop(),

                        // arithmetic / logic (wrap semantics by default)
                        "+" => self.binop("add"),
                        "-" => self.binop("sub"),
                        "*" => self.binop("mul"),
                        "/" => self.div_mod(false),
                        "MOD" => self.div_mod(true),

                        "NEGATE" => self.unary_negate(),
                        "AND" => self.and(),

                        // comparisons: return -1/0
                        "=" => self.cmp_to_bool_minus1("eq"),
                        "<>" => self.cmp_to_bool_minus1("ne"),
                        "<" => self.cmp_to_bool_minus1("slt"),
                        "<=" => self.cmp_to_bool_minus1("sle"),
                        ">" => self.cmp_to_bool_minus1("sgt"),
                        ">=" => self.cmp_to_bool_minus1("sge"),
                        "0=" => self.zero_eq(),
                        "0<" => self.zero_lt(),

                        // control sugar
                        "IF" => self.begin_if()?,
                        "ELSE" => self.do_else()?,
                        "THEN" => self.end_then()?,
                        "BEGIN" => self.begin_begin(),
                        "UNTIL" => self.end_until()?,
                        "WHILE" => self.begin_while()?,
                        "REPEAT" => self.end_repeat()?,

                        // service calls (extern)
                        "PWRITE-I32" => self.call_extern("PWRITE-I32", ExternArgMode::PopI32Void, None)?,
                        "PWRITE-BOOL" => self.call_extern("PWRITE-BOOL", ExternArgMode::PopI32Void, None)?,
                        "PWRITE-CHAR" => self.call_extern("PWRITE-CHAR", ExternArgMode::PopI32Void, None)?,
                        "PWRITELN" => self.call_extern("PWRITELN", ExternArgMode::Void, None)?,
                        "PWRITE-HEX" => self.call_extern("PWRITE-HEX", ExternArgMode::PopI32Void, None)?,

                        "PREAD-I32" => self.call_extern("PREAD-I32", ExternArgMode::RetI32Push, None)?,
                        "PREAD-BOOL" => self.call_extern("PREAD-BOOL", ExternArgMode::RetI32Push, None)?,
                        "PREAD-CHAR" => self.call_extern("PREAD-CHAR", ExternArgMode::RetI32Push, None)?,
                        "PREADLN" => self.call_extern("PREADLN", ExternArgMode::Void, None)?,

                        "PBOOL" => self.call_extern("PBOOL", ExternArgMode::PopI32RetI32Push, None)?,

                        // You can add PVAR!/PVAR@ etc here later (needs calling convention design)
                        _ => return Err(format!("Unknown word: {}", w)),
                    }
                }
                Tok::Colon | Tok::Semi => {
                    return Err("Unexpected ':' or ';' inside body (top-level parser should split defs)".into())
                }
            }
            i += 1;
        }

        if !self.ctrl.is_empty() {
            return Err(format!("Unclosed control structure(s): {:?}", self.ctrl));
        }
        Ok(())
    }
}

#[derive(Copy, Clone)]
enum ExternArgMode {
    PopI32Void,
    Void,
    RetI32Push,
    PopI32RetI32Push,
    StrVoid,
}

fn parse_defs(toks: &[Tok]) -> Result<Vec<(String, Vec<Tok>)>, String> {
    // Parses:
    // : name ... ;
    // Returns list of (name, body_tokens)
    let mut defs = Vec::new();
    let mut i = 0usize;

    while i < toks.len() {
        match &toks[i] {
            Tok::Colon => {
                i += 1;
                let name = match toks.get(i) {
                    Some(Tok::Word(w)) => w.clone(),
                    _ => return Err("Expected word name after ':'".into()),
                };
                i += 1;

                let mut body = Vec::new();
                while i < toks.len() {
                    if matches!(toks[i], Tok::Semi) {
                        break;
                    }
                    body.push(toks[i].clone());
                    i += 1;
                }
                if i >= toks.len() || !matches!(toks[i], Tok::Semi) {
                    return Err(format!("Definition {} missing ';'", name));
                }
                i += 1; // consume ';'
                defs.push((name, body));
            }
            _ => {
                // Allow top-level tokens to be ignored (or error). Here: error to keep strict.
                return Err("Only ': name ... ;' definitions are allowed at top-level in this prototype.".into());
            }
        }
    }

    Ok(defs)
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!("Usage: {} <input.fth> <output.ll>", args[0]));
    }
    let input = fs::read_to_string(&args[1]).map_err(|e| format!("Read error: {}", e))?;
    let toks = tokenize(&input)?;
    let defs = parse_defs(&toks)?;

    let mut cg = Codegen::new();
    cg.emit_prelude();

    // Compile all defs
    for (name, body) in &defs {
        cg.begin_func(name);
        cg.compile_body(body)?;
        cg.end_func();
    }

    // If there is a word named MAIN, create @main wrapper calling it.
    // Otherwise, if exactly one def exists, call it.
    let entry = if defs.iter().any(|(n, _)| n == "MAIN") {
        "MAIN".to_string()
    } else if defs.len() == 1 {
        defs[0].0.clone()
    } else {
        return Err("No entry point. Define : MAIN ... ; or provide exactly one definition.".into());
    };
    cg.emit_main_wrapper(&entry);

    let module = format!("{}\n{}", cg.b.out, cg.b.globals);
    fs::write(&args[2], module).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}
