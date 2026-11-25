import java.sql.*;
class App {
  static void read(ResultSet rows) throws SQLException {
    String name = rows.getString("name");
    ResultSetMetaData meta = rows.getMetaData();
    System.out.println(name + meta);
  }
}
