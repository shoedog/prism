class Main {
  static class System {
    static String getenv(String key) { return "sample"; }
  }
  static String[] preview() {
    String mocked = System.getenv("EDITION");
    String actual = java.lang.System.getenv("EDITION");
    return new String[] { mocked, actual };
  }
}
