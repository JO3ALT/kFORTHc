use std::collections::{HashMap, HashSet};
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
            if i < chars.len() && is_space(chars[i]) {
                i += 1; // Forth-style parsed-string delimiter
            }
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

fn parse_f32_token_bits(s: &str) -> Option<i32> {
    let lower = s.to_ascii_lowercase();
    let bits = match lower.as_str() {
        "inf" | "+inf" => f32::INFINITY.to_bits(),
        "-inf" => f32::NEG_INFINITY.to_bits(),
        "nan" | "+nan" | "-nan" => f32::NAN.to_bits(),
        _ => s.parse::<f32>().ok()?.to_bits(),
    };
    Some(bits as i32)
}

fn llvm_word_sym(word: &str) -> String {
    let mut out = String::from("w");
    for b in word.as_bytes() {
        let ch = *b as char;
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push_str(&format!("_x{:02X}", b));
        }
    }
    out
}

fn extract_routine_aliases(src: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for line in src.lines() {
        let line = line.trim();
        if !line.starts_with("( ROUTINE ") || !line.ends_with(')') {
            continue;
        }
        let body = &line[2..line.len() - 1].trim(); // drop parens
        if let Some((lhs, rhs)) = body
            .strip_prefix("ROUTINE ")
            .and_then(|x| x.split_once(" => "))
        {
            let alias = lhs.trim().to_string();
            let word = rhs.trim().to_string();
            if !alias.is_empty() && !word.is_empty() {
                m.insert(word, alias);
            }
        }
    }
    m
}

