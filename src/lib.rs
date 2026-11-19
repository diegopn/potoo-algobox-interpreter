//! Text-mode interpreter for documented AlgoBox algorithms.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

pub type ListReference = Rc<RefCell<Vec<Value>>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}",
            self.span.line, self.span.column, self.message
        )
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
    Text(String),
    Boolean(bool),
    List(ListReference),
}
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::List(left), Self::List(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(n) if n.fract() == 0.0 && n.abs() < i64::MAX as f64 => {
                write!(f, "{}", *n as i64)
            }
            Self::Number(n) => write!(f, "{n}"),
            Self::Text(s) => write!(f, "{s}"),
            Self::Boolean(b) => write!(f, "{}", if *b { "VRAI" } else { "FAUX" }),
            Self::List(items) => {
                let items = items.borrow();
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeclaredType {
    Number,
    Text,
    Boolean,
    List,
}

#[derive(Clone, Debug)]
pub struct Program {
    declarations: HashMap<String, DeclaredType>,
    functions: HashMap<String, Function>,
    statements: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct Function {
    name: String,
    parameters: Vec<String>,
    declarations: HashMap<String, DeclaredType>,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
enum Stmt {
    Assign {
        name: String,
        index: Option<Expr>,
        value: Expr,
    },
    Read(String),
    Display(Expr, bool),
    DisplayCalculation(Expr, bool),
    Evaluate(Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    For(String, Expr, Expr, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    Return(Expr),
    Pause,
    Draw(String, Vec<Expr>),
}

#[derive(Clone, Debug)]
enum Expr {
    Number(f64),
    Text(String),
    Boolean(bool),
    Variable(String),
    Index(Box<Expr>, Box<Expr>),
    List(Vec<Expr>),
    Call(String, Vec<Expr>),
    Unary(Unary, Box<Expr>),
    Binary(Box<Expr>, Op, Box<Expr>),
}
#[derive(Clone, Copy, Debug)]
enum Unary {
    Negate,
    Not,
}
#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
enum Tk {
    Ident(String),
    Number(f64),
    Text(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Eof,
}
#[derive(Clone, Debug)]
struct Token {
    kind: Tk,
    column: usize,
}

fn tokenize(source: &str, line: usize) -> Result<Vec<Token>, Diagnostic> {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        let col = i + 1;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            break;
        }
        if c == '!' {
            if chars.get(i + 1) == Some(&'=') {
                out.push(Token {
                    kind: Tk::Ne,
                    column: col,
                });
                i += 2;
                continue;
            }
            return Err(Diagnostic {
                span: Span { line, column: col },
                message: "`!` doit être suivi de `=`".into(),
            });
        }
        let single = match c {
            '(' => Some(Tk::LParen),
            ')' => Some(Tk::RParen),
            '[' => Some(Tk::LBracket),
            ']' => Some(Tk::RBracket),
            ',' => Some(Tk::Comma),
            ':' => Some(Tk::Colon),
            '+' => Some(Tk::Plus),
            '-' => Some(Tk::Minus),
            '*' => Some(Tk::Star),
            '/' => Some(Tk::Slash),
            '%' => Some(Tk::Percent),
            _ => None,
        };
        if let Some(kind) = single {
            out.push(Token { kind, column: col });
            i += 1;
            continue;
        }
        let mut pair = |b: char, yes: Tk, no: Tk| {
            if chars.get(i + 1) == Some(&b) {
                i += 2;
                yes
            } else {
                i += 1;
                no
            }
        };
        match c {
            '=' => out.push(Token {
                kind: pair('=', Tk::Eq, Tk::Eq),
                column: col,
            }),
            '<' => out.push(Token {
                kind: pair('=', Tk::Le, Tk::Lt),
                column: col,
            }),
            '>' => out.push(Token {
                kind: pair('=', Tk::Ge, Tk::Gt),
                column: col,
            }),
            '"' | '\'' => {
                let quote = c;
                i += 1;
                let mut text = String::new();
                while i < chars.len() && chars[i] != quote {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        text.push(match chars[i] {
                            'n' => '\n',
                            't' => '\t',
                            other => other,
                        });
                    } else {
                        text.push(chars[i]);
                    }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(Diagnostic {
                        span: Span { line, column: col },
                        message: "chaîne non terminée".into(),
                    });
                }
                i += 1;
                out.push(Token {
                    kind: Tk::Text(text),
                    column: col,
                });
            }
            d if d.is_ascii_digit() => {
                let begin = i;
                i += 1;
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || (chars[i] == '.'
                            && chars.get(i + 1).is_some_and(|v| v.is_ascii_digit())))
                {
                    i += 1;
                }
                let text: String = chars[begin..i].iter().collect();
                let n = text.parse().map_err(|_| Diagnostic {
                    span: Span { line, column: col },
                    message: "nombre incorrect".into(),
                })?;
                out.push(Token {
                    kind: Tk::Number(n),
                    column: col,
                });
            }
            a if a.is_ascii_alphabetic() || a == '_' => {
                let begin = i;
                i += 1;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    i += 1;
                }
                out.push(Token {
                    kind: Tk::Ident(chars[begin..i].iter().collect()),
                    column: col,
                });
            }
            _ => {
                return Err(Diagnostic {
                    span: Span { line, column: col },
                    message: format!("caractère inattendu `{c}`"),
                });
            }
        }
    }
    out.push(Token {
        kind: Tk::Eof,
        column: source.chars().count() + 1,
    });
    Ok(out)
}

struct ExprParser {
    tokens: Vec<Token>,
    at: usize,
    line: usize,
}
impl ExprParser {
    fn parse(source: &str, line: usize) -> Result<Expr, Diagnostic> {
        let tokens = tokenize(source, line)?;
        let mut p = Self {
            tokens,
            at: 0,
            line,
        };
        let expr = p.expression(0)?;
        if p.current().kind != Tk::Eof {
            return Err(p.error("texte après l'expression"));
        }
        Ok(expr)
    }
    fn expression(&mut self, min: u8) -> Result<Expr, Diagnostic> {
        let mut left = self.prefix()?;
        loop {
            if self.take(&Tk::LBracket) {
                let index = self.expression(0)?;
                self.expect(Tk::RBracket, "`]` attendu")?;
                left = Expr::Index(Box::new(left), Box::new(index));
                continue;
            }
            if self.current().kind == Tk::LParen {
                if let Expr::Variable(name) = left {
                    self.at += 1;
                    let mut args = Vec::new();
                    if self.current().kind != Tk::RParen {
                        loop {
                            args.push(self.expression(0)?);
                            if !self.take(&Tk::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(Tk::RParen, "`)` attendu")?;
                    left = Expr::Call(name, args);
                    continue;
                }
            }
            if self.current().kind == Tk::Colon {
                if min <= 0 {
                    let mut items = vec![left];
                    while self.take(&Tk::Colon) {
                        items.push(self.expression(1)?);
                    }
                    left = Expr::List(items);
                    continue;
                }
            }
            let Some((op, prec)) = self.operator() else {
                break;
            };
            if prec < min {
                break;
            }
            self.at += 1;
            let right = self.expression(prec + 1)?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }
    fn prefix(&mut self) -> Result<Expr, Diagnostic> {
        let tok = self.current().clone();
        match tok.kind {
            Tk::Number(n) => {
                self.at += 1;
                Ok(Expr::Number(n))
            }
            Tk::Text(s) => {
                self.at += 1;
                Ok(Expr::Text(s))
            }
            Tk::Ident(s) if s.eq_ignore_ascii_case("VRAI") || s.eq_ignore_ascii_case("TRUE") => {
                self.at += 1;
                Ok(Expr::Boolean(true))
            }
            Tk::Ident(s) if s.eq_ignore_ascii_case("FAUX") || s.eq_ignore_ascii_case("FALSE") => {
                self.at += 1;
                Ok(Expr::Boolean(false))
            }
            Tk::Ident(s) if s.eq_ignore_ascii_case("NON") || s.eq_ignore_ascii_case("NOT") => {
                self.at += 1;
                Ok(Expr::Unary(Unary::Not, Box::new(self.expression(6)?)))
            }
            Tk::Ident(s) => {
                self.at += 1;
                Ok(Expr::Variable(s))
            }
            Tk::Minus => {
                self.at += 1;
                Ok(Expr::Unary(Unary::Negate, Box::new(self.expression(6)?)))
            }
            Tk::LParen => {
                self.at += 1;
                let expr = self.expression(0)?;
                self.expect(Tk::RParen, "`)` attendu")?;
                Ok(expr)
            }
            Tk::LBracket => {
                self.at += 1;
                let mut values = Vec::new();
                if self.current().kind != Tk::RBracket {
                    loop {
                        values.push(self.expression(0)?);
                        if !self.take(&Tk::Comma) {
                            break;
                        }
                    }
                }
                self.expect(Tk::RBracket, "`]` attendu")?;
                Ok(Expr::List(values))
            }
            _ => Err(Diagnostic {
                span: Span {
                    line: self.line,
                    column: tok.column,
                },
                message: "expression attendue".into(),
            }),
        }
    }
    fn operator(&self) -> Option<(Op, u8)> {
        let v = match &self.current().kind {
            Tk::Ident(s) if s.eq_ignore_ascii_case("OU") => (Op::Or, 1),
            Tk::Ident(s) if s.eq_ignore_ascii_case("ET") => (Op::And, 2),
            Tk::Eq => (Op::Eq, 3),
            Tk::Ne => (Op::Ne, 3),
            Tk::Lt => (Op::Lt, 3),
            Tk::Le => (Op::Le, 3),
            Tk::Gt => (Op::Gt, 3),
            Tk::Ge => (Op::Ge, 3),
            Tk::Plus => (Op::Add, 4),
            Tk::Minus => (Op::Sub, 4),
            Tk::Star => (Op::Mul, 5),
            Tk::Slash => (Op::Div, 5),
            Tk::Percent => (Op::Mod, 5),
            _ => return None,
        };
        Some(v)
    }
    fn current(&self) -> &Token {
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }
    fn take(&mut self, kind: &Tk) -> bool {
        if std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, kind: Tk, msg: &str) -> Result<(), Diagnostic> {
        if self.take(&kind) {
            Ok(())
        } else {
            Err(self.error(msg))
        }
    }
    fn error(&self, msg: &str) -> Diagnostic {
        Diagnostic {
            span: Span {
                line: self.line,
                column: self.current().column,
            },
            message: msg.into(),
        }
    }
}

pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    let lines: Vec<(usize, String)> = source
        .lines()
        .enumerate()
        .map(|(n, source_line)| {
            let mut line = source_line.trim().to_owned();
            if let Some((prefix, rest)) = line.split_once(": ") {
                if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
                    line = rest.to_owned();
                }
            }
            (n + 1, line)
        })
        .collect();
    let mut declarations = HashMap::new();
    let mut start = None;
    let mut variables_header = None;
    let mut saw_variables = false;
    for (index, (line_no, line)) in lines.iter().enumerate() {
        if line.eq_ignore_ascii_case("VARIABLES") {
            saw_variables = true;
            variables_header = Some(index);
            continue;
        }
        if line.eq_ignore_ascii_case("DEBUT_ALGORITHME") {
            start = Some(index + 1);
            break;
        }
        if saw_variables && !line.is_empty() && !line.starts_with("//") {
            if let Some((name, ty)) = line.split_once(" EST_DU_TYPE ") {
                let kind = match declared_type(ty.trim()) {
                    Some(kind) => kind,
                    None => {
                        return Err(vec![Diagnostic {
                            span: Span {
                                line: *line_no,
                                column: 1,
                            },
                            message: format!("type AlgoBox inconnu `{}`", ty.trim()),
                        }]);
                    }
                };
                declarations.insert(name.trim().to_ascii_lowercase(), kind);
            } else {
                return Err(vec![Diagnostic {
                    span: Span {
                        line: *line_no,
                        column: 1,
                    },
                    message: "déclaration attendue sous VARIABLES".into(),
                }]);
            }
        }
    }
    if !saw_variables {
        return Err(vec![Diagnostic {
            span: Span { line: 1, column: 1 },
            message: "section VARIABLES manquante".into(),
        }]);
    }
    let Some(start) = start else {
        return Err(vec![Diagnostic {
            span: Span {
                line: lines.len().max(1),
                column: 1,
            },
            message: "DEBUT_ALGORITHME manquant".into(),
        }]);
    };
    let functions = parse_functions(&lines, variables_header.unwrap_or(0), &declarations)?;
    let mut parser = LineParser {
        lines: lines.clone(),
        at: start,
        declarations: &declarations,
    };
    let statements = parser.block(&["FIN_ALGORITHME"])?;
    if !parser.is_line("FIN_ALGORITHME") {
        return Err(vec![parser.error("FIN_ALGORITHME manquant")]);
    }
    parser.at += 1;
    if parser.lines[parser.at..]
        .iter()
        .any(|(_, s)| !s.is_empty() && !s.starts_with("//"))
    {
        return Err(vec![parser.error("instructions après FIN_ALGORITHME")]);
    }
    Ok(Program {
        declarations,
        functions,
        statements,
    })
}

fn declared_type(source: &str) -> Option<DeclaredType> {
    match source.to_ascii_uppercase().as_str() {
        "NOMBRE" => Some(DeclaredType::Number),
        "CHAINE" => Some(DeclaredType::Text),
        "LISTE" => Some(DeclaredType::List),
        _ => None,
    }
}

fn parse_functions(
    lines: &[(usize, String)],
    before_variables: usize,
    globals: &HashMap<String, DeclaredType>,
) -> Result<HashMap<String, Function>, Vec<Diagnostic>> {
    let mut functions = HashMap::new();
    let mut at = 0;
    while at < before_variables {
        let (line_no, line) = &lines[at];
        let upper = line.to_ascii_uppercase();
        if !upper.starts_with("FONCTION ") {
            at += 1;
            continue;
        }
        let header = line[9..].trim();
        let open = header.find('(').ok_or_else(|| {
            vec![Diagnostic {
                span: Span {
                    line: *line_no,
                    column: 1,
                },
                message: "`(` attendu dans l'en-tête FONCTION".into(),
            }]
        })?;
        let close = header.rfind(')').ok_or_else(|| {
            vec![Diagnostic {
                span: Span {
                    line: *line_no,
                    column: 1,
                },
                message: "`)` attendu dans l'en-tête FONCTION".into(),
            }]
        })?;
        let name = header[..open].trim().to_owned();
        let parameters: Vec<String> = if header[open + 1..close].trim().is_empty() {
            Vec::new()
        } else {
            split_args(&header[open + 1..close])
                .iter()
                .map(|p| p.to_string())
                .collect()
        };
        let vars_marker = (at + 1..before_variables)
            .find(|i| lines[*i].1.eq_ignore_ascii_case("VARIABLES_FONCTION"))
            .ok_or_else(|| {
                vec![Diagnostic {
                    span: Span {
                        line: *line_no,
                        column: 1,
                    },
                    message: "VARIABLES_FONCTION manquant".into(),
                }]
            })?;
        let body_marker = (vars_marker + 1..before_variables)
            .find(|i| lines[*i].1.eq_ignore_ascii_case("DEBUT_FONCTION"))
            .ok_or_else(|| {
                vec![Diagnostic {
                    span: Span {
                        line: *line_no,
                        column: 1,
                    },
                    message: "DEBUT_FONCTION manquant".into(),
                }]
            })?;
        let mut local_declarations = HashMap::new();
        for (decl_line_no, decl) in &lines[vars_marker + 1..body_marker] {
            if decl.is_empty() || decl.starts_with("//") {
                continue;
            }
            let Some((var, ty)) = decl.split_once(" EST_DU_TYPE ") else {
                return Err(vec![Diagnostic {
                    span: Span {
                        line: *decl_line_no,
                        column: 1,
                    },
                    message: "déclaration locale attendue".into(),
                }]);
            };
            let Some(kind) = declared_type(ty.trim()) else {
                return Err(vec![Diagnostic {
                    span: Span {
                        line: *decl_line_no,
                        column: 1,
                    },
                    message: format!("type AlgoBox inconnu `{}`", ty.trim()),
                }]);
            };
            local_declarations.insert(var.trim().to_ascii_lowercase(), kind);
        }
        let mut scope_declarations = globals.clone();
        scope_declarations.extend(local_declarations.clone());
        for param in &parameters {
            scope_declarations.insert(param.to_ascii_lowercase(), DeclaredType::Number);
        }
        let mut parser = LineParser {
            lines: lines.to_vec(),
            at: body_marker + 1,
            declarations: &scope_declarations,
        };
        let body = parser.block(&["FIN_FONCTION"])?;
        if !parser.is_line("FIN_FONCTION") {
            return Err(vec![parser.error("FIN_FONCTION manquant")]);
        }
        let function = Function {
            name: name.clone(),
            parameters,
            declarations: local_declarations,
            body,
        };
        let key = name.to_ascii_lowercase();
        if functions.insert(key, function).is_some() {
            return Err(vec![Diagnostic {
                span: Span {
                    line: *line_no,
                    column: 1,
                },
                message: format!("fonction `{name}` déclarée plusieurs fois"),
            }]);
        }
        at = parser.at + 1;
    }
    Ok(functions)
}

struct LineParser<'a> {
    lines: Vec<(usize, String)>,
    at: usize,
    declarations: &'a HashMap<String, DeclaredType>,
}
impl LineParser<'_> {
    fn block(&mut self, ends: &[&str]) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while self.at < self.lines.len() {
            let line = self.lines[self.at].1.clone();
            if line.is_empty() || line.starts_with("//") {
                self.at += 1;
                continue;
            }
            if ends.iter().any(|e| line.eq_ignore_ascii_case(e)) {
                break;
            }
            let upper = line.to_ascii_uppercase();
            if upper.starts_with("SI ") && upper.contains(" ALORS") {
                let condition_text = line[3..upper.find(" ALORS").unwrap()]
                    .trim()
                    .trim_matches(['(', ')'])
                    .to_owned();
                let condition =
                    ExprParser::parse(&condition_text, self.line_no()).map_err(|e| vec![e])?;
                self.at += 1;
                self.expect_line("DEBUT_SI")?;
                let yes = self.block(&["FIN_SI"])?;
                self.expect_line("FIN_SI")?;
                let mut no = Vec::new();
                if self.is_line("SINON") {
                    self.at += 1;
                    self.expect_line("DEBUT_SINON")?;
                    no = self.block(&["FIN_SINON"])?;
                    self.expect_line("FIN_SINON")?;
                }
                statements.push(Stmt::If(condition, yes, no));
                continue;
            }
            if upper.starts_with("POUR ") {
                let rest = &line[5..];
                let upper_rest = rest.to_ascii_uppercase();
                let Some(split) = upper_rest.find(" ALLANT_DE ") else {
                    return Err(vec![self.error(
                        "syntaxe POUR attendue: POUR variable ALLANT_DE début A fin",
                    )]);
                };
                let name = rest[..split].trim().to_owned();
                match self.declarations.get(&name.to_ascii_lowercase()) {
                    Some(DeclaredType::Number) => {}
                    Some(_) => {
                        return Err(vec![
                            self.error("la variable POUR doit être de type NOMBRE"),
                        ]);
                    }
                    None => {
                        return Err(vec![
                            self.error(&format!("variable POUR `{name}` non déclarée")),
                        ]);
                    }
                }
                let range = &rest[split + 11..];
                let upper_range = range.to_ascii_uppercase();
                let Some(a_pos) = upper_range.find(" A ") else {
                    return Err(vec![self.error("`A` attendu dans la borne POUR")]);
                };
                let start_expr = ExprParser::parse(range[..a_pos].trim(), self.line_no())
                    .map_err(|e| vec![e])?;
                let end_expr = ExprParser::parse(range[a_pos + 3..].trim(), self.line_no())
                    .map_err(|e| vec![e])?;
                self.at += 1;
                self.expect_line("DEBUT_POUR")?;
                let body = self.block(&["FIN_POUR"])?;
                self.expect_line("FIN_POUR")?;
                statements.push(Stmt::For(name, start_expr, end_expr, body));
                continue;
            }
            if upper.starts_with("TANT_QUE ") {
                let Some(pos) = upper.rfind(" FAIRE") else {
                    return Err(vec![self.error("mot-clé FAIRE attendu après TANT_QUE")]);
                };
                let condition_text = line[9..pos].trim().trim_matches(['(', ')']).to_owned();
                let condition =
                    ExprParser::parse(&condition_text, self.line_no()).map_err(|e| vec![e])?;
                self.at += 1;
                self.expect_line("DEBUT_TANT_QUE")?;
                let body = self.block(&["FIN_TANT_QUE"])?;
                self.expect_line("FIN_TANT_QUE")?;
                statements.push(Stmt::While(condition, body));
                continue;
            }
            if upper.starts_with("RENVOYER ") {
                let value =
                    ExprParser::parse(line[9..].trim(), self.line_no()).map_err(|e| vec![e])?;
                statements.push(Stmt::Return(value));
                self.at += 1;
                continue;
            }
            if upper.starts_with("AFFICHERCALCUL*") || upper.starts_with("AFFICHERCALCUL ") {
                let newline = upper.starts_with("AFFICHERCALCUL*");
                let skip = if newline {
                    "AFFICHERCALCUL*".len()
                } else {
                    "AFFICHERCALCUL".len()
                };
                let expression = ExprParser::parse(line[skip..].trim(), self.line_no())
                    .map_err(|error| vec![error])?;
                statements.push(Stmt::DisplayCalculation(expression, newline));
                self.at += 1;
                continue;
            }
            if upper.starts_with("AFFICHER*") || upper.starts_with("AFFICHER ") {
                let newline = upper.starts_with("AFFICHER*");
                let skip = "AFFICHER".len() + usize::from(newline);
                let expr =
                    ExprParser::parse(line[skip..].trim(), self.line_no()).map_err(|e| vec![e])?;
                statements.push(Stmt::Display(expr, newline));
                self.at += 1;
                continue;
            }
            if upper.starts_with("LIRE ") {
                let name = line[5..].trim().to_owned();
                statements.push(Stmt::Read(name));
                self.at += 1;
                continue;
            }
            if upper == "PAUSE" {
                statements.push(Stmt::Pause);
                self.at += 1;
                continue;
            }
            if upper == "EFFACER_GRAPHIQUE" {
                statements.push(Stmt::Draw("EFFACER_GRAPHIQUE".to_owned(), Vec::new()));
                self.at += 1;
                continue;
            }
            if upper.starts_with("TRACER_POINT") || upper.starts_with("TRACER_SEGMENT") {
                let (command, coordinates) =
                    parse_drawing_statement(&line).map_err(|message| vec![self.error(&message)])?;
                let args = coordinates
                    .iter()
                    .map(|source| ExprParser::parse(source, self.line_no()))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| vec![error])?;
                statements.push(Stmt::Draw(command, args));
                self.at += 1;
                continue;
            }
            if upper.starts_with("APPELER_FONCTION ") {
                let expression =
                    ExprParser::parse(line["APPELER_FONCTION".len()..].trim(), self.line_no())
                        .map_err(|error| vec![error])?;
                if !matches!(expression, Expr::Call(_, _)) {
                    return Err(vec![
                        self.error("un appel de fonction est attendu après APPELER_FONCTION"),
                    ]);
                }
                statements.push(Stmt::Evaluate(expression));
                self.at += 1;
                continue;
            }
            if let Some(pos) = upper.find(" PREND_LA_VALEUR ") {
                let lhs = line[..pos].trim();
                let rhs = line[pos + 17..].trim();
                let (name, index) = if let Some(bracket) = lhs.find('[') {
                    let close = lhs
                        .rfind(']')
                        .ok_or_else(|| vec![self.error("`]` attendu dans l'indice")])?;
                    (
                        lhs[..bracket].trim().to_owned(),
                        Some(
                            ExprParser::parse(&lhs[bracket + 1..close], self.line_no())
                                .map_err(|e| vec![e])?,
                        ),
                    )
                } else {
                    (lhs.to_owned(), None)
                };
                if !self.declarations.contains_key(&name.to_ascii_lowercase()) {
                    return Err(vec![self.error(&format!("variable `{name}` non déclarée"))]);
                }
                let value = ExprParser::parse(rhs, self.line_no()).map_err(|e| vec![e])?;
                statements.push(Stmt::Assign { name, index, value });
                self.at += 1;
                continue;
            }
            return Err(vec![self.error("instruction AlgoBox non reconnue")]);
        }
        Ok(statements)
    }
    fn expect_line(&mut self, expected: &str) -> Result<(), Vec<Diagnostic>> {
        if self.is_line(expected) {
            self.at += 1;
            Ok(())
        } else {
            Err(vec![self.error(&format!("ligne `{expected}` attendue"))])
        }
    }
    fn is_line(&self, expected: &str) -> bool {
        self.lines
            .get(self.at)
            .is_some_and(|(_, s)| s.eq_ignore_ascii_case(expected))
    }
    fn line_no(&self) -> usize {
        self.lines.get(self.at).map(|(n, _)| *n).unwrap_or(1)
    }
    fn error(&self, message: &str) -> Diagnostic {
        Diagnostic {
            span: Span {
                line: self.line_no(),
                column: 1,
            },
            message: message.into(),
        }
    }
}

