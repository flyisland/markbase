use super::tokenizer::{Lexer, Token};

#[derive(Debug, Clone)]
pub enum AstNode {
    Binary {
        left: Box<AstNode>,
        op: String,
        right: Box<AstNode>,
    },
    Field(String),
    StringLiteral(String),
    NumberLiteral(String),
    FunctionCall {
        name: String,
        args: Vec<AstNode>,
    },
    Grouping(Box<AstNode>),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> AstNode {
        self.parse_or()
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        self.pos += 1;
        token
    }

    fn parse_or(&mut self) -> AstNode {
        let mut left = self.parse_and();

        while matches!(self.current(), Token::Or) {
            self.advance();
            let right = self.parse_and();
            left = AstNode::Binary {
                left: Box::new(left),
                op: "OR".to_string(),
                right: Box::new(right),
            };
        }

        left
    }

    fn parse_and(&mut self) -> AstNode {
        let mut left = self.parse_comparison();

        while matches!(self.current(), Token::And) {
            self.advance();
            let right = self.parse_comparison();
            left = AstNode::Binary {
                left: Box::new(left),
                op: "AND".to_string(),
                right: Box::new(right),
            };
        }

        left
    }

    fn parse_comparison(&mut self) -> AstNode {
        let left = self.parse_primary();

        if let Token::Operator(op) = self.current().clone() {
            if ["==", "!=", ">", "<", ">=", "<=", "=~", "="].contains(&op.as_str()) {
                self.advance();
                let right = self.parse_primary();
                return AstNode::Binary {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                };
            }
        }

        left
    }

    fn parse_primary(&mut self) -> AstNode {
        match self.current().clone() {
            Token::LParen => {
                self.advance();
                let expr = self.parse_or();
                self.advance();
                AstNode::Grouping(Box::new(expr))
            }
            Token::Function(name) => {
                self.advance();
                if let Token::LParen = self.current().clone() {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.current(), Token::RParen) {
                        args.push(self.parse_primary());
                        if matches!(self.current(), Token::RParen) {
                            break;
                        }
                        // Skip comma if present
                        if matches!(self.current(), Token::Comma) {
                            self.advance();
                        }
                    }
                    self.advance();
                    return AstNode::FunctionCall { name, args };
                }
                AstNode::FunctionCall { name, args: vec![] }
            }
            Token::Field(name) => {
                self.advance();
                AstNode::Field(name)
            }
            Token::StringLiteral(val) => {
                self.advance();
                AstNode::StringLiteral(val)
            }
            Token::NumberLiteral(val) => {
                self.advance();
                AstNode::NumberLiteral(val)
            }
            _ => AstNode::StringLiteral(String::new()),
        }
    }
}

pub fn parse(query: &str) -> AstNode {
    let mut lexer = Lexer::new(query);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_field() {
        let ast = parse("file.name");
        assert!(matches!(ast, AstNode::Field(ref f) if f == "file.name"));
    }

    #[test]
    fn test_parse_equality_comparison() {
        let ast = parse("file.name == 'readme'");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert!(matches!(*left, AstNode::Field(ref f) if f == "file.name"));
                assert_eq!(op, "==");
                assert!(matches!(*right, AstNode::StringLiteral(ref s) if s == "readme"));
            }
            _ => panic!("Expected Binary node"),
        }
    }

    #[test]
    fn test_parse_numeric_comparison() {
        let ast = parse("file.size > 1000");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert!(matches!(*left, AstNode::Field(ref f) if f == "file.size"));
                assert_eq!(op, ">");
                assert!(matches!(*right, AstNode::NumberLiteral(ref n) if n == "1000"));
            }
            _ => panic!("Expected Binary node"),
        }
    }

    #[test]
    fn test_parse_and_operator() {
        let ast = parse("a == 1 and b == 2");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert_eq!(op, "AND");
                assert!(matches!(*left, AstNode::Binary { .. }));
                assert!(matches!(*right, AstNode::Binary { .. }));
            }
            _ => panic!("Expected Binary node with AND"),
        }
    }

    #[test]
    fn test_parse_or_operator() {
        let ast = parse("a == 1 or b == 2");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert_eq!(op, "OR");
                assert!(matches!(*left, AstNode::Binary { .. }));
                assert!(matches!(*right, AstNode::Binary { .. }));
            }
            _ => panic!("Expected Binary node with OR"),
        }
    }

    #[test]
    fn test_parse_and_or_precedence() {
        let ast = parse("a == 1 and b == 2 or c == 3");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert_eq!(op, "OR");
                assert!(matches!(*left, AstNode::Binary { ref op, .. } if op == "AND"));
                assert!(matches!(*right, AstNode::Binary { .. }));
            }
            _ => panic!("Expected OR at top level"),
        }
    }

    #[test]
    fn test_parse_grouping() {
        let ast = parse("(a == 1)");
        match ast {
            AstNode::Grouping(expr) => {
                assert!(matches!(*expr, AstNode::Binary { .. }));
            }
            _ => panic!("Expected Grouping node"),
        }
    }

    #[test]
    fn test_parse_complex_grouping() {
        let ast = parse("(a == 1 or b == 2) and c == 3");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert_eq!(op, "AND");
                assert!(matches!(*left, AstNode::Grouping(_)));
                assert!(matches!(*right, AstNode::Binary { .. }));
            }
            _ => panic!("Expected Binary node"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let ast = parse("has(note.tags, 'important')");
        match ast {
            AstNode::FunctionCall { name, args } => {
                assert_eq!(name, "has");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], AstNode::Field(ref f) if f == "note.tags"));
                assert!(matches!(args[1], AstNode::StringLiteral(ref s) if s == "important"));
            }
            _ => panic!("Expected FunctionCall node"),
        }
    }

    #[test]
    fn test_parse_all_operators() {
        let operators = vec!["==", "!=", ">", "<", ">=", "<=", "=~", "="];
        for op in operators {
            let query = format!("file.size {} 100", op);
            let ast = parse(&query);
            match ast {
                AstNode::Binary { op: parsed_op, .. } => {
                    assert_eq!(parsed_op, op, "Operator {} was not parsed correctly", op);
                }
                _ => panic!("Expected Binary node for operator {}", op),
            }
        }
    }

    #[test]
    fn test_parse_pattern_match() {
        let ast = parse("file.name =~ '%test%'");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert!(matches!(*left, AstNode::Field(ref f) if f == "file.name"));
                assert_eq!(op, "=~");
                assert!(matches!(*right, AstNode::StringLiteral(ref s) if s == "%test%"));
            }
            _ => panic!("Expected Binary node"),
        }
    }

    #[test]
    fn test_parse_nested_function_calls() {
        let ast = parse("has(note.tags, 'a') and has(note.links, 'b')");
        match ast {
            AstNode::Binary { left, op, right } => {
                assert_eq!(op, "AND");
                assert!(matches!(*left, AstNode::FunctionCall { .. }));
                assert!(matches!(*right, AstNode::FunctionCall { .. }));
            }
            _ => panic!("Expected Binary node with AND"),
        }
    }
}
