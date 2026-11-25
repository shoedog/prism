import java.sql.*;
class App {
    static Object[] edition(Connection db) throws SQLException {
        try (Statement s = db.createStatement();
             ResultSet rs = s.executeQuery("SELECT title AS label FROM editions")) {
            if (!rs.next()) return new Object[0];
            int index = rs.findColumn("label");
            String text = rs.getString(index);
            return new Object[]{index, text};
        }
    }
}
