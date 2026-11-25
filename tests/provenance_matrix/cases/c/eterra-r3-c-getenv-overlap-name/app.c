#include <stdlib.h>
void consume(const char *);
void run(void) {
  const char *query = getenv("QUERY");
  consume(query);
}
