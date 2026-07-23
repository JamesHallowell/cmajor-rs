use cmajor_lang::lexer::tokenize;

const SOURCE: &str = r#"
processor Gain
{
    input stream float in;
    output stream float out;
    input event float volume [[ name: "Volume", min: 0.0f, max: 1.0f, init: 1.0f ]];

    float gain = 1.0f;

    event volume (float v)
    {
        gain = v;
    }

    void main()
    {
        loop
        {
            out <- in * gain;
            advance();
        }
    }
}
"#;

fn main() {
    let mut pos = 0usize;

    for token in tokenize(SOURCE) {
        let start = pos;
        pos += token.len as usize;
        let text = &SOURCE[start..pos];

        if token.kind.is_trivia() {
            continue;
        }

        println!(
            "{:>4}..{:<4} {:<15} {:?}",
            start,
            pos,
            format!("{:?}", token.kind),
            text
        );
    }
}
