import java.io.*;
import java.util.Properties;
class BookConfig {
    static Object[] read() throws IOException {
        Properties defaults = new Properties();
        try (var file = new FileReader("book.properties")) {
            defaults.load(file);
        }
        Properties overlay = new Properties(defaults);
        String inherited = overlay.getProperty("edition");
        Object direct = overlay.get("edition");
        return new Object[]{inherited, direct};
    }
}
