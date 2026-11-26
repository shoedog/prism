import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
class App {
    void load(Statement statement) throws SQLException {
        boolean hasRows = statement.execute("select label from catalog");
        if (!hasRows) return;
        try (ResultSet rows = statement.getResultSet()) {
            if (!rows.next()) return;
            String label = rows.getString("label");
            consume(hasRows, label);
        }
    }
    void consume(boolean hasRows, String label) {}
}
