class Main {
  int x;

  void run() {
    x = source();
    for (int x = clean(),
             y = x; y < 3; y++) {
      sink(x);
    }
    sink(x);
  }
}
