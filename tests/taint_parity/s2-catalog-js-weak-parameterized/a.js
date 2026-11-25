function entry(knex) {
  const t = acquire(); // @SRC
  return knex('items').whereRaw('item_id = ?', [t]).limit(1); // @SINK
}
