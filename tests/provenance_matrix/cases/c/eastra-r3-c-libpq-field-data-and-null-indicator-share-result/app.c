#include <libpq-fe.h>
void record(const char*, int);
void edition(PGconn *db) {
    PGresult *rows = PQexec(db, "SELECT title FROM editions");
    if (PQresultStatus(rows) == PGRES_TUPLES_OK && PQntuples(rows) > 0) {
        int absent = PQgetisnull(rows, 0, 0);
        if (!absent) {
            const char *label = PQgetvalue(rows, 0, 0);
            record(label, absent);
        }
    }
    PQclear(rows);
}
