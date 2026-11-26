class App {
  static void run(boolean repeat) {
    String mode = "default";
    do {
      System.out.println(mode);
      mode = System.getenv("MODE");
    } while (repeat);
  }
}
