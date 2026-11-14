// Fold 5: a `#` in a comment before the owner refuses the exemption (documented conservative
// cost of the allow-list).
// issue #1
int sink(int);
void *pbuf();
void *pother();

void n1_hash_comment_keep() {
    const T* p = pbuf();  // @n1x:D p
    p = pother();
    sink(*p + 1);  // @n1x:U:keep:hash_in_a_comment_before_the_owner_refuses_the_exemption
}
