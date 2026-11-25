import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;

class App {
    String read() throws IOException {
        BufferedReader input = new BufferedReader(new InputStreamReader(System.in));
        try (input) {
            String label = input.readLine();
            return label;
        }
    }
}
