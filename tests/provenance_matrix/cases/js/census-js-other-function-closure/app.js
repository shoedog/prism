function length() { return 7; }

function external() {
  const mode = process.argv[2];
  return mode;
}

function local() {
  const mode = length();
  const show = () => mode;
  return show();
}
