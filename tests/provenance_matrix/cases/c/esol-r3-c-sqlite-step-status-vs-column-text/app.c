#include <stddef.h>
#include <sqlite3.h>

const unsigned char *title(sqlite3_stmt *statement, int *out_status) {
    int step = sqlite3_step(statement);
    const unsigned char *label = NULL;
    if (step == SQLITE_ROW) {
        label = sqlite3_column_text(statement, 0);
    }
    *out_status = step;
    return label;
}
