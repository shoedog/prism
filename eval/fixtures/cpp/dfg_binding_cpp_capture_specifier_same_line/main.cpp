void f() {
  int x = source();
  auto call = [x] { sink(x); };
  sink(x);
}
