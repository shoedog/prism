class App {
    String label() {
        String selected = System.getenv("PREVIEW_LABEL");
        try {
            return selected;
        } finally {
            selected = "fallback";
        }
    }
}
