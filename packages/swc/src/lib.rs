use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};
use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::{Expr, Lit, Program, Str},
        transforms::testing::test,
        visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
    },
};

pub struct TransformVisitor;

impl VisitMut for TransformVisitor {
    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);

        if let Expr::TaggedTpl(tpl) = expr {
            if let Expr::Ident(ident) = tpl.tag.as_mut() {
                if ident.sym.eq("md") {
                    if tpl.tpl.quasis.len() != 1 {
                        panic!("md`` template literal shouldn't have any expressions inside");
                    }
                    *expr = Lit::Str(Str {
                        span: DUMMY_SP,
                        value: markdown::to_html(&tpl.tpl.quasis.first().unwrap().cooked.as_ref().unwrap().to_string()).into(),
                        raw: None,
                    })
                    .into()
                }
            }
        }
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    program.fold_with(&mut as_folder(TransformVisitor))
}

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_markdown_paragraph,
    r#"console.log(md`foo`);"#,
    r#"console.log("<p>foo</p>");"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_markdown_bold,
    r#"console.log(md`**foo**`);"#,
    r#"console.log("<p><strong>foo</strong></p>");"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    processes_escaped_markdown,
    r#"console.log(md`**\`foo\`**`);"#,
    r#"console.log("<p><strong><code>foo</code></strong></p>");"#
);
