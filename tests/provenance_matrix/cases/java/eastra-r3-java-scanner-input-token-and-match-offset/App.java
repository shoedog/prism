import java.util.Scanner;
class App {
    static Object[] first() {
        Scanner scan = new Scanner(System.in);
        if (!scan.hasNext()) return new Object[0];
        String word = scan.next();
        int start = scan.match().start();
        return new Object[]{word, start};
    }
}
