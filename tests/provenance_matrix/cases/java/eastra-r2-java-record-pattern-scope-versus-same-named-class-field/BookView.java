class BookView {
    record Book(String edition) {}
    static final String edition = System.getenv("EDITION");
    static String label(Object item) {
        if (item instanceof Book(String edition)) {
            return edition;
        }
        return edition;
    }
}
