void f(Values values) {
  int x = source();
  for (auto x : values) {
    sink(x);
  }
  sink(x);
}
