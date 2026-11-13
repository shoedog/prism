// C++ U/D obligations (#630 slice 6c): a parse error in the owner turns it off.
int src();
void sink(int);

void error_owner_off() {
    int x = src();            // @er:D x
    x = 1;
    sink(x + 1);              // @er:U:keep:parse_error_in_the_owner
    int );
}

void clean_neighbour_drop() {
    int y = src();            // @en:D y
    y = 1;
    sink(y + 1);              // @en:U:drop:the_error_stays_in_its_owner
}
