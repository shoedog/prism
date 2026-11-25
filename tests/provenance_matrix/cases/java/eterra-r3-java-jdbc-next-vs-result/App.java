import java.sql.*;
class App {
  static void run(Statement stmt) throws SQLException {
    ResultSet rows = stmt.executeQuery("select 1");
    boolean available = rows.next();
    System.out.println(available);
  }
}
