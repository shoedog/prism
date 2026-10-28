class Main {
  String text;

  void f(Object o) {
    text = source();
    if (o instanceof String text && guard(text)) {
      sink(text);
      sink(this.text);
    }
    sink(this.text);
  }

  String source() { return "field"; }
  boolean guard(Object value) { return true; }
  void sink(Object value) {}
}
