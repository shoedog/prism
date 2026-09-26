record R(int value) {}

class Main {
  int f(Object o) {
    int x = source();
    if (o instanceof R(var x)) {
      sink(x);
    }
    return x;
  }
}
