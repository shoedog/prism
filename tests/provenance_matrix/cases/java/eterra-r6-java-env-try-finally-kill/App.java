class App {
  static String read() {
    String mode = System.getenv("MODE");
    try { } finally { mode = "fixed"; }
    return mode;
  }
}
