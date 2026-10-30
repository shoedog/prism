use crate::languages::Language;

pub(super) fn non_js() -> Vec<(&'static str, Language, String)> {
    let mut go = String::from("package fixture\n");
    for i in 0..64 {
        go += &format!("var name{i} = {i}\n");
    }
    go += "func owner(this int, obj T) int {\n x := this\n { x := 2; consume(x) }\n obj.field = x\n consume(obj.field)\n return x\n}\n";
    vec![
        ("fixture.go", Language::Go, go),
        ("fixture.py", Language::Python, "global_name = 1\ndef owner(this, obj):\n    x = this\n    with obj as resource:\n        consume(x, resource)\n    def nested(x):\n        return x\n    obj.field = x\n    return obj.field + x\n".into()),
        ("fixture.rs", Language::Rust, "static TOP: i32 = 1;\nfn owner(this: i32, obj: T) -> i32 {\n let x = this;\n { let x = 2; consume(x); }\n obj.field = x;\n consume(obj.field);\n x\n}\n".into()),
        ("fixture.java", Language::Java, "class x {\n int field;\n int owner(int x, x obj) {\n  this.field = x;\n  { int y = x; consume(y); }\n  obj.field = x;\n  consume(obj.field);\n  return x;\n }\n}\n".into()),
        ("fixture.c", Language::C, "int top, other, more;\nint owner(int this, struct T *obj) {\n int x = this;\n { int x = 2; consume(x); }\n obj->field = x;\n consume(obj->field);\n return x;\n}\n".into()),
        ("fixture.cpp", Language::Cpp, "int top, other, more;\nstruct T { int field; int owner(int x) {\n this->field = x;\n { int x = 2; consume(x); }\n auto nested = [x]() { return x; };\n consume(this->field);\n return x;\n}};\n".into()),
        ("fixture.lua", Language::Lua, "local top, other, more = 1, 2, 3\nfunction owner(this, obj)\n local x = this\n do local x = 2; consume(x) end\n local function nested(x) return x end\n obj.field = x\n consume(obj.field)\n return x\nend\n".into()),
        ("fixture.sh", Language::Bash, "top=1\nowner() {\n local this=$1\n local x=$this\n if true; then local x=2; consume \"$x\"; fi\n consume \"${this}\"\n printf '%s' \"$x\"\n}\n".into()),
        ("fixture.tf", Language::Terraform, "variable \"this\" { default = 1 }\nlocals {\n top = var.this\n names = [for this in [1, 2] : this + var.this]\n member = var.obj.field\n}\nresource \"example\" \"owner\" {\n value = local.top\n nested { value = local.member }\n}\n".into()),
    ]
}

pub(super) fn js() -> Vec<(&'static str, Language, &'static str)> {
    let common = "function owner(x, q = x.field) {\n var x;\n { let x = q; consume(x); }\n x.field = q;\n consume(x.field);\n consume((x) => {\n x.field = 2; return x.field; });\n consume(function(x = q) { var x; return x; });\n try { consume(x); } catch(x) { consume(x); }\n}\nfunction receivers() {\n this.field = 1;\n consume(() => this.field);\n consume(function() { return this.field; });\n}\nfunction dynamic(x, obj) { with (obj) { consume(x); } return x; }\n";
    vec![
        ("fixture.js", Language::JavaScript, common),
        ("fixture.ts", Language::TypeScript, common),
        ("fixture.tsx", Language::Tsx, common),
        (
            "fixture.jsx",
            Language::JavaScript,
            "const owner = (x) => {\n x.field = 1;\n return <View value={x.field}/>; };",
        ),
        (
            "fixture.mjs",
            Language::JavaScript,
            "export const owner = (x) => {\n x.field = 1;\n return x.field; };",
        ),
        (
            "fixture.cjs",
            Language::JavaScript,
            "exports.owner = function(x) {\n x.field = 1;\n return x.field; };",
        ),
    ]
}
