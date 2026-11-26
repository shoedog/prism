class App {
  static String read(String arg, boolean choose) {
    String value;
    if (choose) value = System.getenv("MODE");
    else value = arg;
    return value;
  }
}
