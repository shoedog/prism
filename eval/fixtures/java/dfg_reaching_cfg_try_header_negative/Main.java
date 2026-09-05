class Main {
  void f() {
    int x = source();
    try {
      x = clean();
      throw new Failure();
    } catch (Failure error) {
      sink(x);
    }
  }
}
