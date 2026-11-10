function d_chain(c) {
  let x = c;
  x = 1;
  x = 2;
  return x;
}

function d_cond(c) {
  let x = 0;
  if (c) {
    x = 1;
  }
  return x;
}

function d_inner(c) {
  let x = 0;
  if (c) {
    x = 1;
    x = 2;
    use(x);
  }
  return x;
}

function d_loop(n) {
  let x = 0;
  for (let i = 0; i < n; i++) {
    use(x);
    x = i;
  }
  return x;
}

function d_switch(k) {
  let x = 0;
  switch (k) {
    case 1:
      x = 1;
    case 2:
      use(x);
      x = 2;
      use(x);
      break;
    default:
      x = 3;
      use(x);
  }
  return x;
}

function d_try(c) {
  let x = 0;
  try {
    x = 1;
    risky();
    x = 2;
  } catch (e) {
    use(x);
  } finally {
    x = 5;
    use(x);
  }
  return x;
}

function d_closure(c) {
  let x = 0;
  const f = () => {
    x = 9;
  };
  x = 2;
  f();
  return x;
}

function d_nested_use(c) {
  let x = 0;
  x = 1;
  const g = () => x;
  return g();
}

function d_param(p) {
  p = 5;
  return p;
}

function* d_gen(c) {
  let x = 0;
  x = 1;
  yield x;
  x = 2;
  yield x;
}

async function d_async(c) {
  let x = 0;
  x = await c;
  x = 3;
  return x;
}

function d_mid(c) {
  let x = 0;
  x = 1;
  use(x);
  x = 2;
  use(x);
}

function d_static_block() {
  class K {
    static {
      var s = 0;
      s = 1;
      use(s);
    }
  }
  return K;
}
