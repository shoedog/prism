import java.util.function.Function;
class App {
    void load() {
        Function<String, String> reader = System::getenv;
        String mode = System.getenv("MODE");
        consume(reader, mode);
    }
    void consume(Function<String, String> reader, String mode) {}
}
