// Shared inputs keep CLI, MCP, and library classification checks on the same lines.
pub const CLASS_CASES: &[(&str, &str, &[usize])] = &[
    (
        "a.py",
        "@dataclass\nclass Location:\n    x: int\n\n    @staticmethod\n    def method():\n        thunk = lambda: 1\n        return thunk()\n",
        &[1, 2, 3, 4],
    ),
    (
        "a.py",
        "@attr.s\nclass Location:\n    x: int\n\n    def method(self):\n        return self.x\n",
        &[1, 2, 3, 4],
    ),
    (
        "a.cpp",
        "template<class T>\nstruct S {\n    T x;\n\n    T get() {\n        return x;\n    }\n};\n",
        &[1, 2, 3, 4, 8],
    ),
    ("a.py", "class Location:\n    x: int\n", &[1, 2]),
    ("a.cpp", "struct S {\n    int x;\n};\n", &[1, 2, 3]),
];

pub const CALLABLE_CONTROLS: &[(&str, &str, &[usize], &str)] = &[
    (
        "a.py",
        "@decorator\ndef wrapped():\n    return 1\n",
        &[1, 2, 3],
        "wrapped",
    ),
    // A lambda inside a decorated method still resolves to the named method.
    ("a.py", CLASS_CASES[0].1, &[5, 6, 7, 8], "method"),
    ("a.py", CLASS_CASES[1].1, &[5, 6], "method"),
    (
        "a.cpp",
        "template<class T>\nT identity(T x) {\n    return x;\n}\n",
        &[1, 2, 3, 4],
        "identity",
    ),
    ("a.cpp", CLASS_CASES[2].1, &[5, 6, 7], "get"),
];