fn parse_drawing_statement(line: &str) -> Result<(String, Vec<String>), String> {
    let trimmed = line.trim();
    let upper = trimmed.to_ascii_uppercase();
    let (base, coordinate_count) = if upper.starts_with("TRACER_POINT") {
        ("TRACER_POINT", 2)
    } else if upper.starts_with("TRACER_SEGMENT") {
        ("TRACER_SEGMENT", 4)
    } else {
        return Err("commande graphique AlgoBox inconnue".into());
    };
    let suffix = trimmed
        .get(base.len()..)
        .ok_or_else(|| "commande graphique AlgoBox incorrecte".to_owned())?;
    let (color, remainder) = if let Some(color_suffix) = suffix.strip_prefix('_') {
        let color_end = color_suffix
            .find(char::is_whitespace)
            .unwrap_or(color_suffix.len());
        let color = &color_suffix[..color_end];
        let normalized = match color.to_ascii_lowercase().as_str() {
            "rouge" => "Rouge",
            "vert" => "Vert",
            "bleu" => "Bleu",
            "blanc" => "Blanc",
            _ => return Err(format!("couleur de tracé AlgoBox inconnue `{color}`")),
        };
        (Some(normalized), color_suffix[color_end..].trim())
    } else {
        (None, suffix.trim())
    };
    if !suffix.is_empty()
        && !suffix.starts_with('_')
        && !suffix.chars().next().is_some_and(char::is_whitespace)
    {
        return Err(format!("instruction `{base}` mal formée"));
    }
    let (first, rest) = take_parenthesized_pair(remainder)?;
    let mut coordinates = first;
    if coordinate_count == 4 {
        let segment_rest = rest
            .trim_start()
            .strip_prefix("->")
            .ok_or_else(|| "`->` attendu entre les extrémités du segment".to_owned())?
            .trim_start();
        let (second, trailing) = take_parenthesized_pair(segment_rest)?;
        if !trailing.trim().is_empty() {
            return Err("texte après les coordonnées du segment".into());
        }
        coordinates.extend(second);
    } else if !rest.trim().is_empty() {
        return Err("texte après les coordonnées du point".into());
    }
    let command = match color {
        Some(color) => format!("{base}_{color}"),
        None => base.to_owned(),
    };
    Ok((command, coordinates))
}

