import java.io.IOException;
class BookReader {
    static byte[] read() throws IOException {
        byte[] pair = {42, 0};
        if (System.in.readNBytes(pair, 1, 1) != 1) return new byte[0];
        byte marker = pair[0], payload = pair[1];
        return new byte[]{marker, payload};
    }
}
