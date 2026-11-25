class App {
    static String edition() {
        String current = "preview";
        assert (current = System.getenv("EDITION")) != null;
        return current;
    }
}
