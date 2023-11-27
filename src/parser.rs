use std::{fmt::Debug, collections::{HashSet, HashMap}};

use crate::lexer::{Token, TokenType};

#[derive(Debug)]
enum BinaryOperatorType {
    Plus,
    Minus,
    Times,
    LessThan,
    GreaterThan,
    Equals,
    NotEquals,
}

#[derive(Debug)]
enum UnaryOperatorType {
    Negation,
    Deref,
}

trait Statement: Debug {
    fn codegen(&self, state: &mut CodeGenState);
}
trait Expression: Debug {
    fn codegen(&self, state: &mut CodeGenState);
    //fn static_eval(&self) -> Option<usize>;
}


#[derive(Debug, Clone)]
struct MemoryAllocation {
    name: String,
    start: usize,
    end: usize,
    scope: usize,
}

impl MemoryAllocation {
    fn has(&self, n: usize) -> bool { n >= self.start && n <= self.end }
}

struct MemoryTracker {
    allocations: Vec<MemoryAllocation>,
}

impl MemoryTracker {
    fn new() -> MemoryTracker { return MemoryTracker { allocations: Vec::new() }; }

    fn is_addr_occupied(&self, addr: usize) -> bool {
        for m in &self.allocations {
            if m.has(addr) {
                return true;
            }
        }
        return false;
    }

    fn get_free(&self, len: usize) -> usize {
        let mut addr: usize = usize::MAX;
        'outer: loop {
            addr = addr.wrapping_add(1);

            if len == 1 {
                if self.is_addr_occupied(addr) {
                    continue;
                }
                return addr;
            } else {
                for i in 0..len {
                    if self.is_addr_occupied(addr + i) {
                        continue 'outer;
                    }
                }
                return addr;
            }
        }
    }

    fn alloc(&mut self, name: &String) {
        let addr = self.get_free(1);
        self.allocations.push(MemoryAllocation { name: name.clone(), start: addr, end: addr, scope: 0 });
    }

    fn alloc_temp(&mut self) -> usize {
        let addr = self.get_free(1);
        self.allocations.push(MemoryAllocation { name: "".to_string(), start: addr, end: addr, scope: 0 });
        return addr;
    }

    fn alloc_overlay(&mut self, name: &String, addr: usize) {
        self.allocations.push(MemoryAllocation { name: name.clone(), start: addr, end: addr, scope: 0 });
    }

    fn alloc_array(&mut self, name: &String, len: usize) {
        let addr = self.get_free(len);
        self.allocations.push(MemoryAllocation { name: name.clone(), start: addr, end: addr + len - 1, scope: 0 });
    }

    fn alloc_temp_array(&mut self, len: usize) -> usize {
        let addr = self.get_free(len);
        self.allocations.push(MemoryAllocation { name: "".to_string(), start: addr, end: addr + len - 1, scope: 0 });
        return addr;
    }

    fn dealloc_name(&mut self, name: &String) {
        let mut i = 0;
        for m in &self.allocations {
            if &m.name == name {
                break;
            }
            i += 1;
        }
        self.allocations.remove(i);
    }

    fn dealloc_addr(&mut self, addr: usize) {
        let mut i = 0;
        for m in &self.allocations {
            if m.start == addr {
                break;
            }
            i += 1;
        }
        self.allocations.remove(i);
    }

    fn get(&self, name: &String) -> usize {
        for m in &self.allocations {
            if &m.name == name {
                return m.start;
            }
        }

        panic!("Variable not found: \"{}\"!", name);
    }

    fn inc_scope(&mut self) {
        for m in &mut self.allocations {
            m.scope += 1;
        }
    }

    fn dec_scope(&mut self) {
        let mut del: Vec<String> = Vec::new();

        for m in &mut self.allocations {
            if m.scope == 0 {
                del.push(m.name.clone());
            } else {
                m.scope -= 1;
            }
        }

        for x in del {
            self.dealloc_name(&x);
        }
    }
}

struct FuncSign {
    nargs: usize,
    aargs: usize,
}

