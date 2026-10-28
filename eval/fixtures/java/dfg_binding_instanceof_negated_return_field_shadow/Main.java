class Main {
  String text;

  void f(Object o) {
    text = source();
    if (!(o instanceof String text)) {
      sink(text);
      return;
    }
    sink(text);
    sink(this.text);
  }

  String source() { return "field"; }
  void sink(Object value) {}
}
