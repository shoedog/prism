import java.util.function.Supplier;
class Main {
  static void inspect(String key) {
    Supplier<String> deferred = () -> System.getenv(key);
    observe(deferred);
  }
  static void observe(Object value) {}
}