struct CodeGenState {
    tracker: MemoryTracker,
    code: String,
    labels: HashSet<u16>,
    functions: HashMap<String, FuncSign>,
    curr_func: String,
}

impl CodeGenState {
    fn new() -> CodeGenState { return CodeGenState {
        tracker: MemoryTracker::new(),
        code: String::new(),
        labels: HashSet::new(),
        functions: HashMap::new(),
        curr_func: String::new(),
    }; }

    fn add(&mut self, t: &str) {
        self.code.push_str(t);
        self.code.push('\n');
    }
    fn adds(&mut self, t: String) {
        self.add(t.as_str());
    }

    fn gen_label(&mut self) -> String {
        let mut num = rand::random::<u16>();
        while self.labels.contains(&num) {
            num = rand::random::<u16>();
        }
        self.labels.insert(num);
        return format!("label_{}", num);
    }
}


#[derive(Debug)]
struct BlockStatement {
    statements: Vec<Box<dyn Statement>>,
}

impl Statement for BlockStatement {
    fn codegen(&self, state: &mut CodeGenState) {
        state.tracker.inc_scope();
        for s in &self.statements {
            s.codegen(state);
        }
        state.tracker.dec_scope();
    }
}


#[derive(Debug)]
struct VarDeclaration {
    varname: String
}

impl Statement for VarDeclaration {
    fn codegen(&self, state: &mut CodeGenState) {
        state.tracker.alloc(&self.varname);
        //state.adds(format!("# VarDeclaration \"{}\"", &self.varname));
    }
}


#[derive(Debug)]
struct VarAssignment {
    varname: String,
    value: Box<dyn Expression>,
}

impl Statement for VarAssignment {
    fn codegen(&self, state: &mut CodeGenState) {
        //state.add("# VarAssignment");
        self.value.codegen(state);
        state.adds(format!("sta {}", state.tracker.get(&self.varname)));
    }
}


#[derive(Debug)]
struct DerefAssignment {
    addr: Box<dyn Expression>,
    value: Box<dyn Expression>,
}

impl Statement for DerefAssignment {
    fn codegen(&self, state: &mut CodeGenState) {
        // Address
        self.addr.codegen(state);
        let addr_addr = state.tracker.alloc_temp();
        state.adds(format!("sta {}", addr_addr));

        // Value
        self.value.codegen(state);
        state.adds(format!("stad {}", addr_addr));

        state.tracker.dealloc_addr(addr_addr);
    }
}


#[derive(Debug)]
struct ArrDeclaration {
    arrname: String,
    arrlen: usize,
}

impl Statement for ArrDeclaration {
    fn codegen(&self, state: &mut CodeGenState) {
        state.tracker.alloc_array(&self.arrname, self.arrlen);
        //return format!("# ArrDeclaration \"{}\" len: {}\n", &self.arrname, &self.arrlen);
    }
}


#[derive(Debug)]
struct ArrAssignment {
    arrname: String,
    index: Box<dyn Expression>,
    value: Box<dyn Expression>,
}

impl Statement for ArrAssignment {
    fn codegen(&self, state: &mut CodeGenState) {
        // Store value
        self.value.codegen(state);
        let value_addr = state.tracker.alloc_temp();
        state.adds(format!("sta {}", value_addr));

        // Store index
        self.index.codegen(state);
        let index_addr = state.tracker.alloc_temp();
        state.adds(format!("sta {}", index_addr));
        // Add index to address of array
        state.adds(format!("mka {}", state.tracker.get(&self.arrname)));
        state.adds(format!("add {}", index_addr));
        state.adds(format!("sta {}", index_addr));

        // Load and store value at index
        state.adds(format!("lda {}", value_addr));
        state.adds(format!("stad {}", index_addr));

        state.tracker.dealloc_addr(value_addr);
        state.tracker.dealloc_addr(index_addr);
    }
}


#[derive(Debug)]
struct BinaryOperator {
    operator: BinaryOperatorType,
    lhs: Box<dyn Expression>,
    rhs: Box<dyn Expression>,
}

