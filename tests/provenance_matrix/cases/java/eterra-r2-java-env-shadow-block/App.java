class App {
  static void run() {
    String mode = System.getenv("MODE");
    { String other = "fixed"; System.out.println(other); }
    System.out.println(mode);
  }
}
