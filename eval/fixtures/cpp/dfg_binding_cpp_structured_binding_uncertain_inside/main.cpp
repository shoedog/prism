void f(Pair pair) {
  int x = source();
  {
    auto [x, y] = pair;
    sink(x);
  }
  sink(x);
}
