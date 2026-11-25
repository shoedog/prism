function load() {
  let mode = process.env.MODE;
  mode &&= 'fixed';
  return mode;
}
