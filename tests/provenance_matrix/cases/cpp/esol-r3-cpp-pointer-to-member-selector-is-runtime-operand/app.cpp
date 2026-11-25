struct Item {
    int priority;
    int group;
};

int read(const Item& item, int Item::* selector) {
    return item.*selector;
}
