//! #622 S1 hybrid-policy matrix, C/C++/Lua/Rust/Bash/Java: a sink needs call position; only proven non-sinks are
//! suppressed, an unknown receiver keeps firing. Rows marked `probe:` replay the review r1 harness
//! (expectation = the pre-change result unless a drop is deliberate).

use super::taint_sink_rules_support::*;
use crate::common::*;

#[rustfmt::skip]
const OTHER_ROWS: &[Row] = &[
    Row { sink: true, label: "C sqlite3_prepare_v2", path: "a.c", lang: Language::C, source: "void h(sqlite3 *db, struct in *in) {\n  char *x = in->v; // SEED\n  sqlite3_prepare_v2(db, x, -1, &st, 0); // SINK\n}\n" },
    Row { sink: true, label: "C++ sqlite3_prepare_v2", path: "a.cpp", lang: Language::Cpp, source: "void h(sqlite3 *db, In *in) {\n  char *x = in->v; // SEED\n  sqlite3_prepare_v2(db, x, -1, &st, 0); // SINK\n}\n" },
    Row { sink: true, label: "C chmod", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  chmod(x, 0777); // SINK\n}\n" },
    Row { sink: true, label: "C chown", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  chown(x, 0, 0); // SINK\n}\n" },
    Row { sink: true, label: "C libcurl setopt URL", path: "a.c", lang: Language::C, source: "void h(CURL *curl, struct in *in) {\n  char *x = in->v; // SEED\n  curl_easy_setopt(curl, CURLOPT_URL, x); // SINK\n}\n" },
    Row { sink: true, label: "C fw_setenv", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  fw_setenv(\"a\", x); // SINK\n}\n" },
    Row { sink: true, label: "C mtd", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  flash(mtd, x); // SINK\n}\n" },
    Row { sink: true, label: "C system", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  system(x); // SINK\n}\n" },
    Row { sink: true, label: "C++ chmod", path: "a.cpp", lang: Language::Cpp, source: "void h(In *in) {\n  const char *x = in->v; // SEED\n  chmod(x, 0777); // SINK\n}\n" },
    Row { sink: true, label: "C get_user", path: "a.c", lang: Language::C, source: "long h(int __user *p) {\n  int x = *p; // SEED\n  get_user(x, p); // SINK\n}\n" },
    Row { sink: true, label: "Lua uci:set", path: "a.lua", lang: Language::Lua, source: "local uci = require('luci.model.uci').cursor()\nfunction h(inp)\n  local x = inp.v -- SEED\n  uci:set('network', 'lan', 'ipaddr', x) -- SINK\nend\n" },
    Row { sink: true, label: "Lua os.execute", path: "a.lua", lang: Language::Lua, source: "function h(inp)\n  local x = inp.v -- SEED\n  os.execute('ping ' .. x) -- SINK\nend\n" },
    Row { sink: true, label: "Lua loadstring", path: "a.lua", lang: Language::Lua, source: "function h(inp)\n  local x = inp.v -- SEED\n  loadstring(x) -- SINK\nend\n" },
    Row { sink: true, label: "Rust Command::new", path: "a.rs", lang: Language::Rust, source: "fn h(inp: &In) {\n    let x = inp.v.clone(); // SEED\n    std::process::Command::new(x).spawn(); // SINK\n}\n" },
    Row { sink: true, label: "Rust curl handle", path: "a.rs", lang: Language::Rust, source: "fn h(inp: &In) {\n    let x = inp.v.clone(); // SEED\n    fetch(curl, x); // SINK\n}\n" },
    Row { sink: true, label: "Bash eval", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  eval \"$x\" # SINK\n}\n" },
    Row { sink: true, label: "Bash uci", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  uci set network.lan.ipaddr=\"$x\" # SINK\n}\n" },
    Row { sink: true, label: "Bash sed", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  sed \"$x\" f # SINK\n}\n" },
    Row { sink: true, label: "Bash xargs", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  echo \"$x\" | xargs rm # SINK\n}\n" },
    Row { sink: true, label: "Bash sudo", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  sudo \"$x\" # SINK\n}\n" },
    Row { sink: true, label: "Bash chmod", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  chmod \"$x\" f # SINK\n}\n" },
    Row { sink: true, label: "Bash sqlite3", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  sqlite3 db \"select $x\" # SINK\n}\n" },
    Row { sink: true, label: "Bash curl", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  curl \"$x\" # SINK\n}\n" },
    Row { sink: true, label: "Bash wget", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  wget \"$x\" # SINK\n}\n" },
    Row { sink: true, label: "Bash ip", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  ip addr add \"$x\" dev eth0 # SINK\n}\n" },
    Row { sink: false, label: "Rust variable named source", path: "a.rs", lang: Language::Rust, source: "fn h(inp: &In) {\n    let x = inp.v.clone(); // SEED\n    let source = x; // SINK\n}\n" },
    Row { sink: false, label: "Rust variable named ip", path: "a.rs", lang: Language::Rust, source: "fn h(inp: &In) {\n    let x = inp.v.clone(); // SEED\n    let ip = x; // SINK\n}\n" },
    Row { sink: false, label: "C variable named find", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  char *find = x; // SINK\n}\n" },
    Row { sink: false, label: "Lua variable named sed", path: "a.lua", lang: Language::Lua, source: "function h(inp)\n  local x = inp.v -- SEED\n  local sed = x -- SINK\nend\n" },
    Row { sink: false, label: "Java variable named source", path: "A.java", lang: Language::Java, source: "class A {\n  void h(String in) {\n    String x = in; // SEED\n    String source = x; // SINK\n  }\n}\n" },
    Row { sink: false, label: "C get_user_id", path: "a.c", lang: Language::C, source: "int h(int in) {\n  int x = in; // SEED\n  return get_user_id(x); // SINK\n}\n" },
    Row { sink: false, label: "C get_user_pages (deliberate: not a sink)", path: "a.c", lang: Language::C, source: "long h(unsigned long a) {\n  unsigned long x = a; // SEED\n  get_user_pages(x, 1, 0, pages); // SINK\n}\n" },
    Row { sink: true, label: "probe: c control: system", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  system(x); // SINK\n}\n" },
    Row { sink: true, label: "probe: c sqlite3_prepare_v2", path: "a.c", lang: Language::C, source: "void h(sqlite3 *db, struct in *in) {\n  char *x = in->v; // SEED\n  sqlite3_prepare_v2(db, x, -1, &st, 0); // SINK\n}\n" },
    Row { sink: true, label: "probe: c chmod", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  chmod(x, 0777); // SINK\n}\n" },
    Row { sink: true, label: "probe: c chown", path: "a.c", lang: Language::C, source: "void h(struct in *in) {\n  char *x = in->v; // SEED\n  chown(x, 0, 0); // SINK\n}\n" },
    Row { sink: true, label: "probe: c libcurl setopt URL", path: "a.c", lang: Language::C, source: "void h(CURL *curl, struct in *in) {\n  char *x = in->v; // SEED\n  curl_easy_setopt(curl, CURLOPT_URL, x); // SINK\n}\n" },
    Row { sink: true, label: "probe: c get_user (kernel)", path: "a.c", lang: Language::C, source: "long h(int __user *p) {\n  int x = *p; // SEED\n  get_user(x, p); // SINK\n}\n" },
    Row { sink: false, label: "probe: c get_user_pages", path: "a.c", lang: Language::C, source: "long h(unsigned long a) {\n  unsigned long x = a; // SEED\n  get_user_pages(x, 1, 0, pages); // SINK\n}\n" },
    Row { sink: true, label: "probe: lua uci:set", path: "a.lua", lang: Language::Lua, source: "local uci = require('luci.model.uci').cursor()\nfunction h(inp)\n  local x = inp.v -- SEED\n  uci:set('network', 'lan', 'ipaddr', x) -- SINK\nend\n" },
    Row { sink: true, label: "probe: lua os.execute", path: "a.lua", lang: Language::Lua, source: "function h(inp)\n  local x = inp.v -- SEED\n  os.execute('ping ' .. x) -- SINK\nend\n" },
    Row { sink: true, label: "probe: rust Command::new", path: "a.rs", lang: Language::Rust, source: "fn h(inp: &In) {\n    let x = inp.v.clone(); // SEED\n    std::process::Command::new(x).spawn(); // SINK\n}\n" },
    Row { sink: false, label: "probe: java Runtime.exec", path: "A.java", lang: Language::Java, source: "class A {\n  void h(In inp) {\n    String x = inp.v; // SEED\n    Runtime.getRuntime().exec(x); // SINK\n  }\n}\n" },
    Row { sink: true, label: "probe: bash eval", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  eval \"$x\" # SINK\n}\n" },
    Row { sink: true, label: "probe: bash uci", path: "a.sh", lang: Language::Bash, source: "h() {\n  x=\"$1\" # SEED\n  uci set network.lan.ipaddr=\"$x\" # SINK\n}\n" },
];

#[test]
fn hybrid_other_matrix() {
    assert_rows(OTHER_ROWS);
}
