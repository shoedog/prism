struct Item { int request; };
int read_value(void) {
    Item item = {7};
    return item.request;
}
