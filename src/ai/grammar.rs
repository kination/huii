
pub const RUST_GBNF: &str = r##"
root ::= ( ws? item ws? )+

ws ::= [ \t\n]+
comment ::= "//" [^\n]*

item ::= attribute | macro_use | decl | comment

attribute ::= "#[" [^\]]* "]"
macro_use ::= [a-z0-9_]+ "!" [^;{]* "{" nested "}"

# Regular declarations
decl ::= ( "pub" ws )? ( "async" ws )? ( "unsafe" ws )? keyword [^;{]* ( ";" | "{" nested "}" )
keyword ::= "fn" | "struct" | "enum" | "use" | "impl" | "mod" | "type" | "const" | "static" | "trait" 

nested ::= ( content | "{" nested "}" | "(" parens ")" | "[" brackets "]" )*
content ::= [^}{"'/\[\]()]+ | string | char | comment | "/" 

parens ::= ( content | "{" nested "}" | "(" parens ")" | "[" brackets "]" )*
brackets ::= ( content | "{" nested "}" | "(" parens ")" | "[" brackets "]" )*

string ::= "\"" ( [^"\\] | "\\" . )* "\""
char ::= "'" ( [^'\\] | "\\" . ) "'"
"##;
