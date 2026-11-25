function length() { return 7; }

function external() {
  const mode = process.env.MODE;
  return mode;
}

function local(mode) {

  const show = () => mode;
  return show();
}