fn take_parenthesized_pair(source: &str) -> Result<(Vec<String>, &str), String> {
    let source = source.trim_start();
    let Some(after_open) = source.strip_prefix('(') else {
        return Err("coordonnées graphiques devem estar entre parênteses".into());
    };
    let mut depth = 1_i32;
    let mut quote = None;
    let mut close_at = None;
    for (index, character) in after_open.char_indices() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    close_at = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close_at = close_at.ok_or_else(|| "`)` attendu après as coordenadas".to_owned())?;
    let inner = &after_open[..close_at];
    let coordinates = split_args(inner);
    if coordinates.len() != 2 || coordinates.iter().any(|coordinate| coordinate.is_empty()) {
        return Err("cada ponto ou extremidade exige duas coordenadas".into());
    }
    Ok((
        coordinates.into_iter().map(str::to_owned).collect(),
        &after_open[close_at + 1..],
    ))
}

fn split_args(source: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let (mut depth, mut quoted, mut start) = (0_i32, None, 0_usize);
    for (i, c) in source.char_indices() {
        if let Some(q) = quoted {
            if c == q {
                quoted = None
            }
        } else {
            match c {
                '"' | '\'' => quoted = Some(c),
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                ',' if depth == 0 => {
                    args.push(source[start..i].trim());
                    start = i + 1
                }
                _ => {}
            }
        }
    }
    args.push(source[start..].trim());
    args
}

