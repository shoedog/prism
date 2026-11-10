function case_assign(k) {
  let x = 1;
  switch (k) {
    case 0:
    case x = 5:
      use(x);
  }
}

function case_aug(k) {
  let x = 1;
  switch (k) {
    case 0:
    case x += 1:
      use(x);
  }
}

function case_logical(k) {
  let x = 1;
  switch (k) {
    case 0:
    case x ||= 5:
      use(x);
  }
}

function case_formal(k, x) {
  switch (k) {
    case 0:
    case x = 5:
      use(x);
  }
}

function swap_two(o) {
  o.x.y = 1;
  const keep = o.x;
  o.x = {};
  o.x.y = 2;
  o.x = keep;
  return o.x.y;
}

function swap_this(c) {
  this.a.b = 1;
  const keep = this.a;
  this.a = {};
  this.a.b = 2;
  this.a = keep;
  return this.a.b;
}

function swap_three(o) {
  o.a.b.c = 1;
  const keep = o.a.b;
  o.a.b = {};
  o.a.b.c = 2;
  o.a.b = keep;
  return o.a.b.c;
}

function single_field(o) {
  o.f = 1;
  o.f = 2;
  return o.f;
}

function this_field(c) {
  this.f = 1;
  this.f = 2;
  return this.f;
}

function eval_paren(a) {
  a = (eval)("a");
  return a;
}

function eval_ts_escaped(a) {
  a = (ev\u0061l)("a");
  return a;
}

function eval_control(a) {
  a = (0, eval)("a");
  return a;
}

function shorthand_args(o) {
  const old = o;
  const box = { arguments };
  o.x = 1;
  box.arguments[0] = {};
  o.x = 2;
  box.arguments[0] = old;
  return o.x;
}

function args_u(a) {
  a = arguments[0];
  return a;
}

function args_u_cb(cb) {
  log();
  cb = arguments[0] || noop;
  return cb();
}

function strict_args_u(a) {
  "use strict";
  a = arguments[0];
  return a;
}

function nonsimple_args_u(a, b = 2) {
  a = arguments[0];
  return a;
}

function strict_args_d(a) {
  "use strict";
  let x = 1;
  x = 2;
  use(arguments);
  return x;
}

function outer_with(o) {
  with (o) {
    return function f() {
      x = 1;
      x = o["x"];
      return x;
    };
  }
}

function escaped_read(a) {
  a = 1;
  a = 2; \u0062 = 0;
  return a; }

function stable_inner(c) {
  o.x = 1;
  o.x = 2;
  use(o.x);
  function g() {
    const o = {};
    return o;
  }
}

function stable_block(c) {
  {
    const o = {};
  }
  o.x = 1;
  o.x = 2;
  use(o.x);
}

function stable_own(c) {
  const o = {};
  o.x = 1;
  o.x = 2;
  use(o.x);
}

with (obj) {
  var wf = function () {
    x = 1;
    x = obj["x"];
    return x;
  };
}

function inherited_alias_outer(a) {
  var args = arguments;
  function inherited_alias(b) {
    a = 1;
    a = args[0];
    return a;
  }
  return inherited_alias(0);
}

function inherited_arrow_outer(a) {
  const inherited_arrow_strict = () => {
    "use strict";
    a = 1;
    a = arguments[0];
    return a;
  };
  const inherited_arrow_rest = (...rest) => {
    a = 1;
    a = arguments[0];
    return a;
  };
  return inherited_arrow_strict() + inherited_arrow_rest();
}

function computed_key_arguments(a) {
  log();
  a = 1;
  a = { [arguments[0]]() {} };
  return a;
}

function escaped_key_outer(a) {
  var args;
  ({ [(args = \u0061rguments)]() {} });
  function escaped_key_inner(b) {
    a = 1;
    a = args[0];
    return a;
  }
  return escaped_key_inner(0);
}

const program_arrow = () => {
  exports = 1;
  exports = arguments[0];
  return exports;
};
