import java.sql.*;
class App {
  void run(ResultSet result) throws SQLException {
    String name = result.getString(1);
    System.out.println(name);
  }
}
