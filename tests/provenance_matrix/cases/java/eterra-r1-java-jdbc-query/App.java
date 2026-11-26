import java.sql.*;
class App {
  void run(Statement stmt) throws SQLException {
    ResultSet rows = stmt.executeQuery("select 1");
    System.out.println(rows);
  }
}
