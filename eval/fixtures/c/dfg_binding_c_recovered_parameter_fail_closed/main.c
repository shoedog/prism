#define container_of(a, b, c) (a)
void f(int *conn) {
    if (ready()) {
        sink(conn);
        use(container_of(conn, struct msg_client, conn), 0);
    }
}
