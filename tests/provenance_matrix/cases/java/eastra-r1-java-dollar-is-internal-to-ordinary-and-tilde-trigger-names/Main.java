class Main {
  static String config$query() { return "left"; }
  static String query$config() { return "right"; }
  static String[] labels() {
    String left = config$query();
    String right = query$config();
    String live = System.getenv("EDITION");
    return new String[] { left, right, live };
  }
}
