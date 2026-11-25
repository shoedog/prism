class App {
    String[] labels() {
        String property = System.getProperty("preview.label");
        String environment = System.getenv("PREVIEW_LABEL");
        return new String[] { property, environment };
    }
}
