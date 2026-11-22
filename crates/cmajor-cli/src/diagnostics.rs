use cmajor_lang::{Diagnostic, Program, resolver::UnitId};

pub fn eprint(program: &Program, diagnostics: &[(UnitId, Diagnostic)]) {
    for (unit, diagnostic) in diagnostics {
        let unit_name = program[*unit].source.name().unwrap_or("<unknown>");
        eprintln!("{unit_name}:{}", diagnostic.in_unit(*unit, program));
    }
}