pub trait Host {
    fn input(&mut self) -> Result<String, String>;
    fn display(&mut self, value: &Value, newline: bool) -> Result<(), String>;
    /// Called after each 1,000 output operations. Return true to continue.
    fn confirm_continue_after_output_limit(&mut self) -> Result<bool, String> {
        Ok(false)
    }
    fn pause(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn should_stop(&mut self) -> bool {
        false
    }
    fn draw(&mut self, command: &str, args: &[Value]) -> Result<(), String>;
    fn random_integer(&mut self, lower: i64, upper: i64) -> Result<i64, String>;
    fn random_real(&mut self, lower: f64, upper: f64) -> Result<f64, String> {
        const SCALE: i64 = 1_000_000_000_000;
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err("invalid real random range".to_owned());
        }
        if lower == upper {
            return Ok(lower);
        }
        let sample = self.random_integer(0, SCALE - 1)?;
        if !(0..SCALE).contains(&sample) {
            return Err("host returned an integer outside the requested random range".to_owned());
        }
        let unit = sample as f64 / SCALE as f64;
        let value = if lower.is_sign_positive() == upper.is_sign_positive() {
            lower + (upper - lower) * unit
        } else {
            lower * (1.0 - unit) + upper * unit
        };
        Ok(value.max(lower).min(upper.next_down()))
    }
    fn call_local_function(
        &mut self,
        _name: &str,
        _args: &[Value],
    ) -> Result<Option<Value>, String> {
        Ok(None)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Execution {
    pub variables: HashMap<String, Value>,
    pub instructions: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
}
impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RuntimeError {}

pub struct Interpreter<H: Host> {
    host: H,
    variables: HashMap<String, Value>,
    types: HashMap<String, DeclaredType>,
    functions: HashMap<String, Function>,
    instructions: usize,
    call_depth: usize,
    loop_iterations: usize,
    outputs_since_confirmation: usize,
}
impl<H: Host> Interpreter<H> {
    pub fn new(host: H) -> Self {
        Self {
            host,
            variables: HashMap::new(),
            types: HashMap::new(),
            functions: HashMap::new(),
            instructions: 0,
            call_depth: 0,
            loop_iterations: 0,
            outputs_since_confirmation: 0,
        }
    }
    pub fn host(&self) -> &H {
        &self.host
    }
    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }
    pub fn run(&mut self, program: &Program) -> Result<Execution, RuntimeError> {
        self.types = program.declarations.clone();
        self.functions = program.functions.clone();
        self.variables.clear();
        self.instructions = 0;
        self.call_depth = 0;
        self.loop_iterations = 0;
        self.outputs_since_confirmation = 0;
        for (name, ty) in &self.types {
            self.variables.insert(
                name.clone(),
                match ty {
                    DeclaredType::Number => Value::Number(0.0),
                    DeclaredType::Text => Value::Text(String::new()),
                    DeclaredType::Boolean => Value::Boolean(false),
                    DeclaredType::List => Value::List(Rc::new(RefCell::new(Vec::new()))),
                },
            );
        }
        let statements = program.statements.clone();
        if self.block(&statements)?.is_some() {
            return Err(err(
                "RENVOYER ne peut être utilisé qu'à l'intérieur d'une fonction",
            ));
        }
        Ok(Execution {
            variables: self.variables.clone(),
            instructions: self.instructions,
        })
    }
    fn block(&mut self, statements: &[Stmt]) -> Result<Option<Value>, RuntimeError> {
        for stmt in statements {
            self.tick()?;
            match stmt {
                Stmt::Assign { name, index, value } => {
                    let val = self.eval(value)?;
                    if let Some(index) = index {
                        let pos = as_index(self.eval(index)?)?;
                        let target = self.variable(name)?;
                        let Value::List(items) = target else {
                            return Err(err("indexation sur une variable non liste"));
                        };
                        if !matches!(&val, Value::Number(_)) {
                            return Err(err("une liste AlgoBox ne contient que des nombres"));
                        }
                        let mut items = items.borrow_mut();
                        let slot = list_slot(pos, items.len())?;
                        items[slot] = val;
                    } else {
                        self.set(name, val)?;
                    }
                }
                Stmt::Read(name) => {
                    let input = self.host.input().map_err(err)?;
                    let value = match self.types.get(&name.to_ascii_lowercase()) {
                        Some(DeclaredType::Number) => Value::Number(
                            input
                                .trim()
                                .parse()
                                .map_err(|_| err("entrée numérique attendue"))?,
                        ),
                        Some(DeclaredType::Text) => Value::Text(input),
                        Some(DeclaredType::Boolean) => {
                            match input.trim().to_ascii_lowercase().as_str() {
                                "true" | "vrai" => Value::Boolean(true),
                                "false" | "faux" => Value::Boolean(false),
                                _ => return Err(err("valeur logique attendue")),
                            }
                        }
                        Some(DeclaredType::List) => {
                            let values = input
                                .split(':')
                                .map(|part| {
                                    part.trim()
                                        .parse::<f64>()
                                        .map(Value::Number)
                                        .map_err(|_| err("liste attendue: nombres séparés par `:`"))
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            Value::List(Rc::new(RefCell::new(values)))
                        }
                        None => return Err(err("variable non déclarée")),
                    };
                    self.set(name, value)?;
                }
                Stmt::Display(expr, newline) => {
                    let value = self.eval(expr)?;
                    self.host.display(&value, *newline).map_err(err)?;
                    self.note_output()?;
                }
                Stmt::DisplayCalculation(expression, newline) => {
                    let value = Value::Number(as_number(self.eval(expression)?)?);
                    self.host.display(&value, *newline).map_err(err)?;
                    self.note_output()?;
                }
                Stmt::Evaluate(expression) => {
                    self.eval(expression)?;
                }
                Stmt::If(cond, yes, no) => {
                    if as_bool(self.eval(cond)?)? {
                        if let Some(value) = self.block(yes)? {
                            return Ok(Some(value));
                        }
                    } else {
                        if let Some(value) = self.block(no)? {
                            return Ok(Some(value));
                        }
                    }
                }
                Stmt::For(name, start, end, body) => {
                    let first = as_number(self.eval(start)?)?;
                    let last = as_number(self.eval(end)?)?;
                    if first <= last {
                        let mut n = first;
                        let mut iterations = 0;
                        while n <= last {
                            self.tick_loop_iteration(&mut iterations, "POUR")?;
                            self.tick()?;
                            self.set(name, Value::Number(n))?;
                            if let Some(value) = self.block(body)? {
                                return Ok(Some(value));
                            }
                            let next = n + 1.0;
                            if next == n {
                                break;
                            }
                            n = next;
                        }
                    }
                }
                Stmt::While(cond, body) => {
                    let mut iterations = 0;
                    while as_bool(self.eval(cond)?)? {
                        self.tick_loop_iteration(&mut iterations, "TANT_QUE")?;
                        self.tick()?;
                        if let Some(value) = self.block(body)? {
                            return Ok(Some(value));
                        }
                    }
                }
                Stmt::Pause => self.host.pause().map_err(err)?,
                Stmt::Draw(command, args) => {
                    let values = args
                        .iter()
                        .map(|x| self.eval(x))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.host.draw(command, &values).map_err(err)?;
                }
                Stmt::Return(expression) => return Ok(Some(self.eval(expression)?)),
            }
        }
        Ok(None)
    }
    fn eval(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Boolean(b) => Ok(Value::Boolean(*b)),
            Expr::Variable(name) if name.eq_ignore_ascii_case("Math.PI") => {
                Ok(Value::Number(std::f64::consts::PI))
            }
            Expr::Variable(name) if name.to_ascii_lowercase().ends_with(".length") => {
                let base = &name[..name.len() - ".length".len()];
                let Value::Text(value) = self.variable(base)? else {
                    return Err(err("`.length` nécessite une chaîne de caractères"));
                };
                Ok(Value::Number(value.chars().count() as f64))
            }
            Expr::Variable(name) => self.variable(name),
            Expr::List(values) => values
                .iter()
                .map(|v| self.eval(v))
                .collect::<Result<Vec<_>, _>>()
                .map(|items| Value::List(Rc::new(RefCell::new(items)))),
            Expr::Index(list, index) => {
                let Value::List(values) = self.eval(list)? else {
                    return Err(err("indexation d'une valeur non liste"));
                };
                let pos = as_index(self.eval(index)?)?;
                let values = values.borrow();
                Ok(values[list_slot(pos, values.len())?].clone())
            }
            Expr::Unary(op, expr) => {
                let v = self.eval(expr)?;
                match op {
                    Unary::Negate => Ok(Value::Number(-as_number(v)?)),
                    Unary::Not => Ok(Value::Boolean(!as_bool(v)?)),
                }
            }
            Expr::Binary(left, op, right) => {
                let a = self.eval(left)?;
                if matches!(op, Op::And) && !as_bool(a.clone())? {
                    return Ok(Value::Boolean(false));
                }
                if matches!(op, Op::Or) && as_bool(a.clone())? {
                    return Ok(Value::Boolean(true));
                }
                let b = self.eval(right)?;
                self.binary(a, *op, b)
            }
            Expr::Call(name, args) => {
                let values = args
                    .iter()
                    .map(|a| self.eval(a))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call(name, &values)
            }
        }
    }
    fn call(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        let key = name.to_ascii_uppercase();
        if matches!(key.as_str(), "MAX" | "MIN") {
            return math_extremum(name, &key, args);
        }
        if key == "STRING.FROMCHARCODE" {
            require_arity(name, args, 1)?;
            let code = as_integer(args[0].clone())?;
            if !(0..=127).contains(&code) {
                return Err(err("code ASCII attendu entre 0 et 127"));
            }
            let character = char::from(code as u8);
            return Ok(Value::Text(character.to_string()));
        }
        if let Some((receiver, method)) = name.rsplit_once('.') {
            match method.to_ascii_lowercase().as_str() {
                "tostring" => {
                    require_arity(name, args, 0)?;
                    let Value::Number(value) = self.variable(receiver)? else {
                        return Err(err("`.toString()` nécessite un nombre"));
                    };
                    return Ok(Value::Text(Value::Number(value).to_string()));
                }
                "substr" => {
                    require_arity(name, args, 2)?;
                    let Value::Text(value) = self.variable(receiver)? else {
                        return Err(err("`.substr()` nécessite une chaîne de caractères"));
                    };
                    let start = as_integer(args[0].clone())?;
                    let length = as_integer(args[1].clone())?;
                    let characters: Vec<char> = value.chars().collect();
                    let start = if start < 0 {
                        (characters.len() as i64 + start).max(0) as usize
                    } else {
                        (start as usize).min(characters.len())
                    };
                    let length = length.max(0) as usize;
                    let end = start.saturating_add(length).min(characters.len());
                    return Ok(Value::Text(characters[start..end].iter().collect()));
                }
                "charcodeat" => {
                    require_arity(name, args, 1)?;
                    let Value::Text(value) = self.variable(receiver)? else {
                        return Err(err("`.charCodeAt()` nécessite une chaîne de caractères"));
                    };
                    let index = as_index_zero_based(args[0].clone())?;
                    let character = value
                        .chars()
                        .nth(index)
                        .ok_or_else(|| err("position de caractère hors limites"))?;
                    if !character.is_ascii() {
                        return Err(err("le code ASCII n'est pas défini pour ce caractère"));
                    }
                    return Ok(Value::Number(character as u8 as f64));
                }
                _ => {}
            }
        }
        if let Some(expected) = builtin_arity(&key)
            && args.len() != expected
        {
            return Err(err(format!(
                "`{name}` attend {expected} argument(s), reçu {}",
                args.len()
            )));
        }
        let n = |i: usize| {
            args.get(i)
                .cloned()
                .ok_or_else(|| err("argument manquant"))
                .and_then(as_number)
        };
        if matches!(key.as_str(), "MATH.MAX" | "MATH.MIN") {
            return math_extremum(name, &key[5..], args);
        }
        let numeric = match key.as_str() {
            "SQRT" => Some(n(0)?.sqrt()),
            "POW" => Some(n(0)?.powf(n(1)?)),
            "FLOOR" => Some(n(0)?.floor()),
            "COS" => Some(n(0)?.cos()),
            "SIN" => Some(n(0)?.sin()),
            "TAN" => Some(n(0)?.tan()),
            "EXP" => Some(n(0)?.exp()),
            "LOG" => Some(n(0)?.ln()),
            "ABS" => Some(n(0)?.abs()),
            "ROUND" => Some((n(0)? + 0.5).floor()),
            "ACOS" => Some(n(0)?.acos()),
            "ASIN" => Some(n(0)?.asin()),
            "ATAN" => Some(n(0)?.atan()),
            "ALGOBOX_ARRONDIR" => {
                let value = n(0)?;
                let precision = n(1)?;
                Some(round_to_decimal_places(value, precision)?)
            }
            "RANDOM" => {
                let value = self.host.random_real(0.0, 1.0).map_err(err)?;
                if !value.is_finite() || !(0.0..1.0).contains(&value) {
                    return Err(err(
                        "l'hôte doit fournir une valeur pseudo-aléatoire dans [0, 1)",
                    ));
                }
                Some(value)
            }
            "ALGOBOX_ALEA_ENT" => {
                let low = as_integer(
                    args.first()
                        .cloned()
                        .ok_or_else(|| err("borne basse manquante"))?,
                )?;
                let high = as_integer(
                    args.get(1)
                        .cloned()
                        .ok_or_else(|| err("borne haute manquante"))?,
                )?;
                if low > high {
                    return Err(err("borne basse supérieure à la borne haute"));
                }
                let value = self.host.random_integer(low, high).map_err(err)?;
                if value < low || value > high {
                    return Err(err("l'hôte a renvoyé un entier hors des bornes demandées"));
                }
                Some(value as f64)
            }
            "ALGOBOX_FACTORIELLE" => {
                let value = as_integer(args[0].clone())?;
                if !(0..70).contains(&value) {
                    return Err(err("la factorielle AlgoBox accepte un entier de 0 à 69"));
                }
                Some((1..=value).fold(1.0, |result, factor| result * factor as f64))
            }
            _ => None,
        };
        if let Some(value) = numeric {
            return Ok(Value::Number(value));
        }
        match key.as_str() {
            "PARSEINT" => {
                require_arity(name, args, 1)?;
                let Value::Text(value) = &args[0] else {
                    return Err(err("`parseInt` nécessite une chaîne de caractères"));
                };
                Ok(Value::Number(parse_integer_prefix(value)))
            }
            "PARSEFLOAT" => {
                require_arity(name, args, 1)?;
                let Value::Text(value) = &args[0] else {
                    return Err(err("`parseFloat` nécessite une chaîne de caractères"));
                };
                Ok(Value::Number(parse_real_prefix(value)))
            }
            name if is_list_summary_function(name) => list_summary(name, args),
            "ALGOBOX_NB_COMBINAISONS" | "ALGOBOX_COEFF_BINOMIAL" => {
                let n = as_integer(args.first().cloned().ok_or_else(|| err("n manquant"))?)?;
                let p = as_integer(args.get(1).cloned().ok_or_else(|| err("p manquant"))?)?;
                if n < 0 || n > 100 || p < 0 || p > n {
                    return Err(err("paramètres binomiaux hors limites"));
                }
                let k = p.min(n - p);
                let mut r = 1.0;
                for i in 0..k {
                    r *= ((n - i) as f64) / ((i + 1) as f64);
                }
                Ok(Value::Number(r))
            }
            "ALGOBOX_LOI_BINOMIALE" => {
                let trials = as_integer(args.first().cloned().ok_or_else(|| err("n manquant"))?)?;
                let probability =
                    as_number(args.get(1).cloned().ok_or_else(|| err("p manquant"))?)?;
                let successes = as_integer(args.get(2).cloned().ok_or_else(|| err("k manquant"))?)?;
                if trials < 0
                    || trials > 100
                    || successes < 0
                    || successes > trials
                    || !(0.0..=1.0).contains(&probability)
                {
                    return Err(err("paramètres de loi binomiale hors limites"));
                }
                let k = successes.min(trials - successes);
                let mut coefficient = 1.0;
                for i in 0..k {
                    coefficient *= (trials - i) as f64 / (i + 1) as f64;
                }
                Ok(Value::Number(
                    coefficient
                        * probability.powf(successes as f64)
                        * (1.0 - probability).powf((trials - successes) as f64),
                ))
            }
            "ALGOBOX_LOI_NORMALE_CR" => Ok(Value::Number(standard_normal_cdf(n(0)?))),
            "ALGOBOX_LOI_NORMALE" => {
                let mean = n(0)?;
                let deviation = n(1)?;
                if !mean.is_finite() || !deviation.is_finite() || deviation <= 0.0 {
                    return Err(err("l'écart-type doit être strictement positif"));
                }
                Ok(Value::Number(standard_normal_cdf(
                    (n(2)? - mean) / deviation,
                )))
            }
            "ALGOBOX_INVERSE_LOI_NORMALE_CR" => Ok(Value::Number(inverse_standard_normal(n(0)?)?)),
            "ALGOBOX_INVERSE_LOI_NORMALE" => {
                let mean = n(0)?;
                let deviation = n(1)?;
                if !mean.is_finite() || !deviation.is_finite() || deviation <= 0.0 {
                    return Err(err("l'écart-type doit être strictement positif"));
                }
                Ok(Value::Number(
                    mean + deviation * inverse_standard_normal(n(2)?)?,
                ))
            }
            "F1" => {
                let value = n(0)?;
                match self.host.call_local_function("F1", &[Value::Number(value)]) {
                    Ok(Some(value)) => match value {
                        Value::Number(_) => Ok(value),
                        _ => Err(err("la fonction F1 doit renvoyer un nombre")),
                    },
                    Ok(None) => Err(err("la fonction F1 n'est pas configurée par l'hôte")),
                    Err(message) => Err(err(message)),
                }
            }
            _ if self.functions.contains_key(&name.to_ascii_lowercase()) => {
                self.call_function(name, args)
            }
            _ => {
                if let Some(value) = self.host.call_local_function(name, args).map_err(err)? {
                    Ok(value)
                } else {
                    Err(err(&format!("fonction AlgoBox inconnue `{name}`")))
                }
            }
        }
    }
    fn call_function(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        let function = self
            .functions
            .get(&name.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| err("fonction introuvable"))?;
        if args.len() != function.parameters.len() {
            return Err(err(&format!(
                "la fonction `{}` attend {} argument(s), reçu {}",
                function.name,
                function.parameters.len(),
                args.len()
            )));
        }
        if self.call_depth >= 256 {
            return Err(err("profondeur maximale de récursion dépassée"));
        }
        let mut names: Vec<String> = function
            .parameters
            .iter()
            .map(|n| n.to_ascii_lowercase())
            .collect();
        names.extend(function.declarations.keys().cloned());
        names.sort();
        names.dedup();
        let saved: HashMap<String, (Option<Value>, Option<DeclaredType>)> = names
            .iter()
            .map(|key| {
                (
                    key.clone(),
                    (
                        self.variables.get(key).cloned(),
                        self.types.get(key).copied(),
                    ),
                )
            })
            .collect();
        for (parameter, value) in function.parameters.iter().zip(args.iter().cloned()) {
            let key = parameter.to_ascii_lowercase();
            self.types.insert(key.clone(), value_type(&value));
            self.variables.insert(key, value);
        }
        for (name, ty) in &function.declarations {
            self.types.insert(name.clone(), *ty);
            self.variables.insert(name.clone(), default_value(*ty));
        }
        self.call_depth += 1;
        let result = self.block(&function.body);
        self.call_depth -= 1;
        for (key, (variable, ty)) in saved {
            if let Some(value) = variable {
                self.variables.insert(key.clone(), value);
            } else {
                self.variables.remove(&key);
            }
            if let Some(ty) = ty {
                self.types.insert(key.clone(), ty);
            } else {
                self.types.remove(&key);
            }
        }
        match result? {
            Some(value) => Ok(value),
            None => Err(err(&format!(
                "la fonction `{}` n'a pas renvoyé de valeur",
                function.name
            ))),
        }
    }
    fn binary(&self, a: Value, op: Op, b: Value) -> Result<Value, RuntimeError> {
        use Op::*;
        match op {
            Add => match (a, b) {
                (Value::Number(x), Value::Number(y)) => Ok(Value::Number(x + y)),
                (Value::Text(mut x), Value::Text(y)) => {
                    x.push_str(&y);
                    Ok(Value::Text(x))
                }
                _ => Err(err("types incompatibles pour +")),
            },
            Sub => Ok(Value::Number(as_number(a)? - as_number(b)?)),
            Mul => Ok(Value::Number(as_number(a)? * as_number(b)?)),
            Div => {
                let d = as_number(b)?;
                if d == 0.0 {
                    Err(err("division par zéro"))
                } else {
                    Ok(Value::Number(as_number(a)? / d))
                }
            }
            Mod => {
                let d = as_number(b)?;
                if d == 0.0 {
                    Err(err("reste par zéro"))
                } else {
                    Ok(Value::Number(as_number(a)? % d))
                }
            }
            Eq => Ok(Value::Boolean(a == b)),
            Ne => Ok(Value::Boolean(a != b)),
            Lt | Le | Gt | Ge => {
                let r = match (a, b) {
                    (Value::Number(x), Value::Number(y)) => match op {
                        Lt => x < y,
                        Le => x <= y,
                        Gt => x > y,
                        _ => x >= y,
                    },
                    (Value::Text(x), Value::Text(y)) => match op {
                        Lt => x < y,
                        Le => x <= y,
                        Gt => x > y,
                        _ => x >= y,
                    },
                    _ => return Err(err("comparaison de valeurs incompatibles")),
                };
                Ok(Value::Boolean(r))
            }
            And => Ok(Value::Boolean(as_bool(a)? && as_bool(b)?)),
            Or => Ok(Value::Boolean(as_bool(a)? || as_bool(b)?)),
        }
    }
    fn variable(&self, name: &str) -> Result<Value, RuntimeError> {
        self.variables
            .get(&name.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| err(&format!("variable `{name}` non déclarée")))
    }
    fn set(&mut self, name: &str, value: Value) -> Result<(), RuntimeError> {
        let key = name.to_ascii_lowercase();
        let ty = self
            .types
            .get(&key)
            .ok_or_else(|| err(&format!("variable `{name}` non déclarée")))?;
        let valid = matches!(
            (ty, &value),
            (DeclaredType::Number, Value::Number(_))
                | (DeclaredType::Text, Value::Text(_))
                | (DeclaredType::Boolean, Value::Boolean(_))
                | (DeclaredType::List, Value::List(_))
        );
        let numeric_list = !matches!(ty, DeclaredType::List)
            || matches!(&value, Value::List(items) if items.borrow().iter().all(|item| matches!(item, Value::Number(_))));
        if !valid || !numeric_list {
            return Err(err(&format!(
                "valeur incompatible avec le type de `{name}`"
            )));
        }
        self.variables.insert(key, value);
        Ok(())
    }
    fn tick(&mut self) -> Result<(), RuntimeError> {
        if self.host.should_stop() {
            return Err(err("exécution interrompue par l'hôte"));
        }
        self.instructions = self.instructions.saturating_add(1);
        Ok(())
    }

    fn note_output(&mut self) -> Result<(), RuntimeError> {
        self.outputs_since_confirmation += 1;
        if self.outputs_since_confirmation == 1_000 {
            if !self
                .host
                .confirm_continue_after_output_limit()
                .map_err(err)?
            {
                return Err(err("exécution arrêtée après 1 000 affichages"));
            }
            self.outputs_since_confirmation = 0;
        }
        Ok(())
    }

    fn tick_loop_iteration(
        &mut self,
        loop_iterations: &mut usize,
        loop_kind: &str,
    ) -> Result<(), RuntimeError> {
        if *loop_iterations >= 500_000 {
            return Err(err(format!(
                "limite de 500000 itérations {loop_kind} dépassée"
            )));
        }
        if self.loop_iterations >= 5_000_000 {
            return Err(err("limite globale de cinq millions d'itérations dépassée"));
        }
        *loop_iterations += 1;
        self.loop_iterations += 1;
        Ok(())
    }
}

fn as_number(value: Value) -> Result<f64, RuntimeError> {
    if let Value::Number(n) = value {
        Ok(n)
    } else {
        Err(err("nombre attendu"))
    }
}
fn value_type(value: &Value) -> DeclaredType {
    match value {
        Value::Number(_) => DeclaredType::Number,
        Value::Text(_) => DeclaredType::Text,
        Value::Boolean(_) => DeclaredType::Boolean,
        Value::List(_) => DeclaredType::List,
    }
}
fn default_value(ty: DeclaredType) -> Value {
    match ty {
        DeclaredType::Number => Value::Number(0.0),
        DeclaredType::Text => Value::Text(String::new()),
        DeclaredType::Boolean => Value::Boolean(false),
        DeclaredType::List => Value::List(Rc::new(RefCell::new(Vec::new()))),
    }
}
fn as_integer(value: Value) -> Result<i64, RuntimeError> {
    let n = as_number(value)?;
    if n.fract() != 0.0 || !n.is_finite() || n < i64::MIN as f64 || n >= 9_223_372_036_854_775_808.0
    {
        Err(err("entier attendu"))
    } else {
        Ok(n as i64)
    }
}
fn as_bool(value: Value) -> Result<bool, RuntimeError> {
    if let Value::Boolean(v) = value {
        Ok(v)
    } else {
        Err(err("valeur logique attendue"))
    }
}
fn as_index(value: Value) -> Result<usize, RuntimeError> {
    let n = as_integer(value)?;
    if n < 1 {
        Err(err("les indices de liste commencent à 1"))
    } else {
        Ok(n as usize)
    }
}
fn list_slot(index: usize, len: usize) -> Result<usize, RuntimeError> {
    if index == 0 || index > len {
        Err(err(&format!(
            "poste de liste {index} hors limites 1..={len}"
        )))
    } else {
        Ok(index - 1)
    }
}

fn as_index_zero_based(value: Value) -> Result<usize, RuntimeError> {
    let index = as_integer(value)?;
    if index < 0 {
        Err(err("la position de caractère ne peut pas être négative"))
    } else {
        usize::try_from(index).map_err(|_| err("position de caractère hors limites"))
    }
}

fn require_arity(name: &str, args: &[Value], expected: usize) -> Result<(), RuntimeError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(err(format!(
            "`{name}` attend {expected} argument(s), reçu {}",
            args.len()
        )))
    }
}

