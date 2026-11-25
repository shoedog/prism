function run(items) {
  let value = "fixed";
  for (const item of items) {
    console.log(value);
    value = process.env.MODE;
  }
}
