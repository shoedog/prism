import java.sql.*;
class BookStore {
    static long[] insert(Connection db) throws SQLException {
        try (PreparedStatement stmt = db.prepareStatement(
                "insert into books (title) values (?)", Statement.RETURN_GENERATED_KEYS)) {
            stmt.setString(1, "sample");
            int affected = stmt.executeUpdate();
            try (ResultSet keys = stmt.getGeneratedKeys()) {
                if (!keys.next()) return new long[0];
                long id = keys.getLong(1);
                return new long[]{id, affected};
            }
        }
    }
}