fn math_extremum(name: &str, key: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    let maximum = key.eq_ignore_ascii_case("MAX");
    let mut result = if maximum {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    for argument in args {
        let value = as_number(argument.clone())?;
        if value.is_nan() {
            return Ok(Value::Number(f64::NAN));
        }
        result = if maximum {
            result.max(value)
        } else {
            result.min(value)
        };
    }
    if args.is_empty() && name.ends_with(".MAX") {
        Ok(Value::Number(f64::NEG_INFINITY))
    } else if args.is_empty() && name.ends_with(".MIN") {
        Ok(Value::Number(f64::INFINITY))
    } else {
        Ok(Value::Number(result))
    }
}

fn round_to_decimal_places(value: f64, precision: f64) -> Result<f64, RuntimeError> {
    if !value.is_finite() || !precision.is_finite() {
        return Err(err("ALGOBOX_ARRONDIR exige des nombres finis"));
    }
    let digits = precision.trunc();
    if !(0.0..=100.0).contains(&digits) {
        return Err(err(
            "la précision d'ALGOBOX_ARRONDIR doit être comprise entre 0 et 100",
        ));
    }
    if value.abs() >= 1.0e21 || value == 0.0 {
        return Ok(value);
    }

    let negative = value.is_sign_negative();
    let magnitude = value.abs();
    let decimal_places = digits as usize;
    let magnitude = if is_exact_decimal_rounding_tie(magnitude, decimal_places) {
        magnitude.next_up()
    } else {
        magnitude
    };
    let fixed = format!("{magnitude:.decimal_places$}");
    let signed = if negative { format!("-{fixed}") } else { fixed };
    signed
        .parse::<f64>()
        .map_err(|_| err("ALGOBOX_ARRONDIR n'a pas pu produire un nombre"))
}

fn is_exact_decimal_rounding_tie(value: f64, decimal_places: usize) -> bool {
    if value == 0.0 || !value.is_finite() {
        return false;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (significand, binary_exponent) = if exponent == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, exponent - 1023 - 52)
    };
    let scaled_exponent = binary_exponent + decimal_places as i32;
    if scaled_exponent >= 0 {
        return false;
    }
    let denominator_power = (-scaled_exponent) as u32;
    significand.trailing_zeros().saturating_add(1) == denominator_power
}

