import java.sql.ResultSet;
class App {
  static void run(ResultSet row) throws Exception {
    int count = row.getInt("count");
    boolean missing = row.wasNull();
    System.out.println(count + ":" + missing);
  }
}
