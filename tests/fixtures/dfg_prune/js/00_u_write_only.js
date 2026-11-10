function u_basic(c) {
  let x = c;
  x = 2;
  x += 1;
  return x;
}

function u_selfread(c) {
  let y = c;
  y = y + 1;
  return y;
}

function u_destruct(c) {
  let a = c;
  let b = 2;
  [a, b] = [b, a];
  return a + b;
}

function u_member(c, i) {
  let z = c;
  z[i] = 1;
  z.f = 2;
  return z;
}

function u_decl(c) {
  var w = c;
  var w = 3;
  return w;
}