fn parse_integer_prefix(source: &str) -> f64 {
    let source = source
        .trim_start_matches(|character: char| character.is_whitespace() || character == '\u{feff}');
    let (negative, unsigned) = match source.as_bytes().first() {
        Some(b'-') => (true, &source[1..]),
        Some(b'+') => (false, &source[1..]),
        _ => (false, source),
    };
    let (radix, digits) = if unsigned.starts_with("0x") || unsigned.starts_with("0X") {
        (16_u32, &unsigned[2..])
    } else {
        (10_u32, unsigned)
    };
    let mut value = 0.0_f64;
    let mut found_digit = false;
    for character in digits.chars() {
        let Some(digit) = character.to_digit(radix) else {
            break;
        };
        found_digit = true;
        value = value * radix as f64 + digit as f64;
    }
    if !found_digit {
        return f64::NAN;
    }
    if negative { -value } else { value }
}

fn parse_real_prefix(source: &str) -> f64 {
    let source = source
        .trim_start_matches(|character: char| character.is_whitespace() || character == '\u{feff}');
    let bytes = source.as_bytes();
    let mut end = 0;
    if matches!(bytes.first(), Some(b'+') | Some(b'-')) {
        end = 1;
    }
    let sign_end = end;
    if source
        .get(end..)
        .is_some_and(|rest| rest.starts_with("Infinity"))
    {
        let sign = &source[..sign_end];
        return if sign == "-" {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }

    let integer_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let mut has_digit = end > integer_start;
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        let fractional_start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        has_digit |= end > fractional_start;
    }
    if !has_digit {
        return f64::NAN;
    }
    let significand_end = end;
    if matches!(bytes.get(end), Some(b'e') | Some(b'E')) {
        let mut exponent_end = end + 1;
        if matches!(bytes.get(exponent_end), Some(b'+') | Some(b'-')) {
            exponent_end += 1;
        }
        let exponent_start = exponent_end;
        while bytes.get(exponent_end).is_some_and(u8::is_ascii_digit) {
            exponent_end += 1;
        }
        if exponent_end > exponent_start {
            end = exponent_end;
        } else {
            end = significand_end;
        }
    }
    source[..end].parse().unwrap_or(f64::NAN)
}

