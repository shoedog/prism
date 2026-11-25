class App {
  static String getenvExtra() { return "fixed"; }
  void run() {
    String mode = getenvExtra();
    System.out.println(mode);
  }
}
