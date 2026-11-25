class App {
    String load(int profile) {
        String mode = switch (profile) {
            case 1 -> {
                yield System.getenv("MODE");
            }
            default -> "fixed";
        };
        return mode;
    }
}
