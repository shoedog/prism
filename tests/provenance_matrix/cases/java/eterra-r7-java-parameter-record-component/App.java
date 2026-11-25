record Config(String mode) {}
class App {
  String load(Config cfg) {
    return cfg.mode();
  }
}
