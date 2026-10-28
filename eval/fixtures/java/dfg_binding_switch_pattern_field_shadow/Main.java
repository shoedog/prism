class Main {
  String text;
  void f(Object o) {
    text = source();
    switch (o) {
      case String text -> sink(text);
      default -> sink(text);
    }
    sink(text);
  }

  String source() { return "field"; }
  void sink(Object value) {}
}
