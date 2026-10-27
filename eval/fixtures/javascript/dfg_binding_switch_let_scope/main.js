function f(v, o) {
  let x = source();
  switch (v) { case 1: let x = clean(); sink(x); break; }
  sink(x);
}
