// C++ U/D obligations (#630 slice 6d-2s): structural pointer declarations are exempt from the
// nonscalar_decl escape when a keyword token proves them declarations; look-alikes keep.
int sink(int);
void *pbuf();
void *pother();

void drop_direct_const_lead() {
    const T* p = pbuf();  // @c1:D p
    p = pother();
    sink(*p + 1);  // @c1:U:drop:keyword_proves_declaration_direct_const_lead
}

void drop_direct_const_trail() {
    T const* p = pbuf();  // @c2:D p
    p = pother();
    sink(*p + 1);  // @c2:U:drop:keyword_proves_declaration_direct_const_trail
}

void drop_ptr_const() {
    T* const p = pbuf();  // @c3:D p
    p = pother();
    sink(*p + 1);  // @c3:U:drop:keyword_proves_declaration_ptr_const
}

void drop_const_both() {
    const T* const p = pbuf();  // @c4:D p
    p = pother();
    sink(*p + 1);  // @c4:U:drop:keyword_proves_declaration_const_both
}

void drop_const_ptr_ptr() {
    T * const * p = pbuf();  // @c5:D p
    p = pother();
    sink(*p + 1);  // @c5:U:drop:keyword_proves_declaration_const_ptr_ptr
}

void drop_elab_struct() {
    struct T* p = pbuf();  // @e1:D p
    p = pother();
    sink(*p + 1);  // @e1:U:drop:keyword_proves_declaration_elab_struct
}

void drop_elab_class() {
    class T* p = pbuf();  // @e2:D p
    p = pother();
    sink(*p + 1);  // @e2:U:drop:keyword_proves_declaration_elab_class
}

void drop_elab_union() {
    union T* p = pbuf();  // @e3:D p
    p = pother();
    sink(*p + 1);  // @e3:U:drop:keyword_proves_declaration_elab_union
}

void drop_elab_enum() {
    enum E* p = pbuf();  // @e4:D p
    p = pother();
    sink(*p + 1);  // @e4:U:drop:keyword_proves_declaration_elab_enum
}

void drop_auto_ptr() {
    auto* p = pbuf();  // @a1:D p
    p = pother();
    sink(*p + 1);  // @a1:U:drop:keyword_proves_declaration_auto_ptr
}

void drop_const_auto_ptr() {
    const auto* p = pbuf();  // @a2:D p
    p = pother();
    sink(*p + 1);  // @a2:U:drop:keyword_proves_declaration_const_auto_ptr
}

void drop_two_declarators() {
    const T *p = pbuf(), *q = pbuf();  // @m1:D p
    p = pother();
    sink(*p + 1);  // @m1:U:drop:keyword_proves_declaration_two_declarators
}

void drop_const_ptr_ptr2() {
    const T** p = pbuf();  // @c6:D p
    p = pother();
    sink(*p + 1);  // @c6:U:drop:keyword_proves_declaration_const_ptr_ptr2
}

void keep_plain_class_ptr() {
    T* p = pbuf();  // @k1:D p
    p = pother();
    sink(*p + 1);  // @k1:U:keep:fold5_plain_pointer_to_class_may_be_misparsed
}

void keep_attribute() {
    [[maybe_unused]] T* p = pbuf();  // @k2:D p
    p = pother();
    sink(*p + 1);  // @k2:U:keep:attributed_expression_statement_is_legal
}

void keep_qualified_base() {
    ns::T* p = pbuf();  // @k3:D p
    p = pother();
    sink(*p + 1);  // @k3:U:keep:qualified_base_not_exempt
}

void keep_qualified_const_base() {
    const ns::T* p = pbuf();  // @k4:D p
    p = pother();
    sink(*p + 1);  // @k4:U:keep:qualified_base_not_exempt_even_with_const
}

void keep_template_const_arg() {
    ga<const T>* p = pbuf();  // @k5:D p
    p = pother();
    sink(*p + 1);  // @k5:U:keep:const_inside_template_argument_proves_nothing
}

void keep_mixed_declarators() {
    T* p = pbuf(), *const q = pbuf();  // @k6:D p
    p = pother();
    sink(*p + 1);  // @k6:U:keep:a_declarator_without_const_so_no_declaration_level_proof
}

void keep_volatile_ptr() {
    T* volatile p = pbuf();  // @k7:D p
    p = pother();
    sink(*p + 1);  // @k7:U:keep:volatile_is_not_on_the_list
}

void keep_paren_declarator() {
    const T* (p) = pbuf();  // @k8:D p
    p = pother();
    sink(*p + 1);  // @k8:U:keep:parenthesized_declarator_not_exempt
}

void keep_decltype_base() {
    const decltype(a)* p = pbuf();  // @k9:D p
    p = pother();
    sink(*p + 1);  // @k9:U:keep:decltype_base_not_exempt
}

void keep_auto_plain() {
    auto p = pbuf();  // @k10:D p
    p = pother();
    sink(*p + 1);  // @k10:U:keep:auto_without_pointer_not_exempt
}

void keep_const_nonpointer() {
    const T p = pbuf();  // @k11:D p
    p = pother();
    sink(*p + 1);  // @k11:U:keep:non_pointer_not_exempt_class_typed_fails_E2
}

void keep_const_ref() {
    const T& p = pbuf();  // @k13:D p
    p = pother();
    sink(*p + 1);  // @k13:U:keep:reference_declarator_not_exempt
}

void keep_struct_body() {
    struct T { int a; }* p = pbuf();  // @k14:D p
    p = pother();
    sink(*p + 1);  // @k14:U:keep:elaborated_with_body_not_exempt
}

void keep_struct_qualified() {
    struct ns::T* p = pbuf();  // @k15:D p
    p = pother();
    sink(*p + 1);  // @k15:U:keep:qualified_elaborated_name_not_exempt
}

void keep_typename_base() {
    typename T::U const* p = pbuf();  // @k16:D p
    p = pother();
    sink(*p + 1);  // @k16:U:keep:dependent_base_not_exempt
}
