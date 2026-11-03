//! #622 S1: language gating of the legacy name-fragment list outside JS/TS, Python and Go.

use super::taint_sink_rules_support::*;
use crate::common::*;

#[test]
fn kernel_get_user_is_exact_and_c_only() {
    let c_pos = [Case {
        label: "C get_user",
        source:
            "int h(int *p, int in) {\n  int x = in; // SEED\n  return get_user(x, p); // SINK\n}\n",
    }];
    let c_neg = [Case {
        label: "C get_user_id",
        source: "int h(int in) {\n  int x = in; // SEED\n  return get_user_id(x); // SINK\n}\n",
    }];
    assert_all_sinks("a.c", Language::C, &c_pos);
    assert_no_sinks("a.c", Language::C, &c_neg);
}

#[test]
fn bash_only_patterns_stay_for_bash_and_leave_other_languages() {
    let bash = [
        Case {
            label: "bash eval",
            source: "f() {\n  local x=\"$1\" # SEED\n  eval \"$x\" # SINK\n}\n",
        },
        Case {
            label: "bash source",
            source: "f() {\n  local x=\"$1\" # SEED\n  source \"$x\" # SINK\n}\n",
        },
        Case {
            label: "bash find",
            source: "f() {\n  local x=\"$1\" # SEED\n  find \"$x\" -name a # SINK\n}\n",
        },
    ];
    assert_all_sinks("a.sh", Language::Bash, &bash);
    let java = [Case { label: "java variable named source", source: "class A {\n  void h(String in) {\n    String x = in; // SEED\n    String source = x; // SINK\n  }\n}\n" }];
    assert_no_sinks("A.java", Language::Java, &java);
}

#[test]
fn c_buffer_sinks_are_unchanged() {
    let c = [
        Case {
            label: "strcpy",
            source:
                "void h(char *d, char *in) {\n  char *x = in; // SEED\n  strcpy(d, x); // SINK\n}\n",
        },
        Case {
            label: "system",
            source: "void h(char *in) {\n  char *x = in; // SEED\n  system(x); // SINK\n}\n",
        },
    ];
    assert_all_sinks("a.c", Language::C, &c);
}
