function h_with(o) {
  let x = 0;
  x = 1;
  with (o) {
    x = 2;
  }
  return x;
}

function h_eval(src) {
  let x = 0;
  x = 1;
  eval(src);
  x = 2;
  return x;
}

function h_args(a) {
  a = 1;
  arguments[0] = 7;
  return a;
}

function h_annexb(c) {
  let x = 0;
  x = 1;
  {
    function x() {}
  }
  return x;
}

function h_var_redecl(c) {
  var x = 0;
  if (c) {
    var x = 1;
  }
  x = 2;
  return x;
}

function h_fn_same(c) {
  let r = 0;
  r = 1;
  return r;
  function r() {}
}

function h_label(c) {
  let x = 0;
  x = 1;
  out: {
    if (c) break out;
    x = 2;
  }
  return x;
}

function h_logical(c) {
  let x = 0;
  x ||= c;
  x = 3;
  return x;
}

function h_logical2(c) {
  let x = 0;
  x = 1;
  x ??= c;
  return x;
}

function h_nested_assign(c) {
  let x = 0;
  c || (x = 1);
  return x;
}

function h_comma(c) {
  let x = 0;
  x = 1, c();
  return x;
}

function h_catch(c) {
  let x = 0;
  try {
    c();
  } catch (x) {
    use(x);
  }
  return x;
}

function h_forof(xs) {
  let x = 0;
  for (x of xs) {
    use(x);
  }
  return x;
}

function h_incr(c) {
  let x = 0;
  x++;
  x = 5;
  return x;
}

function h_no_init(c) {
  let x = 0;
  x = 1;
  var x;
  return x;
}

function h_destruct_d(o) {
  let x = 0;
  ({ x } = o);
  return x;
}

function h_cond_expr(c) {
  let x = 0;
  c ? (x = 1) : 0;
  return x;
}

function h_tdz(c) {
  let x = 0;
  x = 1;
  {
    use(x);
    let x = 2;
  }
  return x;
}

function h_var_redecl2(c) {
  var x = 0;
  if (c) {
    var x = 1;
  }
  return x;
}

function h_incr2(c) {
  let x = 0;
  x = 1;
  x++;
  return x;
}
