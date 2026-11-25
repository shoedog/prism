#include <sqlite3.h>
#include <stdio.h>
static int row(void *context, int columns, char **values, char **names) {
    if (columns != 1 || !values[0]) return 0;
    char *title = values[0];
    char *column = names[0];
    const char *label = context;
    printf("%s:%s:%s\n", title, column, label);
    return 0;
}
void show(sqlite3 *db) {
    sqlite3_exec(db, "select title as display from books", row, "preview", 0);
}
