function labelRow(field, value = "Pending") {
  class Labelled {
    set label(text) { this.caption = text; }
  }
  class Row extends Labelled {
    apply(key) { super[key] = value; }
  }
  const row = new Row();
  row.apply(field);
  return row;
}

module.exports = labelRow;
