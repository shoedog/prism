function f(v, o) {
  let x = source();
  switch (v) { case 1: const {x} = o; sink(x); break; }
  sink(x);
}