fn resolve_prev_compile_time_value(
    toks: &[Tok],
    i: usize,
    here: i32,
    constant_words: &HashMap<String, i32>,
    created_words: &HashMap<String, i32>,
) -> Option<i32> {
    match toks.get(i.wrapping_sub(1))? {
        Tok::Num(v) => Some(*v),
        Tok::Word(w) if w == "HERE" => Some(here),
        Tok::Word(w) => constant_words
            .get(w)
            .copied()
            .or_else(|| created_words.get(w).copied()),
        _ => None,
    }
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
    rstack_base: &'a str,
    rsp_ptr: &'a str,
    ctrl: Vec<Control>,
    externs: HashMap<String, String>, // word -> llvm callee
    created_words: HashMap<String, i32>,
    constant_words: HashMap<String, i32>,
    known_defs: HashSet<String>,
    here: i32,
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
        externs.insert("PREAD-F32".into(), "pread_f32_bits".into());
        externs.insert("PREADLN".into(), "preadln".into());
        externs.insert("HERE".into(), "rt_here".into());
        externs.insert("ALLOT".into(), "rt_allot".into());
        externs.insert("__RT_HEAP_RESET".into(), "rt_heap_reset".into());

        // Variable/field accessors as services (you can later lower them)
        externs.insert("PVAR@".into(), "pvar_get".into());
        externs.insert("PVAR!".into(), "pvar_set".into());
        externs.insert("PFIELD@".into(), "pfield_get".into());
        externs.insert("PFIELD!".into(), "pfield_set".into());

        externs.insert("PBOOL".into(), "pbool".into());
        externs.insert("PWRITE-F32".into(), "pwrite_f32_bits".into());
        externs.insert("FADD".into(), "fadd_bits".into());
        externs.insert("FSUB".into(), "fsub_bits".into());
        externs.insert("FMUL".into(), "fmul_bits".into());
        externs.insert("FDIV".into(), "fdiv_bits".into());
        externs.insert("FNEGATE".into(), "fnegate_bits".into());
        externs.insert("FABS".into(), "fabs_bits".into());
        externs.insert("F=".into(), "feq_bits".into());
        externs.insert("F<".into(), "flt_bits".into());
        externs.insert("F<=".into(), "fle_bits".into());
        externs.insert("FZERO?".into(), "fzero_bits".into());
        externs.insert("FINF?".into(), "finf_bits".into());
        externs.insert("FNAN?".into(), "fnan_bits".into());
        externs.insert("FFINITE?".into(), "ffinite_bits".into());
        externs.insert("S>F".into(), "s_to_f_bits".into());
        externs.insert("F>S".into(), "f_bits_to_s".into());
        externs.insert("Q16.16>F".into(), "q16_16_to_f_bits".into());
        externs.insert("F>Q16.16".into(), "f_bits_to_q16_16".into());
        externs.insert("FROUND-I32".into(), "fround_i32_bits".into());

        externs.insert("__KP_FABS_F32".into(), "kp_fabs_f32_bits".into());
        externs.insert("__KP_FSQRT_F32".into(), "kp_fsqrt_f32_bits".into());
        externs.insert("__KP_FSIN_F32".into(), "kp_fsin_f32_bits".into());
        externs.insert("__KP_FCOS_F32".into(), "kp_fcos_f32_bits".into());
        externs.insert("__KP_FPOW_F32_I32".into(), "kp_fpow_f32_i32_bits".into());
        externs.insert("__KP_FFLOOR_F32".into(), "kp_ffloor_f32_bits".into());
        externs.insert("__KP_FCEIL_F32".into(), "kp_fceil_f32_bits".into());

        externs.insert("__KP_FX_SQRT".into(), "kp_fx_sqrt_i32".into());
        externs.insert("__KP_FX_SIN".into(), "kp_fx_sin_deg_i32".into());
        externs.insert("__KP_FX_COS".into(), "kp_fx_cos_deg_i32".into());
        externs.insert("__KP_FX_TAN".into(), "kp_fx_tan_deg_i32".into());
        externs.insert("__KP_FX_ASIN".into(), "kp_fx_asin_fixed_i32".into());
        externs.insert("__KP_FX_ACOS".into(), "kp_fx_acos_fixed_i32".into());
        externs.insert("__KP_FX_ATAN".into(), "kp_fx_atan_fixed_i32".into());
        externs.insert("__KP_FX_LN".into(), "kp_fx_ln_i32".into());
        externs.insert("__KP_FX_LOG".into(), "kp_fx_log_i32".into());

        Self {
            b: LlvmBuilder::new(),
            stack_base: "%stack_base",
            sp_ptr: "%sp_ptr",
            rstack_base: "%rstack_base",
            rsp_ptr: "%rsp_ptr",
            ctrl: Vec::new(),
            externs,
            created_words: HashMap::new(),
            constant_words: HashMap::new(),
            known_defs: HashSet::new(),
            here: 0,
        }
    }

    fn set_program_symbols(
        &mut self,
        created_words: HashMap<String, i32>,
        constant_words: HashMap<String, i32>,
        known_defs: HashSet<String>,
        here: i32,
    ) {
        self.created_words = created_words;
        self.constant_words = constant_words;
        self.known_defs = known_defs;
        self.here = here;
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
        self.b.emit_line("declare i32 @pread_f32_bits()");
        self.b.emit_line("declare void @preadln()");
        self.b.emit_line("declare i32 @rt_here()");
        self.b.emit_line("declare void @rt_allot(i32)");
        self.b.emit_line("declare void @rt_heap_reset(i32)");

        self.b.emit_line("declare i32 @pvar_get(i32)");
        self.b.emit_line("declare void @pvar_set(i32, i32)");
        self.b.emit_line("declare i32 @pfield_get(i32, i32)");
        self.b.emit_line("declare void @pfield_set(i32, i32, i32)");
        self.b.emit_line("declare i32 @pbool(i32)");
        self.b.emit_line("declare void @pwrite_f32_bits(i32)");
        self.b.emit_line("declare i32 @fadd_bits(i32, i32)");
        self.b.emit_line("declare i32 @fsub_bits(i32, i32)");
        self.b.emit_line("declare i32 @fmul_bits(i32, i32)");
        self.b.emit_line("declare i32 @fdiv_bits(i32, i32)");
        self.b.emit_line("declare i32 @fnegate_bits(i32)");
        self.b.emit_line("declare i32 @fabs_bits(i32)");
        self.b.emit_line("declare i32 @feq_bits(i32, i32)");
        self.b.emit_line("declare i32 @flt_bits(i32, i32)");
        self.b.emit_line("declare i32 @fle_bits(i32, i32)");
        self.b.emit_line("declare i32 @fzero_bits(i32)");
        self.b.emit_line("declare i32 @finf_bits(i32)");
        self.b.emit_line("declare i32 @fnan_bits(i32)");
        self.b.emit_line("declare i32 @ffinite_bits(i32)");
        self.b.emit_line("declare i32 @s_to_f_bits(i32)");
        self.b.emit_line("declare i32 @f_bits_to_s(i32)");
        self.b.emit_line("declare i32 @q16_16_to_f_bits(i32)");
        self.b.emit_line("declare i32 @f_bits_to_q16_16(i32)");
        self.b.emit_line("declare i32 @fround_i32_bits(i32)");
        self.b.emit_line("declare i32 @kp_fabs_f32_bits(i32)");
        self.b.emit_line("declare i32 @kp_fsqrt_f32_bits(i32)");
        self.b.emit_line("declare i32 @kp_fsin_f32_bits(i32)");
        self.b.emit_line("declare i32 @kp_fcos_f32_bits(i32)");
        self.b
            .emit_line("declare i32 @kp_fpow_f32_i32_bits(i32, i32)");
        self.b.emit_line("declare i32 @kp_ffloor_f32_bits(i32)");
        self.b.emit_line("declare i32 @kp_fceil_f32_bits(i32)");
        self.b.emit_line("declare i32 @kp_fx_sqrt_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_sin_deg_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_cos_deg_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_tan_deg_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_asin_fixed_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_acos_fixed_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_atan_fixed_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_ln_i32(i32)");
        self.b.emit_line("declare i32 @kp_fx_log_i32(i32)");
        self.b.emit_line("");
    }

    fn emit_main_wrapper(&mut self, entry: &str) {
        let entry = llvm_word_sym(entry);
        // A tiny C-like main in LLVM, allocating stack+sp on entry.
        // You can also do this in C instead; this is just convenience.
        self.b.emit_line("define i32 @main() {");
        self.b.emit_line("entry:");
        self.b.emit_line("  %stack = alloca [1024 x i32], align 16");
        self.b.emit_line("  %sp = alloca i32, align 4");
        self.b.emit_line("  store i32 0, i32* %sp, align 4");
        self.b.emit_line(
            "  %base = getelementptr inbounds [1024 x i32], [1024 x i32]* %stack, i32 0, i32 0",
        );
        self.b
            .emit_line(&format!("  call void @rt_heap_reset(i32 {})", self.here));
        self.b
            .emit_line(&format!("  call void @{}(i32* %base, i32* %sp)", entry));
        self.b.emit_line("  ret i32 0");
        self.b.emit_line("}");
        self.b.emit_line("");
    }

    fn begin_func(&mut self, name: &str) {
        let name = llvm_word_sym(name);
        self.b.emit_line(&format!(
            "define void @{}(i32* %stack_base, i32* %sp_ptr) {{",
            name
        ));
        self.b.emit_line("entry:");
        self.b
            .emit_line("  %rstack = alloca [1024 x i32], align 16");
        self.b.emit_line("  %rsp_ptr = alloca i32, align 4");
        self.b.emit_line("  store i32 0, i32* %rsp_ptr, align 4");
        self.b.emit_line(
            "  %rstack_base = getelementptr inbounds [1024 x i32], [1024 x i32]* %rstack, i32 0, i32 0",
        );
    }

    fn end_func(&mut self) {
        self.b.emit_line("  ret void");
        self.b.emit_line("}");
        self.b.emit_line("");
    }

    // stack ops: push/pop using memory stack + sp_ptr
    fn load_sp(&mut self) -> String {
        let t = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = load i32, i32* {}, align 4",
            t, self.sp_ptr
        ));
        t
    }
    fn store_sp(&mut self, sp: &str) {
        self.b.emit_line(&format!(
            "  store i32 {}, i32* {}, align 4",
            sp, self.sp_ptr
        ));
    }

    fn push_i32(&mut self, v: &str) {
        let sp = self.load_sp();
        let ptr = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = getelementptr inbounds i32, i32* {}, i32 {}",
            ptr, self.stack_base, sp
        ));
        self.b
            .emit_line(&format!("  store i32 {}, i32* {}, align 4", v, ptr));
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
        self.b
            .emit_line(&format!("  {} = load i32, i32* {}, align 4", v, ptr));
        v
    }

    fn load_rsp(&mut self) -> String {
        let t = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = load i32, i32* {}, align 4",
            t, self.rsp_ptr
        ));
        t
    }

    fn store_rsp(&mut self, rsp: &str) {
        self.b.emit_line(&format!(
            "  store i32 {}, i32* {}, align 4",
            rsp, self.rsp_ptr
        ));
    }

    fn rpush_i32(&mut self, v: &str) {
        let rsp = self.load_rsp();
        let ptr = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = getelementptr inbounds i32, i32* {}, i32 {}",
            ptr, self.rstack_base, rsp
        ));
        self.b
            .emit_line(&format!("  store i32 {}, i32* {}, align 4", v, ptr));
        let rsp2 = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = add i32 {}, 1", rsp2, rsp));
        self.store_rsp(&rsp2);
    }

    fn rpop_i32(&mut self) -> String {
        let rsp = self.load_rsp();
        let rsp2 = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = sub i32 {}, 1", rsp2, rsp));
        self.store_rsp(&rsp2);
        let ptr = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = getelementptr inbounds i32, i32* {}, i32 {}",
            ptr, self.rstack_base, rsp2
        ));
        let v = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = load i32, i32* {}, align 4", v, ptr));
        v
    }

    fn rpeek_i32(&mut self) -> String {
        let rsp = self.load_rsp();
        let rsp2 = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = sub i32 {}, 1", rsp2, rsp));
        let ptr = self.b.fresh_tmp();
        self.b.emit_line(&format!(
            "  {} = getelementptr inbounds i32, i32* {}, i32 {}",
            ptr, self.rstack_base, rsp2
        ));
        let v = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = load i32, i32* {}, align 4", v, ptr));
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
        self.b
            .emit_line(&format!("  {} = {} i32 {}, {}", r, op, a, b));
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
            self.b
                .emit_line(&format!("  {} = srem i32 {}, {}", r, a, b)); // 0方向
        } else {
            self.b
                .emit_line(&format!("  {} = sdiv i32 {}, {}", r, a, b)); // 0方向
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
        self.b
            .emit_line(&format!("  {} = icmp slt i32 {}, 0", c, a));
        let z = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = zext i1 {} to i32", z, c));
        let neg = self.b.fresh_tmp();
        self.b.emit_line(&format!("  {} = sub i32 0, {}", neg, z));
        self.push_i32(&neg);
    }

    // Control flow sugar (IF/ELSE/THEN, BEGIN/UNTIL, BEGIN/WHILE/REPEAT)
    fn emit_br_cond_zero_to(&mut self, cond_i32: &str, if_zero_lbl: &str, if_nz_lbl: &str) {
        let c = self.b.fresh_tmp();
        self.b
            .emit_line(&format!("  {} = icmp eq i32 {}, 0", c, cond_i32));
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
        let body: String = bytes.into_iter().map(|b| format!("\\{:02X}", b)).collect();

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

    fn call_extern(
        &mut self,
        word: &str,
        arg_mode: ExternArgMode,
        str_arg: Option<String>,
    ) -> Result<(), String> {
        let callee = self
            .externs
            .get(word)
            .cloned()
            .ok_or_else(|| format!("Unknown extern service word: {}", word))?;

        match arg_mode {
            ExternArgMode::PopI32Void => {
                let v = self.pop_i32();
                self.b
                    .emit_line(&format!("  call void @{}(i32 {})", callee, v));
            }
            ExternArgMode::Void => {
                self.b.emit_line(&format!("  call void @{}()", callee));
            }
            ExternArgMode::RetI32Push => {
                let r = self.b.fresh_tmp();
                self.b
                    .emit_line(&format!("  {} = call i32 @{}()", r, callee));
                self.push_i32(&r);
            }
            ExternArgMode::PopI32RetI32Push => {
                let v = self.pop_i32();
                let r = self.b.fresh_tmp();
                self.b
                    .emit_line(&format!("  {} = call i32 @{}(i32 {})", r, callee, v));
                self.push_i32(&r);
            }
            ExternArgMode::StrVoid => {
                let s = str_arg.ok_or("Missing string argument for PWRITE-STR")?;
                let p = self.emit_string_global(&s);
                self.b
                    .emit_line(&format!("  call void @{}(i8* {})", callee, p));
            }
            ExternArgMode::Pop2I32Void => {
                let b = self.pop_i32();
                let a = self.pop_i32();
                self.b
                    .emit_line(&format!("  call void @{}(i32 {}, i32 {})", callee, a, b));
            }
            ExternArgMode::Pop2I32RetI32Push => {
                let b = self.pop_i32();
                let a = self.pop_i32();
                let r = self.b.fresh_tmp();
                self.b.emit_line(&format!(
                    "  {} = call i32 @{}(i32 {}, i32 {})",
                    r, callee, a, b
                ));
                self.push_i32(&r);
            }
            ExternArgMode::Pop3I32Void => {
                let c = self.pop_i32();
                let b = self.pop_i32();
                let a = self.pop_i32();
                self.b.emit_line(&format!(
                    "  call void @{}(i32 {}, i32 {}, i32 {})",
                    callee, a, b, c
                ));
            }
        }
        Ok(())
    }

    fn call_word(&mut self, word: &str) {
        let word = llvm_word_sym(word);
        self.b.emit_line(&format!(
            "  call void @{}(i32* {}, i32* {})",
            word, self.stack_base, self.sp_ptr
        ));
    }

    fn try_emit_native_pascal_routine(&mut self, alias: Option<&str>) -> Result<bool, String> {
        let Some(alias) = alias else {
            return Ok(false);
        };
        let key = match alias {
            "program::abs" => Some(("__KP_FABS_F32", ExternArgMode::PopI32RetI32Push)),
            "program::sqrt" => Some(("__KP_FSQRT_F32", ExternArgMode::PopI32RetI32Push)),
            "program::sin" => Some(("__KP_FSIN_F32", ExternArgMode::PopI32RetI32Push)),
            "program::cos" => Some(("__KP_FCOS_F32", ExternArgMode::PopI32RetI32Push)),
            "program::pow" => Some(("__KP_FPOW_F32_I32", ExternArgMode::Pop2I32RetI32Push)),
            "program::floor" => Some(("__KP_FFLOOR_F32", ExternArgMode::PopI32RetI32Push)),
            "program::ceil" => Some(("__KP_FCEIL_F32", ExternArgMode::PopI32RetI32Push)),

            "program::fx_sqrt" => Some(("__KP_FX_SQRT", ExternArgMode::PopI32RetI32Push)),
            "program::fx_sin" => Some(("__KP_FX_SIN", ExternArgMode::PopI32RetI32Push)),
            "program::fx_cos" => Some(("__KP_FX_COS", ExternArgMode::PopI32RetI32Push)),
            "program::fx_tan" => Some(("__KP_FX_TAN", ExternArgMode::PopI32RetI32Push)),
            "program::fx_asin" => Some(("__KP_FX_ASIN", ExternArgMode::PopI32RetI32Push)),
            "program::fx_acos" => Some(("__KP_FX_ACOS", ExternArgMode::PopI32RetI32Push)),
            "program::fx_atan" => Some(("__KP_FX_ATAN", ExternArgMode::PopI32RetI32Push)),
            "program::fx_ln" => Some(("__KP_FX_LN", ExternArgMode::PopI32RetI32Push)),
            "program::fx_log" => Some(("__KP_FX_LOG", ExternArgMode::PopI32RetI32Push)),
            _ => None,
        };
        if let Some((word, mode)) = key {
            self.call_extern(word, mode, None)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn compile_body(&mut self, toks: &[Tok]) -> Result<(), String> {
        let mut i = 0usize;
        while i < toks.len() {
            match &toks[i] {
                Tok::Num(v) => self.push_i32(&format!("{}", v)),
                Tok::Str(s) => {
                    // Compile-time handling for a few bootstrap-style immediate string consumers.
                    if i + 1 >= toks.len() {
                        return Err("S\" must be followed by a word (e.g., PWRITE-STR)".into());
                    }
                    match &toks[i + 1] {
                        Tok::Word(w) if w == "PWRITE-STR" => {
                            self.call_extern("PWRITE-STR", ExternArgMode::StrVoid, Some(s.clone()))?;
                            i += 1; // consume following word
                        }
                        Tok::Word(w) if w == "READ-F32" || w == "FNUMBER?" => {
                            if let Some(bits) = parse_f32_token_bits(s) {
                                self.push_i32(&bits.to_string());
                                self.push_i32("-1");
                            } else {
                                self.push_i32("0");
                            }
                            i += 1; // consume following word
                        }
                        _ => {
                            return Err(
                                "S\" currently only supported as: S\" ...\" PWRITE-STR / READ-F32 / FNUMBER?"
                                    .into(),
                            )
                        }
                    }
                }
                Tok::Word(w) => {
                    if let Some(v) = self.constant_words.get(w) {
                        self.push_i32(&v.to_string());
                        i += 1;
                        continue;
                    }
                    if let Some(addr) = self.created_words.get(w) {
                        self.push_i32(&addr.to_string());
                        i += 1;
                        continue;
                    }
                    match w.as_str() {
                        // stack ops
                        "DUP" => self.dup(),
                        "DROP" => self.drop(),
                        "SWAP" => {
                            let b = self.pop_i32();
                            let a = self.pop_i32();
                            self.push_i32(&b);
                            self.push_i32(&a);
                        }
                        "OVER" => {
                            let b = self.pop_i32();
                            let a = self.pop_i32();
                            self.push_i32(&a);
                            self.push_i32(&b);
                            self.push_i32(&a);
                        }
                        ">R" => {
                            let v = self.pop_i32();
                            self.rpush_i32(&v);
                        }
                        "R>" => {
                            let v = self.rpop_i32();
                            self.push_i32(&v);
                        }
                        "R@" => {
                            let v = self.rpeek_i32();
                            self.push_i32(&v);
                        }

                        // arithmetic / logic (wrap semantics by default)
                        "+" => self.binop("add"),
                        "-" => self.binop("sub"),
                        "*" => self.binop("mul"),
                        "/" => self.div_mod(false),
                        "MOD" => self.div_mod(true),

                        "NEGATE" => self.unary_negate(),
                        "AND" => self.and(),
                        "OR" => self.binop("or"),
                        "XOR" => self.binop("xor"),
                        "LSHIFT" => {
                            let b = self.pop_i32();
                            let a = self.pop_i32();
                            let sh = self.b.fresh_tmp();
                            self.b.emit_line(&format!("  {} = and i32 {}, 31", sh, b));
                            let r = self.b.fresh_tmp();
                            self.b
                                .emit_line(&format!("  {} = shl i32 {}, {}", r, a, sh));
                            self.push_i32(&r);
                        }
                        "RSHIFT" => {
                            let b = self.pop_i32();
                            let a = self.pop_i32();
                            let sh = self.b.fresh_tmp();
                            self.b.emit_line(&format!("  {} = and i32 {}, 31", sh, b));
                            let r = self.b.fresh_tmp();
                            self.b
                                .emit_line(&format!("  {} = lshr i32 {}, {}", r, a, sh));
                            self.push_i32(&r);
                        }
                        "/MOD" => {
                            let b = self.pop_i32();
                            let a = self.pop_i32();
                            let rem = self.b.fresh_tmp();
                            let quo = self.b.fresh_tmp();
                            self.b
                                .emit_line(&format!("  {} = srem i32 {}, {}", rem, a, b));
                            self.b
                                .emit_line(&format!("  {} = sdiv i32 {}, {}", quo, a, b));
                            // Forth: remainder quotient
                            self.push_i32(&rem);
                            self.push_i32(&quo);
                        }

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
                        "PWRITE-I32" => {
                            self.call_extern("PWRITE-I32", ExternArgMode::PopI32Void, None)?
                        }
                        "." => self.call_extern("PWRITE-I32", ExternArgMode::PopI32Void, None)?,
                        "PWRITE-BOOL" => {
                            self.call_extern("PWRITE-BOOL", ExternArgMode::PopI32Void, None)?
                        }
                        "PWRITE-CHAR" => {
                            self.call_extern("PWRITE-CHAR", ExternArgMode::PopI32Void, None)?
                        }
                        "EMIT" => {
                            self.call_extern("PWRITE-CHAR", ExternArgMode::PopI32Void, None)?
                        }
                        "PWRITELN" => self.call_extern("PWRITELN", ExternArgMode::Void, None)?,
                        "PWRITE-HEX" => {
                            self.call_extern("PWRITE-HEX", ExternArgMode::PopI32Void, None)?
                        }

                        "PREAD-I32" => {
                            self.call_extern("PREAD-I32", ExternArgMode::RetI32Push, None)?
                        }
                        "PREAD-BOOL" => {
                            self.call_extern("PREAD-BOOL", ExternArgMode::RetI32Push, None)?
                        }
                        "PREAD-CHAR" => {
                            self.call_extern("PREAD-CHAR", ExternArgMode::RetI32Push, None)?
                        }
                        "PREADLN" => self.call_extern("PREADLN", ExternArgMode::Void, None)?,

                        "PBOOL" => {
                            self.call_extern("PBOOL", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "PVAR!" => self.call_extern("PVAR!", ExternArgMode::Pop2I32Void, None)?,
                        "PVAR@" => {
                            self.call_extern("PVAR@", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "PFIELD!" => {
                            self.call_extern("PFIELD!", ExternArgMode::Pop3I32Void, None)?
                        }
                        "PFIELD@" => {
                            self.call_extern("PFIELD@", ExternArgMode::Pop2I32RetI32Push, None)?
                        }

                        // Float32-on-cell words from bootstrap treated as primitives.
                        "PREAD-F32" => {
                            self.call_extern("PREAD-F32", ExternArgMode::RetI32Push, None)?
                        }
                        "FADD" => {
                            self.call_extern("FADD", ExternArgMode::Pop2I32RetI32Push, None)?
                        }
                        "FSUB" => {
                            self.call_extern("FSUB", ExternArgMode::Pop2I32RetI32Push, None)?
                        }
                        "FMUL" => {
                            self.call_extern("FMUL", ExternArgMode::Pop2I32RetI32Push, None)?
                        }
                        "FDIV" => {
                            self.call_extern("FDIV", ExternArgMode::Pop2I32RetI32Push, None)?
                        }
                        "FNEGATE" => {
                            self.call_extern("FNEGATE", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "FABS" => {
                            self.call_extern("FABS", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "F=" => self.call_extern("F=", ExternArgMode::Pop2I32RetI32Push, None)?,
                        "F<" => self.call_extern("F<", ExternArgMode::Pop2I32RetI32Push, None)?,
                        "F<=" => self.call_extern("F<=", ExternArgMode::Pop2I32RetI32Push, None)?,
                        "FZERO?" | "F0=" => {
                            self.call_extern("FZERO?", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "FINF?" => {
                            self.call_extern("FINF?", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "FNAN?" => {
                            self.call_extern("FNAN?", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "FFINITE?" => {
                            self.call_extern("FFINITE?", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "S>F" => self.call_extern("S>F", ExternArgMode::PopI32RetI32Push, None)?,
                        "F>S" => self.call_extern("F>S", ExternArgMode::PopI32RetI32Push, None)?,
                        "Q16.16>F" => {
                            self.call_extern("Q16.16>F", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "F>Q16.16" => {
                            self.call_extern("F>Q16.16", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "FROUND-I32" => {
                            self.call_extern("FROUND-I32", ExternArgMode::PopI32RetI32Push, None)?
                        }
                        "F." | "WRITE-F32" | "PWRITE-F32" => {
                            self.call_extern("PWRITE-F32", ExternArgMode::PopI32Void, None)?
                        }
                        "F+INF" => self.push_i32(&(f32::INFINITY.to_bits() as i32).to_string()),
                        "F-INF" => self.push_i32(&(f32::NEG_INFINITY.to_bits() as i32).to_string()),
                        "FNAN" => self.push_i32(&(f32::NAN.to_bits() as i32).to_string()),

                        // Minimal compile-time dictionary words used by generated IL.
                        "CONSTANT" => {
                            let val = resolve_prev_compile_time_value(
                                toks,
                                i,
                                self.here,
                                &self.constant_words,
                                &self.created_words,
                            )
                            .ok_or_else(|| {
                                "CONSTANT currently requires a compile-time value before it"
                                    .to_string()
                            })?;
                            let _ = self.pop_i32();
                            let name = match toks.get(i + 1) {
                                Some(Tok::Word(name)) => name.clone(),
                                _ => return Err("CONSTANT requires a following name".into()),
                            };
                            self.constant_words.insert(name, val);
                            i += 1; // consume name
                        }
                        "CREATE" => {
                            let name = match toks.get(i + 1) {
                                Some(Tok::Word(name)) => name.clone(),
                                _ => return Err("CREATE requires a following name".into()),
                            };
                            self.created_words.insert(name, self.here);
                            i += 1; // consume name
                        }
                        "HERE" => {
                            self.call_extern("HERE", ExternArgMode::RetI32Push, None)?
                        }
                        "," => {
                            // Forth comma allocates one 32-bit cell (4 bytes).
                            let _ = self.pop_i32();
                            self.here = self.here.wrapping_add(4);
                        }
                        "ALLOT" => {
                            self.call_extern("ALLOT", ExternArgMode::PopI32Void, None)?
                        }

                        _ if self.known_defs.contains(w) => self.call_word(w),
                        _ => return Err(format!("Unknown word: {}", w)),
                    }
                }
                Tok::Colon | Tok::Semi => {
                    return Err(
                        "Unexpected ':' or ';' inside body (top-level parser should split defs)"
                            .into(),
                    )
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
    Pop2I32Void,
    Pop2I32RetI32Push,
    Pop3I32Void,
}

struct ParsedProgram {
    defs: Vec<(String, Vec<Tok>)>,
    created_words: HashMap<String, i32>,
    constant_words: HashMap<String, i32>,
    here: i32,
    entry_call: Option<String>,
}

fn parse_program(toks: &[Tok]) -> Result<ParsedProgram, String> {
    let mut defs = Vec::new();
    let mut created_words = HashMap::new();
    let mut constant_words = HashMap::new();
    let mut here: i32 = 0;
    let mut entry_call: Option<String> = None;
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
            Tok::Word(w) if w == "CREATE" => {
                let name = match toks.get(i + 1) {
                    Some(Tok::Word(name)) => name.clone(),
                    _ => return Err("CREATE requires a following name at top-level".into()),
                };
                created_words.insert(name, here);
                i += 2;
            }
            Tok::Word(w) if w == "VARIABLE" => {
                let name = match toks.get(i + 1) {
                    Some(Tok::Word(name)) => name.clone(),
                    _ => return Err("VARIABLE requires a following name at top-level".into()),
                };
                created_words.insert(name, here);
                here = here.wrapping_add(4);
                i += 2;
            }
            Tok::Word(w) if w == "," => {
                here = here.wrapping_add(4);
                i += 1;
            }
            Tok::Word(w) if w == "HERE" => {
                i += 1;
            }
            Tok::Word(w) if w == "ALLOT" => {
                let n =
                    resolve_prev_compile_time_value(toks, i, here, &constant_words, &created_words)
                        .ok_or_else(|| {
                            "Top-level ALLOT requires a compile-time value before it".to_string()
                        })?;
                here = here.wrapping_add(n);
                i += 1;
            }
            Tok::Word(w) if w == "CONSTANT" => {
                let val =
                    resolve_prev_compile_time_value(toks, i, here, &constant_words, &created_words)
                        .ok_or_else(|| {
                            "Top-level CONSTANT requires a compile-time value before it".to_string()
                        })?;
                let name = match toks.get(i + 1) {
                    Some(Tok::Word(name)) => name.clone(),
                    _ => return Err("CONSTANT requires a following name at top-level".into()),
                };
                constant_words.insert(name, val);
                i += 2;
            }
            Tok::Word(w) => {
                // kpascal output usually ends with `MAIN` invocation.
                entry_call = Some(w.clone());
                i += 1;
            }
            Tok::Num(_) | Tok::Str(_) | Tok::Semi => {
                i += 1;
            }
        }
    }

    Ok(ParsedProgram {
        defs,
        created_words,
        constant_words,
        here,
        entry_call,
    })
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!("Usage: {} <input.fth> <output.ll>", args[0]));
    }
    let input = fs::read_to_string(&args[1]).map_err(|e| format!("Read error: {}", e))?;
    let routine_aliases = extract_routine_aliases(&input);
    let toks = tokenize(&input)?;
    let parsed = parse_program(&toks)?;
    let defs = parsed.defs;
    let mut known_defs = HashSet::new();
    for (name, _) in &defs {
        known_defs.insert(name.clone());
    }

    let mut cg = Codegen::new();
    cg.emit_prelude();
    cg.set_program_symbols(
        parsed.created_words,
        parsed.constant_words,
        known_defs,
        parsed.here,
    );

    // Compile all defs
    for (name, body) in &defs {
        cg.begin_func(name);
        let alias = routine_aliases.get(name).map(|s| s.as_str());
        if !cg.try_emit_native_pascal_routine(alias)? {
            cg.compile_body(body)?;
        }
        cg.end_func();
    }

    // If there is a word named MAIN, create @main wrapper calling it.
    // Otherwise, if exactly one def exists, call it.
    let entry = if let Some(entry) = parsed.entry_call {
        entry
    } else if defs.iter().any(|(n, _)| n == "MAIN") {
        "MAIN".to_string()
    } else if defs.len() == 1 {
        defs[0].0.clone()
    } else {
        return Err(
            "No entry point. Define : MAIN ... ; or provide exactly one definition.".into(),
        );
    };
    cg.emit_main_wrapper(&entry);

    let module = format!("{}\n{}", cg.b.out, cg.b.globals);
    fs::write(&args[2], module).map_err(|e| format!("Write error: {}", e))?;
    Ok(())
}
