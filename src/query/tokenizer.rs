#[derive(Debug, Clone)]
pub enum Token {
    Field(String),
    Operator(String),
    StringLiteral(String),
    NumberLiteral(String),
    LParen,
    RParen,
    Comma,
    Function(String),
    And,
    Or,
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.pos < self.input.len() {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }
            let ch = self.input[self.pos];
            if ch.is_ascii_digit() {
                tokens.push(self.read_number());
            } else if ch == '\'' || ch == '"' {
                tokens.push(self.read_string());
            } else if ch == '(' {
                tokens.push(Token::LParen);
                self.pos += 1;
            } else if ch == ')' {
                tokens.push(Token::RParen);
                self.pos += 1;
            } else if ch.is_alphabetic() || ch == '_' {
                tokens.push(self.read_identifier());
            } else if ch == '=' || ch == '!' || ch == '>' || ch == '<' {
                tokens.push(self.read_operator());
            } else if ch == ',' {
                tokens.push(Token::Comma);
                self.pos += 1;
            } else {
                self.pos += 1;
            }
        }
        tokens.push(Token::EOF);
        tokens
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input[self.pos].is_ascii_digit() || self.input[self.pos] == '.')
        {
            self.pos += 1;
        }
        Token::NumberLiteral(self.input[start..self.pos].iter().collect())
    }

    fn read_string(&mut self) -> Token {
        let quote = self.input[self.pos];
        self.pos += 1;
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos] != quote {
            self.pos += 1;
        }
        let val = self.input[start..self.pos].iter().collect();
        self.pos += 1;
        Token::StringLiteral(val)
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len()
            && (self.input[self.pos].is_alphanumeric()
                || self.input[self.pos] == '_'
                || self.input[self.pos] == '.')
        {
            self.pos += 1;
        }
        let ident: String = self.input[start..self.pos].iter().collect();

        if ident == "has" {
            return Token::Function(ident);
        }
        if ident == "exists" {
            return Token::Function(ident);
        }

        if ident == "and" {
            return Token::And;
        }
        if ident == "or" {
            return Token::Or;
        }

        Token::Field(ident)
    }

    fn read_operator(&mut self) -> Token {
        let start = self.pos;
        let ch = self.input[self.pos];
        self.pos += 1;

        if self.pos < self.input.len() {
            let next = self.input[self.pos];
            if (ch == '=' && next == '=')
                || (ch == '!' && next == '=')
                || (ch == '>' && next == '=')
                || (ch == '<' && next == '=')
                || (ch == '=' && next == '~')
            {
                self.pos += 1;
                return Token::Operator(self.input[start..self.pos].iter().collect());
            }
        }

        Token::Operator(ch.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_field_tokenization() {
        let mut lexer = Lexer::new("file.name");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0], Token::Field(ref f) if f == "file.name"));
        assert!(matches!(tokens[1], Token::EOF));
    }

    #[test]
    fn test_equality_operator() {
        let mut lexer = Lexer::new("file.name == 'readme'");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0], Token::Field(ref f) if f == "file.name"));
        assert!(matches!(tokens[1], Token::Operator(ref o) if o == "=="));
        assert!(matches!(tokens[2], Token::StringLiteral(ref s) if s == "readme"));
        assert!(matches!(tokens[3], Token::EOF));
    }

    #[test]
    fn test_all_comparison_operators() {
        let operators = vec!["==", "!=", ">", "<", ">=", "<=", "=~"];
        for op in operators {
            let query = format!("file.size {} 100", op);
            let mut lexer = Lexer::new(&query);
            let tokens = lexer.tokenize();
            assert!(
                matches!(tokens[1], Token::Operator(ref o) if o == op),
                "Failed for operator: {}",
                op
            );
        }
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = Lexer::new("'hello world' \"double quotes\"");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0], Token::StringLiteral(ref s) if s == "hello world"));
        assert!(matches!(tokens[1], Token::StringLiteral(ref s) if s == "double quotes"));
    }

    #[test]
    fn test_number_literals() {
        let mut lexer = Lexer::new("123 45.67");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 3);
        assert!(matches!(tokens[0], Token::NumberLiteral(ref n) if n == "123"));
        assert!(matches!(tokens[1], Token::NumberLiteral(ref n) if n == "45.67"));
    }

    #[test]
    fn test_logical_operators() {
        let mut lexer = Lexer::new("a and b or c");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 6);
        assert!(matches!(tokens[0], Token::Field(ref f) if f == "a"));
        assert!(matches!(tokens[1], Token::And));
        assert!(matches!(tokens[2], Token::Field(ref f) if f == "b"));
        assert!(matches!(tokens[3], Token::Or));
        assert!(matches!(tokens[4], Token::Field(ref f) if f == "c"));
    }

    #[test]
    fn test_function_tokenization() {
        let mut lexer = Lexer::new("has(note.tags, 'important')");
        let tokens = lexer.tokenize();
        // has ( note.tags , 'important' ) EOF = 7 tokens
        assert_eq!(tokens.len(), 7);
        assert!(matches!(tokens[0], Token::Function(ref f) if f == "has"));
        assert!(matches!(tokens[1], Token::LParen));
        assert!(matches!(tokens[2], Token::Field(ref f) if f == "note.tags"));
        assert!(matches!(tokens[3], Token::Comma));
        assert!(matches!(tokens[4], Token::StringLiteral(ref s) if s == "important"));
        assert!(matches!(tokens[5], Token::RParen));
        assert!(matches!(tokens[6], Token::EOF));
    }

    #[test]
    fn test_exists_function_tokenization() {
        let mut lexer = Lexer::new("exists(note.category)");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[0], Token::Function(ref f) if f == "exists"));
        assert!(matches!(tokens[1], Token::LParen));
        assert!(matches!(tokens[2], Token::Field(ref f) if f == "note.category"));
        assert!(matches!(tokens[3], Token::RParen));
        assert!(matches!(tokens[4], Token::EOF));
    }

    #[test]
    fn test_parentheses() {
        let mut lexer = Lexer::new("(a == 1)");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 6);
        assert!(matches!(tokens[0], Token::LParen));
        assert!(matches!(tokens[4], Token::RParen));
    }

    #[test]
    fn test_complex_query() {
        let query = "file.name == 'readme' and file.size > 1000 or has(note.tags, 'todo')";
        let mut lexer = Lexer::new(query);
        let tokens = lexer.tokenize();
        assert!(tokens.len() > 10);
        assert!(matches!(tokens[0], Token::Field(ref f) if f == "file.name"));
        assert!(matches!(tokens[2], Token::StringLiteral(ref s) if s == "readme"));
        assert!(matches!(tokens[3], Token::And));
        assert!(matches!(tokens[4], Token::Field(ref f) if f == "file.size"));
        assert!(matches!(tokens[6], Token::NumberLiteral(ref n) if n == "1000"));
        assert!(matches!(tokens[7], Token::Or));
        assert!(matches!(tokens[8], Token::Function(ref f) if f == "has"));
    }

    #[test]
    fn test_note_namespace() {
        let mut lexer = Lexer::new("note.content");
        let tokens = lexer.tokenize();
        assert!(matches!(tokens[0], Token::Field(ref f) if f == "note.content"));
    }

    #[test]
    fn test_shorthand_property() {
        let mut lexer = Lexer::new("category == 'project'");
        let tokens = lexer.tokenize();
        assert!(matches!(tokens[0], Token::Field(ref f) if f == "category"));
    }

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0], Token::EOF));
    }

    #[test]
    fn test_whitespace_handling() {
        let mut lexer = Lexer::new("  file.name   ==    'test'  ");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 4);
        assert!(matches!(tokens[0], Token::Field(ref f) if f == "file.name"));
        assert!(matches!(tokens[1], Token::Operator(ref o) if o == "=="));
        assert!(matches!(tokens[2], Token::StringLiteral(ref s) if s == "test"));
    }
}