impl Expression for BinaryOperator {
    fn codegen(&self, state: &mut CodeGenState) {
        //state.adds(format!("BinaryOperation: {:?}", self.operator));

        let addr_a = state.tracker.alloc_temp();
        let addr_b = state.tracker.alloc_temp();

        self.lhs.codegen(state);
        state.adds(format!("sta {}", addr_a));
        self.rhs.codegen(state);
        state.adds(format!("sta {}", addr_b));

        match self.operator {
            BinaryOperatorType::Plus => {
                state.adds(format!("lda {}", addr_a));
                state.adds(format!("add {}", addr_b));
            },
            BinaryOperatorType::Minus => {
                state.adds(format!("lda {}", addr_b));
                state.add("neg");
                state.adds(format!("sta {}", addr_b));

                state.adds(format!("lda {}", addr_a));
                state.adds(format!("add {}", addr_b));
            },
            BinaryOperatorType::Times => {
                let addr_result = state.tracker.alloc_temp();
                state.add("mka 0");
                state.adds(format!("sta {}", addr_result));

                let loop_label = state.gen_label();
                state.adds(format!("label {}", loop_label));

                state.add("mka -1");
                state.adds(format!("add {}", addr_a));
                state.adds(format!("sta {}", addr_a));

                state.adds(format!("lda {}", addr_result));
                state.adds(format!("add {}", addr_b));
                state.adds(format!("sta {}", addr_result));

                state.adds(format!("lda {}", addr_a));
                let end_label = state.gen_label();
                state.adds(format!("jz {}", end_label));
                state.adds(format!("jmp {}", loop_label));
                state.adds(format!("label {}", end_label));
                state.adds(format!("lda {}", addr_result));

                state.tracker.dealloc_addr(addr_result);
            },
            BinaryOperatorType::Equals => {
                state.adds(format!("lda {}", addr_a));
                state.add("neg");
                state.adds(format!("add {}", addr_b));

                let zero_label = state.gen_label();
                state.adds(format!("jz {}", zero_label));
                
                state.add("mka 0");
                let finish_label = state.gen_label();
                state.adds(format!("jmp {}", finish_label));

                state.adds(format!("label {}", zero_label));
                state.add("mka 1");

                state.adds(format!("label {}", finish_label));
            },
            BinaryOperatorType::NotEquals => {
                // Equals, nur invertiert
                state.adds(format!("lda {}", addr_a));
                state.add("neg");
                state.adds(format!("add {}", addr_b));

                let zero_label = state.gen_label();
                state.adds(format!("jz {}", zero_label));
                
                state.add("mka 1");
                let finish_label = state.gen_label();
                state.adds(format!("jmp {}", finish_label));

                state.adds(format!("label {}", zero_label));
                state.add("mka 0");

                state.adds(format!("label {}", finish_label));
            },
            BinaryOperatorType::GreaterThan => {
                // a - b
                state.adds(format!("lda {}", addr_b));
                state.add("neg");
                state.adds(format!("sta {}", addr_b));

                state.adds(format!("lda {}", addr_a));
                state.adds(format!("add {}", addr_b));

                let l_a = state.gen_label();
                let l_b = state.gen_label();
                let l_end = state.gen_label();

                state.adds(format!("jz {}", l_b));
                state.adds(format!("jp {}", l_a));

                state.adds(format!("label {}", l_b));

                state.add("mka 0");
                state.adds(format!("jmp {}", l_end));

                state.adds(format!("label {}", l_a));
                state.add("mka 1");

                state.adds(format!("label {}", l_end));
            },
            BinaryOperatorType::LessThan => {
                // a - b
                state.adds(format!("lda {}", addr_b));
                state.add("neg");
                state.adds(format!("sta {}", addr_b));

                state.adds(format!("lda {}", addr_a));
                state.adds(format!("add {}", addr_b));

                let l_a = state.gen_label();
                let l_end = state.gen_label();

                state.adds(format!("jn {}", l_a));

                state.add("mka 0");
                state.adds(format!("jmp {}", l_end));

                state.adds(format!("label {}", l_a));
                state.add("mka 1");

                state.adds(format!("label {}", l_end));
            },
        }

        state.tracker.dealloc_addr(addr_a);
        state.tracker.dealloc_addr(addr_b);
    }
}