fn builtin_arity(name: &str) -> Option<usize> {
    match name {
        "SQRT"
        | "FLOOR"
        | "COS"
        | "SIN"
        | "TAN"
        | "EXP"
        | "LOG"
        | "ABS"
        | "ROUND"
        | "ACOS"
        | "ASIN"
        | "ATAN"
        | "RANDOM"
        | "ALGOBOX_LOI_NORMALE_CR"
        | "ALGOBOX_INVERSE_LOI_NORMALE_CR"
        | "ALGOBOX_FACTORIELLE"
        | "PARSEINT"
        | "PARSEFLOAT"
        | "F1" => Some(if name == "RANDOM" { 0 } else { 1 }),
        "POW"
        | "ALGOBOX_ARRONDIR"
        | "ALGOBOX_ALEA_ENT"
        | "ALGOBOX_NB_COMBINAISONS"
        | "ALGOBOX_COEFF_BINOMIAL" => Some(2),
        "ALGOBOX_LOI_BINOMIALE" | "ALGOBOX_LOI_NORMALE" | "ALGOBOX_INVERSE_LOI_NORMALE" => Some(3),
        name if is_list_summary_function(name) => Some(3),
        _ => None,
    }
}

fn is_list_summary_function(name: &str) -> bool {
    matches!(
        name,
        "ALGOBOX_SOMME"
            | "ALGOBOX_MOYENNE"
            | "ALGOBOX_VARIANCE"
            | "ALGOBOX_ECART_TYPE"
            | "ALGOBOX_MEDIANE"
            | "ALGOBOX_QUARTILE1"
            | "ALGOBOX_QUARTILE3"
            | "ALGOBOX_QUARTILE1_BIS"
            | "ALGOBOX_QUARTILE3_BIS"
            | "ALGOBOX_MINIMUM"
            | "ALGOBOX_MAXIMUM"
            | "ALGOBOX_POS_MINIMUM"
            | "ALGOBOX_POS_MAXIMUM"
    )
}

