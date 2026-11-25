class App {
    String load() {
        Object value = System.getenv("MODE");
        if (!(value instanceof String mode)) return "default";
        return mode;
    }
}