#[derive(Debug)]
struct UnaryOperator {
    operator: UnaryOperatorType,
    val: Box<dyn Expression>,
}

impl Expression for UnaryOperator {
    fn codegen(&self, state: &mut CodeGenState) {
        let addr = state.tracker.alloc_temp();
        self.val.codegen(state);
        state.adds(format!("sta {}", addr));

        match self.operator {
            UnaryOperatorType::Negation => {
                state.adds(format!("lda {}", addr));
                state.add("neg");
            },
            UnaryOperatorType::Deref => {
                state.adds(format!("ldad {}", addr));
            },
        }

        state.tracker.dealloc_addr(addr);
    }
}


#[derive(Debug)]
struct Number {
    num: isize,
}

impl Expression for Number {
    fn codegen(&self, state: &mut CodeGenState) {
        //state.adds(format!("# Number {}", self.num));
        state.adds(format!("mka {}", self.num));
    }
}


#[derive(Debug)]
struct Variable {
    varname: String,
}

impl Expression for Variable {
    fn codegen(&self, state: &mut CodeGenState) {
        let val = state.tracker.get(&self.varname);
        state.adds(format!("lda {}", val));
    }
}


#[derive(Debug)]
struct Array {
    arrname: String,
    index: Box<dyn Expression>,
}

impl Expression for Array {
    fn codegen(&self, state: &mut CodeGenState) {
        state.adds(format!("mka {}", state.tracker.get(&self.arrname)));
        let tmp = state.tracker.alloc_temp();
        state.adds(format!("sta {}", tmp));
        
        self.index.codegen(state);

        state.adds(format!("add {}", tmp));
        state.adds(format!("sta {}", tmp));
        state.adds(format!("ldad {}", tmp));

        state.tracker.dealloc_addr(tmp);
    }
}


#[derive(Debug)]
struct IfStatement {
    condition: Box<dyn Expression>,
    block: BlockStatement,
}

impl Statement for IfStatement {
    fn codegen(&self, state: &mut CodeGenState) {
        self.condition.codegen(state);

        let label = state.gen_label();
        state.adds(format!("jz {}", label));

        self.block.codegen(state);

        state.adds(format!("label {}", label));
    }
}


#[derive(Debug)]
struct WhileLoop {
    condition: Box<dyn Expression>,
    block: BlockStatement,
}

impl Statement for WhileLoop {
    fn codegen(&self, state: &mut CodeGenState) {
        let start_label = state.gen_label();
        state.adds(format!("label {}", start_label));

        self.condition.codegen(state);

        let end_label = state.gen_label();
        state.adds(format!("jz {}", end_label));

        self.block.codegen(state);

        state.adds(format!("jmp {}", start_label));
        state.adds(format!("label {}", end_label));
    }
}


#[derive(Debug)]
struct FunctionDeclaration {
    name: String,
    body: BlockStatement,
    param_names: Vec<String>,
}

impl Statement for FunctionDeclaration {
    fn codegen(&self, state: &mut CodeGenState) {
        state.curr_func = self.name.clone();

        let skip_label = state.gen_label();
        state.adds(format!("jmp {}", skip_label));
        state.adds(format!("label func_{}", self.name));

        if self.param_names.len() != 0 {
            let fargs_addr = state.tracker.alloc_temp_array(self.param_names.len());
            state.functions.insert(self.name.clone(), FuncSign { nargs: self.param_names.len(), aargs: fargs_addr });

            // Alloc all the names
            let mut a = fargs_addr;
            for pname in &self.param_names {
                state.tracker.alloc_overlay(pname, a);
                a += 1;
            }
        } else {
            state.functions.insert(self.name.clone(), FuncSign { nargs: 0, aargs: 0 });
        }

        // Code
        self.body.codegen(state);

        state.add("ret");

        state.adds(format!("label {}", skip_label));

        // Dealloc all the names
        for p in self.param_names.clone() {
            state.tracker.dealloc_name(&p);
        }

        state.curr_func = String::new();
    }
}


