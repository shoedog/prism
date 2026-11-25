class App {
    static class Box implements AutoCloseable {
        String edition = System.getenv("EDITION");
        public void close() { edition = "closed"; }
    }
    static String read() {
        Box box = new Box();
        try (box) {
            throw new IllegalStateException();
        } catch (IllegalStateException stopped) {
            String label = box.edition;
            return label;
        }
    }
}
