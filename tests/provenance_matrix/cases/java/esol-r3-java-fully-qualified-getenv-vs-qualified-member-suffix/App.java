class App {
    static class System {
        static String getenvSnapshot(String key) {
            return key.strip();
        }
    }

    String[] labels() {
        String snapshot = System.getenvSnapshot("preview");
        String actual = java.lang.System.getenv("PREVIEW_LABEL");
        return new String[] { snapshot, actual };
    }
}