#[derive(Debug)]
struct FunctionCall {
    name: String,
    params: Vec<Box<dyn Expression>>,
}

impl Expression for FunctionCall {
    fn codegen(&self, state: &mut CodeGenState) {
        let fun_sign = match state.functions.get(&self.name) {
            Some(val) => val,
            None => {panic!("Function {} not found!", self.name)},
        };

        if &self.name == &state.curr_func {
            panic!("No recusion allowed!");
        }

        if fun_sign.nargs != self.params.len() {
            panic!("Invalid number of arguments!");
        }

        let mut a = fun_sign.aargs;
        for p in &self.params {
            p.codegen(state);
            state.adds(format!("sta {}", a));
            a += 1;
        }

        state.adds(format!("call func_{}", self.name));
    }
}


#[derive(Debug)]
struct ReturnStatement {
    value: Box<dyn Expression>,
}

impl Statement for ReturnStatement {
    fn codegen(&self, state: &mut CodeGenState) {
        self.value.codegen(state);
        state.add("ret");
    }
}


#[derive(Debug)]
struct AddrOf {
    varname: String,
}

impl Expression for AddrOf {
    fn codegen(&self, state: &mut CodeGenState) {
        let addr = state.tracker.get(&self.varname);
        state.adds(format!("mka {}", addr));
    }
}


#[derive(Debug)]
struct NOPStatement {}
impl Statement for NOPStatement { fn codegen(&self, _state: &mut CodeGenState) {} }


#[derive(Debug)]
pub struct AST {
    nodes: BlockStatement,
}

impl AST {
    pub fn codegen(&self) -> String {
        let mut state = CodeGenState::new();
        self.nodes.codegen(&mut state);
        return state.code;
    }
}

// ========== PARSER ==========

struct ParserState {
    tokenlist: Vec<Token>,
    i: usize,
}
impl ParserState {
    fn curr(&self) -> Token { return self.tokenlist[self.i].clone(); }
    fn next(&self) -> Token { return self.tokenlist[self.i + 1].clone(); }
    //fn prev(&self) -> Token { return self.tokenlist[self.i - 1].clone(); }
    //fn offset(&self, offset: usize) -> Token { return self.tokenlist[self.i + offset].clone(); }
    fn advance_newlines(&mut self) {
        while self.curr().ttype == TokenType::Newline {
            self.i += 1;
        }
    }
    fn expect_token(&self, ttype: TokenType, val: &str) {
        if !self.curr().equals(ttype, val) {
            panic!("Expected \'{}\', but found \'{}\' at {}", val, self.curr().value, self.curr().position());
        }
    }
    fn expect_token_type(&self, ttype: TokenType) {
        if self.curr().ttype != ttype {
            panic!("Expected {:?}, but found \'{}\' at {}", ttype, self.curr().value, self.curr().position());
        }
    }
}

fn parse_atom(state: &mut ParserState) -> Box<dyn Expression> {
    // Number
    if state.curr().ttype == TokenType::Number {
        let n = state.curr().value.parse::<isize>().unwrap();
        state.i += 1;
        return Box::new(Number{num: n});
    }

    if state.curr().ttype == TokenType::Identifier {
        // Array
        if state.next().equals(TokenType::Parenthesis, "[") {
            let aname = state.curr().value;
            state.i += 2;
            let index = parse_expression(state);
            state.expect_token(TokenType::Parenthesis, "]");
            state.i += 1;

            return Box::new(Array{arrname: aname, index: index});
        }

        // FunctionCall
        else if state.next().equals(TokenType::Parenthesis, "(") {
            let mut params: Vec<Box<dyn Expression>> = Vec::new();

            let fname = state.curr().value;
            state.i += 2;

            while !state.curr().equals(TokenType::Parenthesis, ")") {
                params.push(parse_expression(state));
                
                if state.curr().ttype == TokenType::Comma {
                    state.i += 1;
                }
            }
            state.i += 1;

            return Box::new(FunctionCall{ name: fname, params: params });
        }

        // Variable
        else {
            let vname = state.curr().value;
            state.i += 1;
            return Box::new(Variable{varname: vname});
        }
    }

    // Address of
    if state.curr().equals(TokenType::Keyword, "addr") {
        state.i += 1;
        state.expect_token_type(TokenType::Identifier);
        let varname = state.curr().value.clone();
        state.i += 1;
        return Box::new(AddrOf{varname: varname});
    }

    if state.curr().equals(TokenType::Keyword, "deref") {
        state.i += 1;
        let val = parse_atom(state);
        return Box::new(UnaryOperator{operator: UnaryOperatorType::Deref, val: val});
    }

    // (Expresssion)
    state.expect_token(TokenType::Parenthesis, "(");
    state.i += 1;
    let a = parse_expression(state);
    state.expect_token(TokenType::Parenthesis, "(");
    state.i += 1;
    return a;
}

