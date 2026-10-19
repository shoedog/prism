//! Generated source cases shared by the operation census and identity oracle.
use super::*;
use crate::languages::Language;

pub(super) fn languages() -> Vec<(&'static str, Language, &'static str)> {
    vec![
        ("py", Language::Python, "def helper(x):\n    if x:\n        return x\n    return 0\ndef empty(x):\n    pass\ndef caller(x):\n    a = helper(x)\n    b = empty(x)\n    return a\nclass A:\n    def method(self, x):\n        return x\ndef free(self, x):\n    return x\ndef receivers(x):\n    a = A()\n    b = a.method(x)\n    c = free(x,x)\n    return b\n"),
        ("js", Language::JavaScript, "function helper(x) { if (x) { return x; } function nested() { return 99; } return 0; }\nfunction empty(x) { let y = x; }\nfunction caller(x) { let a = helper(x); let b = empty(x); return a; }\nfunction holes({x}, y) { return y; }\nfunction defaults(x=0, y) { return y; }\nfunction rest(x, ...y) { return x; }\nfunction callHoles(x,y) { holes(x,y); defaults(x,y); rest(x,y); }\nclass A { same(x) { return x; } } class B { same(y) { return y; } }\nfunction ambiguous(a,x) { let v = a.same(x); return v; }\nfunction methods(x) { let a = new A(); let b = new B(); let y = a.same(x); let z = b.same(x); return y; }\n"),
        ("ts", Language::TypeScript, "function helper(x: number) { if (x) { return x; } function nested() { return 99; } return 0; }\nfunction empty(x: number) { let y = x; }\nfunction caller(x: number) { let a = helper(x); let b = empty(x); return a; }\nfunction holes({x}: {x:number}, y: number) { return y; }\nfunction optional(x?: number, y?: number) { return y; }\nfunction callHoles(x:number,y:number) { holes(x,y); optional(x,y); }\nclass A { same(x: number) { return x; } } class B { same(y: number) { return y; } }\n"),
        ("tsx", Language::Tsx, "function helper(x: number) { if (x) { return x; } return 0; }\nfunction empty(x: number) {}\nfunction caller(x: number) { let a = helper(x); let b = empty(x); return a; }\n"),
        ("go", Language::Go, "package sample\nfunc helper(x int) int { if x > 0 { return x }; return 0 }\nfunc empty(x int) {}\nfunc pair(x int) (int,int) { return x, 0 }\nfunc named(x int) (y int) { y = x; return }\nfunc caller(x int) int { a := helper(x); empty(x); b,c := pair(x); d := named(x); return a+b+c+d }\n"),
        ("java", Language::Java, "class Sample {\n static int helper(int x) { if (x > 0) { return x; } return 0; }\n static void empty(int x) {}\n int caller(int x) { int a = helper(x); empty(x); return a; }\n}\n"),
        ("c", Language::C, "int helper(int x) { if(x) { return x; } return 0; }\nvoid empty(int x) {}\nint caller(int x) { int a = helper(x); empty(x); return a; }\n"),
        ("cpp", Language::Cpp, "int helper(int x) { if(x) { return x; } return 0; }\nvoid empty(int x) {}\nint caller(int x) { int a = helper(x); empty(x); return a; }\n"),
        ("rs", Language::Rust, "fn helper(x: i32) -> i32 { if x > 0 { return x; } x + 1 }\nfn empty(x: i32) { let _y = x; }\nfn caller(x: i32) -> i32 { let a = helper(x); let b = empty(x); let c = branches(x); let d = matches(x); a+c+d }\nfn branches(x: i32) -> i32 { if x > 0 { x } else { 0 } }\nfn matches(x: i32) -> i32 { match x { 0 => 1, _ => x } }\n"),
        ("lua", Language::Lua, "function helper(x)\n if x then return x end\n return 0\nend\nfunction empty(x)\n local y = x\nend\nfunction caller(x)\n local a = helper(x)\n local b = empty(x)\n return a\nend\n"),
        ("sh", Language::Bash, "helper() { echo \"$1\"; }\ncaller() { local x=1; local a=$(helper \"$x\"); helper \"$x\"; }\n"),
        ("tf", Language::Terraform, "variable \"x\" { default = 1 }\nlocals { a = helper(var.x) }\n"),
        ("tf", Language::Terraform, "helper { value = 1 }\nlocals { a = helper(1) }\nother { b = helper(2) }\n"),
    ]
}

pub(super) fn parsed(ext: &str, language: Language, source: &str) -> BTreeMap<String, ParsedFile> {
    let path = format!("case.{ext}");
    BTreeMap::from([(
        path.clone(),
        ParsedFile::parse(&path, source, language).unwrap(),
    )])
}
