import java.util.Properties;
class App {
  static void run(Properties settings) {
    String mode = settings.getProperty("mode");
    System.out.println(mode);
  }
}
