use super::Generator;
use super::super::AST;

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use crate::error::*;
use crate::parser::*;


pub struct CFEngine {
    current_cases: Vec<String>, //TODO
    // match enum variables with class prefixes
    var_prefixes: HashMap<String,String>,
    // already used class prefix
    prefixes: HashMap<String,u32>,
}

impl CFEngine {
    pub fn new() -> CFEngine {
        CFEngine {
            current_cases: Vec::new(),
            var_prefixes: HashMap::new(),
            prefixes: HashMap::new(),
        }
    }

    fn new_var(&mut self, prefix: &str) {
        let id = self.prefixes.get(prefix).unwrap_or(&0) + 1;
        self.prefixes.insert(prefix.to_string(), id);
        let var = format!("{}{}", prefix, id);
        self.var_prefixes.insert(prefix.to_string(), var);
    }
    fn reset_cases(&mut self) {
        self.current_cases = Vec::new();
    }
    fn reset_context(&mut self) {
        self.var_prefixes = HashMap::new();
    }

    fn parameter_to_cfengine(&mut self, param: &PValue) -> String {
        match param {
            PValue::String(s) => format!("\"{}\"", s),
            _ => "XXX".to_string(), // TODO remove _
        }
    }

    fn format_case(&mut self, gc: &AST, case: &PEnumExpression) -> String {
        let expr = self.format_case_expr(gc, case);
        let result = format!("    {}::\n",&expr);
        self.current_cases.push(expr);
        result
    }
    fn format_case_expr(&mut self, gc: &AST, case: &PEnumExpression) -> String {
        match case {
            PEnumExpression::And(e1, e2) => format!("({}).({})", self.format_case_expr(gc,e1), self.format_case_expr(gc,e2)),
            PEnumExpression::Or(e1, e2) => format!("({})|({})", self.format_case_expr(gc,e1), self.format_case_expr(gc,e2)),
            PEnumExpression::Not(e1) => format!("!({})", self.format_case_expr(gc,e1)),
            PEnumExpression::Compare(var, e, item) => {
                "TODO".to_string()
//                let e1 = e.unwrap();
//                if gc.enumlist.is_global(e1) {
//                    // find global class with exception
//                    "TODO".to_string()
//                } else {
//                    // concat var name + item
//                    let prefix = self.prefixes[var.unwrap().fragment()];
//                    // TODO there may still be some conflicts with var or enum containing '_'
//                    format!("{}_{}_{}", prefix, e1.fragment(), item.fragment())
//                }
            },
            PEnumExpression::Default => {
                // extract current cases and build an opposite expression
                if self.current_cases.is_empty() {
                    "any".to_string()
                } else {
                    self.current_cases
                        .iter()
                        .map(|x| format!("!({})",x))
                        .collect::<Vec<_>>()
                        .join(".")
                }
            },
        }
    }

    // TODO underscore escapement
    fn format_statement(&mut self, gc: &AST, st: &PStatement) -> String {
        match st {
            PStatement::StateCall(out, mode, res, call, params) => {
                if let Some(var) = out {
                    self.new_var(var);
                }
                // TODO setup mode and output var by calling ... bundle
                let param_str = res
                    .parameters
                    .iter()
                    .chain(params.iter())
                    .map(|x| self.parameter_to_cfengine(x))
                    .collect::<Vec<String>>()
                    .join(",");
                format!(
                    "      \"method_call\" usebundle => {}_{}({});\n",
                    res.name.fragment(),
                    call.fragment(),
                    param_str
                )
            },
            PStatement::Case(vec) => {
                self.reset_cases();
                vec .iter()
                    .map(|(case, vst)| {
                        format!(
                            "{}{}",
                            self.format_case(gc, case),
                            vst.iter()
                                .map(|st| self.format_statement(gc, st))
                                .collect::<Vec<String>>()
                                .join("")
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("")
            },
            _ => String::new(), // TODO ?
        }
    }
}

impl Generator for CFEngine {
    // TODO generate only one file
    fn generate_one(&mut self, gc: &AST, file: &str) -> Result<()> { Ok(()) }

    fn generate_all(&mut self, gc: &AST) -> Result<()> {
        let mut files: HashMap<&str, String> = HashMap::new();
        for (rn, res) in gc.resources.iter() {
            for (sn, state) in res.states.iter() {
                self.reset_context();
                let mut content = match files.get(sn.file()) {
                    Some(s) => s.to_string(),
                    None => String::new(),
                };
                let params = res
                    .parameters
                    .iter()
                    .chain(state.parameters.iter())
                    .map(|p| p.name.fragment())
                    .collect::<Vec<&str>>()
                    .join(",");
                content.push_str(&format!(
                    "bundle agent {}_{} ({})\n",
                    rn.fragment(),
                    sn.fragment(),
                    params
                ));
                content.push_str("{\n  methods:\n");
                /*for st in state.statements.iter() {
                    content.push_str(&self.format_statement(gc, st));
                }*/
                content.push_str("}\n");
                files.insert(sn.file(), content.to_string()); // TODO there is something smelly with this to_string
            }
        }
        for (name, content) in files.iter() {
            let mut file = File::create(format!("{}.cf", name)).unwrap();
            file.write_all(content.as_bytes()).unwrap();
        }
        Ok(())
    }
}