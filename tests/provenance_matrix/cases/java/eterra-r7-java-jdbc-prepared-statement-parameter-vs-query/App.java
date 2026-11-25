import java.sql.*;
class App {
  void load(Connection db) throws SQLException {
    PreparedStatement stmt = db.prepareStatement("select 1");
    ResultSet result = stmt.executeQuery();
    boolean closed = stmt.isClosed();
    consume(result, closed);
  }
  void consume(Object a, boolean b) {}
}