fn list_summary(name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    require_arity(name, args, 3)?;
    let Value::List(items) = &args[0] else {
        return Err(err("une liste de nombres est attendue"));
    };
    let items = items.borrow();
    let first = as_index(args[1].clone())?;
    let last = as_index(args[2].clone())?;
    let start = list_slot(first, items.len())?;
    let end = list_slot(last, items.len())?;
    if start > end {
        return Err(err("intervalle de liste inversé"));
    }
    let numbers = items[start..=end]
        .iter()
        .cloned()
        .map(as_number)
        .collect::<Result<Vec<_>, _>>()?;
    if numbers.iter().any(|number| !number.is_finite()) {
        return Err(err("les opérations statistiques exigent des nombres finis"));
    }
    let count = numbers.len();
    let sum = numbers.iter().sum::<f64>();
    let mean = sum / count as f64;
    let variance = numbers
        .iter()
        .map(|number| (number - mean).powi(2))
        .sum::<f64>()
        / count as f64;

    let result = match name {
        "ALGOBOX_SOMME" => sum,
        "ALGOBOX_MOYENNE" => mean,
        "ALGOBOX_VARIANCE" => variance,
        "ALGOBOX_ECART_TYPE" => variance.sqrt(),
        "ALGOBOX_MINIMUM" => numbers.iter().copied().fold(f64::INFINITY, f64::min),
        "ALGOBOX_MAXIMUM" => numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        "ALGOBOX_POS_MINIMUM" | "ALGOBOX_POS_MAXIMUM" => {
            let selected = if name == "ALGOBOX_POS_MINIMUM" {
                numbers
                    .iter()
                    .enumerate()
                    .min_by(|(_, left), (_, right)| left.total_cmp(right))
            } else {
                numbers
                    .iter()
                    .enumerate()
                    .max_by(|(_, left), (_, right)| left.total_cmp(right))
            }
            .map(|(index, _)| index)
            .ok_or_else(|| err("intervalle de liste vide"))?;
            return Ok(Value::Number((first + selected) as f64));
        }
        "ALGOBOX_MEDIANE" => median(numbers.clone()),
        "ALGOBOX_QUARTILE1" | "ALGOBOX_QUARTILE3" => {
            if count < 4 {
                return Err(err("les quartiles exigent au moins 4 valeurs"));
            }
            let mut sorted = numbers.clone();
            sorted.sort_by(f64::total_cmp);
            let midpoint = count / 2;
            let half = if name == "ALGOBOX_QUARTILE1" {
                &sorted[..midpoint]
            } else if count % 2 == 0 {
                &sorted[midpoint..]
            } else {
                &sorted[midpoint + 1..]
            };
            median(half.to_vec())
        }
        "ALGOBOX_QUARTILE1_BIS" | "ALGOBOX_QUARTILE3_BIS" => {
            if count < 4 {
                return Err(err("les quartiles exigent au moins 4 valeurs"));
            }
            let mut sorted = numbers;
            sorted.sort_by(f64::total_cmp);
            let fraction = if name == "ALGOBOX_QUARTILE1_BIS" {
                0.25
            } else {
                0.75
            };
            let rank = ((count as f64 * fraction).ceil() as usize).max(1);
            sorted[rank - 1]
        }
        _ => return Err(err(format!("fonction de liste inconnue `{name}`"))),
    };
    Ok(Value::Number(result))
}

fn median(mut sorted: Vec<f64>) -> f64 {
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn standard_normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.231_641_9 * x.abs());
    let density = 0.398_942_280_401_432_7 * (-0.5 * x * x).exp();
    let tail = density
        * t
        * (0.319_381_53
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    if x >= 0.0 { 1.0 - tail } else { tail }
}

fn inverse_standard_normal(probability: f64) -> Result<f64, RuntimeError> {
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(err("la probabilité doit être comprise entre 0 et 1"));
    }
    if probability == 0.0 {
        return Ok(f64::NEG_INFINITY);
    }
    if probability == 1.0 {
        return Ok(f64::INFINITY);
    }
    let (mut lower, mut upper) = (-38.0, 38.0);
    for _ in 0..128 {
        let middle = (lower + upper) / 2.0;
        if standard_normal_cdf(middle) < probability {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Ok((lower + upper) / 2.0)
}

fn err(message: impl ToString) -> RuntimeError {
    RuntimeError {
        message: message.to_string(),
    }
}
