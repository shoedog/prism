function run() {
  let value = "fixed";
  value = process.argv[2]; value = process.env.MODE;
  console.log(value);
}
