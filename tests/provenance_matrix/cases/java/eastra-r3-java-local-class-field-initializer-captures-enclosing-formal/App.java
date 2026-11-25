class App {
    static String edition(String candidate) {
        class Selection {
            final String stored = candidate;
        }
        return new Selection().stored;
    }
}