fn parse_negation(state: &mut ParserState) -> Box<dyn Expression> {
    if state.curr().equals(TokenType::Operator, "-") {
        state.i += 1;
        let a = parse_atom(state);
        return Box::new(UnaryOperator{operator: UnaryOperatorType::Negation, val: a});
    }
    return parse_atom(state);
}

fn parse_multiplication(state: &mut ParserState) -> Box<dyn Expression> {
    let a = parse_negation(state);

    if state.curr().equals(TokenType::Operator, "*") {
        state.i += 1;
        let b = parse_multiplication(state);
        return Box::new(BinaryOperator{operator: BinaryOperatorType::Times, lhs: a, rhs: b});
    }

    return a;
}

fn parse_addition(state: &mut ParserState) -> Box<dyn Expression> {
    let a = parse_multiplication(state);

    if state.curr().equals(TokenType::Operator, "+") {
        state.i += 1;
        let b = parse_addition(state);
        return Box::new(BinaryOperator{operator: BinaryOperatorType::Plus, lhs: a, rhs: b});
    }
    if state.curr().equals(TokenType::Operator, "-") {
        state.i += 1;
        let b = parse_addition(state);
        return Box::new(BinaryOperator{operator: BinaryOperatorType::Minus, lhs: a, rhs: b});
    }

    return a;
}

fn parse_comparision(state: &mut ParserState) -> Box<dyn Expression> {
    let a = parse_addition(state);

    if state.curr().ttype == TokenType::Operator && vec!["==", "!=", "<", ">"].contains(&state.curr().value.as_str()) {
        let op = match state.curr().value.as_str() {
            "==" => BinaryOperatorType::Equals,
            "!=" => BinaryOperatorType::NotEquals,
            ">" => BinaryOperatorType::GreaterThan,
            "<" => BinaryOperatorType::LessThan,
            _ => BinaryOperatorType::Equals,
        };

        state.i += 1;
        let b = parse_addition(state);
        return Box::new(BinaryOperator{operator: op, lhs: a, rhs: b});
    }

    return a;
}

fn parse_expression(state: &mut ParserState) -> Box<dyn Expression> {
    return parse_comparision(state);
}

