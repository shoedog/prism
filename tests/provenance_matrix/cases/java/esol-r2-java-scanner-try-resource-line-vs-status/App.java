import java.util.Scanner;
class App {
    void load() {
        try (Scanner terminal = new Scanner(System.in)) {
            if (!terminal.hasNextLine()) return;
            String line = terminal.nextLine();
            boolean more = terminal.hasNextLine();
            consume(line, more);
        }
    }
    void consume(String line, boolean more) {}
}
