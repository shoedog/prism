class Main {
  static void observe(String value, int size) {}
  public static void main(String[] supplied) {
    int count = supplied.length;
    for (String option : supplied) {
      observe(option, count);
    }
  }
}
