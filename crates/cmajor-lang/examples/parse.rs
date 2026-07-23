use cmajor_lang::{ast, parser};

const SOURCE: &str = r#"
let x = 1 + 2 * 3;
var y;

if (x > 5)
{
    y = x - 1;
}
else
{
    y = x + 1;
}

while (y > 0)
{
    y = y - 1;
}

loop (4)
{
    advance();
}

return x + y;
"#;

fn main() {
    let parser::Parse { ast, root, tokens } = parser::parse(SOURCE);

    println!("{} tokens, {} AST nodes\n", tokens.len(), ast.len());
    print!("{}", ast::dump(&ast, &tokens, SOURCE, root));
}