fn parse_statement(state: &mut ParserState) -> Box<dyn Statement> {
    // EOF funkioniert, könnte sich aber ändern
    if state.next().ttype == TokenType::EOF {
        return Box::new(NOPStatement{});
    }

    // BlockStatement
    if state.curr().equals(TokenType::Parenthesis, "{") {
        return Box::new(parse_blockstatement(state));
    }

    // VarDeclaration
    if state.curr().ttype == TokenType::Keyword && state.curr().value == "var" {
        state.i += 1;
        state.expect_token_type(TokenType::Identifier);
        let varname = state.curr().value;
        state.i += 1;
        return Box::new(VarDeclaration{varname: varname});
    }

    // VarAssignment
    if state.curr().ttype == TokenType::Identifier && state.next().ttype == TokenType::Equals {
        let varname = state.curr().value;
        state.i += 2;
        let value = parse_expression(state);
        return Box::new(VarAssignment{varname: varname, value: value});
    }

    // FuncDeclaration
    if state.curr().equals(TokenType::Keyword, "func") {
        let mut parm_names: Vec<String> = Vec::new();

        state.i += 1;
        state.expect_token_type(TokenType::Identifier);
        let fname = state.curr().value;
        state.i += 1;
        state.expect_token(TokenType::Parenthesis, "(");

        state.i += 1;
        while state.curr().ttype == TokenType::Identifier {
            parm_names.push(state.curr().value);
            state.i += 1;

            if state.curr().ttype == TokenType::Comma {
                state.i += 1;
            }
        }
        state.i += 1;

        let bs = parse_blockstatement(state);
        return Box::new(FunctionDeclaration{name: fname, body: bs, param_names: parm_names});
    }

    // IfStatement
    if state.curr().equals(TokenType::Keyword, "if") {
        state.i += 1;
        let condition = parse_expression(state);
        let bs = parse_blockstatement(state);
        return Box::new(IfStatement{condition: condition, block: bs});
    }

    // WhileLoop
    if state.curr().equals(TokenType::Keyword, "while") {
        state.i += 1;
        let condition = parse_expression(state);
        let bs = parse_blockstatement(state);
        return Box::new(WhileLoop{condition: condition, block: bs});
    }

    // ArrDeclaration
    if state.curr().equals(TokenType::Keyword, "arr") {
        state.i += 1;
        state.expect_token_type(TokenType::Identifier);
        let aname = state.curr().value;
        state.i += 1;
        state.expect_token(TokenType::Parenthesis, "[");
        state.i += 1;
        let alen = state.curr().value.parse::<usize>().unwrap();
        state.i += 1;
        state.expect_token(TokenType::Parenthesis, "]");
        state.i += 1;
        state.advance_newlines();
        return Box::new(ArrDeclaration{arrname: aname, arrlen: alen});
    }

    // ArrAssignment
    if state.curr().ttype == TokenType::Identifier && state.next().equals(TokenType::Parenthesis, "[") {
        let aname = state.curr().value;
        state.i += 2;
        let index = parse_expression(state);
        state.expect_token(TokenType::Parenthesis, "]");
        state.i += 1;
        state.expect_token_type(TokenType::Equals);
        state.i += 1;
        let value = parse_expression(state);
        state.advance_newlines();
        return Box::new(ArrAssignment{arrname: aname, index: index, value: value});
    }

    // ReturnStatement
    if state.curr().equals(TokenType::Keyword, "return") {
        state.i += 1;
        let val = parse_expression(state);

        return Box::new(ReturnStatement{value: val});
    }

    // DerefAssignment
    if state.curr().equals(TokenType::Keyword, "deref") {
        state.i += 1;
        let addr = parse_expression(state);

        state.expect_token_type(TokenType::Equals);
        state.i += 1;
        let value = parse_expression(state);

        return Box::new(DerefAssignment{addr: addr, value: value});
    }

    panic!("Syntax error at {}!", state.curr().position());
    //return Box::new(NOPStatement{});
}

fn parse_blockstatement(state: &mut ParserState) -> BlockStatement {
    state.expect_token(TokenType::Parenthesis, "{");
    state.i += 1;
    state.advance_newlines();

    let mut n_bs = BlockStatement {statements: Vec::new()};
    n_bs.statements.push(parse_statement(state));
    state.advance_newlines();

    while !state.curr().equals(TokenType::Parenthesis, "}") {
        n_bs.statements.push(parse_statement(state));
        state.advance_newlines();
    }

    state.i += 1;
    state.advance_newlines();

    return n_bs;
}

fn parse_program(state: &mut ParserState) -> AST {
    let mut nodes = Vec::new();

    while state.curr().ttype != TokenType::EOF {
        nodes.push(parse_statement(state));
        state.advance_newlines();
    }

    return AST { nodes: BlockStatement { statements: nodes } };
}


pub fn parse(tokenlist: Vec<Token>) -> AST {
    let mut state = ParserState {
        tokenlist: tokenlist,
        i: 0,
    };

    return parse_program(&mut state);
}