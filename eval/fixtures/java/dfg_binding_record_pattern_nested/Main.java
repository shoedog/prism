record Inner(int value) {}
record Outer(Inner inner) {}

class Main {
  int f(Object o) {
    int x = source();
    if (o instanceof Outer(Inner(var x))) {
      sink(x);
    }
    return x;
  }
}
