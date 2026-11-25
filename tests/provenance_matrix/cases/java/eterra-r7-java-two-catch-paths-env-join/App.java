class App {
  String load(boolean fail) {
    String mode;
    try { if (fail) throw new IllegalStateException(); mode = System.getenv("MODE"); }
    catch (IllegalStateException e) { mode = "safe"; }
    return mode;
  }
}
