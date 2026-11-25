import java.sql.*;
class Main {
  static void preview(Connection connection) throws SQLException {
    try (CallableStatement proc = connection.prepareCall("{call get_edition(?)}")) {
      proc.registerOutParameter(1, Types.VARCHAR);
      boolean returnedRows = proc.execute();
      String edition = proc.getString(1);
      observe(edition, returnedRows);
    }
  }
  static void observe(Object... values) {}
}
