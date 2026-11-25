import java.util.Map;
class App {
    String load() {
        Map<String, String> runtime = System.getenv();
        String mode = runtime.get("MODE");
        return mode;
    }
}
