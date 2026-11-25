import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;

class App {
    ResultSet load(Statement stmt) throws SQLException {
        ResultSet rows;
        rows = stmt.executeQuery("select 1");
        return rows;
    }
}
